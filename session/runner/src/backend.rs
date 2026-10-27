use std::io;

use tta_launcher::Session;

/// Lookup acquires session facts only. Launch execution is a separate seam in
/// [`crate::execution`]; an absent server is an empty inventory, not permission
/// to retry a failed mutation.
pub trait SessionLookup {
    fn sessions(&self) -> Result<Vec<Session>, LookupError>;

    /// Override when the observer can distinguish classified server absence.
    fn inventory(&self) -> Result<tta_launcher::workflow::SessionInventory, LookupError> {
        self.sessions().map(tta_launcher::workflow::SessionInventory::Present)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[cfg(feature = "native")]
    #[error(transparent)]
    Native(#[from] tmax::error::OperationError),
    #[error("cannot run tmux session lookup: {0}")]
    Io(#[from] io::Error),
    #[error("tmux session lookup exited with {status}: {stderr}")]
    Command { status: std::process::ExitStatus, stderr: String },
    #[error("tmux session output is not UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Build(#[from] tmax::operation::BuildError),
    #[error(transparent)]
    Decode(#[from] tmax::operation::DecodeError),
    #[error("tmux session capture was incomplete: {0:?}")]
    Incomplete(Box<crate::capture::Output>),
}

#[cfg(feature = "native")]
#[derive(Clone, Debug)]
pub struct Native {
    pub endpoint: tmax::endpoint::Tmux,
}

#[cfg(feature = "native")]
impl SessionLookup for Native {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok(self.inventory()?.sessions().to_vec())
    }

    fn inventory(&self) -> Result<tta_launcher::workflow::SessionInventory, LookupError> {
        use tta_launcher::workflow::SessionInventory;
        match self.endpoint.query(tmax::query::Sessions::new()) {
            Ok(sessions) => Ok(SessionInventory::Present(sessions.into_iter().map(Session::from).collect())),
            Err(error) if absent_server(&error) => {
                if matches!(error, tmax::error::OperationError::Rejected(_)) {
                    Ok(SessionInventory::Present(Vec::new()))
                } else {
                    Ok(SessionInventory::Absent)
                }
            }
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(feature = "native")]
fn absent_server(error: &tmax::error::OperationError) -> bool {
    use tmax::error::OperationError;
    match error {
        OperationError::ServerConnection { source, .. } => matches!(source.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused),
        // A deliberately kept-alive server may have no sessions.
        OperationError::Rejected(diagnostic) => {
            diagnostic.return_code == Some(1) && !diagnostic.truncated && diagnostic.body.is_empty() && diagnostic.stderr == b"no sessions\n"
        }
        _ => false,
    }
}

#[cfg(feature = "cli")]
#[derive(Clone, Debug)]
pub struct Cli {
    pub program: std::ffi::OsString,
    pub socket: std::path::PathBuf,
}

#[cfg(feature = "cli")]
impl SessionLookup for Cli {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok(self.inventory()?.sessions().to_vec())
    }

    fn inventory(&self) -> Result<tta_launcher::workflow::SessionInventory, LookupError> {
        use tmax::operation::{ReplyDecoder, TypedOperation};
        use tta_launcher::workflow::SessionInventory;
        let query = tmax::query::Sessions::new();
        let mut command = std::process::Command::new(&self.program);
        command
            // Match native's explicit CLIENT_UTF8 even with C-locale errors.
            // Without -u, tmux sanitizes the typed schema's tab delimiters.
            .arg("-u")
            .arg("-S")
            .arg(&self.socket)
            .args(query.invocation()?.argv()?)
            .env("LC_ALL", "C");
        let output = crate::capture::run(&mut command, crate::capture::Limits::default())?;
        if !output.complete() {
            return Err(LookupError::Incomplete(Box::new(output)));
        }
        let status = output.status.expect("complete capture has status");
        if !status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if status.code() == Some(1) && output.stdout.is_empty() && cli_absent(&stderr, &self.socket) {
                return Ok(if stderr == "no sessions\n" {
                    SessionInventory::Present(Vec::new())
                } else {
                    SessionInventory::Absent
                });
            }
            return Err(LookupError::Command { status, stderr });
        }
        let text = std::str::from_utf8(&output.stdout)?;
        let mut decoder = query.decoder();
        let mut rows = Vec::new();
        if !text.is_empty() {
            for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
                if let Some(row) = decoder.push(line)? {
                    rows.push(row);
                }
            }
        }
        decoder.finish()?;
        Ok(SessionInventory::Present(
            tmax::query::Sessions::collect(rows)?.into_iter().map(Session::from).collect(),
        ))
    }
}

#[cfg(any(feature = "cli", test))]
fn cli_absent(stderr: &str, socket: &std::path::Path) -> bool {
    // tmux's CLI exposes errno only as a diagnostic. Match complete C-locale
    // messages for this exact endpoint, never a loose "connect" substring.
    let socket = socket.to_string_lossy();
    stderr == "no sessions\n"
        || stderr == format!("no server running on {socket}\n")
        || stderr == format!("error connecting to {socket} (No such file or directory)\n")
        || stderr == format!("error connecting to {socket} (Connection refused)\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "native")]
    #[test]
    fn only_missing_or_refused_socket_connections_are_empty() {
        for kind in [
            io::ErrorKind::NotFound,
            io::ErrorKind::ConnectionRefused,
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::TimedOut,
        ] {
            let error = tmax::error::OperationError::ServerConnection {
                socket_path: "/private/tmux".into(),
                source: io::Error::from(kind),
            };
            assert_eq!(
                absent_server(&error),
                matches!(kind, io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused)
            );
        }
        assert!(!absent_server(&tmax::error::OperationError::Io(io::Error::from(io::ErrorKind::NotFound))));
        assert!(!absent_server(&tmax::error::OperationError::Engine("bad protocol".to_owned())));
    }

    #[test]
    fn cli_absence_does_not_swallow_permission_protocol_or_other_endpoints() {
        let path = std::path::Path::new("/private/tmux");
        assert!(cli_absent("error connecting to /private/tmux (No such file or directory)\n", path));
        assert!(cli_absent("no server running on /private/tmux\n", path));
        assert!(!cli_absent("error connecting to /private/tmux (Permission denied)\n", path));
        assert!(!cli_absent("protocol version mismatch\n", path));
        assert!(!cli_absent("no server running on /other\n", path));
        assert!(!cli_absent("no server running on /private/tmux\nextra\n", path));
    }
}
