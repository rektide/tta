//! The same instrumented runtime driver powers `cargo verify` and these tests.
#![cfg(unix)]

use tta_verify::{Backend, Config, Mode, Scenario, exercise};

fn verify(backend: Backend, mode: Mode, scenario: Scenario, inside: bool) {
    let config = Config::builder().tta(env!("CARGO_BIN_EXE_tta").into()).backend(backend).inside(inside).build();
    let report = exercise(&config, mode, scenario);
    assert!(report.success, "runtime verification failed: {report:#?}");
}

macro_rules! cases {
    ($backend:ident) => {
        #[test]
        fn direct_new() {
            verify(Backend::$backend, Mode::Direct, Scenario::New, false);
        }
        #[test]
        fn direct_existing() {
            verify(Backend::$backend, Mode::Direct, Scenario::Existing, false);
        }
        #[test]
        fn shell_new() {
            verify(Backend::$backend, Mode::Shell, Scenario::New, false);
        }
        #[test]
        fn shell_existing() {
            verify(Backend::$backend, Mode::Shell, Scenario::Existing, false);
        }
        #[test]
        fn inside_direct_new() {
            verify(Backend::$backend, Mode::Direct, Scenario::New, true);
        }
        #[test]
        fn inside_direct_existing() {
            verify(Backend::$backend, Mode::Direct, Scenario::Existing, true);
        }
        #[test]
        fn inside_shell_new() {
            verify(Backend::$backend, Mode::Shell, Scenario::New, true);
        }
        #[test]
        fn inside_shell_existing() {
            verify(Backend::$backend, Mode::Shell, Scenario::Existing, true);
        }
    };
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
mod native {
    use super::*;
    cases!(Native);
}

#[cfg(feature = "cli")]
mod cli {
    use super::*;
    cases!(Cli);
}
