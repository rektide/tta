#![cfg(feature = "scaffold-rekon")]

use std::fs;
use std::process::{Command, Output};

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
    let temp = tempfile::Builder::new().prefix("tta-scaffold-cli-").tempdir_in("/tmp/opencode").unwrap();
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
