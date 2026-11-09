use std::fs;

use tta_launcher::{ResolveError, Selection, Session};
use tta_runner::{
    backend::{LookupError, SessionLookup},
    resolve::resolve,
};

struct Inventory(Vec<Session>);

impl SessionLookup for Inventory {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok(self.0.clone())
    }
}

fn inventory(names: &[&str]) -> Inventory {
    Inventory(
        names
            .iter()
            .enumerate()
            .map(|(index, name)| Session {
                id: format!("${index}").parse().unwrap(),
                name: (*name).to_owned(),
                group: None,
            })
            .collect(),
    )
}

#[test]
fn sessions_win_without_reading_a_root_or_requiring_home() {
    let lookup = inventory(&["project", "project-two"]);
    let selected = resolve()
        .lookup(&lookup)
        .argument("project")
        .roots(&["/nonexistent-tta-test-root".into()])
        .call()
        .unwrap();
    assert!(matches!(selected, Selection::Existing(Session { name, .. }) if name == "project"));
    let selected = resolve().lookup(&lookup).argument("two").call().unwrap();
    assert!(matches!(selected, Selection::Existing(Session { name, .. }) if name == "project-two"));
    assert!(matches!(resolve().lookup(&inventory(&[])).argument("x").call(), Err(ResolveError::MissingRoot)));
}

#[test]
fn shortest_session_wins_before_project_search() {
    let selected = resolve()
        .lookup(&inventory(&["long-project", "project-two"]))
        .argument("project")
        .roots(&["/nonexistent-tta-test-root".into()])
        .call()
        .unwrap();
    assert!(matches!(selected, Selection::Existing(Session { name, .. }) if name == "project-two"));
}

#[test]
fn shortest_project_wins_including_create_mode() {
    let root = tempfile::tempdir().unwrap();
    for name in ["long-project", "project-two"] {
        fs::create_dir(root.path().join(name)).unwrap();
    }
    for create in [false, true] {
        let selected = resolve()
            .lookup(&inventory(&[]))
            .argument("project")
            .roots(&[root.path().into()])
            .create(create)
            .call()
            .unwrap();
        assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "project-two"));
    }
}

#[test]
fn session_ambiguity_is_sorted_and_does_not_fall_back_to_projects() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("project")).unwrap();
    let error = resolve()
        .lookup(&inventory(&["z-project", "a-project"]))
        .argument("project")
        .roots(&[root.path().into()])
        .call()
        .unwrap_err();
    let ResolveError::Sessions { candidates, .. } = error else {
        panic!("{error}")
    };
    assert_eq!(candidates, ["a-project ($1)", "z-project ($0)"]);
}

#[test]
fn project_exact_wins_and_substring_ambiguity_is_sorted() {
    let root = tempfile::tempdir().unwrap();
    for name in ["z-project", "project", "a-project"] {
        fs::create_dir(root.path().join(name)).unwrap();
    }
    let selection = resolve()
        .lookup(&inventory(&[]))
        .argument("project")
        .roots(&[root.path().into()])
        .call()
        .unwrap();
    assert!(
        matches!(selection, Selection::Project { directory, session_name, sources } if directory == root.path().canonicalize().unwrap().join("project") && session_name == "project" && sources.len() == 1)
    );
    fs::remove_dir(root.path().join("project")).unwrap();
    let error = resolve()
        .lookup(&inventory(&[]))
        .argument("project")
        .roots(&[root.path().into()])
        .call()
        .unwrap_err();
    let ResolveError::Projects { candidates, .. } = error else {
        panic!("{error}")
    };
    assert_eq!(candidates, ["@1/a-project", "@1/z-project"]);
}

#[test]
fn only_immediate_directories_match_not_files_or_descendants() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("project-file"), "").unwrap();
    fs::create_dir_all(root.path().join("container/project-nested")).unwrap();
    assert!(matches!(
        resolve().lookup(&inventory(&[])).argument("project").roots(&[root.path().into()]).call(),
        Err(ResolveError::Missing { .. })
    ));
    fs::create_dir(root.path().join("literal*project")).unwrap();
    let selected = resolve().lookup(&inventory(&[])).argument("*").roots(&[root.path().into()]).call().unwrap();
    assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "literal*project"));
}

#[test]
fn invalid_names_require_an_override_and_overrides_cannot_collide() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("project.name")).unwrap();
    let empty = inventory(&[]);
    assert!(matches!(
        resolve().lookup(&empty).argument("project").roots(&[root.path().into()]).call(),
        Err(ResolveError::InvalidName(_))
    ));
    let selected = resolve()
        .lookup(&empty)
        .argument("project")
        .roots(&[root.path().into()])
        .session_name("project-name")
        .call()
        .unwrap();
    assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "project-name"));
    assert!(matches!(
        resolve()
            .lookup(&inventory(&["occupied"]))
            .argument("project")
            .roots(&[root.path().into()])
            .session_name("occupied")
            .call(),
        Err(ResolveError::Collision { .. })
    ));
    for invalid in ["", "a:b", "a.b", "a\nb", "a\tb"] {
        assert!(matches!(
            resolve()
                .lookup(&empty)
                .argument("project")
                .roots(&[root.path().into()])
                .session_name(invalid)
                .call(),
            Err(ResolveError::InvalidName(_))
        ));
    }
}

#[test]
fn empty_argument_is_rejected_before_lookup() {
    struct Unreachable;
    impl SessionLookup for Unreachable {
        fn sessions(&self) -> Result<Vec<Session>, LookupError> {
            panic!("empty argument must not cause I/O")
        }
    }
    assert!(matches!(resolve().lookup(&Unreachable).argument("").call(), Err(ResolveError::Empty)));
}

#[cfg(unix)]
#[test]
fn project_symlinks_keep_the_selected_basename() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("real")).unwrap();
    std::os::unix::fs::symlink(root.path().join("real"), root.path().join("alias")).unwrap();
    let selected = resolve().lookup(&inventory(&[])).argument("alias").roots(&[root.path().into()]).call().unwrap();
    assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "alias"));
}

#[test]
fn create_mode_turns_a_clean_miss_into_a_fresh_project_selection() {
    let root = tempfile::tempdir().unwrap();
    let empty = inventory(&[]);
    assert!(matches!(
        resolve().lookup(&empty).argument("fresh").roots(&[root.path().into()]).call(),
        Err(ResolveError::Missing { .. })
    ));
    let selected = resolve()
        .lookup(&empty)
        .argument("fresh")
        .roots(&[root.path().into()])
        .create(true)
        .call()
        .unwrap();
    assert!(
        matches!(selected, Selection::Create { directory, session_name, sources } if directory == root.path().canonicalize().unwrap().join("fresh") && session_name == "fresh" && sources.len() == 1)
    );
}

#[test]
fn create_mode_refuses_unsafe_names_and_occupied_paths() {
    let root = tempfile::tempdir().unwrap();
    let empty = inventory(&[]);
    for unsafe_name in ["nested/path", ".", "..", "/absolute"] {
        assert!(
            matches!(
                resolve().lookup(&empty).argument(unsafe_name).roots(&[root.path().into()]).create(true).call(),
                Err(ResolveError::UnsafeName { .. })
            ),
            "{unsafe_name}"
        );
    }
    fs::write(root.path().join("taken"), "").unwrap();
    assert!(matches!(
        resolve().lookup(&empty).argument("taken").roots(&[root.path().into()]).create(true).call(),
        Err(ResolveError::Exists { .. })
    ));
    // Dangling symlinks never match a directory search but still collide.
    std::os::unix::fs::symlink("missing", root.path().join("dangling")).unwrap();
    assert!(matches!(
        resolve().lookup(&empty).argument("dangling").roots(&[root.path().into()]).create(true).call(),
        Err(ResolveError::Exists { .. })
    ));
}

#[test]
fn destination_inspection_errors_are_not_absence() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    fs::write(&file, "not a directory").unwrap();
    assert!(matches!(
        tta_runner::resolve::destination_state(&file.join("child")),
        Err(ResolveError::Directory { .. })
    ));
    assert_eq!(
        tta_runner::resolve::destination_state(&root.path().join("missing")).unwrap(),
        tta_launcher::DestinationState::Absent
    );
    assert_eq!(tta_runner::resolve::destination_state(&file).unwrap(), tta_launcher::DestinationState::Present);
}

#[test]
fn create_mode_keeps_matching_priority_ambiguity_and_name_rules() {
    let root = tempfile::tempdir().unwrap();
    for name in ["a-fresh", "z-fresh"] {
        fs::create_dir(root.path().join(name)).unwrap();
    }
    let empty = inventory(&[]);
    // Ambiguity still stops; creation never resolves it by guessing.
    assert!(matches!(
        resolve().lookup(&empty).argument("fresh").roots(&[root.path().into()]).create(true).call(),
        Err(ResolveError::Projects { .. })
    ));
    // Existing sessions and directories still win over creation.
    fs::create_dir(root.path().join("present")).unwrap();
    assert!(matches!(
        resolve().lookup(&empty).argument("present").roots(&[root.path().into()]).create(true).call(),
        Ok(Selection::Project { .. })
    ));
    assert!(matches!(
        resolve()
            .lookup(&inventory(&["live"]))
            .argument("live")
            .roots(&[root.path().into()])
            .create(true)
            .call(),
        Ok(Selection::Existing { .. })
    ));
    // Session-name rules and collisions apply before any generator runs.
    assert!(matches!(
        resolve()
            .lookup(&empty)
            .argument("dotted.name")
            .roots(&[root.path().into()])
            .create(true)
            .call(),
        Err(ResolveError::InvalidName(_))
    ));
    assert!(matches!(
        resolve().lookup(&empty).argument("dotted.name").roots(&[root.path().into()]).create(true).session_name("dotted").call(),
        Ok(Selection::Create { session_name, .. }) if session_name == "dotted"
    ));
    assert!(matches!(
        resolve()
            .lookup(&inventory(&["occupied"]))
            .argument("fresh-two")
            .roots(&[root.path().into()])
            .create(true)
            .session_name("occupied")
            .call(),
        Err(ResolveError::Collision { .. })
    ));
}
