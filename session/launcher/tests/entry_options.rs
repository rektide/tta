use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

use tta_command::{Expression, Handoff};
use tta_launcher::{ClientMode, PlanError, Selection, Session, launch, socket};

#[test]
fn existing_attach_options_are_opt_in_and_startup_is_ignored() {
    let selection = Selection::Existing(Session {
        id: "$7".parse().unwrap(),
        name: "seed".into(),
    });
    for grouped in [false, true] {
        let expression = launch()
            .selection(&selection)
            .socket(&socket::Resolved::path("/tmp/opencode/sock"))
            .client_mode(ClientMode::Attach)
            .program(OsStr::new("tmux"))
            .grouped(grouped)
            .maybe_satellite_name(Some("_seed-1"))
            .startup(&["ignored".into()])
            .maybe_directory(Some(Path::new("/work/#{session_name}")))
            .call()
            .unwrap();
        let expected = if grouped {
            "'tmux' '-S' '/tmp/opencode/sock' 'new-session' '-d' '-s' '_seed-1' '-t' '$7' '-c' '/work/#{a:35}{session_name}' ';' 'attach-session' '-t' '=_seed-1' ';' 'set-option' '-t' '=_seed-1:' 'destroy-unattached' 'keep-last'"
        } else {
            "'tmux' '-S' '/tmp/opencode/sock' 'attach-session' '-t' '$7'"
        };
        assert_eq!(expression.shell(Handoff::Run).unwrap(), expected);
    }
}

#[test]
fn non_utf8_startup_is_supported_directly_but_not_in_shell_output() {
    let selection = Selection::Project {
        directory: "/project".into(),
        session_name: "project".into(),
    };
    let expression = launch()
        .selection(&selection)
        .socket(&socket::Resolved::path("/tmp/opencode/sock"))
        .client_mode(ClientMode::Switch)
        .program(OsStr::new("tmux"))
        .directory(Path::new("/override"))
        .startup(&["program".into(), OsString::from_vec(b"\xff\\;".to_vec())])
        .call()
        .unwrap();
    let Expression::And(and) = &expression else { panic!("expected sequence") };
    let Expression::ChangeDir(cd) = &and.steps[0] else { panic!("expected cd") };
    assert_eq!(cd.path.as_os_str(), "/override");
    let Expression::Command(create) = &and.steps[1] else {
        panic!("expected create")
    };
    assert_eq!(create.args.last().unwrap().as_os_str().as_encoded_bytes(), b"\xff\\\\;");
    assert!(matches!(expression.shell(Handoff::Run), Err(tta_command::Error::NonUtf8(_))));
}

#[test]
fn relative_directory_overrides_fail_before_execution() {
    let selection = Selection::Project {
        directory: "/project".into(),
        session_name: "project".into(),
    };
    assert!(matches!(
        launch()
            .selection(&selection)
            .socket(&socket::Resolved::path("/tmp/opencode/sock"))
            .client_mode(ClientMode::Attach)
            .program(OsStr::new("tmux"))
            .directory(Path::new("relative"))
            .call(),
        Err(PlanError::Directory(_))
    ));
}
