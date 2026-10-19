use std::fs;
use std::path::Path;

use tta_launcher::{
    ResolveError, Selection, Session,
    backend::{LookupError, SessionLookup},
    resolve,
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
        .root(Path::new("/nonexistent-tta-test-root"))
        .call()
        .unwrap();
    assert!(matches!(selected, Selection::Existing(Session { name, .. }) if name == "project"));
    let selected = resolve().lookup(&lookup).argument("two").call().unwrap();
    assert!(matches!(selected, Selection::Existing(Session { name, .. }) if name == "project-two"));
    assert!(matches!(resolve().lookup(&inventory(&[])).argument("x").call(), Err(ResolveError::MissingRoot)));
}

#[test]
fn session_ambiguity_is_sorted_and_does_not_fall_back_to_projects() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("project")).unwrap();
    let error = resolve()
        .lookup(&inventory(&["z-project", "a-project"]))
        .argument("project")
        .root(root.path())
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
    let selection = resolve().lookup(&inventory(&[])).argument("project").root(root.path()).call().unwrap();
    assert_eq!(
        selection,
        Selection::Project {
            directory: root.path().canonicalize().unwrap().join("project"),
            session_name: "project".to_owned()
        }
    );
    fs::remove_dir(root.path().join("project")).unwrap();
    let error = resolve().lookup(&inventory(&[])).argument("project").root(root.path()).call().unwrap_err();
    let ResolveError::Projects { candidates, .. } = error else {
        panic!("{error}")
    };
    assert_eq!(candidates, ["a-project", "z-project"]);
}

#[test]
fn only_immediate_directories_match_not_files_or_descendants() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("project-file"), "").unwrap();
    fs::create_dir_all(root.path().join("container/project-nested")).unwrap();
    assert!(matches!(
        resolve().lookup(&inventory(&[])).argument("project").root(root.path()).call(),
        Err(ResolveError::Missing { .. })
    ));
    fs::create_dir(root.path().join("literal*project")).unwrap();
    let selected = resolve().lookup(&inventory(&[])).argument("*").root(root.path()).call().unwrap();
    assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "literal*project"));
}

#[test]
fn invalid_names_require_an_override_and_overrides_cannot_collide() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("project.name")).unwrap();
    let empty = inventory(&[]);
    assert!(matches!(
        resolve().lookup(&empty).argument("project").root(root.path()).call(),
        Err(ResolveError::InvalidName(_))
    ));
    let selected = resolve()
        .lookup(&empty)
        .argument("project")
        .root(root.path())
        .session_name("project-name")
        .call()
        .unwrap();
    assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "project-name"));
    assert!(matches!(
        resolve()
            .lookup(&inventory(&["occupied"]))
            .argument("project")
            .root(root.path())
            .session_name("occupied")
            .call(),
        Err(ResolveError::Collision { .. })
    ));
    for invalid in ["", "a:b", "a.b", "a\nb", "a\tb"] {
        assert!(matches!(
            resolve().lookup(&empty).argument("project").root(root.path()).session_name(invalid).call(),
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
    let selected = resolve().lookup(&inventory(&[])).argument("alias").root(root.path()).call().unwrap();
    assert!(matches!(selected, Selection::Project { session_name, .. } if session_name == "alias"));
}
