//! Worker-free process-carrier codec tests, explicitly NOT authenticated SSH.
mod common;
use std::path::Path;
use tta_launcher::{
    entry::{EntryAttempt, context, probe, protocol::*, registry},
    target,
};
use tta_runner::capture::{self, Limits};

fn request(kind: ReadKind, setup: target::TargetSetup) -> ReadRequest {
    let mut attempt = EntryAttempt::start(common::entry::input(Path::new("/target"), "work")).unwrap();
    let Waiting::Read(request) = attempt.waiting() else { panic!() };
    let mut request = request.clone();
    request.kind = kind;
    request.scope = Box::new(context::Scope {
        origin: context::Origin::Ssh { host: "test".into(), setup },
        cwd: None,
    });
    // Construct the sessions request with the actual issuing target binding.
    if let ReadKind::Sessions { endpoint, .. } = &mut request.kind {
        endpoint.target = request.target;
    }
    attempt.advance(Event::Cancel).unwrap();
    request
}
fn execute(kind: ReadKind, setup: target::TargetSetup) -> (probe::Probe, capture::Output) {
    let probe = probe::Probe::new(request(kind, setup), "process-test-token").unwrap();
    let output = capture::run(std::process::Command::new("/bin/sh").args(["-c", &probe.shell().unwrap()]), Limits::default()).unwrap();
    (probe, output)
}
fn decode(probe: &probe::Probe, output: &capture::Output) -> anyhow::Result<probe::Decoded> {
    probe.decode(tta_launcher::probe::Output {
        stdout: &output.stdout,
        stderr: &output.stderr,
        status: output.status.and_then(|s| s.code()),
        complete: output.complete(),
    })
}
fn projects(root: &Path, filter: &str) -> ReadKind {
    ReadKind::Projects {
        slot: RootSlot(1),
        root: root.into(),
        filter: filter.into(),
    }
}
fn destination(path: std::path::PathBuf) -> ReadKind {
    ReadKind::Destination {
        path,
        purpose: DestinationPurpose::Preparation,
    }
}

#[test]
fn binary_envelope_preserves_startup_noise_and_raw_target_context() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = common::scratch_dir("tta-probe-context-");
    let shell = tmp.path().join("selected shell");
    std::fs::write(
        &shell,
        "#!/bin/sh\nprintf 'startup stdout\\n'\nprintf 'startup stderr\\n' >&2\nexport HOME='/target home' TTA_ROOT='target root'\nexec /bin/sh \"$@\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700)).unwrap();
    let (probe, out) = execute(
        ReadKind::TargetContext,
        target::TargetSetup::builder()
            .shell(shell.to_str().unwrap().into())
            .cwd(tmp.path().into())
            .build(),
    );
    let decoded = decode(&probe, &out).unwrap();
    assert_eq!(decoded.diagnostics.startup_stdout, b"startup stdout\n");
    assert_eq!(decoded.diagnostics.transport_stderr, b"startup stderr\n");
    let probe::Reply::Observation(Observation::Context(facts)) = decoded.reply else {
        panic!()
    };
    assert_eq!(facts.cwd, tmp.path());
    assert_eq!(facts.environment["HOME"], "/target home");
    assert_eq!(facts.environment["TTA_ROOT"], "target root");
}

#[test]
fn complete_inventory_supports_hostile_names_physical_aliases_and_lstat_collision() {
    let tmp = common::scratch_dir("tta-probe-fs-");
    let name = "a\n' $; 雪";
    std::fs::create_dir(tmp.path().join(name)).unwrap();
    std::os::unix::fs::symlink("missing", tmp.path().join("dangling")).unwrap();
    std::os::unix::fs::symlink(name, tmp.path().join("alias")).unwrap();
    for candidate in ["dangling", "absent"] {
        let (probe, out) = execute(destination(tmp.path().join(candidate)), target::TargetSetup::default());
        assert!(
            matches!(decode(&probe, &out).unwrap().reply, probe::Reply::Observation(Observation::Destination(state)) if (state == DestinationEvidence::Present) == (candidate == "dangling"))
        );
    }
    let (probe, out) = execute(projects(tmp.path(), ""), target::TargetSetup::default());
    let probe::Reply::Observation(Observation::Projects(inventory)) = decode(&probe, &out).unwrap().reply else {
        panic!()
    };
    assert_eq!(inventory.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), [name, "alias"]);
    assert!(inventory.entries.iter().all(|e| e.directory == tmp.path().join(name)));
    let (probe, out) = execute(destination(tmp.path().join("not-a-parent/absent")), target::TargetSetup::default());
    assert!(decode(&probe, &out).is_err());
}

#[test]
fn truncation_wrong_nonce_nonzero_carrier_and_incomplete_capture_fail_closed() {
    let (probe, out) = execute(ReadKind::TargetContext, target::TargetSetup::default());
    assert!(decode(&probe, &out).is_ok());
    for trim in [1, 10, out.stdout.len() / 2] {
        assert!(
            probe
                .decode(tta_launcher::probe::Output {
                    stdout: &out.stdout[..out.stdout.len() - trim],
                    stderr: &out.stderr,
                    status: Some(0),
                    complete: true
                })
                .is_err()
        );
    }
    for (status, complete) in [(Some(255), true), (Some(0), false), (None, true)] {
        assert!(
            probe
                .decode(tta_launcher::probe::Output {
                    stdout: &out.stdout,
                    stderr: &out.stderr,
                    status,
                    complete
                })
                .is_err()
        );
    }
    assert!(decode(&probe::Probe::new(probe.request.clone(), "different-token").unwrap(), &out).is_err());
}

#[test]
fn exact_absence_is_required_and_command_diagnostics_remain_distinct() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = common::scratch_dir("tta-probe-sessions-");
    let program = tmp.path().join("tmux");
    let seed = request(ReadKind::TargetContext, target::TargetSetup::default());
    let sessions = || ReadKind::Sessions {
        endpoint: EndpointKey {
            target: seed.target,
            socket: "/owned/socket".into(),
        },
        program: program.as_os_str().into(),
        purpose: SessionReadPurpose::Selection,
    };
    for (message, absent) in [
        ("error connecting to /owned/socket (No such file or directory)\n", true),
        ("error connecting to /other/socket (No such file or directory)\n", false),
        ("permission denied\n", false),
        ("no server running on /owned/socket\nextra\n", false),
    ] {
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\nprintf '%s' {} >&2\nexit 1\n",
                tta_command::Word::new(message).unwrap().shell().unwrap()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let (probe, out) = execute(sessions(), target::TargetSetup::default());
        assert_eq!(decode(&probe, &out).is_ok(), absent, "{out:?}");
    }
    std::fs::write(&program, "#!/bin/sh\nprintf 'successful diagnostic\\n' >&2\n").unwrap();
    let (probe, out) = execute(sessions(), target::TargetSetup::default());
    assert_eq!(decode(&probe, &out).unwrap().diagnostics.command_stderr, b"successful diagnostic\n");
}

#[test]
fn symlink_missing_not_directory_loop_and_registry_component_classification() {
    let tmp = common::scratch_dir("tta-probe-links-");
    std::fs::write(tmp.path().join("file"), "x").unwrap();
    std::fs::create_dir(tmp.path().join("ordinary")).unwrap();
    for (name, target) in [("not-directory", "file/child"), ("loop", "loop"), ("dangling quote'\n", "missing")] {
        std::os::unix::fs::symlink(target, tmp.path().join(name)).unwrap();
    }
    for (filter, success) in [("ordinary", true), ("dangling quote'\n", true), ("not-directory", false), ("loop", false)] {
        let (probe, out) = execute(projects(tmp.path(), filter), target::TargetSetup::default());
        assert_eq!(decode(&probe, &out).is_ok(), success, "{filter}: {out:?}");
    }
    for name in ["not-directory", "loop", "dangling quote'\n"] {
        let request = request(
            ReadKind::Registry {
                path: Some(tmp.path().join(name)),
            },
            target::TargetSetup::default(),
        );
        let probe = probe::Probe::registry(
            request,
            registry::Step::Entry {
                parent: tmp.path().into(),
                name: name.into(),
            },
            "registry-token",
        )
        .unwrap();
        let out = capture::run(std::process::Command::new("/bin/sh").args(["-c", &probe.shell().unwrap()]), Limits::default()).unwrap();
        assert_eq!(decode(&probe, &out).is_ok(), name == "dangling quote'\n");
        let (probe, out) = execute(destination(tmp.path().join(name)), target::TargetSetup::default());
        assert!(matches!(
            decode(&probe, &out).unwrap().reply,
            probe::Reply::Observation(Observation::Destination(DestinationEvidence::Present))
        ));
    }
}

#[test]
fn denied_parent_is_not_absence_or_empty_inventory() {
    use std::os::unix::fs::PermissionsExt;
    if rustix::process::geteuid().is_root() {
        eprintln!("not applicable: root bypasses directory permissions");
        return;
    }
    let tmp = common::scratch_dir("tta-probe-denied-");
    let blocked = tmp.path().join("blocked");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (p, out) = execute(projects(&blocked, "work"), target::TargetSetup::default());
    let (d, dest) = execute(destination(blocked.clone().join("work")), target::TargetSetup::default());
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(decode(&p, &out).is_err());
    assert!(decode(&d, &dest).is_err());
}
