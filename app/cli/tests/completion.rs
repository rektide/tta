#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

struct Server(std::path::PathBuf);

impl Server {
    fn start(path: std::path::PathBuf) -> Self {
        let server = Self(path);
        let output = Command::new("tmux")
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .args(["-f", "/dev/null", "-S"])
            .arg(&server.0)
            .args(["new-session", "-d", "-s", "live-work", "sleep", "60"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        server
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = Command::new("tmux").arg("-S").arg(&self.0).arg("kill-server").output();
    }
}

fn tta() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tta"));
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"TTA_") || key == "PROJECTS_DIR" {
            command.env_remove(key);
        }
    }
    command.env_remove("TMUX").env_remove("TMUX_PANE");
    command
}

fn names(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn project_inventory_is_sorted_literal_immediate_and_observation_only() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir_all(root.join("zebra/deep")).unwrap();
    for name in ["alpha", "space ' [x]", "line\nbreak"] {
        fs::create_dir(root.join(name)).unwrap();
    }
    fs::write(root.join("file"), "not a project").unwrap();
    symlink("zebra", root.join("linked")).unwrap();
    symlink("missing", root.join("dangling")).unwrap();
    symlink("file", root.join("linked-file")).unwrap();
    let socket = temp.path().join("socket");
    let output = names(
        tta()
            .env_remove("HOME")
            .env("TTA_ROOT", &root)
            .env("TTA_SOCKET_PATH", &socket)
            .env("TTA_TMUX", "/not/a/tmux")
            .args(["--completion-candidates", "projects"]),
    );
    assert_eq!(output, "alpha\nlinked\nspace ' [x]\nzebra\n");
    assert!(!socket.exists());
    let output = tta().arg("--completion-candidates=projects").arg("alpha").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn completion_root_precedence_matches_launch_configuration() {
    let temp = tempfile::tempdir().unwrap();
    for root in ["src", "legacy", "env", "cli"] {
        fs::create_dir_all(temp.path().join(root).join(root)).unwrap();
    }
    for (env, legacy, cli, expected) in [
        ("", "", None, "src"),
        ("", "legacy", None, "legacy"),
        ("env", "legacy", None, "env"),
        ("env", "legacy", Some("cli"), "cli"),
    ] {
        let mut command = tta();
        command
            .current_dir(temp.path())
            .env("HOME", temp.path())
            .env("TTA_ROOT", env)
            .env("PROJECTS_DIR", legacy)
            .arg("--completion-candidates=projects");
        if let Some(cli) = cli {
            command.args(["--root", cli]);
        }
        assert_eq!(names(&mut command), format!("{expected}\n"));
    }
}

#[test]
fn live_session_inventory_uses_selected_endpoint_without_a_root_or_switching() {
    let temp = tempfile::tempdir().unwrap();
    let server = Server::start(temp.path().join("live-socket"));
    for backend in ["native", "cli"] {
        if (backend == "native" && !cfg!(feature = "native")) || (backend == "cli" && !cfg!(feature = "cli")) {
            continue;
        }
        assert_eq!(
            names(
                tta()
                    .env_remove("HOME")
                    .env("TMUX", "/some-other-server,12,0")
                    .env("TTA_ROOT", "/no-such-root")
                    .env("TTA_SOCKET_NAME", "overridden")
                    .arg("--socket-path")
                    .arg(&server.0)
                    .args(["--backend", backend, "--completion-candidates", "sessions"]),
            ),
            "live-work\n"
        );
        let cold = temp.path().join("cold-socket");
        assert_eq!(
            names(
                tta()
                    .arg("--socket-path")
                    .arg(&cold)
                    .args(["--backend", backend, "--completion-candidates=sessions"])
            ),
            ""
        );
        assert!(!cold.exists());
    }
}

#[test]
fn candidate_errors_leave_stdout_empty_and_backends_match_build_features() {
    let temp = tempfile::tempdir().unwrap();
    let output = tta()
        .arg("--root")
        .arg(temp.path().join("missing"))
        .arg("--completion-candidates=projects")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot inspect project directory"));
    let backends = names(tta().arg("--completion-candidates=backends"));
    assert_eq!(backends.contains("native"), cfg!(feature = "native"));
    assert_eq!(backends.contains("cli"), cfg!(feature = "cli"));
}

#[test]
fn zsh_completes_patterns_fragments_options_and_quoted_names() {
    let temp = tempfile::tempdir().unwrap();
    let server = Server::start(temp.path().join("live-socket"));
    let shell = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shell/zsh");
    let output = Command::new("zsh")
        .arg("-f")
        .arg(shell.join("tests/completion.zsh"))
        .arg(env!("CARGO_BIN_EXE_tta"))
        .arg(shell.join("functions"))
        .arg(temp.path())
        .arg(&server.0)
        .output()
        .expect("Zsh is required for the completion integration test");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
