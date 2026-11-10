//! Local effect execution: foreground generation, produced-directory
//! verification, one native detached creation and the terminal action. No
//! effect is retried and no CLI fallback follows a dispatched native create.
#[cfg(feature = "native")]
use std::io::Write;
use std::path::{Path, PathBuf};

use tta_command::Handoff;
use tta_launcher::entry::protocol::{BoundTarget, EffectFailure, EffectKind, EffectReceipt, EffectRequest, MutationKnowledge};

use super::{Executor, failure};

/// Typed native effect failures; the attempt keeps the original cause and
/// its honest mutation knowledge.
#[derive(Debug, thiserror::Error)]
pub enum EffectError {
    #[error("generator program exited with {code:?}")]
    Generate { code: Option<i32> },
    #[error("expected produced directory {path:?}: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("native session creation failed; it may already have taken effect: {0}")]
    #[cfg(feature = "native")]
    Native(#[from] tmax::error::OperationError),
    #[error("native session creation was acknowledged, but its output could not be forwarded: {0}")]
    Output(#[from] std::io::Error),
    #[error("terminal block exited with {code:?}")]
    Terminal { code: Option<i32> },
    #[error(transparent)]
    Command(#[from] crate::command::Error),
}

/// Executes effects on the invoking host. CreateNative requires the `native`
/// feature; without it, constructing effects for that shape is the attempt's
/// pure eligibility decision, never this executor's fallback.
///
/// `program_environment` carries extra assignments for every program-bearing
/// expression effect (generation and terminal blocks, which embed generator
/// programs); hosts use it to isolate PATH, for example. It never reaches
/// native creation, which is not a program spawn.
#[derive(Default)]
pub struct Local {
    pub program_environment: Vec<tta_command::Assignment>,
}

impl Local {
    fn program(&mut self, program: &tta_command::Expression) -> tta_command::Expression {
        let mut program = program.clone();
        if !self.program_environment.is_empty() {
            match &mut program {
                tta_command::Expression::Command(command) => {
                    command.environment.extend(self.program_environment.iter().cloned());
                }
                tta_command::Expression::And(and) => {
                    for step in &mut and.steps {
                        if let tta_command::Expression::Command(command) = step {
                            command.environment.extend(self.program_environment.iter().cloned());
                        }
                    }
                }
                tta_command::Expression::ChangeDir(_) | tta_command::Expression::Write(_) => {}
            }
        }
        program
    }

    fn generate(&mut self, program: &tta_command::Expression) -> std::result::Result<EffectReceipt, EffectFailure> {
        let program = self.program(program);
        // Foreground generation: a nonzero status or spawn failure may have
        // left residue; later effects never run.
        match crate::command::execute(&program, Handoff::Run) {
            Ok(Some(status)) if status.success() => Ok(EffectReceipt::Generated),
            Ok(Some(status)) => Err(failure(EffectError::Generate { code: status.code() }, MutationKnowledge::MayHaveOccurred)),
            Ok(None) => Ok(EffectReceipt::Generated),
            Err(source) => Err(failure(EffectError::Command(source), MutationKnowledge::MayHaveOccurred)),
        }
    }
}

impl Executor for Local {
    fn perform(&mut self, effect: &EffectRequest, target: &BoundTarget, handoff: Handoff) -> std::result::Result<EffectReceipt, EffectFailure> {
        match &effect.kind {
            EffectKind::Generate { program } => self.generate(program),
            // Post-generation observation: no mutation was attempted.
            EffectKind::CheckCreatedDirectory { path } => check_directory(path),
            EffectKind::CreateNative { invocation, directory } => create_native(invocation, directory, target),
            // A returned aggregate status is never proof of each internal
            // short-circuit step; exec failure after partial runs stays
            // uncertain. Successful exec never returns.
            EffectKind::Terminal { expression } => match crate::command::execute(&self.program(expression), handoff) {
                Ok(Some(status)) if status.success() => Ok(EffectReceipt::Terminal { code: status.code() }),
                Ok(Some(status)) => Err(failure(EffectError::Terminal { code: status.code() }, MutationKnowledge::MayHaveOccurred)),
                // Only directory changes/file writes ran; no client status.
                Ok(None) => Ok(EffectReceipt::Terminal { code: None }),
                Err(source) => Err(failure(EffectError::Command(source), MutationKnowledge::MayHaveOccurred)),
            },
        }
    }
}

fn check_directory(path: &Path) -> std::result::Result<EffectReceipt, EffectFailure> {
    match path.canonicalize() {
        Ok(canonical) if canonical.is_dir() => Ok(EffectReceipt::Directory),
        Ok(_) => Err(failure(
            EffectError::Directory {
                path: path.to_owned(),
                source: std::io::ErrorKind::NotADirectory.into(),
            },
            MutationKnowledge::NotStarted,
        )),
        Err(source) => Err(failure(EffectError::Directory { path: path.to_owned(), source }, MutationKnowledge::NotStarted)),
    }
}

#[cfg(feature = "native")]
fn create_native(invocation: &tmax::operation::Invocation, directory: &Path, target: &BoundTarget) -> std::result::Result<EffectReceipt, EffectFailure> {
    tracing::info!(event = "launch.native", operation = "new-session-detached", socket = ?target.socket().path);
    let endpoint = tmax::endpoint::Tmux::new().socket_path(&target.socket().path);
    // One mutation attempt: a failure here may already have applied and is
    // never retried or answered with a CLI fallback.
    let outcome = endpoint
        .client_directory(directory)
        .run(invocation.clone())
        .map_err(|source| failure(EffectError::Native(source), MutationKnowledge::MayHaveOccurred))?;
    // The creation IS acknowledged; a forwarding failure after it records
    // that fact instead of claiming the launch merely failed.
    let forward = || -> std::io::Result<()> {
        let mut stdout = std::io::stdout().lock();
        for line in &outcome.body {
            writeln!(stdout, "{line}")?;
        }
        stdout.flush()?;
        std::io::stderr().write_all(&outcome.stderr)
    };
    forward().map_err(|source| failure(EffectError::Output(source), MutationKnowledge::Acknowledged))?;
    Ok(EffectReceipt::Created)
}

#[cfg(not(feature = "native"))]
fn create_native(_invocation: &tmax::operation::Invocation, _directory: &Path, _target: &BoundTarget) -> std::result::Result<EffectReceipt, EffectFailure> {
    Err(failure(
        EffectError::Command(crate::command::Error::Execute {
            program: "tmux".into(),
            source: std::io::Error::new(std::io::ErrorKind::Unsupported, "native creation requires the native feature"),
        }),
        MutationKnowledge::NotStarted,
    ))
}
