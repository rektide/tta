//! Read-only non-PTY acquisition, local decisions, then a single prepared SSH
//! terminal action. No target tta, automatic mutation replay, or remote worker.
use std::ffi::OsString;
use std::io::Read;

use anyhow::{Context, Result, ensure};
use bon::Builder;
use tta_command::{Command, Expression, Handoff, Word};
use tta_launcher::{probe, target, workflow};

use crate::capture::{self, Limits};

#[derive(Clone, Debug, Builder)]
pub struct Ssh {
    pub destination: String,
    #[builder(default = "ssh".into())]
    pub program: OsString,
    /// Literal OpenSSH arguments, shared by probes and terminal handoff. The
    /// caller owns connection/config/ControlMaster policy; no second option set.
    #[builder(default)]
    pub options: Vec<OsString>,
}

impl Ssh {
    pub fn command(&self, remote: &str, terminal: bool) -> Result<Command> {
        ensure!(
            !self.destination.is_empty() && !self.destination.starts_with('-'),
            "SSH destination is empty or begins with '-'"
        );
        let mut args = self
            .options
            .iter()
            .map(|arg| Word::new(arg.clone()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        args.push(Word::new(if terminal { "-tt" } else { "-T" })?);
        args.push(Word::new(&self.destination)?);
        // SSH concatenates its command arguments: send exactly one shell line.
        args.push(Word::new(remote)?);
        Ok(Command::builder().program(Word::new(self.program.clone())?).args(args).build())
    }
}

#[derive(Debug)]
pub struct Record {
    pub request: probe::Request,
    pub output: capture::Output,
    pub decoded: std::result::Result<probe::Decoded, String>,
}

/// Write retained probe evidence to a caller-owned diagnostic stream, never to
/// protocol/plan stdout. Failed records include raw bounded bytes because a
/// rejected frame cannot safely be treated as decoded diagnostics.
pub fn write_diagnostics(mut writer: impl std::io::Write, records: &[Record]) -> std::io::Result<()> {
    for record in records {
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
                writer.write_all(&record.output.stderr)?;
                writeln!(
                    writer,
                    "SSH probe failed: {:?}; decode={error}; capture={:?}; status={:?}; complete={}; stdout_eof={}; stderr_eof={}; omitted_stdout={}; omitted_stderr={}; cleanup={:?}; stdout={:?}",
                    record.request,
                    record.output.stop,
                    record.output.status,
                    record.output.complete(),
                    record.output.stdout_eof,
                    record.output.stderr_eof,
                    record.output.omitted_stdout,
                    record.output.omitted_stderr,
                    record.output.cleanup,
                    String::from_utf8_lossy(&record.output.stdout),
                )?;
            }
        }
    }
    Ok(())
}

/// On *every* observation failure retain both streams, process status,
/// completeness/cleanup and any successful earlier facts. No mutation occurred.
#[derive(Debug, thiserror::Error)]
#[error("SSH planning stopped before mutation: {source:#}")]
pub struct Error {
    #[source]
    pub source: anyhow::Error,
    pub observations: Vec<Record>,
}

#[derive(Debug)]
pub struct PreparedRemote {
    pub ssh: Ssh,
    pub setup: target::TargetSetup,
    pub facts: target::Facts,
    pub launch: workflow::PreparedLaunch,
    pub observations: Vec<Record>,
}

impl PreparedRemote {
    /// The only effectful terminal command. Failure/transport loss after this
    /// starts may have mutated the target. Its owner must not blindly replay it.
    pub fn command(&self) -> std::result::Result<Command, crate::driver::Error> {
        let run = || self.ssh.command(&self.setup.launch(&self.launch, Handoff::Exec)?, true);
        run().map_err(Into::into)
    }

    /// Resolved LOCAL SSH invocation, not a target-only block or deferred
    /// discovery workflow. Run vs Exec governs the evaluating *local* shell.
    pub fn shell(&self, handoff: Handoff) -> Result<String> {
        Ok(Expression::from(self.command()?).shell(handoff)?)
    }
}

pub struct Observer {
    pub ssh: Ssh,
    pub setup: target::TargetSetup,
    pub limits: Limits,
    pub observations: Vec<Record>,
}

impl Observer {
    pub fn observe(&mut self, request: probe::Request) -> Result<probe::Reply> {
        let mut nonce = [0; 16];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
        let token = nonce.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let probe = probe::Probe::new(request.clone(), token)?;
        let command = self.ssh.command(&probe.shell(&self.setup)?, false)?;
        let output = capture::run(&mut crate::command::process(&command), self.limits).context("spawn/capture SSH probe")?;
        let decoded = probe.decode(probe::Output {
            stdout: &output.stdout,
            stderr: &output.stderr,
            status: output.status.and_then(|status| status.code()),
            complete: output.complete(),
        });
        let reply = decoded
            .as_ref()
            .map(|decoded| decoded.reply.clone())
            .map_err(|error| anyhow::anyhow!("{error:#}"));
        self.observations.push(Record {
            request,
            output,
            decoded: decoded.map_err(|error| format!("{error:#}")),
        });
        reply
    }
}

impl crate::driver::Observer for Observer {
    fn observe(&mut self, request: &workflow::ObservationRequest) -> Result<workflow::ObservationReply> {
        match self.observe(probe::Request::Workflow(request.clone()))? {
            probe::Reply::Workflow(reply) => Ok(reply),
            _ => unreachable!("probe decoder is request-bound"),
        }
    }
}

/// Prepare without mutation. This includes inspectable context/registry setup,
/// typed framed observations, and the same pure workflow as the local runner.
pub fn prepare(ssh: Ssh, setup: target::TargetSetup, options: target::Options, limits: Limits) -> std::result::Result<PreparedRemote, Error> {
    let mut observer = Observer {
        ssh,
        setup,
        limits,
        observations: Vec::new(),
    };
    let run = |observer: &mut Observer| -> Result<(target::Facts, workflow::PreparedLaunch)> {
        let mut bootstrap = target::Bootstrap::new(options);
        let input = loop {
            match bootstrap.next()? {
                target::Progress::Ready(input) => break *input,
                target::Progress::Observe(request) => {
                    let reply = match observer.observe(probe::Request::Setup(request))? {
                        probe::Reply::Setup(reply) => reply,
                        _ => unreachable!("request-bound decoder"),
                    };
                    bootstrap.supply(reply)?;
                    if let Some(facts) = bootstrap.facts() {
                        // Freeze target invocation cwd, including shell startup
                        // chdir, for all subsequent probes and final action.
                        observer.setup.cwd = Some(facts.cwd.clone());
                    }
                }
            }
        };
        let facts = bootstrap.facts().expect("ready bootstrap").clone();
        let launch = crate::driver::drive(workflow::Workflow::new(input)?, observer)?;
        Ok((facts, launch))
    };
    match run(&mut observer) {
        Ok((facts, launch)) => Ok(PreparedRemote {
            ssh: observer.ssh,
            setup: observer.setup,
            facts,
            launch,
            observations: observer.observations,
        }),
        Err(source) => Err(Error {
            source,
            observations: observer.observations,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_identical_except_explicit_terminal_mode() {
        let ssh = Ssh::builder()
            .destination("host".into())
            .options(vec!["-F".into(), "/private/config".into(), "-o".into(), "ControlMaster=auto".into()])
            .build();
        let probe = ssh.command("literal ' source", false).unwrap();
        let terminal = ssh.command("literal ' source", true).unwrap();
        assert_eq!(&probe.args[..4], &terminal.args[..4]);
        assert_eq!(probe.args[4].as_os_str(), "-T");
        assert_eq!(terminal.args[4].as_os_str(), "-tt");
        assert_eq!(&probe.args[5..], &terminal.args[5..]);
        let invalid = Ssh::builder().destination("-oProxyCommand=bad".into()).build();
        assert!(invalid.command(":", false).is_err());
    }
}
