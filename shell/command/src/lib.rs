//! Small, shell-independent expressions. Arguments are values, never shell source.
#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitStatus};

use bon::Builder;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("command operands cannot contain NUL")]
    Nul,
    #[error("POSIX shell output requires UTF-8 operands: {0:?}")]
    NonUtf8(OsString),
    #[error("cannot enter directory {path:?}: {source}")]
    Directory {
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

/// One unquoted argv operand, validated once at construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Word(OsString);

impl Word {
    pub fn new(value: impl Into<OsString>) -> Result<Self, Error> {
        let value = value.into();
        if value.as_encoded_bytes().contains(&0) {
            return Err(Error::Nul);
        }
        Ok(Self(value))
    }

    pub fn as_os_str(&self) -> &OsStr {
        &self.0
    }

    pub fn shell(&self) -> Result<String, Error> {
        let value = self.0.to_str().ok_or_else(|| Error::NonUtf8(self.0.clone()))?;
        // Always quote, including program names (which could be shell keywords).
        Ok(format!("'{}'", value.replace('\'', "'\\''")))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Builder)]
pub struct Command {
    pub program: Word,
    #[builder(default)]
    pub args: Vec<Word>,
}

impl Command {
    pub fn process(&self) -> Process {
        let mut command = Process::new(self.program.as_os_str());
        command.args(self.args.iter().map(Word::as_os_str));
        command
    }

    fn shell(&self, handoff: bool) -> Result<String, Error> {
        let mut words = Vec::with_capacity(self.args.len() + 2);
        if handoff {
            words.push("exec".to_owned());
        }
        words.push(self.program.shell()?);
        for arg in &self.args {
            words.push(arg.shell()?);
        }
        Ok(words.join(" "))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Builder)]
pub struct ChangeDir {
    pub path: Word,
}

#[derive(Clone, Debug, Eq, PartialEq, Builder)]
pub struct And {
    pub steps: Vec<Expression>,
}

/// `And` short-circuits on a nonzero process status. Directory changes affect
/// subsequent commands, not this Rust process or its caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Command(Command),
    ChangeDir(ChangeDir),
    And(And),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Handoff {
    /// Run child processes, or emit ordinary commands safe for `eval`.
    #[default]
    Run,
    /// Replace the executing process with the final command (Unix only).
    Exec,
}

impl Expression {
    pub fn shell(&self, handoff: Handoff) -> Result<String, Error> {
        let mut steps = Vec::new();
        self.flatten(&mut steps);
        let last = steps.len().checked_sub(1);
        let mut rendered = Vec::with_capacity(steps.len());
        for (index, step) in steps.into_iter().enumerate() {
            rendered.push(match step {
                Self::Command(command) => command.shell(handoff == Handoff::Exec && Some(index) == last)?,
                Self::ChangeDir(cd) => {
                    let path = Path::new(cd.path.as_os_str());
                    // ./ avoids option parsing and CDPATH for relative operands.
                    let path = if path.is_absolute() { path.to_owned() } else { Path::new(".").join(path) };
                    format!("cd -P {}", Word::new(path.into_os_string())?.shell()?)
                }
                Self::And(_) => unreachable!("flattened expressions contain no And"),
            });
        }
        if rendered.is_empty() {
            return Ok(":".to_owned());
        }
        Ok(rendered.join(" && "))
    }

    /// Execute the same argv/cwd expression without invoking a shell. `None`
    /// means an empty expression or only successful directory changes.
    pub fn execute(&self, handoff: Handoff) -> Result<Option<ExitStatus>, Error> {
        let mut steps = Vec::new();
        self.flatten(&mut steps);
        let last = steps.len().checked_sub(1);
        let mut cwd: Option<PathBuf> = None;
        let mut status = None;
        for (index, step) in steps.into_iter().enumerate() {
            match step {
                Self::ChangeDir(cd) => {
                    let path = match &cwd {
                        Some(cwd) => cwd.join(cd.path.as_os_str()),
                        None => PathBuf::from(cd.path.as_os_str()),
                    };
                    let canonical = path.canonicalize().map_err(|source| Error::Directory { path: path.clone(), source })?;
                    if !canonical.is_dir() {
                        return Err(Error::Directory {
                            path,
                            source: io::Error::from(io::ErrorKind::NotADirectory),
                        });
                    }
                    cwd = Some(canonical);
                    status = None;
                }
                Self::Command(command) => {
                    let mut process = command.process();
                    if let Some(cwd) = &cwd {
                        process.current_dir(cwd);
                    }
                    #[cfg(unix)]
                    if handoff == Handoff::Exec && Some(index) == last {
                        use std::os::unix::process::CommandExt;
                        return Err(Error::Execute {
                            program: command.program.0.clone(),
                            source: process.exec(),
                        });
                    }
                    #[cfg(not(unix))]
                    let _ = (handoff, index, last);
                    let result = process.status().map_err(|source| Error::Execute {
                        program: command.program.0.clone(),
                        source,
                    })?;
                    if !result.success() {
                        return Ok(Some(result));
                    }
                    status = Some(result);
                }
                Self::And(_) => unreachable!("flattened expressions contain no And"),
            }
        }
        Ok(status)
    }

    fn flatten<'a>(&'a self, output: &mut Vec<&'a Self>) {
        match self {
            Self::And(and) => and.steps.iter().for_each(|step| step.flatten(output)),
            step => output.push(step),
        }
    }
}

impl From<Command> for Expression {
    fn from(command: Command) -> Self {
        Self::Command(command)
    }
}

impl From<ChangeDir> for Expression {
    fn from(cd: ChangeDir) -> Self {
        Self::ChangeDir(cd)
    }
}

impl From<And> for Expression {
    fn from(and: And) -> Self {
        Self::And(and)
    }
}
