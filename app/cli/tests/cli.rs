#![cfg(unix)]

use std::process::Command;
#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
use std::{fs, os::unix::fs::PermissionsExt};

fn tta() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tta"));
    command.env_remove("TMUX").env_remove("TMUX_PANE");
    command
}

#[test]
fn help_and_invalid_combinations_are_clear() {
    let help = tta().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("not a glob"));
    let error = tta().args(["--exec-shell", "project"]).output().unwrap();
    assert_eq!(error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&error.stderr).contains("--print-shell"));
}

#[test]
fn cross_server_calls_inside_tmux_fail_before_lookup() {
    let temp = tempfile::tempdir().unwrap();
    let selected = temp.path().join("selected");
    let output = tta()
        .env("TMUX", "/tta-current-server,12,0")
        .args(["--print-shell", "--socket-path"])
        .arg(selected)
        .arg("project")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot switch a tmux client between servers"));
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn print_is_observation_only_and_emitted_shell_preserves_the_caller() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let project = root.join("space ' #[x];");
    fs::create_dir_all(&project).unwrap();
    let socket = temp.path().join("socket");
    // An executable recorder proves direct execution uses argv/cwd, and native
    // --print-shell never attempts to run even this executable.
    let recorder = temp.path().join("record-tmux");
    fs::write(&recorder, "#!/bin/sh\nprintf 'cwd:<%s>\\n' \"$PWD\"\nprintf 'arg:<%s>\\n' \"$@\"\n").unwrap();
    fs::set_permissions(&recorder, fs::Permissions::from_mode(0o700)).unwrap();
    let args = [
        "--root".as_ref(),
        root.as_os_str(),
        "--socket-path".as_ref(),
        socket.as_os_str(),
        "--tmux".as_ref(),
        recorder.as_os_str(),
    ];
    let printed = tta().args(args).args(["--print-shell", "space"]).output().unwrap();
    assert!(printed.status.success(), "{}", String::from_utf8_lossy(&printed.stderr));
    assert!(printed.stderr.is_empty());
    assert!(!socket.exists());
    let source = String::from_utf8(printed.stdout).unwrap();
    assert!(source.starts_with("cd -P "));
    assert!(!source.contains("exec "));
    let direct = tta().args(args).arg("space").output().unwrap();
    assert!(direct.status.success());
    let evaluated = Command::new("sh")
        .args(["-c", &format!("{source}\nprintf '%s' 'caller survived'\n")])
        .output()
        .unwrap();
    assert!(evaluated.status.success());
    let mut expected = direct.stdout;
    expected.extend_from_slice(b"caller survived");
    assert_eq!(evaluated.stdout, expected);
    assert!(!socket.exists());
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn errors_keep_stdout_empty_and_root_defaults_to_home_src() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("src/project")).unwrap();
    let socket = temp.path().join("socket");
    let output = tta()
        .env("HOME", temp.path())
        .args(["--print-shell", "--socket-path"])
        .arg(&socket)
        .arg("project")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("new-session"));
    let missing_home = tta()
        .env_remove("HOME")
        .args(["--print-shell", "--socket-path"])
        .arg(&socket)
        .arg("project")
        .output()
        .unwrap();
    assert!(!missing_home.status.success());
    assert!(missing_home.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing_home.stderr).contains("set HOME or pass --root"));
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn relative_socket_is_frozen_before_project_cd() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("project")).unwrap();
    let printed = tta()
        .current_dir(temp.path())
        .args(["--root", ".", "--socket-path", "socket", "--print-shell", "project"])
        .output()
        .unwrap();
    assert!(printed.status.success());
    let source = String::from_utf8(printed.stdout).unwrap();
    assert!(source.contains(&format!("'-S' '{}'", temp.path().join("socket").display())));
}
