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
