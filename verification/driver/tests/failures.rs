#![cfg(unix)]

use std::process::Command;
use std::time::Duration;
use tta_verify::{Backend, Config, Mode, Scenario, exercise};

#[test]
fn early_successful_exit_is_not_a_successful_attach() {
    let config = Config::builder()
        .tta("/bin/true".into())
        .backend(Backend::Native)
        .timeout(Duration::from_secs(1))
        .build();
    let report = exercise(&config, Mode::Direct, Scenario::New);
    assert!(!report.success);
    assert!(report.failure.as_deref().unwrap().contains("exited before attach"), "{report:#?}");
    assert_eq!(report.terminal_exit, Some(0));
    assert!(report.attached.is_none());
    assert!(!report.socket.exists());
}

#[test]
fn command_returns_nonzero_and_structured_failure_for_a_missing_executable() {
    let output = Command::new(env!("CARGO_BIN_EXE_tta-verify"))
        .args([
            "--tta",
            "/tta-test-nonexistent-executable",
            "--mode",
            "shell",
            "--scenario",
            "new",
            "--timeout",
            "1",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["success"], false);
    assert_eq!(report["runs"].as_array().unwrap().len(), 1);
    assert!(report["runs"][0]["failure"].as_str().unwrap().contains("tta-test-nonexistent-executable"));
    assert!(!report["runs"][0]["events"].as_array().unwrap().is_empty());
}
