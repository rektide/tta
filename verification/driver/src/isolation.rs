//! Reusable read-only preflight for exclusively owned local/loopback fixtures.
//! Validate the result before *any* mutation, and retain it in the report.
use std::path::Path;

use anyhow::{Result, ensure};

pub fn verify_absence(socket: &Path, output: &crate::ProcessOutput) -> Result<()> {
    ensure!(
        !socket.components().any(|part| part == std::path::Component::ParentDir),
        "owned fixture socket cannot contain '..'"
    );
    ensure!(
        socket.is_absolute() && socket.starts_with("/tmp/opencode") && socket.parent() != Some(Path::new("/tmp/opencode")),
        "fixture requires a private absolute socket below /tmp/opencode: {socket:?}"
    );
    ensure!(
        output.complete && output.status == Some(1) && output.stdout.is_empty(),
        "endpoint preflight is not complete classified absence: {output:?}"
    );
    let path = socket.to_str().ok_or_else(|| anyhow::anyhow!("fixture socket must be UTF-8"))?;
    ensure!(
        output.stderr == format!("no server running on {path}\n")
            || output.stderr == format!("error connecting to {path} (No such file or directory)\n")
            || output.stderr == format!("error connecting to {path} (Connection refused)\n"),
        "endpoint preflight did not name exactly the owned socket: {output:?}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrong_endpoint_live_empty_server_or_incomplete_output_is_not_proof() {
        let socket = Path::new("/tmp/opencode/owned-fixture/socket");
        let mut output = crate::ProcessOutput {
            complete: true,
            status: Some(1),
            stderr: format!("no server running on {}\n", socket.display()),
            ..Default::default()
        };
        verify_absence(socket, &output).unwrap();
        output.stderr = "no server running on /other\n".into();
        assert!(verify_absence(socket, &output).is_err());
        output.stderr = "no sessions\n".into();
        assert!(verify_absence(socket, &output).is_err());
        output.stderr = format!("no server running on {}\n", socket.display());
        output.complete = false;
        assert!(verify_absence(socket, &output).is_err());
        assert!(verify_absence(Path::new("/tmp/user/socket"), &output).is_err());
    }
}
