use super::*;
use crate::{ResolveError, SessionId};
use std::error::Error as _;

fn input() -> Input {
    Input::builder()
        .argument("work".into())
        .cwd("/target/cwd".into())
        .socket(socket::Resolved::path("/target/tmux.sock"))
        .client_mode(ClientMode::Attach)
        .program("/target/tmux".into())
        .registry(scaffold::Config::parse(None, None).unwrap())
        .scaffold("false".into())
        .root("/target/projects".into())
        .grouped(false)
        .build()
}

fn session(id: u32, name: &str, group: Option<&str>) -> Session {
    Session {
        id: format!("${id}").parse().unwrap(),
        name: name.into(),
        group: group.map(str::to_owned),
    }
}

fn observe(workflow: &Workflow) -> ObservationRequest {
    let Progress::Observe(request) = workflow.next().unwrap() else {
        panic!("expected observation")
    };
    request
}

fn shell(workflow: &Workflow) -> String {
    let Progress::Ready(prepared) = workflow.next().unwrap() else {
        panic!("expected readiness")
    };
    prepared.plan.expression().shell(tta_command::Handoff::Run).unwrap()
}

fn resolve_error(error: &Error) -> &ResolveError {
    let mut source = error.source();
    while let Some(current) = source {
        if let Some(error) = current.downcast_ref::<ResolveError>() {
            return error;
        }
        source = current.source();
    }
    panic!("lost typed resolve source: {error:?}")
}

#[test]
fn pull_and_clone_remain_immutable_send_sync_with_independent_native_facts() {
    fn assert_traits<T: Clone + Send + Sync>() {}
    assert_traits::<Workflow>();
    let mut workflow = Workflow::new(input()).unwrap();
    let pending = observe(&workflow);
    for _ in 0..3 {
        assert_eq!(observe(&workflow), pending);
        assert_eq!(observe(&workflow.clone()), pending);
    }
    workflow.supply(ObservationReply::Sessions(SessionInventory::Absent)).unwrap();
    let mut other = workflow.clone();
    assert!(matches!(observe(&workflow), ObservationRequest::Projects { .. }));
    for (copy, name) in [(&mut workflow, "work-a"), (&mut other, "work-b")] {
        copy.supply(ObservationReply::Projects(ProjectInventory {
            root: "/physical".into(),
            names: vec![name.into()],
        }))
        .unwrap();
    }
    assert!(shell(&workflow).contains("work-a"));
    assert!(shell(&other).contains("work-b"));
    std::thread::scope(|scope| {
        let first = scope.spawn(|| shell(&workflow));
        let second = scope.spawn(|| shell(&workflow));
        assert_eq!(first.join().unwrap(), second.join().unwrap());
    });
}

#[test]
fn typed_session_ambiguity_guards_before_missing_root_or_project_continuation() {
    let mut invocation = input();
    invocation.root = None;
    let mut workflow = Workflow::new(invocation).unwrap();
    // Supplying facts does not eagerly resolve them; the error still belongs to next().
    workflow
        .supply(ObservationReply::Sessions(SessionInventory::Present(vec![
            session(1, "work-a", None),
            session(2, "work-b", None),
        ])))
        .unwrap();
    for _ in 0..3 {
        let error = workflow.next().unwrap_err();
        assert!(matches!(resolve_error(&error), ResolveError::Sessions { candidates, .. } if candidates.len() == 2));
    }
    let error = workflow
        .supply(ObservationReply::Projects(ProjectInventory {
            root: "/wrong".into(),
            names: vec![],
        }))
        .unwrap_err();
    assert!(matches!(resolve_error(&error), ResolveError::Sessions { .. }));
}

#[test]
fn project_ambiguity_guards_before_scaffold_destination_and_mismatches_do_not_advance() {
    let mut workflow = Workflow::new(input()).unwrap();
    workflow.supply(ObservationReply::Sessions(SessionInventory::Absent)).unwrap();
    let pending = observe(&workflow);
    assert!(matches!(workflow.supply(ObservationReply::Destination(DestinationState::Absent)), Err(Error::UnexpectedReply(request)) if request == pending));
    assert_eq!(observe(&workflow), pending);
    workflow
        .supply(ObservationReply::Projects(ProjectInventory {
            root: "/physical".into(),
            names: vec!["work-a".into(), "work-b".into()],
        }))
        .unwrap();
    assert!(matches!(resolve_error(&workflow.next().unwrap_err()), ResolveError::Projects { .. }));
}

#[test]
fn exact_literal_shortest_and_actual_group_resolution_remain_a_guarded_native_stage() {
    let cases = [
        (vec![session(1, "_work-2", Some("g")), session(2, "work", None)], "$2"),
        (vec![session(1, "work-extra", None), session(2, "_work-2", Some("g"))], "$2"),
        (vec![session(1, "_work-42", Some("g")), session(2, "_work-2", Some("g"))], "$2"),
    ];
    for (sessions, expected) in cases {
        let mut invocation = input();
        invocation.root = None;
        let mut workflow = Workflow::new(invocation).unwrap();
        workflow.supply(ObservationReply::Sessions(SessionInventory::Present(sessions))).unwrap();
        let Progress::Ready(prepared) = workflow.next().unwrap() else {
            panic!("session hit consulted projects")
        };
        assert!(matches!(prepared.selection, Selection::Existing(session) if session.id == expected.parse::<SessionId>().unwrap()));
    }
}

#[test]
fn omitted_grouping_and_custom_orphan_alias_replay_preserve_fresh_inventory_boundary() {
    for (mode, explicit, independent) in [
        (ClientMode::Attach, None, true),
        (ClientMode::Switch, None, false),
        (ClientMode::Attach, Some(false), false),
        (ClientMode::Switch, Some(true), true),
    ] {
        let mut invocation = input();
        invocation.client_mode = mode;
        invocation.grouped = explicit;
        invocation.satellite_prefix = "view-".into();
        invocation.root = None;
        let selected = session(7, "view-work-42", Some("actual"));
        let mut workflow = Workflow::new(invocation).unwrap();
        workflow
            .supply(ObservationReply::Sessions(SessionInventory::Present(vec![selected.clone()])))
            .unwrap();
        if independent {
            let pending = observe(&workflow);
            assert!(matches!(pending, ObservationRequest::Sessions { .. }));
            assert_eq!(observe(&workflow), pending);
            let mut changed = workflow.clone();
            changed.supply(ObservationReply::Sessions(SessionInventory::Absent)).unwrap();
            assert!(matches!(changed.next(), Err(Error::SessionChanged(id)) if id == selected.id));
            workflow.supply(ObservationReply::Sessions(SessionInventory::Present(vec![selected]))).unwrap();
        }
        let output = shell(&workflow);
        assert_eq!(output.contains("new-session"), independent);
        assert_eq!(output.contains("view-work-1"), independent);
        assert!(!output.contains("view-view-work"));
        assert!(matches!(
            workflow.supply(ObservationReply::Sessions(SessionInventory::Absent)),
            Err(Error::AlreadyReady)
        ));
    }
}

#[cfg(unix)]
#[test]
fn native_non_utf8_domain_data_never_crosses_the_control_json_boundary() {
    use std::os::unix::ffi::OsStringExt;
    let mut invocation = input();
    invocation.program = OsString::from_vec(vec![b'/', b't', 0xff]);
    invocation.startup = vec![OsString::from_vec(vec![b'e', 0xfe])];
    let expected_program = invocation.program.clone();
    let mut workflow = Workflow::new(invocation).unwrap();
    assert!(matches!(observe(&workflow), ObservationRequest::Sessions { program, .. } if program == expected_program));
    workflow.supply(ObservationReply::Sessions(SessionInventory::Absent)).unwrap();
    workflow
        .supply(ObservationReply::Projects(ProjectInventory {
            root: "/physical".into(),
            names: vec!["work".into()],
        }))
        .unwrap();
    let Progress::Ready(prepared) = workflow.next().unwrap() else {
        panic!("not prepared")
    };
    let steps = prepared.plan.expression().steps();
    let command = steps
        .iter()
        .find_map(|step| {
            if let tta_command::Expression::Command(command) = step {
                Some(command)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(command.program.as_os_str(), expected_program);
    assert!(prepared.plan.expression().shell(tta_command::Handoff::Run).is_err());
}

#[cfg(feature = "scaffold-rekon-native")]
fn creation_input(options: &str) -> Input {
    let mut invocation = input();
    invocation.scaffold = "default".into();
    invocation.registry = scaffold::Config::parse(
        None,
        Some(&format!(
            "[scaffold.generators.native]\nadapter = 'rekon-native'\n[scaffold.generators.native.options]\n{options}"
        )),
    )
    .unwrap();
    invocation
}

#[cfg(feature = "scaffold-rekon-native")]
fn supply_creation_destination(workflow: &mut Workflow) {
    workflow.supply(ObservationReply::Sessions(SessionInventory::Absent)).unwrap();
    workflow
        .supply(ObservationReply::Projects(ProjectInventory {
            root: "/physical".into(),
            names: vec![],
        }))
        .unwrap();
    workflow.supply(ObservationReply::Destination(DestinationState::Absent)).unwrap();
}

#[cfg(feature = "scaffold-rekon-native")]
#[test]
fn generator_validation_guards_before_defaults_but_not_existing_selection() {
    let invocation = creation_input("brunch = 'main'\n");
    let mut existing = Workflow::new(invocation.clone()).unwrap();
    existing
        .supply(ObservationReply::Sessions(SessionInventory::Present(vec![session(1, "work", None)])))
        .unwrap();
    let Progress::Ready(prepared) = existing.next().unwrap() else {
        panic!("unused generator options blocked existing work")
    };
    assert!(prepared.generator.is_none());

    let mut creation = Workflow::new(invocation).unwrap();
    supply_creation_destination(&mut creation);
    // Moving all generator planning after defaults would incorrectly request
    // target Git/environment facts before rejecting these invalid options.
    let error = creation.next().unwrap_err();
    assert!(matches!(error, Error::Scaffold(_)));
    assert!(error.to_string().contains("unknown field"), "{error:?}");
}

#[cfg(feature = "scaffold-rekon-native")]
#[test]
fn absent_generator_defaults_are_supplied_facts_not_a_repeated_request() {
    let mut workflow = Workflow::new(creation_input("install = false\n")).unwrap();
    supply_creation_destination(&mut workflow);
    assert_eq!(
        observe(&workflow),
        ObservationRequest::GeneratorDefaults {
            directory: "/target/cwd".into(),
            author: true,
            license: true,
        }
    );
    workflow.supply(ObservationReply::GeneratorDefaults(scaffold::Defaults::default())).unwrap();
    assert!(shell(&workflow).contains("MIT"));
    assert_eq!(shell(&workflow), shell(&workflow.clone()));
}
