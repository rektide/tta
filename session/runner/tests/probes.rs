//! Process-carrier protocol tests, explicitly NOT SSH evidence.
use std::process::Command;
use tta_launcher::{probe, target, workflow};
use tta_runner::capture::{self, Limits};

fn execute(request: probe::Request, setup: target::TargetSetup) -> (probe::Probe, capture::Output) {
    let probe = probe::Probe::new(request, "process-test-token").unwrap();
    let mut process = Command::new("/bin/sh");
    process.args(["-c", &probe.shell(&setup).unwrap()]);
    let output = capture::run(&mut process, Limits::default()).unwrap();
    (probe, output)
}

fn decode(probe: &probe::Probe, output: &capture::Output) -> anyhow::Result<probe::Decoded> {
    probe.decode(probe::Output {
        stdout: &output.stdout,
        stderr: &output.stderr,
        status: output.status.and_then(|s| s.code()),
        complete: output.complete(),
    })
}

#[test]
fn binary_envelope_preserves_startup_noise_and_target_context() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let shell = dir.path().join("selected shell");
    std::fs::write(
        &shell,
        "#!/bin/sh\nprintf 'startup stdout\\n'\nprintf 'startup stderr\\n' >&2\nexport HOME='/target home' TTA_ROOT='target root'\nexec /bin/sh \"$@\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700)).unwrap();
    let setup = target::TargetSetup::builder()
        .shell(shell.to_str().unwrap().into())
        .cwd(dir.path().into())
        .build();
    let (probe, out) = execute(probe::Request::Setup(target::Request::Context), setup);
    let decoded = decode(&probe, &out).unwrap();
    assert_eq!(decoded.diagnostics.startup_stdout, b"startup stdout\n");
    assert_eq!(decoded.diagnostics.transport_stderr, b"startup stderr\n");
    let probe::Reply::Setup(target::Reply::Context(facts)) = decoded.reply else {
        panic!()
    };
    assert_eq!(facts.cwd, dir.path());
    assert_eq!(facts.value("HOME"), Some("/target home"));
    assert_eq!(facts.value("TTA_ROOT"), Some("target root"));
}

#[test]
fn complete_enumeration_supports_hostile_names_and_dangling_link_collision() {
    let dir = tempfile::tempdir().unwrap();
    let name = "a\n' $; 雪";
    std::fs::create_dir(dir.path().join(name)).unwrap();
    std::os::unix::fs::symlink("missing", dir.path().join("dangling")).unwrap();
    std::os::unix::fs::symlink(name, dir.path().join("alias")).unwrap();
    for candidate in ["dangling", "absent"] {
        let (probe, out) = execute(
            probe::Request::Workflow(workflow::ObservationRequest::Destination {
                path: dir.path().join(candidate),
            }),
            target::TargetSetup::default(),
        );
        let decoded = decode(&probe, &out).unwrap();
        let probe::Reply::Workflow(workflow::ObservationReply::Destination(state)) = decoded.reply else {
            panic!()
        };
        assert_eq!(state == tta_launcher::DestinationState::Present, candidate == "dangling");
    }
    let (probe, out) = execute(
        probe::Request::Workflow(workflow::ObservationRequest::Projects {
            root: dir.path().into(),
            argument: "".into(),
        }),
        target::TargetSetup::default(),
    );
    let decoded = decode(&probe, &out).unwrap();
    let probe::Reply::Workflow(workflow::ObservationReply::Projects(projects)) = decoded.reply else {
        panic!()
    };
    assert_eq!(projects.names, vec![name.to_owned(), "alias".into()]);
    let (probe, out) = execute(
        probe::Request::Workflow(workflow::ObservationRequest::Destination {
            path: dir.path().join("not-a-parent/absent"),
        }),
        target::TargetSetup::default(),
    );
    assert!(decode(&probe, &out).is_err(), "failed parent enumeration is never absence");
}

#[test]
fn truncation_nonzero_carrier_wrong_nonce_and_malformed_data_fail_closed() {
    let (probe, out) = execute(probe::Request::Setup(target::Request::Context), target::TargetSetup::default());
    assert!(decode(&probe, &out).is_ok());
    for trim in [1, 10, out.stdout.len() / 2] {
        assert!(
            probe
                .decode(probe::Output {
                    stdout: &out.stdout[..out.stdout.len() - trim],
                    stderr: &out.stderr,
                    status: Some(0),
                    complete: true
                })
                .is_err()
        );
    }
    assert!(
        probe
            .decode(probe::Output {
                stdout: &out.stdout,
                stderr: &out.stderr,
                status: Some(255),
                complete: true
            })
            .is_err()
    );
    assert!(
        probe
            .decode(probe::Output {
                stdout: &out.stdout,
                stderr: &out.stderr,
                status: Some(0),
                complete: false
            })
            .is_err()
    );
    let other = probe::Probe::new(probe.request.clone(), "different-token").unwrap();
    assert!(decode(&other, &out).is_err());
}

#[test]
fn command_diagnostics_are_distinct_from_startup_and_exact_absence_is_required() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("tmux");
    for (message, absent) in [
        ("error connecting to /owned/socket (No such file or directory)\n", true),
        ("error connecting to /other/socket (No such file or directory)\n", false),
        ("permission denied\n", false),
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
        let (probe, out) = execute(
            probe::Request::Workflow(workflow::ObservationRequest::Sessions {
                socket: "/owned/socket".into(),
                program: program.as_os_str().into(),
            }),
            target::TargetSetup::default(),
        );
        assert_eq!(decode(&probe, &out).is_ok(), absent, "{out:?}");
    }
    std::fs::write(&program, "#!/bin/sh\nprintf 'successful diagnostic\\n' >&2\n").unwrap();
    let (probe, out) = execute(
        probe::Request::Workflow(workflow::ObservationRequest::Sessions {
            socket: "/owned/socket".into(),
            program: program.as_os_str().into(),
        }),
        target::TargetSetup::default(),
    );
    let decoded = decode(&probe, &out).unwrap();
    assert_eq!(decoded.diagnostics.command_stderr, b"successful diagnostic\n");
}

#[test]
fn symlink_missing_non_directory_and_loop_are_distinct_for_projects_and_registry() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file"), "x").unwrap();
    std::fs::create_dir(dir.path().join("ordinary")).unwrap();
    std::os::unix::fs::symlink("file/child", dir.path().join("not-directory")).unwrap();
    std::os::unix::fs::symlink("loop", dir.path().join("loop")).unwrap();
    let dangling = "dangling quote'\n";
    std::os::unix::fs::symlink("missing", dir.path().join(dangling)).unwrap();
    // Errors on irrelevant names do not poison a local-policy-equivalent scan.
    for (argument, success) in [("ordinary", true), (dangling, true), ("not-directory", false), ("loop", false)] {
        let (probe, out) = execute(
            probe::Request::Workflow(workflow::ObservationRequest::Projects {
                root: dir.path().into(),
                argument: argument.into(),
            }),
            target::TargetSetup::default(),
        );
        assert_eq!(decode(&probe, &out).is_ok(), success, "{argument:?}: {out:?}");
    }
    for name in ["not-directory", "loop", dangling] {
        let (probe, out) = execute(
            probe::Request::Setup(target::Request::Entry {
                parent: dir.path().into(),
                name: name.into(),
            }),
            target::TargetSetup::default(),
        );
        assert_eq!(decode(&probe, &out).is_ok(), name == dangling, "registry component {name:?}: {out:?}");
        // Exact scaffold destination uses lstat presence, not the referent.
        let (probe, out) = execute(
            probe::Request::Workflow(workflow::ObservationRequest::Destination { path: dir.path().join(name) }),
            target::TargetSetup::default(),
        );
        assert!(matches!(
            decode(&probe, &out).unwrap().reply,
            probe::Reply::Workflow(workflow::ObservationReply::Destination(tta_launcher::DestinationState::Present))
        ));
    }
}

#[test]
fn denied_parent_is_never_an_absent_destination_or_empty_inventory() {
    use std::os::unix::fs::PermissionsExt;
    if rustix::process::geteuid().is_root() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("blocked");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (project_probe, project) = execute(
        probe::Request::Workflow(workflow::ObservationRequest::Projects {
            root: blocked.clone(),
            argument: "work".into(),
        }),
        target::TargetSetup::default(),
    );
    let (destination_probe, destination) = execute(
        probe::Request::Workflow(workflow::ObservationRequest::Destination { path: blocked.join("work") }),
        target::TargetSetup::default(),
    );
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(decode(&project_probe, &project).is_err());
    assert!(decode(&destination_probe, &destination).is_err());
}
