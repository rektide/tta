//! Frozen worker-free SSH interpretation of the SAME semantic read requests.
use std::io::Read;

use anyhow::{Context, Result, ensure};
use tta_command::{Command, Expression, Handoff};
use tta_launcher::entry::{context, probe, protocol::*, registry};

use super::{Guard, Observer};

#[derive(Debug)]
pub struct Record {
    pub request: ReadRequest,
    pub step: Option<registry::Step>,
    pub output: Option<crate::capture::Output>,
    pub decoded: std::result::Result<probe::Decoded, String>,
}

pub struct Ssh {
    ssh: crate::ssh::Ssh,
    guard: Guard,
    limits: crate::capture::Limits,
    pub observations: Vec<Record>,
}

impl Ssh {
    pub fn acquire(ssh: crate::ssh::Ssh, scope: context::Scope, limits: crate::capture::Limits) -> Result<Self> {
        Self::check_origin(&ssh, &scope)?;
        Ok(Self {
            ssh,
            guard: Guard::new(scope)?,
            limits,
            observations: Vec::new(),
        })
    }
    pub fn supplied(ssh: crate::ssh::Ssh, target: &BoundTarget, limits: crate::capture::Limits) -> Result<Self> {
        Self::check_origin(&ssh, target.scope())?;
        Ok(Self {
            ssh,
            guard: Guard::supplied(target),
            limits,
            observations: Vec::new(),
        })
    }
    fn check_origin(ssh: &crate::ssh::Ssh, scope: &context::Scope) -> Result<()> {
        let context::Origin::Ssh { host, .. } = &scope.origin else {
            anyhow::bail!("SSH interpreter requires SSH context")
        };
        ensure!(host == &ssh.destination, "SSH destination differs from request context");
        Ok(())
    }
    fn run_probe(&mut self, request: &ReadRequest, step: Option<registry::Step>) -> Result<probe::Reply> {
        // Runtime owns nonce generation. Pure directives/codecs have no RNG or I/O.
        let mut nonce = [0; 16];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
        let token = nonce.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let probe = match &step {
            Some(step) => probe::Probe::registry(request.clone(), step.clone(), token)?,
            None => probe::Probe::new(request.clone(), token)?,
        };
        let command = self.ssh.command(&probe.shell()?, false)?;
        let captured = crate::capture::run(&mut crate::command::process(&command), self.limits).context("spawn/capture SSH read probe");
        match captured {
            Err(error) => {
                self.observations.push(Record {
                    request: request.clone(),
                    step,
                    output: None,
                    decoded: Err(format!("{error:#}")),
                });
                Err(error)
            }
            Ok(output) => {
                let decoded = probe.decode(tta_launcher::probe::Output {
                    stdout: &output.stdout,
                    stderr: &output.stderr,
                    status: output.status.and_then(|s| s.code()),
                    complete: output.complete(),
                });
                match decoded {
                    Ok(decoded) => {
                        let reply = decoded.reply.clone();
                        self.observations.push(Record {
                            request: request.clone(),
                            step,
                            output: Some(output),
                            decoded: Ok(decoded),
                        });
                        Ok(reply)
                    }
                    Err(error) => {
                        self.observations.push(Record {
                            request: request.clone(),
                            step,
                            output: Some(output),
                            decoded: Err(format!("{error:#}")),
                        });
                        Err(error)
                    }
                }
            }
        }
    }
    /// Terminal owners must use this same frozen connection/setup.
    /// Constructing the command is read-only and does not claim dispatch success.
    pub fn terminal_command(&self, target: &BoundTarget, expression: &Expression) -> Result<Command> {
        self.guard.check_target(target)?;
        let context::Origin::Ssh { setup, .. } = target.scope().origin.clone() else {
            unreachable!()
        };
        self.ssh.command(&setup.shell_command(&expression.shell(Handoff::Exec)?)?, true)
    }
}

impl Observer for Ssh {
    fn observe(&mut self, request: &ReadRequest) -> Result<Observation> {
        self.guard.check(request)?;
        let observation = if let ReadKind::Registry { path } = &request.kind {
            let mut task = registry::Task::new(path.clone())?;
            while let Some(step) = task.step().cloned() {
                let probe::Reply::Registry(reply) = self.run_probe(request, Some(step.clone()))? else {
                    unreachable!("request-bound decoder")
                };
                task.accept(&step, reply)?;
            }
            Observation::Registry(task.result().expect("completed registry acquisition").clone())
        } else {
            let probe::Reply::Observation(observation) = self.run_probe(request, None)? else {
                unreachable!("request-bound decoder")
            };
            observation
        };
        self.guard.accept(&observation)?;
        Ok(observation)
    }
}

pub fn write_diagnostics(mut writer: impl std::io::Write, observations: &[Record]) -> std::io::Result<()> {
    for record in observations {
        match &record.decoded {
            Ok(decoded) => {
                for bytes in [
                    &decoded.diagnostics.startup_stdout,
                    &decoded.diagnostics.trailing_stdout,
                    &decoded.diagnostics.transport_stderr,
                    &decoded.diagnostics.command_stderr,
                ] {
                    writer.write_all(bytes)?;
                }
            }
            Err(error) => {
                if let Some(output) = &record.output {
                    writer.write_all(&output.stderr)?;
                    writeln!(
                        writer,
                        "SSH read failed: {:?}; step={:?}; decode={error}; capture={:?}; status={:?}; complete={}; stdout_eof={}; stderr_eof={}; omitted_stdout={}; omitted_stderr={}; cleanup={:?}; stdout={:?}",
                        record.request,
                        record.step,
                        output.stop,
                        output.status,
                        output.complete(),
                        output.stdout_eof,
                        output.stderr_eof,
                        output.omitted_stdout,
                        output.omitted_stderr,
                        output.cleanup,
                        String::from_utf8_lossy(&output.stdout)
                    )?;
                } else {
                    writeln!(writer, "SSH read spawn failed: {:?}; step={:?}; {error}", record.request, record.step)?;
                }
            }
        }
    }
    Ok(())
}
