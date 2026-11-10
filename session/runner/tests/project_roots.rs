//! Actual local physical-route acquisition through the public attempt.
mod common;
use common::entry::*;
use tta_launcher::{
    ResolveError,
    entry::{action::EntryDecision, protocol::*},
};

#[test]
fn roots_are_not_priority_and_qualification_limits_required_observations() {
    let tmp = common::scratch_dir("tta-roots-");
    let roots = vec![tmp.path().join("one"), tmp.path().join("two")];
    std::fs::create_dir_all(roots[0].join("long-work")).unwrap();
    std::fs::create_dir_all(roots[1].join("work")).unwrap();
    let run = |selector: &str| {
        let mut i = input(tmp.path(), selector);
        i.intent.roots = roots.clone();
        prepare(i, vec![])
    };
    assert!(matches!(plan(&run("work")).decision(), EntryDecision::StartProject { project, .. } if project.route.slot == 2));
    std::fs::create_dir(roots[0].join("work")).unwrap();
    assert!(matches!(cause(&run("work")), EntryError::Discovery(ResolveError::Projects { candidates, .. }) if candidates == &["@1/work", "@2/work"]));
    let attempt = run("@2/work");
    assert!(matches!(plan(&attempt).decision(), EntryDecision::StartProject { project, .. } if project.route.slot == 2));
    let reads = attempt
        .snapshot()
        .journal
        .into_iter()
        .filter(|r| {
            matches!(
                r,
                AttemptRecord::ReadIssued(ReadRequest {
                    kind: ReadKind::Projects { .. },
                    ..
                })
            )
        })
        .count();
    assert_eq!(reads, 1);
}

#[test]
fn root_and_child_symlinks_preserve_every_route_and_selected_physical_identity() {
    let tmp = common::scratch_dir("tta-routes-");
    let real = tmp.path().join("real");
    let linked = tmp.path().join("linked");
    let other = tmp.path().join("other");
    let name = "work '\n雪";
    std::fs::create_dir_all(real.join(name)).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    std::os::unix::fs::symlink(real.join(name), other.join(name)).unwrap();
    let mut i = input(tmp.path(), "work");
    i.intent.roots = vec![linked.clone(), other, real.clone()];
    i.intent.session_name = Some("work".into());
    let attempt = prepare(i, vec![]);
    let EntryDecision::StartProject { project, .. } = plan(&attempt).decision() else {
        panic!()
    };
    assert_eq!(project.physical, real.join(name));
    assert_eq!(project.routes.len(), 3);
    assert_eq!(project.route.requested_root, linked);
    assert!(project.routes.iter().all(|r| r.directory == project.physical));
}

#[test]
fn failed_required_root_is_not_an_empty_root_and_overlap_is_not_recursion() {
    let tmp = common::scratch_dir("tta-overlap-");
    let nested = tmp.path().join("container");
    std::fs::create_dir_all(nested.join("work")).unwrap();
    let run = |roots, selector| {
        let mut i = input(tmp.path(), selector);
        i.intent.roots = roots;
        prepare(i, vec![])
    };
    let attempt = run(vec![tmp.path().into(), nested.clone()], "work");
    assert!(matches!(plan(&attempt).decision(), EntryDecision::StartProject { project, .. } if project.routes.len() == 1 && project.route.slot == 2));
    assert!(matches!(
        cause(&run(vec![tmp.path().join("missing"), nested.clone()], "work")),
        EntryError::Observation { .. }
    ));
    let attempt = run(vec![tmp.path().join("missing"), nested], "@2/work");
    assert!(matches!(plan(&attempt).decision(), EntryDecision::StartProject { project, .. } if project.route.slot == 2));
}
