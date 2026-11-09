#![cfg(unix)]

use std::fs;
use std::process::Command;

mod common;

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
    let temp = common::scratch_dir("tta-cli-entry-");
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
fn window_name_defaults_to_the_session_and_overrides_or_disables() {
    let temp = common::scratch_dir("tta-cli-window-");
    fs::create_dir(temp.path().join("project")).unwrap();
    let plan = |extra: &[&str]| {
        let mut command = tta(&temp);
        command.args(extra).arg("project");
        let output = command.output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };
    // Absent and bare flag both name the initial window after the session.
    for extra in [&[][..], &["--window-name"][..]] {
        let source = plan(extra);
        assert!(
            source.contains(&format!(
                "'new-session' '-s' 'project' '-n' 'project' '-c' '{}'",
                temp.path().join("project").display()
            )),
            "{source}"
        );
    }
    // A custom name wins and stays one argv word.
    let source = plan(&["--window-name=main editor"]);
    assert!(source.contains("'new-session' '-s' 'project' '-n' 'main editor' '-c'"), "{source}");
    // false leaves the window to tmux automatic-rename: no -n at all.
    let source = plan(&["--window-name=false"]);
    assert!(!source.contains("'-n'"), "{source}");
    assert!(source.contains("'new-session' '-s' 'project'"), "{source}");
}

#[test]
fn double_dash_still_allows_a_fragment_starting_with_hyphen() {
    let temp = common::scratch_dir("tta-cli-entry-");
    fs::create_dir(temp.path().join("-project")).unwrap();
    let output = tta(&temp).args(["--", "-project", "program", "--flag", "space ;"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("'-s' '-project'"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("'--' 'program' '--flag' 'space \\;'"));
    assert!(!temp.path().join("sock").exists());
}

fn raw_tta(temp: &tempfile::TempDir) -> Command {
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
        .args(["--socket-path", "sock", "--print-shell"]);
    command
}

#[test]
fn repeated_roots_rank_globally_and_qualify_by_configured_slot() {
    let temp = common::scratch_dir("tta-cli-roots-");
    fs::create_dir_all(temp.path().join("personal/long-work")).unwrap();
    fs::create_dir_all(temp.path().join("team/work")).unwrap();
    // The shortest basename wins globally; the first root has no priority.
    let output = raw_tta(&temp)
        .args(["--root", "personal", "--root", "team", "work"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let source = String::from_utf8(output.stdout).unwrap();
    assert!(source.contains(&format!("'-c' '{}'", temp.path().join("team/work").display())), "{source}");
    assert!(source.contains("'-s' 'work'"), "{source}");
    // A duplicate basename in two physical directories is actionable, not guessed.
    fs::create_dir(temp.path().join("personal/work")).unwrap();
    let output = raw_tta(&temp)
        .args(["--root", "personal", "--root", "team", "work"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous project fragment"), "{stderr}");
    assert!(stderr.contains("@1/work") && stderr.contains("@2/work"), "{stderr}");
    // The qualified selector observes only its configured root slot.
    let output = raw_tta(&temp)
        .args(["--root", "personal", "--root", "team", "@2/work"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8(output.stdout).unwrap().contains(&format!("'-c' '{}'", temp.path().join("team/work").display())));
}

#[test]
fn root_environment_layers_replace_the_whole_list() {
    let temp = common::scratch_dir("tta-cli-root-env-");
    fs::create_dir_all(temp.path().join("json-a/alpha")).unwrap();
    fs::create_dir_all(temp.path().join("json-b/beta")).unwrap();
    fs::create_dir_all(temp.path().join("legacy/legacy-only")).unwrap();
    fs::create_dir_all(temp.path().join("src/home-only")).unwrap();
    let plan = |extra_env: &[(&str, &str)], extra_args: &[&str], argument: &str| {
        let mut command = raw_tta(&temp);
        for (key, value) in extra_env {
            command.env(key, value);
        }
        for arg in extra_args {
            command.arg(arg);
        }
        let output = command.arg(argument).output().unwrap();
        (output.status.success(), String::from_utf8_lossy(&output.stdout).into_owned(), String::from_utf8_lossy(&output.stderr).into_owned())
    };
    let json = r#"["json-a","json-b"]"#;
    // TTA_ROOTS replaces TTA_ROOT and PROJECTS_DIR as one active layer.
    let (ok, source, _) = plan(&[("TTA_ROOTS", json), ("TTA_ROOT", "legacy"), ("PROJECTS_DIR", "anything")], &[], "beta");
    assert!(ok);
    assert!(source.contains("json-b/beta") && !source.contains("legacy"), "{source}");
    // TTA_ROOT replaces PROJECTS_DIR; an empty TTA_ROOTS string is unset.
    let (ok, source, _) = plan(&[("TTA_ROOTS", ""), ("TTA_ROOT", "legacy"), ("PROJECTS_DIR", "anything")], &[], "legacy-only");
    assert!(ok);
    assert!(source.contains("legacy/legacy-only"), "{source}");
    // The HOME-relative PROJECTS_DIR default layer still works.
    let (ok, source, _) = plan(&[("PROJECTS_DIR", "")], &[], "home-only");
    assert!(ok);
    assert!(source.contains("src/home-only"), "{source}");
    // Explicit repeated roots ignore an invalid lower TTA_ROOTS value.
    let (ok, source, _) = plan(&[("TTA_ROOTS", "not JSON")], &["--root", "legacy"], "legacy-only");
    assert!(ok);
    assert!(source.contains("legacy/legacy-only"), "{source}");
    // An invalid active layer fails before lookup instead of falling through.
    for invalid in ["[]", "null", "not JSON", r#"[""]"#] {
        let (ok, _, stderr) = plan(&[("TTA_ROOTS", invalid)], &[], "alpha");
        assert!(!ok, "{invalid}");
        assert!(stderr.contains("project roots"), "{invalid}: {stderr}");
    }
}

#[test]
fn existing_grouped_plan_ignores_startup_and_preserves_session_first_matching() {
    let temp = common::scratch_dir("tta-cli-entry-");
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
    assert!(
        source.contains("';' 'set-option' '-t' '=_seed-1:' 'destroy-unattached' 'keep-last'"),
        "{source}"
    );
    assert!(!source.contains("must-not-run"));
}
