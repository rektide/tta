use super::*;
use action::{BoundOperation, EntryDecision};
use select::{MatchReason, WorkspaceKey};
use std::error::Error as _;
use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use tta_command::{Expression, Handoff};

fn invocation(selector: &str) -> Invocation {
    Invocation {
        context: SuppliedContext {
            host: "supplied:test-host".into(),
            cwd: "/does/not/need/to/exist".into(),
            socket: crate::socket::Resolved::path("/no/socket/read/by/core"),
            program: "tmux".into(),
            config: None,
            client: ClientEntry::AttachTerminal,
        },
        intent: EntryIntent {
            selector: selector.into(),
            view: ViewPreference::Auto,
            satellite_prefix: "view-".into(),
            view_directory: None,
        },
        registry: crate::scaffold::Config::parse(None, None).unwrap(),
        generator: crate::scaffold::Choice::Off,
    }
}

fn session(id: u32, name: &str, group: Option<&str>) -> crate::Session {
    crate::Session {
        id: format!("${id}").parse().unwrap(),
        name: name.into(),
        group: group.map(str::to_owned),
    }
}

fn orphan() -> crate::Session {
    session(42, "view-project-42", Some("project"))
}

fn request(attempt: &EntryAttempt) -> ReadRequest {
    match attempt.waiting() {
        Waiting::Read(request) => request.clone(),
        _ => panic!("expected read: {:?}", attempt.snapshot()),
    }
}

fn supply(attempt: &mut EntryAttempt, sessions: Vec<crate::Session>) {
    attempt
        .advance(Event::Observed(ReadCompletion {
            request: request(attempt),
            result: Ok(Observation::Sessions(SessionInventory::Present(sessions))),
        }))
        .unwrap();
}

fn plan(attempt: &EntryAttempt) -> &action::EntryPlan {
    match attempt.waiting() {
        Waiting::Planned(plan) => plan,
        _ => panic!("expected plan: {:?}", attempt.snapshot()),
    }
}

fn export(attempt: &mut EntryAttempt, sessions: Vec<crate::Session>) -> &action::ExportedAction {
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    supply(attempt, sessions);
    match attempt.waiting() {
        Waiting::Finished(AttemptOutcome::Exported(exported)) => exported,
        _ => panic!("expected export: {:?}", attempt.snapshot()),
    }
}

fn stopped(attempt: &EntryAttempt) -> (&str, &EntryError) {
    match attempt.waiting() {
        Waiting::Finished(AttemptOutcome::Stopped { boundary, cause }) => (boundary, cause),
        _ => panic!("expected stopped: {:?}", attempt.snapshot()),
    }
}

fn controls(attempt: &EntryAttempt) -> Vec<ControlEvent> {
    attempt
        .journal
        .iter()
        .filter_map(|record| match record {
            AttemptRecord::Control(event) => Some(*event),
            _ => None,
        })
        .collect()
}

#[test]
fn orphan_only_work_reaches_one_exported_queue_with_no_filesystem_request() {
    let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
    let selection_request = request(&attempt);
    assert_eq!(selection_request.sessions_purpose(), SessionReadPurpose::Selection);
    supply(&mut attempt, vec![orphan()]);
    let plan = plan(&attempt);
    assert_eq!(plan.selection_request(), &selection_request);
    let EntryDecision::OpenView { work, .. } = plan.decision() else {
        panic!("outside auto must open a view")
    };
    assert_eq!(work.key, WorkspaceKey::Group("project".into()));
    assert_eq!(work.anchor.id, orphan().id);
    assert_eq!(work.matched_by, MatchReason::Alias { name: "project".into() });
    assert_eq!(work.observation, selection_request.id);
    let selection_plan = plan.clone();
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    let binding_request = request(&attempt);
    assert_eq!(binding_request.sessions_purpose(), SessionReadPurpose::BindAction);
    assert_eq!(binding_request.target, selection_request.target);
    assert_eq!(binding_request.endpoint(), selection_request.endpoint());
    assert_ne!(binding_request.id, selection_request.id);
    supply(&mut attempt, vec![orphan()]);
    let Waiting::Finished(AttemptOutcome::Exported(exported)) = attempt.waiting() else {
        panic!("export")
    };
    assert_eq!(
        exported.shell,
        "'tmux' '-S' '/no/socket/read/by/core' 'new-session' '-d' '-s' 'view-project-1' '-t' '$42' ';' 'attach-session' '-t' '=view-project-1' ';' 'set-option' '-t' '=view-project-1:' 'destroy-unattached' 'keep-last'"
    );
    assert_eq!(exported.expression.steps().len(), 1);
    assert!(matches!(exported.action.operation(), BoundOperation::OpenView { name, .. } if name == "view-project-1"));
    assert_eq!(exported.action.binding_request(), &binding_request);
    assert_eq!(attempt.snapshot().plan.as_ref(), Some(&selection_plan));
    let reads: Vec<_> = attempt
        .journal
        .iter()
        .filter_map(|r| match r {
            AttemptRecord::ReadIssued(r) => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(reads, [&selection_request, &binding_request]);
    assert_eq!(
        controls(&attempt),
        [
            ControlEvent::Validated,
            ControlEvent::Observed,
            ControlEvent::Unique,
            ControlEvent::ViewPolicy { independent: true },
            ControlEvent::Prepared,
            ControlEvent::Export,
            ControlEvent::Observed,
            ControlEvent::Checked,
            ControlEvent::Bound,
            ControlEvent::Exported
        ]
    );
}

fn assert_inert(attempt: &EntryAttempt) {
    let before = format!("{:?}", attempt.snapshot());
    for _ in 0..10 {
        let _ = attempt.waiting();
        assert_eq!(format!("{:?}", attempt.snapshot()), before);
    }
}

#[test]
fn every_inspection_boundary_is_inert_and_artifacts_materialize_once() {
    let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
    assert_inert(&attempt);
    supply(&mut attempt, vec![orphan()]);
    assert_inert(&attempt);
    let address = plan(&attempt) as *const _;
    for _ in 0..10 {
        assert_eq!(plan(&attempt) as *const _, address);
    }
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    assert_inert(&attempt);
    supply(&mut attempt, vec![orphan()]);
    assert_inert(&attempt);
    for directive in [
        Directive::ValidateRegistry,
        Directive::RankSessions,
        Directive::SettleView,
        Directive::PlanOpenView,
        Directive::CheckBinding,
        Directive::BindOperation,
        Directive::ExportAction,
    ] {
        assert_eq!(attempt.journal.iter().filter(|r| **r == AttemptRecord::Directive(directive)).count(), 1);
    }
}

#[test]
fn wrong_attempt_target_endpoint_purpose_program_or_sequence_never_advances() {
    let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
    let other = EntryAttempt::start(invocation("project")).unwrap();
    let expected = request(&attempt);
    let foreign = request(&other);
    let mut variants = vec![foreign.clone()];
    let mut changed = expected.clone();
    changed.id = foreign.id;
    variants.push(changed);
    let mut changed = expected.clone();
    changed.target = foreign.target;
    variants.push(changed);
    let mut changed = expected.clone();
    changed.id.sequence += 1;
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { endpoint, .. } = &mut changed.kind;
    endpoint.socket = "/other/socket".into();
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { endpoint, .. } = &mut changed.kind;
    endpoint.target = foreign.target;
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { purpose, .. } = &mut changed.kind;
    *purpose = SessionReadPurpose::BindAction;
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { program, .. } = &mut changed.kind;
    *program = "different-tmux".into();
    variants.push(changed);
    let before = format!("{:?}", attempt.snapshot());
    for request in variants {
        let result = attempt.advance(Event::Observed(ReadCompletion {
            request,
            result: Ok(Observation::Sessions(SessionInventory::Present(vec![orphan()]))),
        }));
        assert!(matches!(result, Err(ProtocolError::ReadMismatch { .. })));
        assert_eq!(format!("{:?}", attempt.snapshot()), before);
    }
    assert!(matches!(attempt.advance(Event::Deliver(Delivery::Export)), Err(ProtocolError::NotPlanned)));
    assert_eq!(format!("{:?}", attempt.snapshot()), before);
    supply(&mut attempt, vec![orphan()]);
    assert!(matches!(attempt.waiting(), Waiting::Planned(_)));
}

#[test]
fn duplicate_and_stale_replies_are_rejected_at_plan_binding_and_finish() {
    let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
    let initial = request(&attempt);
    supply(&mut attempt, vec![orphan()]);
    let reply = || {
        Event::Observed(ReadCompletion {
            request: initial.clone(),
            result: Ok(Observation::Sessions(SessionInventory::Absent)),
        })
    };
    let before = format!("{:?}", attempt.snapshot());
    assert!(matches!(attempt.advance(reply()), Err(ProtocolError::NoPendingRead)));
    assert_eq!(format!("{:?}", attempt.snapshot()), before);
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    let binding = request(&attempt);
    let before = format!("{:?}", attempt.snapshot());
    assert!(matches!(attempt.advance(reply()), Err(ProtocolError::ReadMismatch { .. })));
    assert!(matches!(attempt.advance(Event::Deliver(Delivery::Export)), Err(ProtocolError::NotPlanned)));
    assert_eq!(format!("{:?}", attempt.snapshot()), before);
    supply(&mut attempt, vec![orphan()]);
    let before = format!("{:?}", attempt.snapshot());
    assert!(matches!(
        attempt.advance(Event::Observed(ReadCompletion {
            request: binding,
            result: Ok(Observation::Sessions(SessionInventory::Absent))
        })),
        Err(ProtocolError::NoPendingRead)
    ));
    assert!(matches!(attempt.advance(Event::Cancel), Err(ProtocolError::Finished)));
    assert_eq!(format!("{:?}", attempt.snapshot()), before);
}

#[test]
fn ambiguity_is_a_native_guard_before_unique_or_empty_and_never_a_plan() {
    for sessions in [
        vec![orphan(), session(9, "view-project-9", Some("other"))],
        vec![session(1, "view-project-1", None), session(2, "view-project-2", None)],
    ] {
        let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
        supply(&mut attempt, sessions.clone());
        let (boundary, error) = stopped(&attempt);
        assert_eq!(boundary, "sessions_rank");
        assert!(matches!(error, EntryError::Ambiguous { candidates, .. } if candidates.len() == 2));
        assert!(!controls(&attempt).contains(&ControlEvent::Empty));
        assert!(!controls(&attempt).contains(&ControlEvent::Unique));
        assert!(attempt.snapshot().plan.is_none());
        assert!(attempt.snapshot().bound.is_none());
        assert_inert(&attempt);
    }
}

#[derive(Debug, thiserror::Error)]
#[error("incomplete observation at {path:?}")]
struct NativeReadFailure {
    path: std::path::PathBuf,
    #[source]
    source: std::io::Error,
}

#[test]
fn incomplete_reads_stop_natively_with_original_bytes_type_and_nested_source_chain() {
    for binding in [false, true] {
        let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
        if binding {
            supply(&mut attempt, vec![orphan()]);
            attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
        }
        let path = std::path::PathBuf::from(OsString::from_vec(b"/bad/\xff".to_vec()));
        attempt
            .advance(Event::Observed(ReadCompletion {
                request: request(&attempt),
                result: Err(ObservationFailure {
                    source: Box::new(NativeReadFailure {
                        path: path.clone(),
                        source: std::io::Error::from_raw_os_error(13),
                    }),
                }),
            }))
            .unwrap();
        let (boundary, error) = stopped(&attempt);
        assert_eq!(boundary, if binding { "binding_read" } else { "sessions_read" });
        let native = error.source().unwrap().source().unwrap().downcast_ref::<NativeReadFailure>().unwrap();
        assert_eq!(native.path, path);
        assert_eq!(native.source().unwrap().downcast_ref::<std::io::Error>().unwrap().raw_os_error(), Some(13));
        assert!(!controls(&attempt).contains(&ControlEvent::Empty));
        assert_eq!(attempt.snapshot().plan.is_some(), binding);
        assert!(attempt.snapshot().bound.is_none());
        let AttemptOutcome::Stopped { cause: first, .. } = attempt.snapshot().outcome.unwrap() else {
            panic!()
        };
        let AttemptOutcome::Stopped { cause: second, .. } = attempt.snapshot().outcome.unwrap() else {
            panic!()
        };
        assert!(Arc::ptr_eq(&first, &second));
    }
}

#[test]
fn classified_absence_and_complete_empty_are_not_failed_reads_but_discovery_is_staged() {
    for inventory in [SessionInventory::Absent, SessionInventory::Present(vec![])] {
        let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
        attempt
            .advance(Event::Observed(ReadCompletion {
                request: request(&attempt),
                result: Ok(Observation::Sessions(inventory)),
            }))
            .unwrap();
        assert_eq!(stopped(&attempt).0, "project_scope");
        assert!(matches!(stopped(&attempt).1, EntryError::DiscoveryNotImplemented));
        assert!(controls(&attempt).contains(&ControlEvent::Empty));
        assert!(attempt.snapshot().plan.is_none());
    }
}

#[test]
fn malformed_inventory_is_not_ambiguity_or_absence() {
    for sessions in [
        vec![orphan(), session(42, "different", None)],
        vec![orphan(), session(9, "view-project-42", None)],
        vec![session(9, "bad:name", None)],
        vec![session(9, "project", Some(""))],
    ] {
        let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
        supply(&mut attempt, sessions);
        assert_eq!(stopped(&attempt).0, "sessions_read");
        assert!(matches!(stopped(&attempt).1, EntryError::Inventory(_) | EntryError::InventoryName(_)));
        assert!(!controls(&attempt).contains(&ControlEvent::Empty));
        assert!(!controls(&attempt).contains(&ControlEvent::Unique));
        assert!(attempt.snapshot().plan.is_none());
    }
}

#[test]
fn literal_alias_unicode_shortest_and_actual_group_rules_survive_semantic_selection() {
    let cases = [
        ("@2/foo", vec![session(1, "@2/foo", None)], 1, MatchReason::Literal),
        (
            "*[",
            vec![session(1, "a*[b", None), session(2, "ab", None)],
            1,
            MatchReason::Substring { name: "a*[b".into() },
        ),
        ("project", vec![orphan(), session(9, "project", None)], 9, MatchReason::Literal),
        (
            "view-project-9",
            vec![orphan(), session(9, "view-project-9", Some("project"))],
            9,
            MatchReason::Literal,
        ),
        (
            "project",
            vec![orphan(), session(7, "view-project-7", Some("project"))],
            42,
            MatchReason::Alias { name: "project".into() },
        ),
        (
            "pro",
            vec![orphan(), session(9, "proj", None)],
            9,
            MatchReason::Substring { name: "proj".into() },
        ),
        (
            "a",
            vec![session(1, "éa", None), session(2, "aaa", None)],
            1,
            MatchReason::Substring { name: "éa".into() },
        ),
        (
            "_project-1",
            vec![session(1, "__project-1-2", None)],
            1,
            MatchReason::Alias { name: "_project-1".into() },
        ),
    ];
    for (selector, sessions, winner, reason) in cases {
        let mut input = invocation(selector);
        if selector == "_project-1" {
            input.intent.satellite_prefix = "_".into();
        }
        let mut attempt = EntryAttempt::start(input).unwrap();
        supply(&mut attempt, sessions);
        let EntryDecision::OpenView { work, .. } = plan(&attempt).decision() else {
            panic!()
        };
        assert_eq!(work.anchor.id, format!("${winner}").parse().unwrap());
        assert_eq!(work.matched_by, reason);
        if work.anchor.group.is_none() {
            assert_eq!(work.key, WorkspaceKey::Singleton(work.anchor.id));
        }
    }
    // A longer member is not admitted by group collapse before global ranking.
    let mut attempt = EntryAttempt::start(invocation("foo")).unwrap();
    supply(
        &mut attempt,
        vec![session(1, "afoo", Some("a")), session(2, "zfoo", None), session(3, "longfoo", Some("a"))],
    );
    let EntryError::Ambiguous { candidates, .. } = stopped(&attempt).1 else {
        panic!()
    };
    assert_eq!(candidates.iter().map(|s| s.id.to_string()).collect::<Vec<_>>(), ["$1", "$2"]);
}

#[test]
fn allocation_uses_fresh_global_complete_names_and_nonrecursive_group_base() {
    for (fresh, expected) in [
        (
            vec![
                orphan(),
                session(1, "view-project-1", Some("unrelated")),
                session(3, "view-project-3", Some("project")),
            ],
            "view-project-2",
        ),
        (vec![orphan(), session(1, "view-project-01", None)], "view-project-1"),
        (
            vec![orphan(), session(1, "renamed", Some("project")), session(2, "p", Some("unrelated"))],
            "view-renamed-1",
        ),
    ] {
        let mut attempt = EntryAttempt::start(invocation("view-project-42")).unwrap();
        supply(&mut attempt, vec![orphan()]);
        let action = &export(&mut attempt, fresh).action;
        assert!(matches!(action.operation(), BoundOperation::OpenView { name, .. } if name == expected));
    }
}

#[test]
fn shared_and_independent_policy_settles_once_with_explicit_override_and_same_queue() {
    for client in [ClientEntry::AttachTerminal, ClientEntry::SwitchWithinServer] {
        for preference in [ViewPreference::Auto, ViewPreference::Shared, ViewPreference::Independent] {
            let mut input = invocation("project");
            input.context.client = client;
            input.intent.view = preference;
            let independent = preference == ViewPreference::Independent || (preference == ViewPreference::Auto && client == ClientEntry::AttachTerminal);
            let mut attempt = EntryAttempt::start(input).unwrap();
            supply(&mut attempt, vec![orphan()]);
            assert_eq!(matches!(plan(&attempt).decision(), EntryDecision::OpenView { .. }), independent);
            assert_eq!(plan(&attempt).basis().matched_by, MatchReason::Alias { name: "project".into() });
            assert_eq!(plan(&attempt).basis().workspace, WorkspaceKey::Group("project".into()));
            let exported = export(&mut attempt, vec![orphan()]);
            let operation = if client == ClientEntry::AttachTerminal {
                "attach-session"
            } else {
                "switch-client"
            };
            assert!(exported.shell.contains(operation));
            assert_eq!(exported.shell.contains("new-session"), independent);
            assert_eq!(exported.shell.contains("keep-last"), independent);
            if !independent {
                assert!(exported.shell.ends_with("'-t' '$42'"));
            }
            assert_eq!(exported.expression.steps().len(), 1);
        }
    }
}

#[test]
fn both_decisions_fail_closed_on_missing_renamed_or_regrouped_anchor_without_reranking() {
    for preference in [ViewPreference::Independent, ViewPreference::Shared] {
        for fresh in [
            vec![],
            vec![session(42, "renamed", Some("project"))],
            vec![session(42, "view-project-42", Some("different"))],
            vec![session(42, "view-project-42", None)],
            vec![session(7, "view-project-7", Some("project"))],
        ] {
            let mut input = invocation("project");
            input.intent.view = preference;
            let mut attempt = EntryAttempt::start(input).unwrap();
            supply(&mut attempt, vec![orphan()]);
            let original = plan(&attempt).clone();
            attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
            supply(&mut attempt, fresh);
            let (boundary, error) = stopped(&attempt);
            assert_eq!(boundary, "binding_check");
            assert!(matches!(error, EntryError::AnchorChanged { expected, .. } if expected.id == orphan().id));
            assert_eq!(attempt.snapshot().plan.as_ref(), Some(&original));
            assert!(attempt.snapshot().bound.is_none());
            assert_eq!(
                attempt
                    .journal
                    .iter()
                    .filter(|r| **r == AttemptRecord::Directive(Directive::RankSessions))
                    .count(),
                1
            );
            assert!(!controls(&attempt).contains(&ControlEvent::Empty));
        }
    }
}

#[test]
fn registry_choice_fails_before_first_read_without_adapter_materialization() {
    let mut input = invocation("project");
    input.generator = crate::scaffold::Choice::Named("missing".into());
    let attempt = EntryAttempt::start(input).unwrap();
    assert_eq!(stopped(&attempt).0, "bind_registry");
    assert!(matches!(stopped(&attempt).1, EntryError::Registry(_)));
    assert!(!attempt.journal.iter().any(|r| matches!(r, AttemptRecord::ReadIssued(_))));
}

#[cfg(feature = "scaffold-rekon-native")]
#[test]
fn existing_work_does_not_validate_unused_adapter_options_or_request_defaults() {
    let mut input = invocation("project");
    input.registry = crate::scaffold::Config::parse(
        None,
        Some("[scaffold.generators.bad]\nadapter='rekon-native'\n[scaffold.generators.bad.options]\nunknown=true\n"),
    )
    .unwrap();
    input.generator = crate::scaffold::Choice::Named("bad".into());
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, vec![orphan()]);
    assert!(matches!(plan(&attempt).decision(), EntryDecision::OpenView { .. }));
    assert!(export(&mut attempt, vec![orphan()]).shell.contains("keep-last"));
}

#[test]
fn native_program_socket_config_and_cwd_bytes_survive_actor_and_errors_without_lossy_serde() {
    for field in ["program", "socket", "config", "cwd"] {
        let raw = OsString::from_vec(b"/native/\xff".to_vec());
        let mut input = invocation("project");
        input.intent.view = ViewPreference::Shared;
        match field {
            "program" => input.context.program = raw.clone(),
            "socket" => input.context.socket = crate::socket::Resolved::path(raw.clone()),
            "config" => input.context.config = Some(raw.clone().into()),
            "cwd" => input.context.cwd = raw.clone().into(),
            _ => unreachable!(),
        }
        let mut attempt = EntryAttempt::start(input).unwrap();
        let ReadKind::Sessions { endpoint, program, .. } = request(&attempt).kind;
        if field == "program" {
            assert_eq!(program, raw);
        }
        if field == "socket" {
            assert_eq!(endpoint.socket.as_os_str(), raw);
        }
        supply(&mut attempt, vec![orphan()]);
        let bound_target = plan(&attempt).target().clone();
        attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
        supply(&mut attempt, vec![orphan()]);
        assert_eq!(attempt.snapshot().bound.as_ref().unwrap().target, bound_target);
        if field == "cwd" {
            assert_eq!(bound_target.cwd.as_os_str(), raw);
            assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Exported(_))));
        } else {
            assert_eq!(stopped(&attempt).0, "export_action");
            let EntryError::Action(action::Error::Command(tta_command::Error::NonUtf8(value))) = stopped(&attempt).1 else {
                panic!()
            };
            assert_eq!(value.as_bytes(), raw.as_bytes());
            // The exact bound operation can still lower to native argv. It is
            // only shell rendering that failed; no data was serialized or lost.
            let expression = action::lower(attempt.snapshot().bound.as_ref().unwrap()).unwrap();
            let Expression::Command(command) = expression else { panic!() };
            if field == "program" {
                assert_eq!(command.program.as_os_str(), raw);
            }
            if field == "socket" || field == "config" {
                assert!(command.args.iter().any(|arg| arg.as_os_str() == raw));
            }
        }
    }
}

#[test]
fn format_escaping_semicolon_boundaries_and_socket_label_policy_belong_to_lowering() {
    let mut input = invocation("#[seed];");
    input.intent.satellite_prefix = "#view-".into();
    input.intent.view_directory = Some("/dir/#[x];".into());
    input.context.config = Some("/tmux config".into());
    input.context.socket = crate::socket::Resolved {
        path: "/private/tmp/tmux-1000/work".into(),
        label: Some(crate::socket::Label {
            name: "work".into(),
            directory: "/private/tmp".into(),
        }),
    };
    let selected = session(7, "#[seed];", None);
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, vec![selected.clone()]);
    let EntryDecision::OpenView { naming, .. } = plan(&attempt).decision() else {
        panic!()
    };
    assert_eq!(naming.directory.as_ref().unwrap().path(), std::path::Path::new("/dir/#[x];"));
    let exported = export(&mut attempt, vec![selected]);
    let Expression::Command(command) = &exported.expression else { panic!() };
    assert_eq!(command.environment[0].name(), "TMUX_TMPDIR");
    assert_eq!(command.environment[0].value().as_os_str(), "/private/tmp");
    let args: Vec<_> = command.args.iter().map(|arg| arg.as_os_str().to_str().unwrap()).collect();
    assert_eq!(
        args,
        [
            "-L",
            "work",
            "-f",
            "/tmux config",
            "new-session",
            "-d",
            "-s",
            "#{a:35}view-#{a:35}[seed];-1",
            "-t",
            "$7",
            "-c",
            "/dir/#{a:35}[x]\\;",
            ";",
            "attach-session",
            "-t",
            "=#view-#[seed];-1",
            ";",
            "set-option",
            "-t",
            "=#view-#[seed];-1:",
            "destroy-unattached",
            "keep-last"
        ]
    );
    assert_eq!(exported.expression.shell(Handoff::Run).unwrap(), exported.shell);
}

#[test]
fn cancellation_at_any_external_boundary_cannot_accept_a_later_read_or_deliver() {
    for boundary in ["read", "planned", "binding"] {
        let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
        let stale = request(&attempt);
        if boundary != "read" {
            supply(&mut attempt, vec![orphan()]);
        }
        if boundary == "binding" {
            attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
        }
        attempt.advance(Event::Cancel).unwrap();
        assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Cancelled { .. })));
        assert!(attempt.snapshot().pending.is_none());
        assert!(attempt.snapshot().bound.is_none());
        let before = format!("{:?}", attempt.snapshot());
        assert!(matches!(
            attempt.advance(Event::Observed(ReadCompletion {
                request: stale,
                result: Ok(Observation::Sessions(SessionInventory::Absent))
            })),
            Err(ProtocolError::NoPendingRead)
        ));
        assert!(matches!(attempt.advance(Event::Deliver(Delivery::Export)), Err(ProtocolError::NotPlanned)));
        assert_eq!(format!("{:?}", attempt.snapshot()), before);
    }
}

#[test]
fn input_validation_rejects_unfrozen_paths_and_bad_prefixes_without_host_reads() {
    for field in ["cwd", "socket", "config", "prefix", "selector", "program"] {
        let mut input = invocation("project");
        match field {
            "cwd" => input.context.cwd = "relative".into(),
            "socket" => input.context.socket = crate::socket::Resolved::path("relative"),
            "config" => input.context.config = Some("relative".into()),
            "prefix" => input.intent.satellite_prefix = "bad:".into(),
            "selector" => input.intent.selector.clear(),
            "program" => input.context.program = OsString::from_vec(b"bad\0program".to_vec()),
            _ => unreachable!(),
        }
        assert!(EntryAttempt::start(input).is_err());
    }
}

#[test]
fn view_directory_is_constructor_checked_native_data_and_unused_for_shared_entry() {
    for path in [
        std::path::PathBuf::from("relative"),
        OsString::from_vec(b"/bad/\xff".to_vec()).into(),
        OsString::from_vec(b"/bad/\0".to_vec()).into(),
    ] {
        let mut input = invocation("project");
        input.intent.view_directory = Some(path.clone());
        let mut attempt = EntryAttempt::start(input).unwrap();
        supply(&mut attempt, vec![orphan()]);
        assert_eq!(stopped(&attempt).0, "open_view");
        assert!(attempt.snapshot().plan.is_none());
        match stopped(&attempt).1 {
            EntryError::Action(action::Error::Directory(value)) => assert_eq!(value, &path),
            EntryError::Action(action::Error::Command(tta_command::Error::Nul)) => assert!(path.as_os_str().as_bytes().contains(&0)),
            error => panic!("unexpected error: {error:?}"),
        }
        let mut input = invocation("project");
        input.intent.view = ViewPreference::Shared;
        input.intent.view_directory = Some(path);
        let mut attempt = EntryAttempt::start(input).unwrap();
        supply(&mut attempt, vec![orphan()]);
        assert!(matches!(plan(&attempt).decision(), EntryDecision::EnterView { .. }));
        assert!(!export(&mut attempt, vec![orphan()]).shell.contains("'-c'"));
    }
}
