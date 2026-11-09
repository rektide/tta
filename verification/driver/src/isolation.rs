//! Reusable read-only preflight for exclusively owned local/loopback fixtures.
//! Validate the result before *any* mutation, and retain it in the report.
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};

/// Owned scratch root for exclusively isolated fixtures: `TTA_TEST_TMPDIR`
/// overrides; `/tmp/opencode` when the current user can create directories
/// there; the system temp directory otherwise. Fixtures stay private through
/// uniquely named subdirectories below this root.
pub fn scratch_root() -> PathBuf {
    if let Some(path) = std::env::var_os("TTA_TEST_TMPDIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    if tempfile::Builder::new().prefix("tta-scratch-probe-").tempdir_in("/tmp/opencode").is_ok() {
        return PathBuf::from("/tmp/opencode");
    }
    std::env::temp_dir()
}

pub fn verify_absence(socket: &Path, output: &crate::ProcessOutput) -> Result<()> {
    let root = scratch_root();
    ensure!(
        !socket.components().any(|part| part == std::path::Component::ParentDir),
        "owned fixture socket cannot contain '..'"
    );
    ensure!(
        socket.is_absolute() && socket.starts_with(&root) && socket.parent() != Some(root.as_path()),
        "fixture requires a private absolute socket below {}: {socket:?}",
        root.display()
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
        let socket = scratch_root().join("owned-fixture/socket");
        let mut output = crate::ProcessOutput {
            complete: true,
            status: Some(1),
            stderr: format!("no server running on {}\n", socket.display()),
            ..Default::default()
        };
        verify_absence(&socket, &output).unwrap();
        output.stderr = "no server running on /other\n".into();
        assert!(verify_absence(&socket, &output).is_err());
        output.stderr = "no sessions\n".into();
        assert!(verify_absence(&socket, &output).is_err());
        output.stderr = format!("no server running on {}\n", socket.display());
        output.complete = false;
        assert!(verify_absence(&socket, &output).is_err());
        assert!(verify_absence(Path::new("/home/user/socket"), &output).is_err());
    }
}
