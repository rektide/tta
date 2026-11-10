//! Selection semantics through real local reads and retained protocol outcomes.
mod common;
use common::entry::*;
use tta_launcher::{
    ResolveError,
    entry::{EntryAttempt, action::EntryDecision, protocol::*},
};

#[test]
fn sessions_win_without_root_reads_and_shortest_ambiguity_is_guarded() {
    let tmp = common::scratch_dir("tta-select-");
    for (names, selected) in [
        (vec!["project", "project-two"], "project"),
        (vec!["long-project", "project-two"], "project-two"),
    ] {
        let mut input = input(tmp.path(), "project");
        input.intent.roots = vec![tmp.path().join("nonexistent")];
        input.intent.view = ViewPreference::Shared;
        let attempt = prepare(input, sessions(&names));
        assert!(matches!(plan(&attempt).decision(), EntryDecision::EnterView { view, .. } if view.fingerprint.name == selected));
        assert!(!attempt.snapshot().journal.iter().any(|r| matches!(
            r,
            AttemptRecord::ReadIssued(ReadRequest {
                kind: ReadKind::Projects { .. },
                ..
            })
        )));
    }
    let attempt = prepare(input(tmp.path(), "project"), sessions(&["z-project", "a-project"]));
    assert!(
        matches!(cause(&attempt), EntryError::Ambiguous { candidates, .. } if candidates.iter().map(|s| s.name.as_str()).collect::<Vec<_>>() == ["a-project", "z-project"])
    );
    let attempt = prepare(input(tmp.path(), "missing"), vec![]);
    assert!(matches!(cause(&attempt), EntryError::Discovery(ResolveError::MissingRoot)));
    assert!(matches!(EntryAttempt::start(input(tmp.path(), "")), Err(InputError::EmptySelector)));
}

#[test]
fn project_exact_shortest_literal_and_immediate_directory_rules() {
    let tmp = common::scratch_dir("tta-project-");
    let root = tmp.path().join("root");
    for name in ["long-project", "project-two", "literal*name", "container/project-nested"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    std::fs::write(root.join("project-file"), "").unwrap();
    for (selector, expected) in [("project", "project-two"), ("*", "literal*name")] {
        let mut input = input(tmp.path(), selector);
        input.intent.roots = vec![root.clone()];
        let attempt = prepare(input, vec![]);
        assert!(
            matches!(plan(&attempt).decision(), EntryDecision::StartProject { project, session, .. } if project.name == expected && session.name == expected)
        );
    }
    std::fs::create_dir(root.join("project")).unwrap();
    let mut invocation = input(tmp.path(), "project");
    invocation.intent.roots = vec![root.clone()];
    assert!(matches!(plan(&prepare(invocation, vec![])).decision(), EntryDecision::StartProject { project, .. } if project.name == "project"));
    let mut invocation = input(tmp.path(), "nested");
    invocation.intent.roots = vec![root];
    assert!(matches!(
        cause(&prepare(invocation, vec![])),
        EntryError::Discovery(ResolveError::Missing { .. })
    ));
}

#[test]
fn invalid_names_need_an_override_and_collisions_stop_before_delivery() {
    let tmp = common::scratch_dir("tta-names-");
    std::fs::create_dir(tmp.path().join("project.name")).unwrap();
    for name in [
        None,
        Some(""),
        Some("a:b"),
        Some("a.b"),
        Some("a\nb"),
        Some("a\tb"),
        Some("safe"),
        Some("occupied"),
    ] {
        let mut invocation = input(tmp.path(), "project");
        invocation.intent.roots = vec![tmp.path().into()];
        invocation.intent.session_name = name.map(Into::into);
        match EntryAttempt::start(invocation) {
            Err(InputError::Name(ResolveError::InvalidName(_))) => assert!(!matches!(name, Some("safe" | "occupied") | None)),
            Ok(mut attempt) => {
                read(&mut attempt, sessions(&["occupied"]));
                match name {
                    Some("safe") => assert!(matches!(plan(&attempt).decision(), EntryDecision::StartProject { session, .. } if session.name == "safe")),
                    Some("occupied") => assert!(matches!(cause(&attempt), EntryError::NameCollision { .. })),
                    None => assert!(matches!(cause(&attempt), EntryError::Discovery(ResolveError::InvalidName(_)))),
                    _ => panic!("invalid input accepted"),
                }
            }
            _ => panic!("unexpected invocation result"),
        }
    }
}

#[cfg(any(feature = "scaffold-rekon", feature = "scaffold-rekon-native"))]
#[test]
fn clean_creation_miss_never_rescues_ambiguity_unsafe_names_or_occupied_destinations() {
    let tmp = common::scratch_dir("tta-create-");
    std::fs::write(tmp.path().join("taken"), "").unwrap();
    std::os::unix::fs::symlink("missing", tmp.path().join("dangling")).unwrap();
    for name in ["fresh", "taken", "dangling", "nested/path", ".", "..", "/absolute"] {
        let mut invocation = input(tmp.path(), name);
        invocation.intent.roots = vec![tmp.path().into()];
        enable_creation(&mut invocation);
        let attempt = prepare(invocation, vec![]);
        match name {
            "fresh" => assert!(
                matches!(plan(&attempt).decision(), EntryDecision::ScaffoldAndStart { destination, .. } if destination.path == tmp.path().join("fresh"))
            ),
            "taken" | "dangling" => assert!(matches!(cause(&attempt), EntryError::DestinationExists { .. })),
            _ => assert!(matches!(cause(&attempt), EntryError::Discovery(ResolveError::UnsafeName { .. }))),
        }
    }
    assert!(!tmp.path().join("fresh").exists());
    for name in ["a-fresh", "z-fresh"] {
        std::fs::create_dir(tmp.path().join(name)).unwrap();
    }
    let mut invocation = input(tmp.path(), "fresh");
    invocation.intent.roots = vec![tmp.path().into()];
    enable_creation(&mut invocation);
    assert!(matches!(
        cause(&prepare(invocation, vec![])),
        EntryError::Discovery(ResolveError::Projects { .. })
    ));
}

#[test]
fn destination_inspection_is_lstat_and_not_directory_errors_are_not_absence() {
    use tta_runner::observe::filesystem::destination_state;
    let tmp = common::scratch_dir("tta-lstat-");
    std::fs::write(tmp.path().join("file"), "").unwrap();
    assert!(matches!(destination_state(&tmp.path().join("file/child")), Err(ResolveError::Directory { .. })));
    assert_eq!(destination_state(&tmp.path().join("file")).unwrap(), DestinationEvidence::Present);
    assert_eq!(destination_state(&tmp.path().join("missing")).unwrap(), DestinationEvidence::Absent);
}
