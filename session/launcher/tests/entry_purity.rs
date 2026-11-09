#![cfg(feature = "entry-internal")]

use std::os::unix::{ffi::OsStringExt, fs::PermissionsExt};

use tta_launcher::entry::{EntryAttempt, catalog, context, protocol::*};

fn pure_journeys() {
    let scope = context::Scope {
        origin: context::Origin::Local {
            host: "supplied-poison-host".into(),
        },
        cwd: None,
    };
    let overrides = context::Overrides {
        roots: Some(vec!["/not/a/real/root".into()]),
        registry: Some("/poison/registry".into()),
        ..Default::default()
    };
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: scope.clone(),
        overrides: overrides.clone(),
        intent: EntryIntent::selector("work"),
        default_scaffold: "false".into(),
    })
    .unwrap();
    let complete = |attempt: &mut EntryAttempt, observation| {
        let Waiting::Read(request) = attempt.waiting() else { panic!() };
        let request = request.clone();
        attempt
            .advance(Event::Observed(ReadCompletion {
                request,
                result: Ok(observation),
            }))
            .unwrap();
    };
    complete(
        &mut attempt,
        Observation::Context(context::Facts {
            cwd: "/not/a/real/cwd".into(),
            uid: 1234,
            environment: [
                ("TTA_ROOTS".into(), std::ffi::OsString::from_vec(vec![0xff])),
                ("PATH".into(), "/poison/commands".into()),
            ]
            .into(),
        }),
    );
    complete(&mut attempt, Observation::Registry(None));
    let sessions = SessionInventory::Present(vec![tta_launcher::Session {
        id: "$42".parse().unwrap(),
        name: "work".into(),
        group: None,
    }]);
    complete(&mut attempt, Observation::Sessions(sessions.clone()));
    assert!(matches!(attempt.waiting(), Waiting::Planned(_)));
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    complete(&mut attempt, Observation::Sessions(sessions));
    assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Exported(_))));

    let mut query = catalog::CatalogQuery::acquire(catalog::Input { scope, overrides });
    let complete = |query: &mut catalog::CatalogQuery, observation| {
        let catalog::Waiting::Read(request) = query.waiting() else { panic!() };
        let request = request.clone();
        query
            .advance(catalog::Event::Observed(ReadCompletion {
                request,
                result: Ok(observation),
            }))
            .unwrap();
    };
    complete(
        &mut query,
        Observation::Context(context::Facts {
            cwd: "/not/a/real/cwd".into(),
            uid: 1234,
            environment: Default::default(),
        }),
    );
    complete(&mut query, Observation::Registry(None));
    complete(&mut query, Observation::Sessions(SessionInventory::Absent));
    complete(
        &mut query,
        Observation::Projects(tta_launcher::ProjectInventory {
            requested_root: "/not/a/real/root".into(),
            root: "/physical/supplied/root".into(),
            entries: vec![tta_launcher::ProjectEntry {
                name: "work".into(),
                directory: "/physical/supplied/root/work".into(),
            }],
        }),
    );
    let catalog::Waiting::Finished(catalog::Outcome::Report(report)) = query.waiting() else {
        panic!()
    };
    assert!(report.complete());
    assert_eq!(report.completion_names(), ["@1/work", "work"]);
}

#[test]
fn supplied_facts_entry_and_catalog_are_runner_free_even_with_poisoned_commands_and_environment() {
    if std::env::var_os("TTA_ENTRY_POISON_CHILD").is_some() {
        pure_journeys();
        return;
    }
    let temp = tempfile::tempdir_in(std::env::var_os("TTA_TEST_TMPDIR").unwrap_or_else(|| "/tmp/opencode".into())).unwrap();
    let marker = temp.path().join("unexpected-io");
    for name in ["tmux", "git", "tta", "ssh", "id", "pwd", "cat", "head", "find", "stat"] {
        let path = temp.path().join(name);
        std::fs::write(&path, format!("#!/bin/sh\nprintf unexpected > '{}'\nexit 99\n", marker.display())).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "supplied_facts_entry_and_catalog_are_runner_free_even_with_poisoned_commands_and_environment",
            "--nocapture",
        ])
        .env("TTA_ENTRY_POISON_CHILD", "1")
        .env("PATH", temp.path())
        .env("HOME", "/poison/no-home")
        .env("TTA_CONFIG", "/poison/no-config")
        .env("TTA_ROOTS", "invalid ignored invoking-host layer")
        .env("TMUX", "/poison/local,1,1")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!marker.exists(), "a core journey attempted host I/O");
}
