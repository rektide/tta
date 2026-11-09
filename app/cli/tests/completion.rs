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
fn project_completion_is_identity_aware_across_configured_roots() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir_all(first.join("shared")).unwrap();
    fs::create_dir_all(first.join("solo-first")).unwrap();
    fs::create_dir_all(second.join("shared")).unwrap();
    fs::create_dir_all(second.join("solo-second")).unwrap();
    // A basename duplicated across distinct physical directories is offered
    // only through qualified selectors; physically unique names stay bare and
    // their redundant qualified forms are dropped from the menu.
    let inventory = |command: &mut Command| {
        command
            .env_remove("HOME")
            .env("TTA_SOCKET_PATH", temp.path().join("unused-socket"))
            .env("TTA_TMUX", "/not/a/tmux");
        names(command)
    };
    assert_eq!(
        inventory(
            tta()
                .arg("--root")
                .arg(&first)
                .arg("--root")
                .arg(&second)
                .args(["--completion-candidates", "projects"])
        ),
        "@1/shared\n@2/shared\nsolo-first\nsolo-second\n"
    );
    // A repeated physical root through a symlink keeps the same identity: the
    // duplicate basename never becomes launchable by bare name.
    let aliased = temp.path().join("aliased-first");
    symlink(&first, &aliased).unwrap();
    assert_eq!(
        inventory(
            tta()
                .arg("--root")
                .arg(&first)
                .arg("--root")
                .arg(&aliased)
                .arg("--root")
                .arg(&second)
                .args(["--completion-candidates", "projects"])
        ),
        "@1/shared\n@2/shared\n@3/shared\nsolo-first\nsolo-second\n"
    );
    // The environment layer drives completion identically to repeated --root.
    let json = format!(r#"["{}","{}"]"#, first.display(), second.display());
    assert_eq!(
        inventory(tta().env("TTA_ROOTS", &json).args(["--completion-candidates", "projects"])),
        "@1/shared\n@2/shared\nsolo-first\nsolo-second\n"
    );
    // One configured root preserves the established single-root shape.
    assert_eq!(
        inventory(tta().arg("--root").arg(&first).args(["--completion-candidates", "projects"])),
        "shared\nsolo-first\n"
    );
    // Without any root, project completion names the missing input; sessions
    // completion stays independently available for rootless entry.
    let rootless = tta().env_remove("HOME").args(["--completion-candidates", "projects"]).output().unwrap();
    assert!(!rootless.status.success());
    assert!(rootless.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rootless.stderr).contains("set HOME or pass --root"));
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
fn orphan_alias_inventory_honors_prefix_environment_and_cli_override_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let server = Server::start(temp.path().join("live-socket"));
    let rename = Command::new("tmux")
        .arg("-S")
        .arg(&server.0)
        .args(["rename-session", "-t", "=live-work", "view-project-42"])
        .output()
        .unwrap();
    assert!(rename.status.success());
    for backend in ["native", "cli"] {
        if (backend == "native" && !cfg!(feature = "native")) || (backend == "cli" && !cfg!(feature = "cli")) {
            continue;
        }
        let inventory = || {
            let mut command = tta();
            command
                .env("TTA_CONFIG", temp.path().join("absent"))
                .env("TTA_SATELLITE_PREFIX", "view-")
                .arg("--socket-path")
                .arg(&server.0)
                .args(["--backend", backend, "--completion-candidates", "sessions"]);
            command
        };
        assert_eq!(names(&mut inventory()), "project\nview-project-42\n");
        assert_eq!(names(inventory().args(["--satellite-prefix", "_"])), "view-project-42\n");
        for prefix in ["", ".", ":", "bad\n"] {
            let result = inventory().arg(format!("--satellite-prefix={prefix}")).output().unwrap();
            assert!(!result.status.success());
            assert!(result.stdout.is_empty());
        }
        let plan = names(
            tta()
                .env("TTA_CONFIG", temp.path().join("absent"))
                .env("TTA_SATELLITE_PREFIX", "wrong-")
                .arg("--socket-path")
                .arg(&server.0)
                .args(["--backend", backend, "--satellite-prefix", "view-", "--print-shell", "project"]),
        );
        assert!(plan.contains("'view-project-1'") && plan.contains("'-t' '$0'"), "{plan}");
        assert!(!plan.contains("view-view-project-42-"));
    }
}

#[test]
fn same_group_aliases_are_one_workspace_but_unrelated_aliases_remain_ambiguous() {
    let temp = tempfile::tempdir().unwrap();
    let server = Server::start(temp.path().join("live-socket"));
    let tmux = |args: &[&str]| {
        let result = Command::new("tmux").arg("-S").arg(&server.0).args(args).output().unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    };
    tmux(&["rename-session", "-t", "=live-work", "_project-42"]);
    tmux(&["new-session", "-d", "-s", "_project-43", "-t", "=_project-42"]);
    let plan = |backend: &str, argument: &str| {
        let mut command = tta();
        command
            .env("TTA_CONFIG", temp.path().join("absent"))
            .arg("--socket-path")
            .arg(&server.0)
            .args(["--backend", backend, "--print-shell", argument]);
        command
    };
    for backend in ["native", "cli"] {
        if (backend == "native" && !cfg!(feature = "native")) || (backend == "cli" && !cfg!(feature = "cli")) {
            continue;
        }
        let source = names(&mut plan(backend, "project"));
        assert!(source.contains("'_project-1'") && source.contains("'-t' '$0'"), "{source}");
    }
    tmux(&["new-session", "-d", "-s", "_project-44", "sleep", "60"]);
    for backend in ["native", "cli"] {
        if (backend == "native" && !cfg!(feature = "native")) || (backend == "cli" && !cfg!(feature = "cli")) {
            continue;
        }
        let result = plan(backend, "project").output().unwrap();
        assert!(!result.status.success() && result.stdout.is_empty());
        let diagnostic = String::from_utf8_lossy(&result.stderr);
        assert!(
            diagnostic.contains("ambiguous session") && diagnostic.contains("_project-42") && diagnostic.contains("_project-44"),
            "{diagnostic}"
        );
        assert!(names(&mut plan(backend, "_project-44")).contains("'-t' '$2'"));
    }
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn scaffold_inventory_lists_configured_generators_and_fails_loudly() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config.toml");
    fs::write(
        &config,
        "[scaffold.generators.zeta]\nadapter = \"rekon\"\n[scaffold.generators.alpha]\nadapter = \"rekon\"\n",
    )
    .unwrap();
    let output = names(tta().env("TTA_CONFIG", &config).arg("--completion-candidates=scaffolds"));
    assert_eq!(output, "alpha\nzeta\n");
    let conflict = tta()
        .env("TTA_CONFIG", &config)
        .arg("--completion-candidates=scaffolds")
        .arg("alpha")
        .output()
        .unwrap();
    assert_eq!(conflict.status.code(), Some(2));
    assert!(conflict.stdout.is_empty());
    // A broken configuration fails the inventory with an error, not silence.
    fs::write(&config, "[scaffold.generators.x]\nadapter = \"nope\"\n").unwrap();
    let output = tta().env("TTA_CONFIG", &config).arg("--completion-candidates=scaffolds").output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("which this tta build does not include"));
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
