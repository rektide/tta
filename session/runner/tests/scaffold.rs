//! Registry selection belongs to the actor; generation cannot rescue a guard.
mod common;
use common::entry::*;
use tta_launcher::{
    entry::{EntryAttempt, protocol::*},
    scaffold,
};

#[test]
fn choice_aliases_and_empty_registry_have_honest_protocol_outcomes() {
    for choice in ["false", "0", "off", "none"] {
        assert_eq!(scaffold::Choice::parse(choice).unwrap(), scaffold::Choice::Off);
    }
    for choice in ["default", "true", "1"] {
        assert_eq!(scaffold::Choice::parse(choice).unwrap(), scaffold::Choice::Default);
    }
    assert_eq!(scaffold::Choice::parse("rk").unwrap(), scaffold::Choice::Named("rk".into()));
    assert!(scaffold::Choice::parse("").is_err());
    let tmp = common::scratch_dir("tta-registry-");
    for choice in [scaffold::Choice::Off, scaffold::Choice::Default] {
        let mut i = input(tmp.path(), "missing");
        i.intent.roots = vec![tmp.path().into()];
        i.generator = choice.clone();
        let attempt = prepare(i, vec![]);
        assert_eq!(
            matches!(cause(&attempt), EntryError::ScaffoldUnavailable { .. }),
            choice == scaffold::Choice::Default
        );
        assert!(!tmp.path().join("missing").exists());
    }
    let mut i = input(tmp.path(), "existing");
    i.generator = scaffold::Choice::Named("unknown".into());
    let attempt = EntryAttempt::start(i).unwrap();
    assert!(matches!(cause(&attempt), EntryError::Registry(_)));
    assert!(!attempt.snapshot().journal.iter().any(|r| matches!(r, AttemptRecord::ReadIssued(_))));
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn external_adapter_refuses_all_multi_root_creation_routes_before_mutation() {
    let tmp = common::scratch_dir("tta-contract-");
    let first = tmp.path().join("one");
    let second = tmp.path().join("two");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    std::os::unix::fs::symlink(&first, tmp.path().join("alias")).unwrap();
    for (roots, selector, expected) in [
        (vec![first.clone(), second.clone()], "@2/fresh", "external rekon adapter"),
        (vec![first.clone(), second.clone()], "fresh", "cannot choose a scaffold destination"),
        (vec![tmp.path().join("alias"), first.clone()], "fresh", "external rekon adapter"),
    ] {
        let mut i = input(tmp.path(), selector);
        i.intent.roots = roots;
        enable_creation(&mut i);
        let attempt = prepare(i, vec![]);
        assert!(cause(&attempt).to_string().contains(expected), "{:?}", cause(&attempt));
        assert!(!first.join("fresh").exists() && !second.join("fresh").exists());
    }
}

// Real generator ordering/short-circuit in both native and aggregate shapes is
// exercised in entry_execute; this file no longer builds a second prefix plan.
