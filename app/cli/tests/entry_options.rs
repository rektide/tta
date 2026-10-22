#![cfg(unix)]

use std::fs;
use std::process::Command;

fn tta(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tta"));
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"TTA_") || key == "PROJECTS_DIR" {
            command.env_remove(key);
        }
    }
    command
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .env("HOME", temp.path())
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .current_dir(temp.path())
        .args(["--root", ".", "--socket-path", "sock", "--print-shell"]);
    command
}

#[test]
fn explicit_dir_is_frozen_and_startup_flags_are_not_launcher_flags() {
    let temp = tempfile::Builder::new().prefix("tta-cli-entry-").tempdir_in("/tmp/opencode").unwrap();
    fs::create_dir(temp.path().join("project")).unwrap();
    let output = tta(&temp)
        .args(["--group", "--dir", "initial", "project", "--", "program", "--dir", "literal", "a;", ""])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let source = String::from_utf8(output.stdout).unwrap();
    assert!(source.starts_with(&format!("cd -P '{}'", temp.path().join("initial").display())), "{source}");
    assert!(source.contains("'-s' 'project'"), "{source}");
    assert!(source.contains("'--' 'program' '--dir' 'literal' 'a\\;' ''"), "{source}");
    assert!(!source.contains("keep-last"));
    assert!(!temp.path().join("sock").exists());
}

#[test]
fn double_dash_still_allows_a_fragment_starting_with_hyphen() {
    let temp = tempfile::Builder::new().prefix("tta-cli-entry-").tempdir_in("/tmp/opencode").unwrap();
    fs::create_dir(temp.path().join("-project")).unwrap();
    let output = tta(&temp).args(["--", "-project", "program", "--flag", "space ;"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("'-s' '-project'"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("'--' 'program' '--flag' 'space \\;'"));
    assert!(!temp.path().join("sock").exists());
}

#[test]
fn existing_grouped_plan_ignores_startup_and_preserves_session_first_matching() {
    let temp = tempfile::Builder::new().prefix("tta-cli-entry-").tempdir_in("/tmp/opencode").unwrap();
    let socket = temp.path().join("sock");
    let tmux = || {
        let mut command = Command::new("tmux");
        command
            .args(["-S"])
            .arg(&socket)
            .args(["-f", "/dev/null"])
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .env("LC_ALL", "C");
        command
    };
    let missing = tmux().arg("ls").output().unwrap();
    assert!(!missing.status.success());
    let error = String::from_utf8(missing.stderr).unwrap();
    assert!(error.contains(socket.to_str().unwrap()), "{error}");
    assert!(error.contains("no server running") || error.contains("No such file or directory"), "{error}");
    let seed = tmux().args(["new-session", "-d", "-s", "seed", "sleep", "120"]).output().unwrap();
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));
    let output = tta(&temp).args(["--group", "--dir", "initial", "--", "seed", "must-not-run"]).output().unwrap();
    // Clean the only session this fixture created before any assertions which
    // might panic. Emission is observation-only and must create no satellite.
    let inventory = tmux().args(["list-sessions", "-F", "#{session_name}"]).output().unwrap();
    let cleanup = tmux().args(["kill-session", "-t", "=seed"]).output().unwrap();
    assert!(cleanup.status.success(), "{}", String::from_utf8_lossy(&cleanup.stderr));
    assert_eq!(String::from_utf8(inventory.stdout).unwrap(), "seed\n");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let source = String::from_utf8(output.stdout).unwrap();
    assert!(source.contains("'new-session' '-d' '-s' '_seed-1' '-t' '$0'"), "{source}");
    assert!(source.contains(&format!("'-c' '{}'", temp.path().join("initial").display())), "{source}");
    assert!(source.contains("';' 'attach-session' '-t' '=_seed-1'"), "{source}");
    assert!(source.contains("';' 'set-option' '-t' '=_seed-1:' 'destroy-unattached' 'keep-last'"), "{source}");
    assert!(!source.contains("must-not-run"));
}
