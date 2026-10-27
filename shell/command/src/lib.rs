//! Small, shell-independent expressions. Arguments are values, never shell source.
//!
//! ```
//! use tta_command::{Command, Expression, Handoff, Word};
//! let expression = Expression::from(Command::builder()
//!     .program(Word::new("printf")?)
//!     .args(vec![Word::new("hello world")?])
//!     .build());
//! assert_eq!(expression.shell(Handoff::Run)?, "'printf' 'hello world'");
//! # Ok::<(), tta_command::Error>(())
//! ```
#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::path::Path;

use bon::Builder;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("command operands cannot contain NUL")]
    Nul,
    #[error("POSIX shell output requires UTF-8 operands: {0:?}")]
    NonUtf8(OsString),
    #[error("invalid portable environment variable name: {0:?}")]
    EnvironmentName(String),
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
    #[builder(default)]
    pub environment: Vec<Assignment>,
}

/// A validated assignment, not executable shell source or a joined argv word.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assignment {
    name: String,
    value: Word,
}

impl Assignment {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &Word {
        &self.value
    }

    pub fn new(name: impl Into<String>, value: Word) -> Result<Self, Error> {
        let name = name.into();
        if !name.as_bytes().first().is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
            || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(Error::EnvironmentName(name));
        }
        Ok(Self { name, value })
    }
}

impl Command {
    fn shell(&self, handoff: bool) -> Result<String, Error> {
        let mut words = Vec::with_capacity(self.args.len() + 2);
        for assignment in &self.environment {
            words.push(format!("{}={}", assignment.name, assignment.value.shell()?));
        }
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

/// A file materialized with literal contents. Direct execution writes in
/// process; shell rendering is `printf '%s' CONTENTS > PATH`, so contents
/// stay one quoted value, never heredoc or eval source.
#[derive(Clone, Debug, Eq, PartialEq, Builder)]
pub struct WriteFile {
    pub path: Word,
    pub contents: Word,
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
    Write(WriteFile),
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
                Self::Write(write) => {
                    format!("'printf' '%s' {} > {}", write.contents.shell()?, write.path.shell()?)
                }
                Self::And(_) => unreachable!("flattened expressions contain no And"),
            });
        }
        if rendered.is_empty() {
            return Ok(":".to_owned());
        }
        Ok(rendered.join(" && "))
    }

    /// Inspect the ordered leaves of a short-circuit sequence without I/O.
    pub fn steps(&self) -> Vec<&Self> {
        let mut steps = Vec::new();
        self.flatten(&mut steps);
        steps
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

impl From<WriteFile> for Expression {
    fn from(write: WriteFile) -> Self {
        Self::Write(write)
    }
}

impl From<And> for Expression {
    fn from(and: And) -> Self {
        Self::And(and)
    }
}
