mod common;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use tta_command::{Expression, Handoff};
use tta_launcher::entry::{EntryAttempt, protocol::*};

#[test]
fn existing_attach_policy_ignores_startup_and_encodes_view_directory_only_for_independence() {
    for view in [ViewPreference::Shared, ViewPreference::Independent] {
        let mut i = common::input("seed");
        i.intent.view = view;
        i.intent.startup = vec!["ignored".into()];
        i.intent.view_directory = Some("/work/#{session_name}".into());
        let mut attempt = EntryAttempt::start(i).unwrap();
        common::answer(&mut attempt, common::seed());
        common::bind(&mut attempt, Delivery::Export);
        let Waiting::Finished(AttemptOutcome::Exported(action)) = attempt.waiting() else {
            panic!()
        };
        let shell = action.shell();
        assert!(!shell.contains("ignored"));
        assert_eq!(shell.contains("new-session"), view == ViewPreference::Independent);
        assert_eq!(shell.contains("/work/#{a:35}{session_name}"), view == ViewPreference::Independent);
        assert!(shell.contains("'$7'"));
    }
}

#[test]
fn native_byte_startup_survives_bound_action_but_portable_export_refuses_it() {
    let mut i = common::input("project");
    i.context.client = ClientEntry::SwitchWithinServer;
    i.intent.roots = vec!["/root".into()];
    i.intent.directory = Some("/override".into());
    i.intent.startup = vec!["program".into(), OsString::from_vec(b"\xff\\;".to_vec())];
    let mut attempt = EntryAttempt::start(i).unwrap();
    common::project(&mut attempt);
    common::bind(&mut attempt, Delivery::Execute { native: true });
    let Waiting::Effect(effect) = attempt.waiting() else {
        panic!("{:?}", attempt.snapshot());
    };
    // Raw argv forces a complete terminal-block shape before any mutation.
    let EffectKind::Terminal { expression } = &effect.kind else { panic!() };
    let Expression::And(and) = expression else { panic!() };
    let Expression::ChangeDir(cd) = &and.steps[0] else { panic!() };
    assert_eq!(cd.path.as_os_str(), "/override");
    let Expression::Command(create) = &and.steps[1] else { panic!() };
    assert_eq!(create.args.last().unwrap().as_os_str().as_encoded_bytes(), b"\xff\\\\;");
    assert!(matches!(expression.shell(Handoff::Run), Err(tta_command::Error::NonUtf8(_))));
}

#[test]
fn supplied_relative_directory_stops_preparation_before_delivery() {
    let mut i = common::input("project");
    i.intent.directory = Some("relative".into());
    i.intent.roots = vec!["/root".into()];
    let mut attempt = EntryAttempt::start(i).unwrap();
    common::project(&mut attempt);
    assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Stopped { cause, .. }) if matches!(&**cause, EntryError::Action(_))));
    assert!(!attempt.snapshot().journal.iter().any(|r| matches!(r, AttemptRecord::EffectIssued(_))));
}
