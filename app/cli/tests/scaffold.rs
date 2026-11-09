#![cfg(feature = "scaffold-rekon")]

use std::fs;
use std::process::{Command, Output};

mod common;

fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tta"));
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"TTA_") || key == "PROJECTS_DIR" {
            command.env_remove(key);
        }
    }
    command.env_remove("TMUX").env_remove("TMUX_PANE");
    command
}

fn tta(temp: &tempfile::TempDir) -> Command {
    let mut command = binary();
    command
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .env("HOME", temp.path())
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .args(["--root"])
        .arg(temp.path().join("projects"))
        .arg("--socket-path")
        .arg(temp.path().join("absent-sock"))
        .args(["--print-shell", "fresh"]);
    command
}

fn fixture() -> tempfile::TempDir {
    let temp = common::scratch_dir("tta-scaffold-cli-");
    fs::create_dir(temp.path().join("projects")).unwrap();
    fs::create_dir_all(temp.path().join("config/tta")).unwrap();
    fs::write(temp.path().join("config/tta/config.toml"), "[scaffold.generators.rk]\nadapter = 'rekon'\n").unwrap();
    temp
}

fn plan(output: Output) -> String {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn cli_layers_choice_then_uses_the_shared_registry_and_prefix() {
    let temp = fixture();
    let source = plan(tta(&temp).env("TTA_SCAFFOLD", "rk").output().unwrap());
    assert!(source.starts_with("'rekon' 'fresh' && cd -P "), "{source}");
    assert!(source.contains("'new-session' '-s' 'fresh'"), "{source}");
    let output = tta(&temp).env("TTA_SCAFFOLD", "rk").arg("--scaffold=false").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no session or immediate project directory matches"));
    let source = plan(tta(&temp).env("TTA_SCAFFOLD", "false").arg("--scaffold=rk").output().unwrap());
    assert!(source.starts_with("'rekon' 'fresh' && "));
    assert!(!temp.path().join("projects/fresh").exists());
    assert!(!temp.path().join("absent-sock").exists());
}

#[test]
fn explicit_config_overrides_xdg_and_completion_shares_its_names() {
    let temp = fixture();
    let explicit = temp.path().join("explicit.toml");
    fs::write(&explicit, "[scaffold.generators.chosen]\nadapter = 'rekon'\n").unwrap();
    let source = plan(tta(&temp).env("TTA_CONFIG", &explicit).arg("--scaffold=chosen").output().unwrap());
    assert!(source.starts_with("'rekon' 'fresh' && "));
    let output = tta(&temp).env("TTA_CONFIG", &explicit).arg("--scaffold=rk").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown scaffold generator"));
    let output = binary()
        .env("TTA_CONFIG", &explicit)
        .args(["--completion-candidates", "scaffolds"])
        .output()
        .unwrap();
    assert_eq!(plan(output), "chosen\n");
}

#[test]
fn cli_compiled_default_posture_is_preserved() {
    let temp = fixture();
    let output = tta(&temp).output().unwrap();
    if cfg!(feature = "scaffold-by-default") {
        assert!(plan(output).starts_with("'rekon' 'fresh' && "));
    } else {
        assert!(!output.status.success());
    }
}

#[test]
fn external_rekon_generation_fails_closed_across_repeated_roots() {
    let temp = fixture();
    fs::create_dir(temp.path().join("elsewhere")).unwrap();
    let run = |argument: &str| {
        binary()
            .env("HOME", temp.path())
            .env("XDG_CONFIG_HOME", temp.path().join("config"))
            .arg("--root")
            .arg(temp.path().join("projects"))
            .arg("--root")
            .arg(temp.path().join("elsewhere"))
            .arg("--socket-path")
            .arg(temp.path().join("absent-sock"))
            .args(["--print-shell", argument])
            .output()
            .unwrap()
    };
    // A qualified selector makes the destination unambiguous; the adapter
    // boundary still refuses because the external script cannot honor it.
    let output = run("@1/fresh");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("external rekon adapter"), "{stderr}");
    assert!(stderr.contains("rekon-native"), "{stderr}");
    // An unqualified clean miss across two roots is stopped earlier by the
    // destination guard, before any generator is consulted.
    let output = run("fresh");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot choose a scaffold destination"), "{stderr}");
    assert!(!stderr.contains("rekon-native"), "{stderr}");
    assert!(!temp.path().join("projects/fresh").exists());
    assert!(!temp.path().join("elsewhere/fresh").exists());
    assert!(!temp.path().join("absent-sock").exists());
}
