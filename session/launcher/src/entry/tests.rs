use super::*;
use action::{BoundOperation, EntryDecision};
use protocol::{DestinationEvidence, RootSlot};
use select::{MatchReason, WorkspaceKey};
use std::error::Error as _;
use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use tta_command::{Expression, Handoff};

fn invocation(selector: &str) -> Invocation {
    invocation_roots(selector, Vec::new())
}

fn invocation_roots(selector: &str, roots: Vec<PathBuf>) -> Invocation {
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
            roots,
            session_name: None,
            startup: Vec::new(),
            directory: None,
            window: InitialWindow::default(),
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

/// A root inventory observation: requested spelling, observed physical root
/// and immediate entries with their canonical directories.
fn inventory(requested: &str, physical: &str, entries: &[(&str, &str)]) -> crate::ProjectInventory {
    crate::ProjectInventory {
        requested_root: requested.into(),
        root: physical.into(),
        entries: entries
            .iter()
            .map(|(name, directory)| crate::ProjectEntry {
                name: (*name).into(),
                directory: (*directory).into(),
            })
            .collect(),
    }
}

fn request(attempt: &EntryAttempt) -> ReadRequest {
    match attempt.waiting() {
        Waiting::Read(request) => request.clone(),
        _ => panic!("expected read: {:?}", attempt.snapshot()),
    }
}

fn supply_observation(attempt: &mut EntryAttempt, result: Result<Observation, ObservationFailure>) {
    attempt
        .advance(Event::Observed(ReadCompletion {
            request: request(attempt),
            result,
        }))
        .unwrap();
}

fn supply(attempt: &mut EntryAttempt, sessions: Vec<crate::Session>) {
    supply_observation(attempt, Ok(Observation::Sessions(SessionInventory::Present(sessions))));
}

fn supply_project(attempt: &mut EntryAttempt, observed: crate::ProjectInventory) {
    supply_observation(attempt, Ok(Observation::Projects(observed)));
}

fn plan(attempt: &EntryAttempt) -> &action::EntryPlan {
    match attempt.waiting() {
        Waiting::Planned(plan) => plan,
        _ => panic!("expected plan: {:?}", attempt.snapshot()),
    }
}

fn deliver(attempt: &mut EntryAttempt) {
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
}

fn export(attempt: &mut EntryAttempt, sessions: Vec<crate::Session>) -> &action::ExportedAction {
    deliver(attempt);
    supply(attempt, sessions);
    match attempt.waiting() {
        Waiting::Finished(AttemptOutcome::Exported(exported)) => exported,
        _ => panic!("expected export: {:?}", attempt.snapshot()),
    }
}

/// Deliver a new-work plan: fresh session inventory, then the binding
/// evidence read (route identity or destination absence) it requires.
fn export_new_work(attempt: &mut EntryAttempt, sessions: Vec<crate::Session>, evidence: Observation) -> &action::ExportedAction {
    deliver(attempt);
    supply(attempt, sessions);
    supply_observation(attempt, Ok(evidence));
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

fn reads(attempt: &EntryAttempt) -> Vec<&ReadRequest> {
    attempt
        .journal
        .iter()
        .filter_map(|record| match record {
            AttemptRecord::ReadIssued(request) => Some(request),
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
    assert_eq!(exported.action.binding_requests(), vec![&binding_request]);
    assert_eq!(attempt.snapshot().plan.as_ref(), Some(&selection_plan));
    assert_eq!(reads(&attempt), [&selection_request, &binding_request]);
    assert_eq!(
        controls(&attempt),
        [
            ControlEvent::Validated,
            ControlEvent::Observed,
            ControlEvent::Unique,
            ControlEvent::ViewPolicy { independent: true },
            ControlEvent::Prepared,
            ControlEvent::Export,
            ControlEvent::BindingObserved { evidence_needed: false },
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
    let ReadKind::Sessions { endpoint, .. } = &mut changed.kind else {
        panic!("sessions kind")
    };
    endpoint.socket = "/other/socket".into();
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { endpoint, .. } = &mut changed.kind else {
        panic!("sessions kind")
    };
    endpoint.target = foreign.target;
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { purpose, .. } = &mut changed.kind else {
        panic!("sessions kind")
    };
    *purpose = SessionReadPurpose::BindAction;
    variants.push(changed);
    let mut changed = expected.clone();
    let ReadKind::Sessions { program, .. } = &mut changed.kind else {
        panic!("sessions kind")
    };
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
fn classified_absence_and_complete_empty_reach_project_scope_and_rootless_fallback_needs_roots() {
    for inventory in [SessionInventory::Absent, SessionInventory::Present(vec![])] {
        let mut attempt = EntryAttempt::start(invocation("project")).unwrap();
        attempt
            .advance(Event::Observed(ReadCompletion {
                request: request(&attempt),
                result: Ok(Observation::Sessions(inventory)),
            }))
            .unwrap();
        assert_eq!(stopped(&attempt).0, "project_scope");
        // A genuine session miss without configured roots is a typed scope
        // error, never absence or a fallback to creation.
        assert!(matches!(stopped(&attempt).1, EntryError::Discovery(source) if matches!(source, crate::ResolveError::MissingRoot)));
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
            let action::DecisionBasis::Workspace { matched_by, workspace, .. } = plan(&attempt).basis() else {
                panic!("workspace basis")
            };
            assert_eq!(matched_by, &MatchReason::Alias { name: "project".into() });
            assert_eq!(workspace, &WorkspaceKey::Group("project".into()));
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
        let request = request(&attempt);
        let ReadKind::Sessions { endpoint, program, .. } = request.kind else {
            panic!("sessions kind")
        };
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

// --- machine-owned discovery: scope, root loop, physical ranking ---

#[test]
fn literal_session_selector_wins_before_project_qualification_even_with_roots() {
    let mut attempt = EntryAttempt::start(invocation_roots("@2/foo", vec!["/r1".into(), "/r2".into()])).unwrap();
    supply(&mut attempt, vec![session(7, "@2/foo", None)]);
    // Session-first on the ORIGINAL literal: the project-looking selector
    // never reaches project scope and no root observation is ever requested.
    assert!(matches!(plan(&attempt).decision(), EntryDecision::OpenView { .. }));
    assert!(reads(&attempt).iter().all(|request| matches!(request.kind, ReadKind::Sessions { .. })));
}

#[test]
fn qualified_selector_observes_only_its_selected_root_and_starts_the_project() {
    let mut attempt = EntryAttempt::start(invocation_roots("@2/foo", vec!["/r1".into(), "/r2".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    let root_request = request(&attempt);
    let ReadKind::Projects { slot, root, filter } = root_request.kind.clone() else {
        panic!("projects kind: {root_request:?}")
    };
    assert_eq!(slot, RootSlot(2));
    assert_eq!(root, PathBuf::from("/r2"));
    assert_eq!(filter, "foo");
    supply_project(&mut attempt, inventory("/r2", "/r2", &[("foo", "/r2/foo")]));
    let EntryDecision::StartProject { project, session, .. } = plan(&attempt).decision() else {
        panic!("project decision: {:?}", attempt.snapshot())
    };
    assert_eq!(project.name, "foo");
    assert_eq!(project.entry, PathBuf::from("/r2/foo"));
    assert_eq!(project.physical, PathBuf::from("/r2/foo"));
    assert_eq!(project.route.slot, 2);
    assert_eq!(session.name, "foo");
    assert_eq!(session.directory.path(), Path::new("/r2/foo"));
    let action::DecisionBasis::Project(basis) = plan(&attempt).basis() else {
        panic!("project basis")
    };
    assert_eq!(basis.roots, std::slice::from_ref(&root_request));
    assert_eq!(basis.routes.len(), 1);
    // Binding reads fresh sessions plus the chosen route's physical identity.
    deliver(&mut attempt);
    supply(&mut attempt, Vec::new());
    let identity = request(&attempt);
    assert!(matches!(identity.kind, ReadKind::ProjectIdentity { .. }));
    supply_observation(&mut attempt, Ok(Observation::ProjectIdentity("/r2/foo".into())));
    let Waiting::Finished(AttemptOutcome::Exported(exported)) = attempt.waiting() else {
        panic!("export: {:?}", attempt.snapshot())
    };
    assert_eq!(
        exported.shell,
        "cd -P '/r2/foo' && 'tmux' '-S' '/no/socket/read/by/core' 'new-session' '-s' 'foo' '-n' 'foo' '-c' '/r2/foo'"
    );
    assert_eq!(exported.action.binding_requests().len(), 2);
    assert_eq!(reads(&attempt).len(), 4);
}

#[test]
fn unqualified_miss_walks_every_required_root_in_order_before_global_ranking() {
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/r1".into(), "/r2".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    let first = request(&attempt);
    let ReadKind::Projects { slot, .. } = first.kind.clone() else {
        panic!("projects kind")
    };
    assert_eq!(slot, RootSlot(1));
    supply_project(&mut attempt, inventory("/r1", "/r1", &[]));
    // The remaining-root summary re-enters the read state for root two.
    let second = request(&attempt);
    let ReadKind::Projects { slot, root, .. } = second.kind.clone() else {
        panic!("projects kind")
    };
    assert_eq!((slot, root), (RootSlot(2), PathBuf::from("/r2")));
    supply_project(&mut attempt, inventory("/r2", "/r2", &[("bar-foo", "/r2/bar-foo"), ("foo", "/r2/foo")]));
    // Global ranking across ALL required roots: shortest basename wins.
    let EntryDecision::StartProject { project, .. } = plan(&attempt).decision() else {
        panic!("project decision")
    };
    assert_eq!(project.name, "foo");
    assert_eq!(
        attempt
            .journal
            .iter()
            .filter(|r| **r == AttemptRecord::Directive(Directive::ReadProjects))
            .count(),
        2
    );
    let action::DecisionBasis::Project(basis) = plan(&attempt).basis() else {
        panic!("project basis")
    };
    assert_eq!(basis.roots.len(), 2);
}

#[test]
fn distinct_physical_directories_tie_ambiguously_before_unique_or_empty() {
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/personal".into(), "/team".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/personal", "/personal", &[("foo", "/personal/foo")]));
    supply_project(&mut attempt, inventory("/team", "/team", &[("foo", "/team/foo")]));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "projects_rank");
    let EntryError::Discovery(crate::ResolveError::Projects { candidates, .. }) = error else {
        panic!("ambiguity: {error:?}")
    };
    assert_eq!(candidates, &["@1/foo".to_string(), "@2/foo".to_string()]);
    assert!(!controls(&attempt).contains(&ControlEvent::ProjectUnique));
    assert!(!controls(&attempt).contains(&ControlEvent::ProjectEmpty));
    assert!(attempt.snapshot().plan.is_none());
    assert_inert(&attempt);
}

#[test]
fn equivalent_routes_share_one_physical_identity_and_retain_every_route() {
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/alias".into(), "/physical".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    // /alias is a symlink to /physical: both configured slots observe one
    // physical root, so both routes identify the same physical directory.
    supply_project(&mut attempt, inventory("/alias", "/physical", &[("foo", "/physical/foo")]));
    supply_project(&mut attempt, inventory("/physical", "/physical", &[("foo", "/physical/foo")]));
    let EntryDecision::StartProject { project, .. } = plan(&attempt).decision() else {
        panic!("unique physical winner: {:?}", attempt.snapshot())
    };
    assert_eq!(project.routes.len(), 2);
    // Root order picks the deterministic entry route, not work priority.
    assert_eq!(project.route.slot, 1);
    assert_eq!(project.entry, PathBuf::from("/physical/foo"));
    let exported = export_new_work(&mut attempt, Vec::new(), Observation::ProjectIdentity("/physical/foo".into()));
    assert!(matches!(exported.action.operation(), BoundOperation::StartProject { .. }));
}

#[test]
fn missing_root_evidence_stops_and_never_supplies_absence() {
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/r1".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    let root_request = request(&attempt);
    supply_observation(
        &mut attempt,
        Err(ObservationFailure {
            source: Box::new(NativeReadFailure {
                path: "/r1".into(),
                source: std::io::Error::from_raw_os_error(13),
            }),
        }),
    );
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "projects_read");
    let EntryError::Observation { request, .. } = error else {
        panic!("observation failure: {error:?}")
    };
    assert_eq!(request.as_ref(), &root_request);
    assert!(!controls(&attempt).contains(&ControlEvent::ProjectEmpty));
    assert!(attempt.snapshot().plan.is_none());
}

#[test]
fn malformed_root_inventory_is_a_native_stop_never_an_empty_root() {
    for observed in [
        inventory("/r1", "relative", &[("foo", "/r1/foo")]),
        inventory("/other", "/other", &[("foo", "/other/foo")]),
        inventory("/r1", "/r1", &[("bad/name", "/r1/bad")]),
        inventory("/r1", "/r1", &[("foo", "/r1/foo"), ("foo", "/r1/foo")]),
    ] {
        let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/r1".into()])).unwrap();
        supply(&mut attempt, Vec::new());
        supply_project(&mut attempt, observed);
        let (boundary, error) = stopped(&attempt);
        assert_eq!(boundary, "projects_read");
        assert!(matches!(error, EntryError::Discovery(crate::ResolveError::InvalidInventory)), "{error:?}");
        assert!(!controls(&attempt).contains(&ControlEvent::ProjectEmpty));
    }
}

#[test]
fn wrong_observation_kind_is_rejected_before_the_pending_request_is_consumed() {
    // A sessions-shaped reply to a pending projects request is a protocol
    // error with NO actor, data or journal change; the read stays pending.
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/r1".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    let pending = request(&attempt);
    let before = format!("{:?}", attempt.snapshot());
    for wrong in [
        Observation::Sessions(SessionInventory::Absent),
        Observation::Destination(DestinationEvidence::Absent),
        Observation::Defaults(GeneratorDefaults::default()),
        Observation::ProjectIdentity("/somewhere".into()),
    ] {
        let result = attempt.advance(Event::Observed(ReadCompletion {
            request: pending.clone(),
            result: Ok(wrong),
        }));
        assert!(matches!(result, Err(ProtocolError::ObservationShape { .. })));
        assert_eq!(format!("{:?}", attempt.snapshot()), before);
    }
    // The correctly-shaped reply then advances normally.
    supply_project(&mut attempt, inventory("/r1", "/r1", &[("foo", "/r1/foo")]));
    assert!(matches!(plan(&attempt).decision(), EntryDecision::StartProject { .. }));
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn wrong_observation_shape_for_destination_and_defaults_reads_is_rejected_too() {
    let mut input = invocation_roots("@1/fresh", vec!["/r1".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r1", "/r1", &[]));
    let destination = request(&attempt);
    let before = format!("{:?}", attempt.snapshot());
    let result = attempt.advance(Event::Observed(ReadCompletion {
        request: destination,
        result: Ok(Observation::Sessions(SessionInventory::Absent)),
    }));
    assert!(matches!(result, Err(ProtocolError::ObservationShape { .. })));
    assert_eq!(format!("{:?}", attempt.snapshot()), before);
}

#[test]
fn wrong_slot_or_root_echo_is_rejected_without_advancing() {
    let mut attempt = EntryAttempt::start(invocation_roots("@2/foo", vec!["/r1".into(), "/r2".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    let pending = request(&attempt);
    let before = format!("{:?}", attempt.snapshot());
    for changed in [
        ReadKind::Projects {
            slot: RootSlot(1),
            root: "/r2".into(),
            filter: "foo".into(),
        },
        ReadKind::Projects {
            slot: RootSlot(2),
            root: "/r1".into(),
            filter: "foo".into(),
        },
        ReadKind::Projects {
            slot: RootSlot(2),
            root: "/r2".into(),
            filter: "other".into(),
        },
    ] {
        let mut reply = pending.clone();
        reply.kind = changed;
        let result = attempt.advance(Event::Observed(ReadCompletion {
            request: reply,
            result: Ok(Observation::Projects(inventory("/r2", "/r2", &[("foo", "/r2/foo")]))),
        }));
        assert!(matches!(result, Err(ProtocolError::ReadMismatch { .. })));
        assert_eq!(format!("{:?}", attempt.snapshot()), before);
    }
    supply_project(&mut attempt, inventory("/r2", "/r2", &[("foo", "/r2/foo")]));
    assert!(matches!(plan(&attempt).decision(), EntryDecision::StartProject { .. }));
}

#[test]
fn duplicate_and_stale_discovery_replies_are_rejected() {
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/r1".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    let root_request = request(&attempt);
    supply_project(&mut attempt, inventory("/r1", "/r1", &[("foo", "/r1/foo")]));
    let before = format!("{:?}", attempt.snapshot());
    let stale = Event::Observed(ReadCompletion {
        request: root_request,
        result: Ok(Observation::Projects(inventory("/r1", "/r1", &[]))),
    });
    assert!(matches!(attempt.advance(stale), Err(ProtocolError::NoPendingRead)));
    assert_eq!(format!("{:?}", attempt.snapshot()), before);
}

// --- creation policy ---

#[test]
fn clean_miss_without_a_generator_is_a_typed_missing_stop() {
    let mut attempt = EntryAttempt::start(invocation_roots("foo", vec!["/r1".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r1", "/r1", &[]));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "create_policy");
    let EntryError::Discovery(crate::ResolveError::Missing { roots, .. }) = error else {
        panic!("missing: {error:?}")
    };
    assert_eq!(roots, &vec![PathBuf::from("/r1")]);
}

#[test]
fn bare_scaffold_mode_with_an_empty_registry_keeps_its_guidance() {
    let mut input = invocation_roots("fresh", vec!["/r1".into()]);
    input.generator = crate::scaffold::Choice::Default;
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r1", "/r1", &[]));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "create_policy");
    let EntryError::ScaffoldUnavailable { registry, .. } = error else {
        panic!("scaffold guidance: {error:?}")
    };
    assert!(registry.contains("default location"), "{registry}");
    assert!(error.to_string().contains("[scaffold.generators]"), "{error}");
    assert!(
        !attempt
            .journal
            .iter()
            .any(|r| matches!(r, AttemptRecord::ReadIssued(r) if matches!(r.kind, ReadKind::Destination { .. })))
    );
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn unsafe_unqualified_creation_name_stops_at_policy() {
    let mut input = invocation_roots("a/b", vec!["/r1".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r1", "/r1", &[]));
    assert_eq!(stopped(&attempt).0, "create_policy");
    assert!(matches!(stopped(&attempt).1, EntryError::Discovery(crate::ResolveError::UnsafeName { .. })));
    // A qualified selector with a slashed fragment is already a selector error.
    let mut input = invocation_roots("@2/a/b", vec!["/r1".into(), "/r2".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    assert_eq!(stopped(&attempt).0, "project_scope");
    assert!(matches!(stopped(&attempt).1, EntryError::Discovery(crate::ResolveError::Selector { .. })));
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn session_name_collision_at_planning_uses_the_selection_inventory() {
    let mut input = invocation_roots("fresh", vec!["/r1".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    input.intent.session_name = Some("taken".into());
    let mut attempt = EntryAttempt::start(input).unwrap();
    // "fresh" misses sessions; the explicit session name collides with live work.
    supply(&mut attempt, vec![session(9, "taken", None)]);
    supply_project(&mut attempt, inventory("/r1", "/r1", &[]));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "create_policy");
    let EntryError::NameCollision { name, id } = error else {
        panic!("collision: {error:?}")
    };
    assert_eq!((name.as_str(), *id), ("taken", session(9, "taken", None).id));
}

#[cfg(feature = "scaffold-rekon")]
fn rekon_registry() -> crate::scaffold::Config {
    crate::scaffold::Config::parse(None, Some("[scaffold]\ndefault = \"rk\"\n\n[scaffold.generators.rk]\nadapter = \"rekon\"\n")).unwrap()
}

#[cfg(not(feature = "scaffold-rekon"))]
fn rekon_registry() -> crate::scaffold::Config {
    crate::scaffold::Config::parse(None, None).unwrap()
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn unqualified_creation_with_multiple_physical_roots_offers_qualified_selectors() {
    let mut input = invocation_roots("fresh", vec!["/one".into(), "/two".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/one", "/one", &[]));
    supply_project(&mut attempt, inventory("/two", "/two", &[]));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "create_policy");
    let EntryError::Discovery(crate::ResolveError::Destinations { candidates, .. }) = error else {
        panic!("destinations: {error:?}")
    };
    assert_eq!(candidates, &["@1/fresh".to_string(), "@2/fresh".to_string()]);
    assert!(
        !attempt
            .journal
            .iter()
            .any(|r| matches!(r, AttemptRecord::ReadIssued(r) if matches!(r.kind, ReadKind::Destination { .. })))
    );
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn destination_absence_precedes_adapter_work_and_present_stops_creation() {
    for (evidence, reached_adapter) in [(DestinationEvidence::Absent, true), (DestinationEvidence::Present, false)] {
        let mut input = invocation_roots("@2/fresh", vec!["/one".into(), "/two".into()]);
        input.generator = crate::scaffold::Choice::Default;
        input.registry = rekon_registry();
        let mut attempt = EntryAttempt::start(input).unwrap();
        supply(&mut attempt, Vec::new());
        supply_project(&mut attempt, inventory("/two", "/two", &[]));
        let destination = request(&attempt);
        let ReadKind::Destination { path, purpose } = destination.kind.clone() else {
            panic!("destination kind")
        };
        assert_eq!(path, PathBuf::from("/two/fresh"));
        assert_eq!(purpose, DestinationPurpose::Preparation);
        supply_observation(&mut attempt, Ok(Observation::Destination(evidence)));
        let (boundary, error) = stopped(&attempt);
        let validated = attempt
            .journal
            .iter()
            .any(|r| matches!(r, AttemptRecord::Directive(Directive::ValidateGenerator)));
        if reached_adapter {
            // With two configured roots the external adapter's declared
            // contract refuses right at adapter validation — after absence.
            assert_eq!(boundary, "generator_spec");
            assert!(validated);
            assert!(matches!(error, EntryError::Generator(_)));
        } else {
            assert_eq!(boundary, "destination_read");
            assert!(!validated);
            assert!(matches!(error, EntryError::DestinationExists { path } if path.as_path() == Path::new("/two/fresh")));
        }
    }
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn external_rekon_keeps_its_documented_single_root_divergence() {
    let mut input = invocation_roots("fresh", vec!["/only".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/only", "/only", &[]));
    supply_observation(&mut attempt, Ok(Observation::Destination(DestinationEvidence::Absent)));
    let EntryDecision::ScaffoldAndStart {
        destination,
        generator,
        session,
        ..
    } = plan(&attempt).decision()
    else {
        panic!("scaffold decision: {:?}", attempt.snapshot())
    };
    assert_eq!(destination.path, PathBuf::from("/only/fresh"));
    assert_eq!(destination.slot, 1);
    assert_eq!(generator.generator, "rk");
    assert_eq!(session.name, "fresh");
    // External rekon needs no observed defaults; no defaults read was issued.
    assert!(!reads(&attempt).iter().any(|request| matches!(request.kind, ReadKind::GeneratorDefaults { .. })));
    let action::DecisionBasis::Creation(basis) = plan(&attempt).basis() else {
        panic!("creation basis")
    };
    assert!(basis.defaults.is_none());
    // Binding rechecks fresh name availability and destination absence.
    let exported = export_new_work(&mut attempt, Vec::new(), Observation::Destination(DestinationEvidence::Absent));
    assert_eq!(
        exported.shell,
        "'rekon' 'fresh' && cd -P '/only/fresh' && 'tmux' '-S' '/no/socket/read/by/core' 'new-session' '-s' 'fresh' '-n' 'fresh' '-c' '/only/fresh'"
    );
}

#[cfg(feature = "scaffold-rekon-native")]
#[test]
fn required_defaults_are_observed_once_and_the_generator_materializes_once() {
    let mut input = invocation_roots("fresh", vec!["/only".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = crate::scaffold::Config::parse(None, Some("[scaffold.generators.rn]\nadapter = \"rekon-native\"\n")).unwrap();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/only", "/only", &[]));
    supply_observation(&mut attempt, Ok(Observation::Destination(DestinationEvidence::Absent)));
    let defaults_request = request(&attempt);
    let ReadKind::GeneratorDefaults { cwd, needs } = defaults_request.kind.clone() else {
        panic!("defaults kind")
    };
    assert_eq!(cwd, PathBuf::from("/does/not/need/to/exist"));
    assert_eq!(needs, DefaultNeeds { author: true, license: true });
    // Explicitly absent license is a completed fact; author is observed.
    supply_observation(
        &mut attempt,
        Ok(Observation::Defaults(GeneratorDefaults {
            author: Some("Observed Author".into()),
            license: None,
        })),
    );
    let EntryDecision::ScaffoldAndStart { generator, .. } = plan(&attempt).decision() else {
        panic!("scaffold decision: {:?}", attempt.snapshot())
    };
    let action::DecisionBasis::Creation(basis) = plan(&attempt).basis() else {
        panic!("creation basis")
    };
    assert_eq!(basis.defaults.as_ref(), Some(&defaults_request));
    assert_eq!(basis.generator, "rn");
    // Exactly one materialization with the ACTUAL facts: the observed author
    // is baked into the program; no fake-empty re-resolution happened.
    let program = generator.program.shell(tta_command::Handoff::Run).unwrap();
    assert!(program.contains("Observed Author"), "{program}");
    for directive in [
        Directive::RankSessions,
        Directive::RankProjects,
        Directive::CreatePolicy,
        Directive::ValidateGenerator,
        Directive::PrepareCreation,
    ] {
        assert_eq!(
            attempt.journal.iter().filter(|r| **r == AttemptRecord::Directive(directive)).count(),
            1,
            "one {directive:?}"
        );
    }
    let exported = export_new_work(&mut attempt, Vec::new(), Observation::Destination(DestinationEvidence::Absent));
    assert!(
        exported.shell.starts_with("'mkdir' '-p' '/only/fresh' && cd -P '/only/fresh'"),
        "{}",
        exported.shell
    );
}

#[cfg(feature = "scaffold-rekon-native")]
#[test]
fn adapter_option_mistakes_stop_at_generator_validation_before_defaults_or_creation() {
    let mut input = invocation_roots("fresh", vec!["/only".into()]);
    input.generator = crate::scaffold::Choice::Named("rn".into());
    input.registry = crate::scaffold::Config::parse(
        None,
        Some("[scaffold.generators.rn]\nadapter = \"rekon-native\"\n\n[scaffold.generators.rn.options]\nbrunch = \"main\"\n"),
    )
    .unwrap();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/only", "/only", &[]));
    supply_observation(&mut attempt, Ok(Observation::Destination(DestinationEvidence::Absent)));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "generator_spec");
    assert!(matches!(error, EntryError::Generator(_)));
    assert!(!reads(&attempt).iter().any(|request| matches!(request.kind, ReadKind::GeneratorDefaults { .. })));
}

// --- new-work binding and lowering ---

#[test]
fn fresh_session_name_collision_at_binding_fails_closed_without_reranking() {
    let mut attempt = EntryAttempt::start(invocation_roots("@2/foo", vec!["/r1".into(), "/r2".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r2", "/r2", &[("foo", "/r2/foo")]));
    let original = plan(&attempt).clone();
    deliver(&mut attempt);
    // Someone created a session named foo between planning and delivery.
    supply(&mut attempt, vec![session(9, "foo", None)]);
    supply_observation(&mut attempt, Ok(Observation::ProjectIdentity("/r2/foo".into())));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "binding_check");
    assert!(matches!(error, EntryError::NameCollision { name, .. } if name == "foo"));
    assert_eq!(attempt.snapshot().plan.as_ref(), Some(&original));
    assert!(attempt.snapshot().bound.is_none());
    assert_eq!(
        attempt
            .journal
            .iter()
            .filter(|r| **r == AttemptRecord::Directive(Directive::RankProjects))
            .count(),
        1
    );
}

#[test]
fn changed_route_and_changed_destination_fail_closed_at_binding() {
    // A symlink retarget between planning and binding changes the physical
    // directory the chosen route resolves to.
    let mut attempt = EntryAttempt::start(invocation_roots("@2/foo", vec!["/r1".into(), "/r2".into()])).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r2", "/r2", &[("foo", "/r2/foo")]));
    deliver(&mut attempt);
    supply(&mut attempt, Vec::new());
    supply_observation(&mut attempt, Ok(Observation::ProjectIdentity("/elsewhere/foo".into())));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "binding_check");
    let EntryError::RouteChanged { expected, observed, .. } = error else {
        panic!("route change: {error:?}")
    };
    assert_eq!((expected.as_path(), observed.as_path()), (Path::new("/r2/foo"), Path::new("/elsewhere/foo")));
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn occupied_creation_destination_at_binding_requires_a_new_attempt() {
    let mut input = invocation_roots("fresh", vec!["/only".into()]);
    input.generator = crate::scaffold::Choice::Default;
    input.registry = rekon_registry();
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/only", "/only", &[]));
    supply_observation(&mut attempt, Ok(Observation::Destination(DestinationEvidence::Absent)));
    deliver(&mut attempt);
    supply(&mut attempt, Vec::new());
    supply_observation(&mut attempt, Ok(Observation::Destination(DestinationEvidence::Present)));
    let (boundary, error) = stopped(&attempt);
    assert_eq!(boundary, "binding_check");
    assert!(matches!(error, EntryError::DestinationChanged { path } if path.as_path() == Path::new("/only/fresh")));
}

#[test]
fn switch_client_creation_detaches_then_switches_with_literal_startup_argv() {
    let mut input = invocation_roots("@1/foo", vec!["/r1".into()]);
    input.context.client = ClientEntry::SwitchWithinServer;
    input.intent.startup = vec!["editor".into(), "file name;".into()];
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r1", "/r1", &[("foo", "/r1/foo")]));
    let exported = export_new_work(&mut attempt, Vec::new(), Observation::ProjectIdentity("/r1/foo".into()));
    let Expression::And(and) = &exported.expression else {
        panic!("short-circuit sequence")
    };
    assert_eq!(and.steps.len(), 3);
    let Expression::Command(create) = &and.steps[1] else { panic!("create step") };
    let args: Vec<_> = create.args.iter().map(|word| word.as_os_str().to_str().unwrap()).collect();
    assert_eq!(
        args,
        [
            "-S",
            "/no/socket/read/by/core",
            "new-session",
            "-d",
            "-s",
            "foo",
            "-n",
            "foo",
            "-c",
            "/r1/foo",
            "--",
            "editor",
            // The trailing semicolon is argv-escaped, never shell source.
            "file name\\;"
        ]
    );
    let Expression::Command(switch) = &and.steps[2] else { panic!("switch step") };
    assert_eq!(switch.args.last().unwrap().as_os_str(), "=foo");
    // A lone executable stays one argv value through the sh wrapper.
    let mut input = invocation_roots("@1/foo", vec!["/r1".into()]);
    input.context.client = ClientEntry::SwitchWithinServer;
    input.intent.startup = vec!["solo prog".into()];
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r1", "/r1", &[("foo", "/r1/foo")]));
    let exported = export_new_work(&mut attempt, Vec::new(), Observation::ProjectIdentity("/r1/foo".into()));
    let Expression::And(and) = &exported.expression else { panic!("sequence") };
    let Expression::Command(create) = &and.steps[1] else { panic!("create") };
    assert_eq!(
        create.args.iter().map(|word| word.as_os_str().to_str().unwrap()).collect::<Vec<_>>()[..],
        [
            "-S",
            "/no/socket/read/by/core",
            "new-session",
            "-d",
            "-s",
            "foo",
            "-n",
            "foo",
            "-c",
            "/r1/foo",
            "--",
            "/bin/sh",
            "-c",
            "exec \"$0\"",
            "solo prog"
        ]
    );
}

#[test]
fn window_policy_and_directory_override_belong_to_new_work_only() {
    for (window, directory, expect_n) in [
        (InitialWindow::Named("edit: main".into()), Some("/other/dir"), Some("edit: main")),
        (InitialWindow::Unnamed, None, None),
        (InitialWindow::AfterSession, None, Some("foo")),
    ] {
        let mut input = invocation_roots("@1/foo", vec!["/r1".into()]);
        input.intent.window = window.clone();
        input.intent.directory = directory.map(PathBuf::from);
        let mut attempt = EntryAttempt::start(input).unwrap();
        supply(&mut attempt, Vec::new());
        supply_project(&mut attempt, inventory("/r1", "/r1", &[("foo", "/r1/foo")]));
        let exported = export_new_work(&mut attempt, Vec::new(), Observation::ProjectIdentity("/r1/foo".into()));
        let expected_dir = directory.unwrap_or("/r1/foo");
        assert!(exported.shell.contains(&format!("'-c' '{expected_dir}'")), "{}", exported.shell);
        assert!(exported.shell.contains(&format!("cd -P '{expected_dir}'")), "{}", exported.shell);
        match expect_n {
            Some(window) => assert!(exported.shell.contains(&format!("'-n' '{window}'")), "{}", exported.shell),
            None => assert!(!exported.shell.contains("'-n'"), "{}", exported.shell),
        }
    }
    // Startup/window/directory fields never leak into existing-work types.
    let mut input = invocation("project");
    input.intent.startup = vec!["editor".into()];
    input.intent.window = InitialWindow::Named("editor".into());
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, vec![orphan()]);
    let exported = export(&mut attempt, vec![orphan()]);
    assert!(!exported.shell.contains("--"));
    assert!(!exported.shell.contains("'-n'"));
}

#[test]
fn explicit_session_name_and_input_validation_for_new_work() {
    let mut input = invocation_roots("@2/foo", vec!["/r1".into(), "/r2".into()]);
    input.intent.session_name = Some("team-foo".into());
    let mut attempt = EntryAttempt::start(input).unwrap();
    supply(&mut attempt, Vec::new());
    supply_project(&mut attempt, inventory("/r2", "/r2", &[("foo", "/r2/foo")]));
    let EntryDecision::StartProject { session, .. } = plan(&attempt).decision() else {
        panic!("start decision")
    };
    assert_eq!(session.name, "team-foo");
    let exported = export_new_work(&mut attempt, Vec::new(), Observation::ProjectIdentity("/r2/foo".into()));
    assert!(exported.shell.contains("'-s' 'team-foo'"), "{}", exported.shell);
    // Invalid new-work inputs are rejected before any host read.
    let relative_root = || {
        let mut input = invocation_roots("foo", vec!["/r1".into()]);
        input.intent.roots = vec![PathBuf::from("relative")];
        input
    };
    let bad_session_name = || {
        let mut input = invocation_roots("foo", vec!["/r1".into()]);
        input.intent.session_name = Some("bad:name".into());
        input
    };
    let bad_window = || {
        let mut input = invocation_roots("foo", vec!["/r1".into()]);
        input.intent.window = InitialWindow::Named("\u{7}".into());
        input
    };
    let bad_startup = || {
        let mut input = invocation_roots("foo", vec!["/r1".into()]);
        input.intent.startup = vec![OsString::from_vec(b"bad\0argv".to_vec())];
        input
    };
    for input in [relative_root(), bad_session_name(), bad_window(), bad_startup()] {
        assert!(EntryAttempt::start(input).is_err());
    }
}
