//! Shell-free interpreter for the generic command language.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{Command as Process, ExitStatus};

use tta_command::{Command, Expression, Handoff, Word};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Build(#[from] tta_command::Error),
    #[error("cannot enter directory {path:?}: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot write file {path:?}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot execute {program:?}: {source}")]
    Execute {
        program: OsString,
        #[source]
        source: io::Error,
    },
}

/// Convert literal command data to a process; no shell parsing.
pub fn process(command: &Command) -> Process {
    let mut process = Process::new(command.program.as_os_str());
    process.args(command.args.iter().map(Word::as_os_str));
    process.envs(command.environment.iter().map(|assignment| (assignment.name(), assignment.value().as_os_str())));
    process
}

/// Convenience extension, explicitly imported from the optional runtime.
pub trait ProcessCommand {
    fn process(&self) -> Process;
}

impl ProcessCommand for Command {
    fn process(&self) -> Process {
        process(self)
    }
}

pub trait Execute {
    fn execute(&self, handoff: Handoff) -> Result<Option<ExitStatus>, Error>;
}

impl Execute for Expression {
    fn execute(&self, handoff: Handoff) -> Result<Option<ExitStatus>, Error> {
        execute(self, handoff)
    }
}

/// Interpret an expression without changing the Rust process's cwd. `None`
/// means an empty expression or only successful directory changes/file writes.
pub fn execute(expression: &Expression, handoff: Handoff) -> Result<Option<ExitStatus>, Error> {
    let steps = expression.steps();
    let last = steps.len().checked_sub(1);
    let mut cwd: Option<PathBuf> = None;
    let mut status = None;
    for (index, step) in steps.into_iter().enumerate() {
        match step {
            Expression::ChangeDir(cd) => {
                let path = cwd
                    .as_ref()
                    .map_or_else(|| PathBuf::from(cd.path.as_os_str()), |cwd| cwd.join(cd.path.as_os_str()));
                let canonical = path.canonicalize().map_err(|source| Error::Directory { path: path.clone(), source })?;
                if !canonical.is_dir() {
                    return Err(Error::Directory {
                        path,
                        source: io::ErrorKind::NotADirectory.into(),
                    });
                }
                cwd = Some(canonical);
                status = None;
            }
            Expression::Write(write) => {
                let path = cwd
                    .as_ref()
                    .map_or_else(|| PathBuf::from(write.path.as_os_str()), |cwd| cwd.join(write.path.as_os_str()));
                std::fs::write(&path, write.contents.as_os_str().as_encoded_bytes()).map_err(|source| Error::Write { path, source })?;
                status = None;
            }
            Expression::Command(command) => {
                let mut process = process(command);
                if let Some(cwd) = &cwd {
                    process.current_dir(cwd);
                }
                #[cfg(unix)]
                if handoff == Handoff::Exec && Some(index) == last {
                    use std::os::unix::process::CommandExt;
                    return Err(Error::Execute {
                        program: command.program.as_os_str().to_owned(),
                        source: process.exec(),
                    });
                }
                #[cfg(not(unix))]
                let _ = (handoff, index, last);
                let result = process.status().map_err(|source| Error::Execute {
                    program: command.program.as_os_str().to_owned(),
                    source,
                })?;
                if !result.success() {
                    return Ok(Some(result));
                }
                status = Some(result);
            }
            Expression::And(_) => unreachable!("steps are flattened"),
        }
    }
    Ok(status)
}
