//! Local acquisition and actual emitted-probe agreement; not SSH certification.
use std::path::Path;
use tta_launcher::{ProjectInventory, ResolveError, Selection, Session, probe, target, workflow};
use tta_runner::{
    backend::{LookupError, SessionLookup},
    resolve::resolve,
};

struct Empty;
impl SessionLookup for Empty {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok(vec![])
    }
}

fn probed(root: &Path, argument: &str) -> anyhow::Result<ProjectInventory> {
    let probe = probe::Probe::new(
        probe::Request::Workflow(workflow::ObservationRequest::Projects {
            root: root.into(),
            argument: argument.into(),
        }),
        "roots-process-test",
    )?;
    let output = std::process::Command::new("/bin/sh")
        .args(["-c", &probe.shell(&target::TargetSetup::default())?])
        .output()?;
    let decoded = probe.decode(probe::Output {
        stdout: &output.stdout,
        stderr: &output.stderr,
        status: output.status.code(),
        complete: true,
    })?;
    let probe::Reply::Workflow(workflow::ObservationReply::Projects(inventory)) = decoded.reply else {
        panic!()
    };
    Ok(inventory)
}

#[test]
fn local_resolution_selects_across_roots_and_never_guesses_creation_destination() {
    let temp = tempfile::tempdir().unwrap();
    let roots = [temp.path().join("a"), temp.path().join("b")];
    std::fs::create_dir_all(roots[0].join("long-work")).unwrap();
    std::fs::create_dir_all(roots[1].join("work")).unwrap();
    assert!(
        matches!(resolve().lookup(&Empty).roots(&roots).argument("work").call().unwrap(), Selection::Project { directory, .. } if directory == roots[1].join("work"))
    );
    assert!(matches!(
        resolve().lookup(&Empty).roots(&roots).argument("fresh").create(true).call(),
        Err(ResolveError::Destinations { .. })
    ));
    assert!(
        matches!(resolve().lookup(&Empty).roots(&roots).argument("@2/fresh").create(true).call().unwrap(), Selection::Create { directory, sources, .. } if directory == roots[1].join("fresh") && sources[0].root_index == 2)
    );
    assert!(!roots.iter().any(|root| root.join("fresh").exists()), "planning is read-only");
    std::fs::create_dir(roots[0].join("work")).unwrap();
    assert!(matches!(
        resolve().lookup(&Empty).roots(&roots).argument("work").call(),
        Err(ResolveError::Projects { .. })
    ));
    assert!(
        matches!(resolve().lookup(&Empty).roots(&roots).argument("@2/work").call().unwrap(), Selection::Project { directory, .. } if directory == roots[1].join("work"))
    );
}

#[test]
fn real_root_and_child_symlinks_preserve_routes_and_match_probe_physical_identity() {
    let temp = tempfile::tempdir().unwrap();
    let real = temp.path().join("real root");
    let linked = temp.path().join("linked root");
    let other = temp.path().join("other");
    let name = "work '\n雪";
    std::fs::create_dir_all(real.join(name)).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    std::os::unix::fs::symlink(real.join(name), other.join(name)).unwrap();
    std::os::unix::fs::symlink("missing", other.join("dangling")).unwrap();
    for root in [&real, &linked, &other] {
        let mut local = tta_runner::resolve::project_inventory(root, "").unwrap();
        local.entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(local, probed(root, "").unwrap());
    }
    let roots = [linked.clone(), other, real.clone()];
    let Selection::Project { directory, sources, .. } = resolve().lookup(&Empty).roots(&roots).argument("work").session_name("work").call().unwrap() else {
        panic!()
    };
    assert_eq!(directory, real.join(name));
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0].requested_root, linked);
    assert!(sources.iter().all(|source| source.directory == real.join(name)));
}

#[test]
fn overlapping_roots_do_not_make_recursive_discovery_and_failures_are_not_empty() {
    let temp = tempfile::tempdir().unwrap();
    let nested = temp.path().join("container");
    std::fs::create_dir_all(nested.join("work")).unwrap();
    let roots = [temp.path().to_owned(), nested.clone()];
    let Selection::Project { sources, directory, .. } = resolve().lookup(&Empty).roots(&roots).argument("work").call().unwrap() else {
        panic!()
    };
    assert_eq!(directory, nested.join("work"));
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].root_index, 2);
    let missing = temp.path().join("missing");
    let broken = [missing.clone(), nested.clone()];
    assert!(matches!(
        resolve().lookup(&Empty).roots(&broken).argument("work").call(),
        Err(ResolveError::Directory { .. })
    ));
    assert!(probed(&missing, "work").is_err());
    assert!(
        matches!(resolve().lookup(&Empty).roots(&broken).argument("@2/work").call().unwrap(), Selection::Project { directory, .. } if directory == nested.join("work"))
    );
}

#[test]
fn shared_prepare_uses_multi_root_workflow_without_a_second_selection_policy() {
    let temp = tempfile::tempdir().unwrap();
    let roots = [temp.path().join("one"), temp.path().join("two")];
    std::fs::create_dir(&roots[0]).unwrap();
    std::fs::create_dir_all(roots[1].join("work")).unwrap();
    let config = tta_launcher::scaffold::Config::parse(None, None).unwrap();
    let prepared = tta_runner::prepare()
        .lookup(&Empty)
        .argument("work")
        .roots(&roots)
        .socket(&tta_launcher::socket::Resolved::path(temp.path().join("socket")))
        .client_mode(tta_launcher::ClientMode::Attach)
        .program(std::ffi::OsStr::new("tmux"))
        .config(&config)
        .choice("false")
        .call()
        .unwrap();
    assert!(matches!(prepared.selection, Selection::Project { directory, sources, .. } if directory == roots[1].join("work") && sources[0].root_index == 2));
    assert!(
        prepared
            .plan
            .expression()
            .shell(tta_command::Handoff::Run)
            .unwrap()
            .contains(roots[1].join("work").to_str().unwrap())
    );
}
