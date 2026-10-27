//! Local process-carrier checks, never a connection to SSH or tmux.
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn tta() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tta"));
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"TTA_") {
            command.env_remove(key);
        }
    }
    command
}

fn failure(output: Output, evidence: &str) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(evidence), "missing {evidence}: {stderr}");
    assert!(stderr.contains("SSH planning stopped before mutation"), "{stderr}");
}

#[test]
fn ssh_probe_limits_are_positive_and_require_ssh() {
    for flag in ["--ssh-probe-timeout", "--ssh-probe-retention"] {
        for value in ["0", "-1", "1.5", "18446744073709551616"] {
            let output = tta().args(["--ssh", "fixture", flag, value, "project"]).output().unwrap();
            assert_eq!(output.status.code(), Some(2));
        }
        assert_eq!(tta().args([flag, "1", "project"]).output().unwrap().status.code(), Some(2));
    }
    for name in ["TTA_SSH_PROBE_TIMEOUT", "TTA_SSH_PROBE_RETENTION"] {
        let output = tta().args(["--ssh", "fixture", "project"]).env(name, "0").output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(name));
    }
}

#[test]
fn ssh_probe_cli_over_env_limits_reach_capture_without_handoff() {
    let dir = tempfile::Builder::new().prefix("tta-ssh-policy-").tempdir_in("/tmp/opencode").unwrap();
    let carrier = dir.path().join("ssh");
    std::fs::write(&carrier, "#!/bin/sh\n[ \"$1\" = -T ] || { touch \"$MUTATION\"; exit 91; }\nprintf 'transport evidence\\n' >&2\ncase \"$MODE\" in\n slow) sleep 0.15; printf 'completed malformed probe';;\n flood) head -c 2048 /dev/zero;;\nesac\n").unwrap();
    std::fs::set_permissions(&carrier, std::fs::Permissions::from_mode(0o700)).unwrap();
    let run = |mode: &str, timeout: &str, retention: &str, extra: &[&str]| {
        let start = Instant::now();
        let output = tta()
            .args(["--ssh", "fixture", "--ssh-program"])
            .arg(&carrier)
            .args(extra)
            .arg("project")
            .env("MODE", mode)
            .env("MUTATION", dir.path().join("mutation"))
            .env("TTA_SSH_PROBE_TIMEOUT", timeout)
            .env("TTA_SSH_PROBE_RETENTION", retention)
            .output()
            .unwrap();
        assert!(start.elapsed() < Duration::from_secs(3), "capture must remain bounded");
        output
    };
    failure(run("slow", "20", "4096", &[]), "capture=Deadline");
    failure(run("slow", "invalid-ignored", "4096", &["--ssh-probe-timeout", "1000"]), "capture=Complete");
    failure(run("flood", "1000", "64", &[]), "capture=OutputCap");
    failure(run("flood", "1000", "invalid-ignored", &["--ssh-probe-retention", "4096"]), "capture=Complete");
    // Both absent and explicitly empty environment values preserve defaults.
    failure(run("slow", "", "", &[]), "capture=Complete");
    assert!(!dir.path().join("mutation").exists());
}
