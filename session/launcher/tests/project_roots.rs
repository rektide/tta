use std::path::{Path, PathBuf};
use tta_launcher::{DestinationState, ProjectEntry, ProjectInventory, Resolution, ResolveError, Selection, Session, projects};

fn inventory(request: &str, physical: &str, entries: &[(&str, &str)]) -> ProjectInventory {
    ProjectInventory {
        requested_root: request.into(),
        root: physical.into(),
        entries: entries
            .iter()
            .map(|(name, directory)| ProjectEntry {
                name: (*name).into(),
                directory: (*directory).into(),
            })
            .collect(),
    }
}

fn resolve(roots: &[PathBuf], facts: &[ProjectInventory], argument: &str, create: bool) -> Result<Resolution, ResolveError> {
    tta_launcher::resolve()
        .sessions(&[])
        .roots(roots)
        .projects(facts)
        .argument(argument)
        .create(create)
        .call()
}

#[test]
fn whole_layer_configuration_preserves_order_and_one_root_semantics() {
    let cwd = Path::new("/invoke");
    let home = Some(Path::new("/home"));
    let configured = |explicit: Option<&[PathBuf]>, many, one: Option<&str>, legacy: Option<&str>, home| {
        projects::configured_roots(
            explicit,
            many,
            one.map(std::ffi::OsStr::new),
            legacy.map(std::ffi::OsStr::new),
            home,
            cwd,
        )
    };
    assert_eq!(
        configured(None, Some(r#"["two","/one: space"]"#), Some("ignored"), None, home).unwrap(),
        vec![PathBuf::from("/invoke/two"), "/one: space".into()]
    );
    assert_eq!(
        configured(Some(&["chosen".into()]), Some("invalid JSON"), None, None, home).unwrap(),
        vec![PathBuf::from("/invoke/chosen")]
    );
    assert_eq!(
        configured(None, Some(""), Some("one"), Some("old"), home).unwrap(),
        vec![PathBuf::from("/invoke/one")]
    );
    assert_eq!(configured(None, None, None, Some("old"), home).unwrap(), vec![PathBuf::from("/home/old")]);
    assert_eq!(configured(None, None, None, Some("/absolute"), None).unwrap(), vec![PathBuf::from("/absolute")]);
    assert!(configured(None, None, None, None, None).unwrap().is_empty());
    for json in ["[]", r#"[""]"#, "null", "not JSON", r#"[1]"#] {
        assert!(
            matches!(
                configured(None, Some(json), Some("ignored"), None, home),
                Err(ResolveError::RootConfiguration(_))
            ),
            "{json}"
        );
    }
    assert!(configured(Some(&[]), None, None, None, home).is_err());
}

#[test]
fn roots_are_not_priority_and_all_required_facts_precede_matching() {
    let roots = vec!["/a".into(), "/b".into()];
    let a = inventory("/a", "/a", &[("long-work", "/a/long-work")]);
    let b = inventory("/b", "/b", &[("work", "/b/work")]);
    assert_eq!(resolve(&roots, &[], "work", false).unwrap(), Resolution::NeedProjects("/a".into()));
    assert_eq!(
        resolve(&roots, std::slice::from_ref(&a), "work", false).unwrap(),
        Resolution::NeedProjects("/b".into())
    );
    let Resolution::Selected(Selection::Project { directory, sources, .. }) = resolve(&roots, &[a, b], "work", false).unwrap() else {
        panic!()
    };
    assert_eq!(directory, Path::new("/b/work"));
    assert_eq!(sources[0].selector(), "@2/work");
}

#[test]
fn duplicate_basename_is_actionable_and_qualifying_observes_only_that_root() {
    let roots = vec!["/a".into(), "/b".into()];
    let a = inventory("/a", "/a", &[("work", "/a/work")]);
    let b = inventory("/b", "/b", &[("work", "/b/work")]);
    assert!(
        matches!(resolve(&roots, &[a.clone(), b.clone()], "work", true), Err(ResolveError::Projects { candidates, .. }) if candidates == ["@1/work", "@2/work"])
    );
    assert_eq!(resolve(&roots, &[], "@2/wo", false).unwrap(), Resolution::NeedProjects("/b".into()));
    assert!(
        matches!(resolve(&roots, std::slice::from_ref(&b), "@2/wo", false).unwrap(), Resolution::Selected(Selection::Project { directory, .. }) if directory == Path::new("/b/work"))
    );
    assert_eq!(projects::completion_candidates(&roots, &[a, b]).unwrap(), ["@1/work", "@2/work"]);
    for selector in ["@0/work", "@3/work", "@2/", "@2/../work", "@99999999999999999999999999/work"] {
        assert!(matches!(resolve(&roots, &[], selector, true), Err(ResolveError::Selector { .. })), "{selector}");
    }
}

#[test]
fn physical_aliases_collapse_after_ranking_and_retain_provenance() {
    let roots = vec!["/alias-root".into(), "/other".into(), "/physical".into()];
    let facts = vec![
        inventory("/alias-root", "/physical", &[("zwork", "/shared/work"), ("awork", "/shared/work")]),
        inventory("/other", "/other", &[("awork", "/shared/work")]),
        inventory("/physical", "/physical", &[("awork", "/shared/work")]),
    ];
    let Resolution::Selected(Selection::Project {
        directory,
        session_name,
        sources,
    }) = resolve(&roots, &facts, "work", false).unwrap()
    else {
        panic!()
    };
    assert_eq!(directory, Path::new("/physical/awork"));
    assert_eq!(session_name, "awork");
    assert_eq!(
        sources.iter().map(|s| s.selector()).collect::<Vec<_>>(),
        ["@1/awork", "@1/zwork", "@2/awork", "@3/awork"]
    );
    assert!(sources.iter().all(|s| s.directory == Path::new("/shared/work")));
    let completion = projects::completion_candidates(&roots, &facts).unwrap();
    assert!(completion.contains(&"awork".into()));
    assert!(completion.contains(&"zwork".into()));
    let mut reversed = facts.clone();
    reversed[0].entries.reverse();
    assert_eq!(
        resolve(&roots, &facts, "work", false).unwrap(),
        resolve(&roots, &reversed, "work", false).unwrap()
    );
}

#[test]
fn clean_miss_does_not_choose_first_destination_and_alias_roots_are_one_destination() {
    let roots = vec!["/a".into(), "/b".into()];
    let facts = vec![inventory("/a", "/a", &[]), inventory("/b", "/b", &[])];
    assert!(matches!(resolve(&roots, &facts, "new", true), Err(ResolveError::Destinations { candidates, .. }) if candidates == ["@1/new", "@2/new"]));
    assert_eq!(
        resolve(&roots, &[facts[1].clone()], "@2/new", true).unwrap(),
        Resolution::NeedDestination("/b/new".into())
    );
    let aliases = vec![inventory("/a", "/physical", &[]), inventory("/b", "/physical", &[])];
    assert_eq!(
        resolve(&roots, &aliases, "new", true).unwrap(),
        Resolution::NeedDestination("/physical/new".into())
    );
    let created = tta_launcher::resolve()
        .sessions(&[])
        .roots(&roots)
        .projects(&[facts[1].clone()])
        .argument("@2/new")
        .create(true)
        .destination(DestinationState::Absent)
        .call()
        .unwrap();
    assert!(
        matches!(created, Resolution::Selected(Selection::Create { directory, sources, .. }) if directory == Path::new("/b/new") && sources[0].selector() == "@2/new")
    );
}

#[test]
fn session_hits_remain_rootless_and_collision_guards_precede_destination_requests() {
    let sessions = [Session {
        id: "$1".parse().unwrap(),
        name: "work".into(),
        group: None,
    }];
    assert!(matches!(
        tta_launcher::resolve().sessions(&sessions).argument("work").call().unwrap(),
        Resolution::Selected(Selection::Existing(_))
    ));
    let roots = vec!["/b".into()];
    let facts = [inventory("/b", "/b", &[])];
    assert!(matches!(
        tta_launcher::resolve()
            .sessions(&sessions)
            .roots(&roots)
            .projects(&facts)
            .argument("@1/new")
            .session_name("work")
            .create(true)
            .call(),
        Err(ResolveError::Collision { .. })
    ));
    assert!(matches!(resolve(&roots, &facts, "bad.name", true), Err(ResolveError::InvalidName(_))));
}

#[test]
fn wrong_root_duplicate_entries_and_nonabsolute_physical_facts_fail_closed() {
    let roots = vec!["/a".into()];
    for facts in [
        inventory("/wrong", "/a", &[]),
        inventory("/a", "relative", &[]),
        inventory("/a", "/a", &[("work", "relative")]),
        inventory("/a", "/a", &[("work", "/one"), ("work", "/two")]),
        inventory("/a", "/a", &[("../work", "/one")]),
    ] {
        assert!(matches!(resolve(&roots, &[facts], "work", true), Err(ResolveError::InvalidInventory)));
    }
}
