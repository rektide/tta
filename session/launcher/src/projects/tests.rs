//! Pure project algorithms used by the actor; scheduling is witnessed by entry
//! tests, not a second discovery scheduler.
use crate::{ProjectEntry, ProjectInventory, ResolveError, projects};
use std::path::{Path, PathBuf};
fn inventory(request: &str, physical: &str, entries: &[(&str, &str)]) -> ProjectInventory {
    ProjectInventory {
        requested_root: request.into(),
        root: physical.into(),
        entries: entries
            .iter()
            .map(|(n, p)| ProjectEntry {
                name: (*n).into(),
                directory: (*p).into(),
            })
            .collect(),
    }
}

#[test]
fn active_whole_layer_preserves_order_and_replaced_invalid_values_are_not_parsed() {
    let configured = |explicit: Option<&[PathBuf]>, many, one: Option<&str>, legacy: Option<&str>, home| {
        projects::configured_roots(
            explicit,
            many,
            one.map(std::ffi::OsStr::new),
            legacy.map(std::ffi::OsStr::new),
            home,
            Path::new("/invoke"),
        )
    };
    assert_eq!(
        configured(None, Some(r#"["two","/one: space"]"#), Some("ignored"), None, Some(Path::new("/home"))).unwrap(),
        vec![PathBuf::from("/invoke/two"), "/one: space".into()]
    );
    assert_eq!(
        configured(Some(&["chosen".into()]), Some("invalid JSON"), None, None, None).unwrap(),
        vec![PathBuf::from("/invoke/chosen")]
    );
    assert_eq!(
        configured(None, Some(""), Some("one"), Some("old"), None).unwrap(),
        vec![PathBuf::from("/invoke/one")]
    );
    assert_eq!(
        configured(None, None, None, Some("old"), Some(Path::new("/home"))).unwrap(),
        vec![PathBuf::from("/home/old")]
    );
    assert_eq!(configured(None, None, None, Some("/absolute"), None).unwrap(), vec![PathBuf::from("/absolute")]);
    assert!(configured(None, None, None, None, None).unwrap().is_empty());
    for json in ["[]", r#"[""]"#, "null", "not JSON", "[1]"] {
        assert!(matches!(
            configured(None, Some(json), Some("ignored"), None, None),
            Err(ResolveError::RootConfiguration(_))
        ));
    }
    assert!(configured(Some(&[]), None, None, None, None).is_err());
}

#[test]
fn ranking_is_global_and_physical_aliases_retain_deterministic_provenance() {
    let roots = vec!["/alias-root".into(), "/other".into(), "/physical".into()];
    let facts = vec![
        inventory("/alias-root", "/physical", &[("zwork", "/shared/work"), ("awork", "/shared/work")]),
        inventory("/other", "/other", &[("awork", "/shared/work")]),
        inventory("/physical", "/physical", &[("awork", "/shared/work")]),
    ];
    let sources = projects::sources(&roots, &[0, 1, 2], &facts).unwrap();
    let selected = projects::select(&sources, "work", "work").unwrap().unwrap();
    assert_eq!(selected.entry_path(), Path::new("/physical/awork"));
    assert_eq!(
        sources.iter().map(|s| s.selector()).collect::<Vec<_>>(),
        ["@1/awork", "@1/zwork", "@2/awork", "@3/awork"]
    );
    assert!(sources.iter().all(|s| s.directory == Path::new("/shared/work")));
    let mut reversed = facts.clone();
    reversed[0].entries.reverse();
    assert_eq!(sources, projects::sources(&roots, &[0, 1, 2], &reversed).unwrap());
    assert_eq!(
        projects::completion_candidates(&roots, &facts).unwrap(),
        ["@1/awork", "@1/zwork", "@2/awork", "@3/awork", "awork", "zwork"]
    );
}

#[test]
fn duplicate_names_require_qualified_selectors_and_invalid_slots_fail_closed() {
    let roots = vec!["/a".into(), "/b".into()];
    let facts = vec![inventory("/a", "/a", &[("work", "/a/work")]), inventory("/b", "/b", &[("work", "/b/work")])];
    let sources = projects::sources(&roots, &[0, 1], &facts).unwrap();
    assert!(matches!(projects::select(&sources, "work", "work"), Err(ResolveError::Projects { candidates, .. }) if candidates == ["@1/work", "@2/work"]));
    assert_eq!(projects::completion_candidates(&roots, &facts).unwrap(), ["@1/work", "@2/work"]);
    let scope = projects::scope("@2/wo", &roots).unwrap();
    assert_eq!(scope.indices, [1]);
    assert_eq!(scope.fragment, "wo");
    for selector in ["@0/work", "@3/work", "@2/", "@2/../work", "@99999999999999999999999999/work"] {
        assert!(matches!(projects::scope(selector, &roots), Err(ResolveError::Selector { .. })));
    }
}

#[test]
fn inventories_must_answer_exact_routes_with_unique_absolute_immediate_entries() {
    let roots = vec!["/a".into()];
    for facts in [
        inventory("/wrong", "/a", &[]),
        inventory("/a", "relative", &[]),
        inventory("/a", "/a", &[("work", "relative")]),
        inventory("/a", "/a", &[("work", "/one"), ("work", "/two")]),
        inventory("/a", "/a", &[("../work", "/one")]),
    ] {
        assert!(matches!(projects::sources(&roots, &[0], &[facts]), Err(ResolveError::InvalidInventory)));
    }
}
