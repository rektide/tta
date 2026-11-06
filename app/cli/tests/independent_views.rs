//! Positive regression with real terminal clients, not control mode clients.
#![cfg(unix)]
use tta_verify::{Backend, Config, Mode};

fn verify(backend: Backend) {
    let config = Config::builder().tta(env!("CARGO_BIN_EXE_tta").into()).backend(backend).build();
    for mode in [Mode::Direct, Mode::Shell] {
        for prefix in ["_", "view-"] {
            let report = tta_verify::views::exercise(&config, mode, prefix);
            assert!(report.success, "two-terminal regression: {report:#?}");
            assert_eq!(report.checks.len(), 3);
        }
    }
}

#[cfg(feature = "native")]
#[test]
fn native_default_views_share_processes_not_window_selection() {
    verify(Backend::Native);
}

#[cfg(feature = "cli")]
#[test]
fn cli_default_views_share_processes_not_window_selection() {
    verify(Backend::Cli);
}
