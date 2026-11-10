//! Pure target-shell carrier construction and classified filesystem entry facts.
//! Context acquisition and entry policy belong to the entry protocol.
use anyhow::{Result, ensure};
use bon::Builder;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tta_command::{Assignment, Word};

/// Applied after the selected shell's startup. None explicitly unsets a value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Environment {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Builder)]
pub struct TargetSetup {
    pub shell: Option<String>,
    #[builder(default)]
    pub login: bool,
    #[builder(default)]
    pub interactive: bool,
    #[builder(default)]
    pub environment: Vec<Environment>,
    pub cwd: Option<PathBuf>,
}

fn quote(value: impl Into<OsString>) -> Result<String> {
    Ok(Word::new(value)?.shell()?)
}

fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn nu_quote(value: &str) -> String {
    use std::fmt::Write;
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            ch if ch.is_control() => write!(&mut out, "\\u{{{:x}}}", ch as u32).unwrap(),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

impl TargetSetup {
    /// The target's selected shell performs startup; POSIX sh carries the actual
    /// source. A new SSH terminal cannot inherit a local tmux client identity.
    pub fn shell_command(&self, source: &str) -> Result<String> {
        Word::new(source)?;
        let mut body = String::new();
        for variable in &self.environment {
            Assignment::new(&variable.name, Word::new(variable.value.as_deref().unwrap_or(""))?)?;
            match &variable.value {
                Some(value) => body.push_str(&format!("export {}={} || exit $?\n", variable.name, quote(value.as_str())?)),
                None => body.push_str(&format!("unset {} || exit $?\n", variable.name)),
            }
        }
        body.push_str("unset TMUX TMUX_PANE || exit $?\n");
        if let Some(cwd) = &self.cwd {
            let path = if cwd.is_absolute() { cwd.clone() } else { Path::new(".").join(cwd) };
            body.push_str(&format!("cd -P {} || exit $?\n", quote(path.into_os_string())?));
        }
        body.push_str(source);
        let carrier = format!("exec /bin/sh -c {}", quote(body.as_str())?);
        let fish = format!("exec /bin/sh -c {}", fish_quote(&body));
        let nu = format!("^/bin/sh -c {}", nu_quote(&body));
        let shell = match &self.shell {
            Some(shell) => {
                ensure!(!shell.is_empty(), "selected target shell is empty");
                quote(shell.as_str())?
            }
            None => "\"${SHELL:-/bin/sh}\"".into(),
        };
        let mut flags = String::new();
        if self.login {
            flags.push_str(" -l");
        }
        if self.interactive {
            flags.push_str(" -i");
        }
        let bootstrap = format!(
            "tta_shell={shell}\ncase \"${{tta_shell##*/}}\" in\nfish) exec \"$tta_shell\"{flags} -c {} ;;\nnu|nushell) exec \"$tta_shell\" -c {} ;;\n*) exec \"$tta_shell\"{flags} -c {} ;;\nesac",
            quote(fish)?,
            quote(nu)?,
            quote(carrier)?
        );
        Ok(format!("/bin/sh -c {}", quote(bootstrap)?))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    pub kind: u8,
    pub referent: u8,
}

impl Entry {
    pub fn directory(self) -> bool {
        self.kind == b'd' || (self.kind == b'l' && self.referent == b'd')
    }
    pub fn dangling(self) -> bool {
        self.kind == b'l' && self.referent == b'N'
    }
    pub fn checked(self) -> Result<Self> {
        ensure!(!matches!(self.referent, b'?' | b'L'), "cannot inspect symlink referent: {self:?}");
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_preserves_literal_values_and_rejects_executable_environment_names() {
        let setup = TargetSetup::builder()
            .shell("/target/shell".into())
            .login(true)
            .interactive(true)
            .environment(vec![Environment {
                name: "HOME".into(),
                value: Some("/target/quote' $;\n".into()),
            }])
            .build();
        assert!(setup.shell_command("printf ok").unwrap().contains(" -l -i -c "));
        let bad = TargetSetup::builder()
            .environment(vec![Environment {
                name: "PATH; touch x".into(),
                value: None,
            }])
            .build();
        assert!(bad.shell_command(":").is_err());
    }
}
