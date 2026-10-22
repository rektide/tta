#![cfg(unix)]

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
use std::fs;
use std::process::Command;

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
fn native_print_is_observation_only_and_does_not_spawn_the_client() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let project = root.join("space ' #[x];");
    fs::create_dir_all(&project).unwrap();
    let socket = temp.path().join("socket");
    // Native --print-shell cannot attempt to run even a missing executable.
    let recorder = temp.path().join("missing-tmux");
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

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn explicit_relative_executable_and_config_are_frozen_before_project_cd() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("project")).unwrap();
    let printed = tta()
        .current_dir(temp.path())
        .args([
            "--root",
            ".",
            "--socket-path",
            "socket",
            "--tmux",
            "./bin/tmux",
            "--tmux-config",
            "./tmux.conf",
            "--print-shell",
            "project",
        ])
        .output()
        .unwrap();
    assert!(printed.status.success(), "{}", String::from_utf8_lossy(&printed.stderr));
    let source = String::from_utf8(printed.stdout).unwrap();
    assert!(source.contains(&format!("'{}'", temp.path().join("./bin/tmux").display())), "{source}");
    assert!(source.contains(&format!("'-f' '{}'", temp.path().join("./tmux.conf").display())), "{source}");
}

#[cfg(feature = "cli")]
#[test]
fn cli_backend_can_emit_a_cold_server_plan_without_starting_it() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("project")).unwrap();
    let socket = temp.path().join("socket");
    let printed = tta()
        .args(["--backend", "cli", "--print-shell", "--socket-path"])
        .arg(&socket)
        .arg("--root")
        .arg(temp.path())
        .arg("project")
        .output()
        .unwrap();
    assert!(printed.status.success(), "{}", String::from_utf8_lossy(&printed.stderr));
    assert!(printed.stderr.is_empty());
    assert!(String::from_utf8_lossy(&printed.stdout).contains("new-session"));
    assert!(!socket.exists());
}

fn assert_plan(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#[cfg(all(feature = "cli", feature = "scaffold-rekon"))]
#[test]
fn scaffold_mode_plans_generation_before_the_new_session() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let config = temp.path().join("config.toml");
    std::fs::write(&config, "[scaffold]\ndefault = \"rk\"\n\n[scaffold.generators.rk]\nadapter = \"rekon\"\n").unwrap();
    let socket = temp.path().join("socket");
    let printed = tta()
        .env("TTA_CONFIG", &config)
        .args(["--backend", "cli", "--root"])
        .arg(&root)
        .arg("--socket-path")
        .arg(&socket)
        .args(["--print-shell", "fresh"])
        .output()
        .unwrap();
    assert!(printed.status.success(), "{}", String::from_utf8_lossy(&printed.stderr));
    let source = String::from_utf8(printed.stdout).unwrap();
    assert!(source.starts_with("'rekon' 'fresh' && "), "{source}");
    assert!(
        source.contains(&format!("cd -P '{}'", root.canonicalize().unwrap().join("fresh").display())),
        "{source}"
    );
    assert!(source.contains("new-session"), "{source}");
    // Printing observes; it neither generates nor starts a server.
    assert!(!root.join("fresh").exists());
    assert!(!socket.exists());
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn scaffold_selection_is_deliberate_and_never_guesses() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let socket = temp.path().join("socket");
    let config = temp.path().join("config.toml");
    std::fs::write(
        &config,
        "[scaffold.generators.rk]\nadapter = \"rekon\"\n[scaffold.generators.alt]\nadapter = \"rekon\"\n",
    )
    .unwrap();
    let mut base = tta();
    base.env("TTA_CONFIG", &config)
        .env("TTA_ROOT", &root)
        .env("TTA_SOCKET_PATH", &socket)
        .args(["--print-shell"]);
    // Multiple generators without a default refuse to guess...
    let ambiguous = base.args(["fresh"]).output().unwrap();
    assert!(!ambiguous.status.success());
    assert!(
        String::from_utf8_lossy(&ambiguous.stderr).contains("--scaffold=NAME"),
        "{}",
        String::from_utf8_lossy(&ambiguous.stderr)
    );
    // ...and an unknown name lists the configured choices.
    let unknown = base.args(["--scaffold=bogus", "fresh"]).output().unwrap();
    assert!(!unknown.status.success());
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(stderr.contains("bogus") && stderr.contains("alt") && stderr.contains("rk"), "{stderr}");
    // Explicit choice wins over an off environment value.
    let source = {
        let mut command = tta();
        command
            .env("TTA_CONFIG", &config)
            .env("TTA_ROOT", &root)
            .env("TTA_SOCKET_PATH", &socket)
            .env("TTA_SCAFFOLD", "off")
            .args(["--backend", "cli", "--scaffold=alt", "--print-shell", "fresh"]);
        assert_plan(&mut command)
    };
    assert!(source.starts_with("'rekon' 'fresh'"), "{source}");
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn scaffold_disabling_and_missing_registries_keep_classic_misses() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let socket = temp.path().join("socket");
    let config = temp.path().join("config.toml");
    std::fs::write(&config, "[scaffold.generators.rk]\nadapter = \"rekon\"\n").unwrap();
    // An explicit --scaffold=false keeps the plain miss error even when configured.
    let output = tta()
        .env("TTA_CONFIG", &config)
        .env("TTA_ROOT", &root)
        .env("TTA_SOCKET_PATH", &socket)
        .args(["--backend", "cli", "--scaffold=false", "--print-shell", "fresh"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no session or immediate project directory matches"), "{stderr}");
    assert!(!stderr.contains("--scaffold=false"), "{stderr}");
    // The default posture with nothing configured explains how to proceed.
    let output = tta()
        .env("TTA_CONFIG", temp.path().join("absent.toml"))
        .env("TTA_ROOT", &root)
        .env("TTA_SOCKET_PATH", &socket)
        .args(["--backend", "cli", "--print-shell", "fresh"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no generators are configured"), "{stderr}");
    assert!(stderr.contains("--scaffold=false"), "{stderr}");
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn scaffold_mode_still_prefers_sessions_directories_and_safe_names() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir_all(root.join("present")).unwrap();
    let config = temp.path().join("config.toml");
    std::fs::write(&config, "[scaffold.generators.rk]\nadapter = \"rekon\"\n").unwrap();
    let socket = temp.path().join("socket");
    // An existing directory wins; the generator never runs.
    let source = {
        let mut command = tta();
        command.env("TTA_CONFIG", &config).env("TTA_ROOT", &root).env("TTA_SOCKET_PATH", &socket).args([
            "--backend",
            "cli",
            "--scaffold=rk",
            "--print-shell",
            "present",
        ]);
        assert_plan(&mut command)
    };
    assert!(source.contains("cd -P "), "{source}");
    assert!(!source.contains("rekon"), "{source}");
    // Fragments must be single path components, even in scaffold mode.
    let output = tta()
        .env("TTA_CONFIG", &config)
        .env("TTA_ROOT", &root)
        .env("TTA_SOCKET_PATH", &socket)
        .args(["--backend", "cli", "--scaffold=rk", "--print-shell", "nested/path"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("single path component"));
}

#[test]
fn environment_roots_layer_without_changing_their_path_meaning() {
    let temp = tempfile::tempdir().unwrap();
    for root in ["src", "legacy", "env", "cli"] {
        std::fs::create_dir_all(temp.path().join(root).join("project")).unwrap();
    }
    let cwd = temp.path().join("cwd");
    std::fs::create_dir(&cwd).unwrap();
    let socket = temp.path().join("socket");
    for (tta_root, projects, cli_root, expected) in [
        ("", "", None, "src"),
        ("", "legacy", None, "legacy"),
        ("../env", "legacy", None, "env"),
        ("../env", "legacy", Some("../cli"), "cli"),
    ] {
        let mut command = tta();
        command
            .current_dir(&cwd)
            .env("HOME", temp.path())
            .env("TTA_ROOT", tta_root)
            .env("PROJECTS_DIR", projects)
            .env("TTA_SOCKET_PATH", &socket)
            .args(["--print-shell", "project"]);
        if let Some(root) = cli_root {
            command.args(["--root", root]);
        }
        let source = assert_plan(&mut command);
        assert!(source.contains(&temp.path().join(expected).join("project").display().to_string()), "{source}");
        assert!(!socket.exists());
    }
    // Absolute legacy roots do not need HOME; literal TTA_ROOT is cwd-relative.
    for key in ["PROJECTS_DIR", "TTA_ROOT"] {
        let source = assert_plan(
            tta()
                .env_remove("HOME")
                .env(key, temp.path().join("legacy"))
                .env("TTA_SOCKET_PATH", &socket)
                .args(["--print-shell", "project"]),
        );
        assert!(source.contains("legacy/project"));
    }
    let output = tta()
        .env_remove("HOME")
        .env("PROJECTS_DIR", "legacy")
        .env("TTA_SOCKET_PATH", socket)
        .args(["--print-shell", "project"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot determine the project root"));
}

#[test]
fn endpoint_cli_selectors_override_the_entire_environment_family() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("project")).unwrap();
    let mut base = tta();
    base.current_dir(temp.path())
        .env("TTA_ROOT", ".")
        .env("TTA_SOCKET_NAME", "env-label")
        .env("TTA_SOCKET_PATH", "env-socket")
        .args(["--print-shell", "project"]);
    let conflict = base.output().unwrap();
    assert!(!conflict.status.success());
    assert!(conflict.stdout.is_empty());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("TTA_SOCKET_NAME and TTA_SOCKET_PATH conflict"));
    let source = assert_plan(base.args(["--socket-path", "cli-socket"]));
    assert!(source.contains(&format!("'-S' '{}'", temp.path().join("cli-socket").display())));
    assert!(!source.contains("env-label"));
    let label = format!("tta-env-test-{}", std::process::id());
    let source = assert_plan(
        tta()
            .env("TTA_ROOT", temp.path())
            .env("TTA_SOCKET_PATH", "ignored")
            .args(["--socket-name", &label, "--print-shell", "project"]),
    );
    assert!(source.contains(&format!("'-L' '{label}'")), "{source}");
    assert!(!source.contains("ignored"));
    assert!(!temp.path().join("cli-socket").exists());
}

#[test]
fn environment_validation_and_explicit_overrides_are_deliberate() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("project")).unwrap();
    let socket = temp.path().join("socket");
    for (key, value) in [("TTA_BACKEND", "unsupported"), ("TTA_TRACE", "yes")] {
        let output = tta().env(key, value).args(["--print-shell", "project"]).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(key));
    }
    let backend = if cfg!(feature = "native") { "native" } else { "cli" };
    let mut empty = tta();
    for key in ["TTA_BACKEND", "TTA_SOCKET_NAME", "TTA_SOCKET_PATH", "TTA_TMUX", "TTA_TMUX_CONFIG", "TTA_TRACE"] {
        empty.env(key, "");
    }
    let source = assert_plan(
        empty
            .env("TTA_ROOT", temp.path())
            .arg("--socket-path")
            .arg(&socket)
            .args(["--print-shell", "project"]),
    );
    assert!(source.contains("'tmux'"));
    let source = assert_plan(
        tta()
            .env("TTA_BACKEND", "unsupported")
            .env("TTA_TRACE", "invalid-but-overridden")
            .env("TTA_ROOT", temp.path())
            .env("TTA_SOCKET_PATH", &socket)
            .args(["--backend", backend, "--trace=false", "--print-shell", "project"]),
    );
    assert!(source.contains("new-session"));
    for trace in ["true", "1", "false", "0", ""] {
        let output = tta()
            .env("TTA_TRACE", trace)
            .env("TTA_BACKEND", backend)
            .env("TTA_ROOT", temp.path())
            .env("TTA_SOCKET_PATH", &socket)
            .args(["--print-shell", "project"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let stderr = String::from_utf8_lossy(&output.stderr);
        if trace == "true" || trace == "1" {
            assert!(stderr.contains("lookup.start"), "{stderr}");
        } else {
            assert!(stderr.is_empty(), "{stderr}");
        }
        assert!(!String::from_utf8_lossy(&output.stdout).contains("lookup.start"));
    }
    #[cfg(not(all(feature = "native", feature = "cli")))]
    {
        let unavailable = if cfg!(feature = "native") { "cli" } else { "native" };
        let output = tta().env("TTA_BACKEND", unavailable).arg("project").output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("TTA_BACKEND"));
    }
}

#[test]
fn environment_client_paths_are_literal_and_cli_options_win() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("project")).unwrap();
    let backend = if cfg!(feature = "native") { "native" } else { "cli" };
    // CLI lookup needs a real executable; native lookup must not invoke this path.
    let executable = if cfg!(feature = "native") { "./missing ' client" } else { "tmux" };
    let mut command = tta();
    command
        .current_dir(temp.path())
        .env("TTA_ROOT", ".")
        .env("TTA_SOCKET_PATH", "socket")
        .env("TTA_TMUX", executable)
        .env("TTA_TMUX_CONFIG", "./config ' file")
        .args(["--backend", backend, "--print-shell", "project"]);
    let source = assert_plan(&mut command);
    assert!(source.contains("config "), "{source}");
    assert!(source.contains("'\\''"), "{source}");
    assert!(!temp.path().join("socket").exists());
    let source = assert_plan(command.args(["--tmux", "tmux", "--tmux-config", "/dev/null"]));
    assert!(source.contains("'-f' '/dev/null'"), "{source}");
    assert!(!source.contains("config "));
}

#[test]
fn existing_sessions_do_not_require_home_or_a_usable_environment_root() {
    struct Server(std::path::PathBuf);
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = Command::new("tmux").arg("-S").arg(&self.0).arg("kill-server").status();
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let server = Server(temp.path().join("socket"));
    let started = Command::new("tmux")
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .args(["-f", "/dev/null", "-S"])
        .arg(&server.0)
        .args(["new-session", "-d", "-s", "existing", "sleep", "60"])
        .output()
        .unwrap();
    assert!(started.status.success(), "{}", String::from_utf8_lossy(&started.stderr));
    for root in [None, Some("/tta-no-such-project-root")] {
        let mut command = tta();
        command
            .env_remove("HOME")
            .env("PROJECTS_DIR", "relative-needs-home")
            .env("TTA_SOCKET_PATH", &server.0)
            .args(["--print-shell", "existing"]);
        if let Some(root) = root {
            command.env("TTA_ROOT", root);
        }
        let source = assert_plan(&mut command);
        assert!(source.contains("attach-session"), "{source}");
        assert!(!source.contains("new-session"));
    }
}
