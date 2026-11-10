//! Literal SSH transport construction. Observation and effect ownership live in
//! `observe::ssh` and `execute::ssh`; callers own connection/master policy.
use anyhow::{Result, ensure};
use bon::Builder;
use std::ffi::OsString;
use tta_command::{Command, Word};

#[derive(Clone, Debug, Builder)]
pub struct Ssh {
    pub destination: String,
    #[builder(default = "ssh".into())]
    pub program: OsString,
    /// Identical literal OpenSSH options for probes and terminal handoff.
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
        // SSH concatenates command arguments: send exactly one shell line.
        args.push(Word::new(remote)?);
        Ok(Command::builder().program(Word::new(self.program.clone())?).args(args).build())
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
        assert!(Ssh::builder().destination("-oProxyCommand=bad".into()).build().command(":", false).is_err());
    }
}
