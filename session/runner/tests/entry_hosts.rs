#![cfg(feature = "entry-internal")]

use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use anyhow::Result;
use tta_launcher::entry::{EntryAttempt, action::EntryDecision, catalog, context, probe, protocol::*, registry};
use tta_runner::observe::{self, Observer, local};

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TTA_TEST_TMPDIR").unwrap_or_else(|| "/tmp/opencode".into())).unwrap()
}
fn executable(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
struct PoisonSessions;
impl local::Sessions for PoisonSessions {
    fn inventory(&mut self, _: &std::ffi::OsStr, _: &Path) -> Result<SessionInventory> {
        panic!("unrequested session read")
    }
}
fn supplied(cwd: &Path) -> (EntryAttempt, BoundTarget, ReadRequest) {
    let attempt = EntryAttempt::start(Invocation {
        context: SuppliedContext {
            host: "local-fixture".into(),
            cwd: cwd.into(),
            socket: tta_launcher::socket::Resolved::path(cwd.join("socket")),
            program: "poison-tmux".into(),
            config: None,
            client: ClientEntry::AttachTerminal,
        },
        intent: EntryIntent::selector("work"),
        registry: tta_launcher::scaffold::Config::parse(None, None).unwrap(),
        generator: tta_launcher::scaffold::Choice::Off,
    })
    .unwrap();
    let target = attempt.snapshot().target.unwrap();
    let Waiting::Read(request) = attempt.waiting() else { panic!() };
    let request = request.clone();
    (attempt, target, request)
}
fn shell_probe(request: &ReadRequest) -> Result<Observation> {
    let probe = probe::Probe::new(request.clone(), "test-owned-request-token")?;
    let output = std::process::Command::new("/bin/sh").arg("-c").arg(probe.script()?).output()?;
    let decoded = probe.decode(tta_launcher::probe::Output {
        stdout: &output.stdout,
        stderr: &output.stderr,
        status: output.status.code(),
        complete: true,
    })?;
    let probe::Reply::Observation(observation) = decoded.reply else { panic!() };
    Ok(observation)
}
fn registry_probe(request: &ReadRequest) -> Result<Observation> {
    let ReadKind::Registry { path } = &request.kind else { panic!() };
    let mut task = registry::Task::new(path.clone())?;
    while let Some(step) = task.step().cloned() {
        let probe = probe::Probe::registry(request.clone(), step.clone(), "test-registry-step-token")?;
        let output = std::process::Command::new("/bin/sh").arg("-c").arg(probe.script()?).output()?;
        let decoded = probe.decode(tta_launcher::probe::Output {
            stdout: &output.stdout,
            stderr: &output.stderr,
            status: output.status.code(),
            complete: true,
        })?;
        let probe::Reply::Registry(reply) = decoded.reply else { panic!() };
        task.accept(&step, reply)?;
    }
    Ok(Observation::Registry(task.result().unwrap().clone()))
}

#[test]
fn local_and_worker_free_probes_agree_on_projects_destinations_defaults_and_physical_routes() {
    let temp = scratch();
    let base = temp.path();
    let root = base.join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(root.join("good")).unwrap();
    symlink(root.join("good"), root.join("good-alias")).unwrap();
    symlink("missing", root.join("dangling")).unwrap();
    symlink("loop", root.join("loop")).unwrap();
    std::fs::write(root.join("file"), "not a directory").unwrap();
    symlink(root.join("file/child"), root.join("not-dir")).unwrap();
    let alias = base.join("root-alias");
    symlink(&root, &alias).unwrap();
    let (_, target, mut request) = supplied(base);
    let mut poison = PoisonSessions;
    let mut local = local::Local::supplied(&target, &mut poison).unwrap();
    for kind in [
        ReadKind::Projects {
            slot: RootSlot(2),
            root: alias.clone(),
            filter: "good".into(),
        },
        ReadKind::Destination {
            path: root.join("absent"),
            purpose: DestinationPurpose::Preparation,
        },
        ReadKind::Destination {
            path: root.join("dangling"),
            purpose: DestinationPurpose::Binding,
        },
        ReadKind::Destination {
            path: root.join("file"),
            purpose: DestinationPurpose::Binding,
        },
        ReadKind::GeneratorDefaults {
            cwd: base.into(),
            needs: DefaultNeeds { author: true, license: true },
        },
        ReadKind::ProjectIdentity {
            route: Box::new(tta_launcher::entry::action::ProjectRoute {
                slot: 2,
                requested_root: alias.clone(),
                root: root.clone(),
                name: "good-alias".into(),
                directory: root.join("good"),
            }),
        },
    ] {
        request.kind = kind;
        assert_eq!(local.observe(&request).unwrap(), shell_probe(&request).unwrap(), "{:?}", request.kind);
    }
    for filter in ["", "loop", "not-dir"] {
        request.kind = ReadKind::Projects {
            slot: RootSlot(2),
            root: root.clone(),
            filter: filter.into(),
        };
        assert!(local.observe(&request).is_err());
        assert!(shell_probe(&request).is_err());
    }
    request.kind = ReadKind::Projects {
        slot: RootSlot(2),
        root: base.join("missing-root"),
        filter: "".into(),
    };
    assert!(local.observe(&request).is_err() && shell_probe(&request).is_err());
    let new_root = base.join("new-root");
    std::fs::create_dir_all(new_root.join("good-alias")).unwrap();
    std::fs::remove_file(&alias).unwrap();
    symlink(&new_root, &alias).unwrap();
    request.kind = ReadKind::ProjectIdentity {
        route: Box::new(tta_launcher::entry::action::ProjectRoute {
            slot: 2,
            requested_root: alias,
            root: root.clone(),
            name: "good-alias".into(),
            directory: root.join("good"),
        }),
    };
    assert_eq!(local.observe(&request).unwrap(), Observation::ProjectIdentity(new_root.join("good-alias")));
    assert_eq!(local.observe(&request).unwrap(), shell_probe(&request).unwrap());
}

#[test]
fn bounded_registry_acquisition_agrees_for_absence_symlinks_parent_components_and_failures() {
    let temp = scratch();
    let base = temp.path();
    std::fs::create_dir(base.join("config")).unwrap();
    std::fs::write(base.join("config/tta.toml"), "# observed\n").unwrap();
    symlink("tta.toml", base.join("config/link")).unwrap();
    symlink("missing", base.join("config/dangling")).unwrap();
    symlink("loop", base.join("config/loop")).unwrap();
    let (_, target, mut request) = supplied(base);
    let mut poison = PoisonSessions;
    let mut local = local::Local::supplied(&target, &mut poison).unwrap();
    for path in [
        None,
        Some(base.join("config/tta.toml")),
        Some(base.join("config/link")),
        Some(base.join("config/dangling")),
        Some(base.join("absent/config.toml")),
        Some(base.join("config/../config/tta.toml")),
    ] {
        request.kind = ReadKind::Registry { path };
        assert_eq!(local.observe(&request).unwrap(), registry_probe(&request).unwrap());
    }
    for path in [base.join("config/tta.toml/child"), base.join("config/loop"), base.join("config")] {
        request.kind = ReadKind::Registry { path: Some(path) };
        assert!(local.observe(&request).is_err() && registry_probe(&request).is_err());
    }
    std::fs::write(base.join("config/large"), vec![b'x'; registry::TEXT_LIMIT + 1]).unwrap();
    request.kind = ReadKind::Registry {
        path: Some(base.join("config/large")),
    };
    assert!(local.observe(&request).is_err() && registry_probe(&request).is_err());
    assert!(registry::Task::new(Some(PathBuf::from(format!("/{}", "part/".repeat(257))))).is_err());
}

#[test]
fn probe_rejects_incomplete_nonce_status_and_unterminated_capture_without_certifying_absence() {
    let temp = scratch();
    let (_, _, mut request) = supplied(temp.path());
    request.kind = ReadKind::Destination {
        path: temp.path().join("absent"),
        purpose: DestinationPurpose::Preparation,
    };
    let probe = probe::Probe::new(request, "test-capture-token").unwrap();
    let output = std::process::Command::new("/bin/sh").args(["-c", &probe.script().unwrap()]).output().unwrap();
    for complete in [false, true] {
        let bytes = if complete {
            &output.stdout[..output.stdout.len() - 1]
        } else {
            &output.stdout
        };
        assert!(
            probe
                .decode(tta_launcher::probe::Output {
                    stdout: bytes,
                    stderr: &[],
                    status: Some(0),
                    complete
                })
                .is_err()
        );
    }
    assert!(
        probe
            .decode(tta_launcher::probe::Output {
                stdout: &output.stdout,
                stderr: &[],
                status: None,
                complete: true
            })
            .is_err()
    );
    let wrong = probe::Probe::new(probe.request.clone(), "different-capture-token").unwrap();
    assert!(
        wrong
            .decode(tta_launcher::probe::Output {
                stdout: &output.stdout,
                stderr: &[],
                status: Some(0),
                complete: true
            })
            .is_err()
    );
}

fn remote_fixture(base: &Path) -> (tta_runner::ssh::Ssh, context::Scope, context::Overrides) {
    let carrier = base.join("ssh-carrier");
    // Literal argv carrier, no target tta. Exercise the actual SSH observer's
    // nonce/capture/setup path without claiming an authenticated loopback run.
    executable(
        &carrier,
        "printf carrier-startup\nprintf carrier-stderr >&2\nfor arg do last=$arg; done\nexec /bin/sh -c \"$last\"",
    );
    let tmux = base.join("target-tmux");
    executable(&tmux, "printf 'no sessions\\n' >&2\nexit 1");
    let setup = tta_launcher::target::TargetSetup {
        shell: Some("/bin/sh".into()),
        cwd: Some(base.into()),
        environment: vec![
            tta_launcher::target::Environment {
                name: "TTA_ROOTS".into(),
                value: Some("[\"target-projects\"]".into()),
            },
            tta_launcher::target::Environment {
                name: "TTA_CONFIG".into(),
                value: Some("missing/config.toml".into()),
            },
            tta_launcher::target::Environment {
                name: "TMUX".into(),
                value: Some("/poison/local,1,1".into()),
            },
        ],
        ..Default::default()
    };
    (
        tta_runner::ssh::Ssh::builder()
            .destination("target-fixture".into())
            .program(carrier.into_os_string())
            .options(vec!["-o".into(), "ControlMaster=no".into()])
            .build(),
        context::Scope {
            origin: context::Origin::Ssh {
                host: "target-fixture".into(),
                setup,
            },
            cwd: None,
        },
        context::Overrides {
            program: Some(tmux.into_os_string()),
            scaffold: Some("false".into()),
            satellite_prefix: Some("view-".into()),
            socket: Some(tta_launcher::socket::Selection::Path(base.join("target.sock"))),
            ..Default::default()
        },
    )
}

#[test]
fn same_thin_driver_prepares_and_exports_over_frozen_ssh_observation_and_queries_partial_catalogs() {
    let temp = scratch();
    let base = temp.path();
    std::fs::create_dir_all(base.join("target-projects/work")).unwrap();
    let (ssh, scope, overrides) = remote_fixture(base);
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: scope.clone(),
        overrides: overrides.clone(),
        intent: EntryIntent::selector("work"),
        default_scaffold: "false".into(),
    })
    .unwrap();
    let mut observer = observe::ssh::Ssh::acquire(ssh.clone(), scope.clone(), Default::default()).unwrap();
    observe::drive_reads(&mut attempt, &mut observer).unwrap();
    let Waiting::Planned(plan) = attempt.waiting() else {
        panic!("{:?}", attempt.snapshot())
    };
    assert!(
        matches!(plan.decision(), EntryDecision::StartProject { via: ClientEntry::AttachTerminal, project, .. } if project.physical == base.join("target-projects/work"))
    );
    assert_eq!(plan.target().scope().cwd, Some(base.into()));
    let target = plan.target().clone();
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    observe::drive_reads(&mut attempt, &mut observer).unwrap();
    assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Exported(_))));
    assert!(observer.observations.len() >= 6);
    assert!(observer.observations.iter().all(|record| {
        record
            .decoded
            .as_ref()
            .is_ok_and(|decoded| decoded.diagnostics.startup_stdout == b"carrier-startup" && decoded.diagnostics.transport_stderr == b"carrier-stderr")
    }));
    let command = observer
        .terminal_command(
            &target,
            &tta_command::Expression::from(tta_command::Command::builder().program(tta_command::Word::new("true").unwrap()).build()),
        )
        .unwrap();
    assert_eq!(command.args[2].as_os_str(), "-tt");
    let mut wrong = match attempt.snapshot().journal.iter().find_map(|r| match r {
        AttemptRecord::ReadIssued(request) if matches!(request.kind, ReadKind::Projects { .. }) => Some(request.clone()),
        _ => None,
    }) {
        Some(request) => request,
        None => panic!(),
    };
    let receipts = observer.observations.len();
    wrong.scope.cwd = Some("/poison/not-read".into());
    assert!(observer.observe(&wrong).is_err());
    assert_eq!(observer.observations.len(), receipts);
    let mut foreign = EntryAttempt::acquire(context::Acquisition {
        scope: scope.clone(),
        overrides: overrides.clone(),
        intent: EntryIntent::selector("work"),
        default_scaffold: "false".into(),
    })
    .unwrap();
    observe::drive_reads(&mut foreign, &mut observer).unwrap();
    assert!(matches!(foreign.waiting(), Waiting::Finished(AttemptOutcome::Stopped { .. })));

    let mut query_overrides = overrides;
    query_overrides.roots = Some(vec![base.join("target-projects"), base.join("unreadable-missing")]);
    let mut query = catalog::CatalogQuery::acquire(catalog::Input {
        scope: scope.clone(),
        overrides: query_overrides,
    });
    let mut observer = observe::ssh::Ssh::acquire(ssh, scope, Default::default()).unwrap();
    observe::drive_reads(&mut query, &mut observer).unwrap();
    let catalog::Waiting::Finished(catalog::Outcome::Report(report)) = query.waiting() else {
        panic!()
    };
    assert!(!report.complete());
    assert_eq!(report.completion_names(), ["@1/work"]);
    let mut diagnostics = Vec::new();
    observe::ssh::write_diagnostics(&mut diagnostics, &observer.observations).unwrap();
    assert!(String::from_utf8_lossy(&diagnostics).contains("SSH read failed"));
}

#[test]
fn host_scope_endpoint_and_attempt_mismatches_are_rejected_before_local_io() {
    let temp = scratch();
    let (_, target, request) = supplied(temp.path());
    let mut poison = PoisonSessions;
    let mut local = local::Local::supplied(&target, &mut poison).unwrap();
    let (_, _, foreign) = supplied(temp.path());
    for mutation in 0..6 {
        let mut wrong = request.clone();
        match mutation {
            0 => wrong.scope.cwd = Some("/poison".into()),
            1 => wrong.scope.origin = context::Origin::Local { host: "wrong".into() },
            2 => wrong.target = foreign.target,
            3 => {
                let ReadKind::Sessions { endpoint, .. } = &mut wrong.kind else { panic!() };
                endpoint.target = foreign.target;
            }
            4 => {
                let ReadKind::Sessions { endpoint, .. } = &mut wrong.kind else { panic!() };
                endpoint.socket = "/wrong/socket".into();
            }
            _ => {
                let ReadKind::Sessions { program, .. } = &mut wrong.kind else { panic!() };
                *program = "wrong-tmux".into();
            }
        }
        assert!(local.observe(&wrong).is_err());
    }
}

#[cfg(feature = "cli")]
#[test]
fn local_cli_and_probe_share_strict_session_classification() {
    let temp = scratch();
    let program = temp.path().join("fake-tmux");
    let mut backend = local::SessionBackend::Cli;
    for (body, expected) in [
        ("printf 'no sessions\\n' >&2; exit 1", Some(SessionInventory::Present(Vec::new()))),
        (
            "printf 'error connecting to /exact (No such file or directory)\\n' >&2; exit 1",
            Some(SessionInventory::Absent),
        ),
        ("printf 'error connecting to /other (No such file or directory)\\n' >&2; exit 1", None),
        ("printf 'truncated row'", None),
    ] {
        executable(&program, body);
        let actual = local::Sessions::inventory(&mut backend, program.as_os_str(), Path::new("/exact"));
        match expected {
            Some(expected) => assert_eq!(actual.unwrap(), expected),
            None => assert!(actual.is_err()),
        }
    }
}

#[test]
fn one_read_driver_also_drives_local_supplied_context_without_bootstrap_or_selection_in_the_host() {
    struct Absent {
        calls: usize,
    }
    impl local::Sessions for Absent {
        fn inventory(&mut self, program: &std::ffi::OsStr, socket: &Path) -> Result<SessionInventory> {
            assert_eq!(program, "not-executed");
            assert!(socket.ends_with("socket"));
            self.calls += 1;
            Ok(SessionInventory::Absent)
        }
    }
    let temp = scratch();
    let root = temp.path().join("projects");
    std::fs::create_dir_all(root.join("work")).unwrap();
    let mut intent = EntryIntent::selector("work");
    intent.roots = vec![root];
    let mut attempt = EntryAttempt::start(Invocation {
        context: SuppliedContext {
            host: "local-fixture".into(),
            cwd: temp.path().into(),
            socket: tta_launcher::socket::Resolved::path(temp.path().join("socket")),
            program: "not-executed".into(),
            config: None,
            client: ClientEntry::AttachTerminal,
        },
        intent,
        registry: tta_launcher::scaffold::Config::parse(None, None).unwrap(),
        generator: tta_launcher::scaffold::Choice::Off,
    })
    .unwrap();
    let target = attempt.snapshot().target.unwrap();
    let mut sessions = Absent { calls: 0 };
    let mut observer = local::Local::supplied(&target, &mut sessions).unwrap();
    observe::drive_reads(&mut attempt, &mut observer).unwrap();
    assert!(matches!(attempt.waiting(), Waiting::Planned(plan) if matches!(plan.decision(), EntryDecision::StartProject { .. })));
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    observe::drive_reads(&mut attempt, &mut observer).unwrap();
    assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Exported(_))));
    assert_eq!(sessions.calls, 2);
}

#[test]
fn raw_target_context_is_not_eagerly_utf8_decoded_before_whole_layer_policy() {
    use std::os::unix::ffi::OsStringExt;
    let temp = scratch();
    let (_, scope, mut overrides) = remote_fixture(temp.path());
    overrides.roots = Some(vec![temp.path().join("chosen")]);
    overrides.program = None;
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope,
        overrides,
        intent: EntryIntent::selector("work"),
        default_scaffold: "false".into(),
    })
    .unwrap();
    let Waiting::Read(request) = attempt.waiting() else { panic!() };
    let request = request.clone();
    let probe = probe::Probe::new(request.clone(), "raw-context-request-token").unwrap();
    let raw = std::ffi::OsString::from_vec(b"tmux-\xff".to_vec());
    let output = std::process::Command::new("/bin/sh")
        .args(["-c", &probe.script().unwrap()])
        .env("TTA_ROOTS", std::ffi::OsString::from_vec(vec![0xff]))
        .env("TTA_TMUX", &raw)
        .output()
        .unwrap();
    let decoded = probe
        .decode(tta_launcher::probe::Output {
            stdout: &output.stdout,
            stderr: &output.stderr,
            status: output.status.code(),
            complete: true,
        })
        .unwrap();
    let probe::Reply::Observation(observation) = decoded.reply else { panic!() };
    attempt
        .advance(Event::Observed(ReadCompletion {
            request,
            result: Ok(observation),
        }))
        .unwrap();
    assert_eq!(attempt.snapshot().target.unwrap().program(), raw);
    assert!(matches!(
        attempt.waiting(),
        Waiting::Read(ReadRequest {
            kind: ReadKind::Registry { .. },
            ..
        })
    ));
}

#[test]
fn ssh_capture_caps_and_decode_failures_keep_receipts_and_native_source_types() {
    use std::error::Error as _;
    let temp = scratch();
    let base = temp.path();
    let (ssh, scope, mut overrides) = remote_fixture(base);
    let registry = base.join("invalid.toml");
    std::fs::write(&registry, [0xff]).unwrap();
    overrides.registry = Some(registry);
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: scope.clone(),
        overrides: overrides.clone(),
        intent: EntryIntent::selector("work"),
        default_scaffold: "false".into(),
    })
    .unwrap();
    let mut observer = observe::ssh::Ssh::acquire(ssh.clone(), scope.clone(), Default::default()).unwrap();
    observe::drive_reads(&mut attempt, &mut observer).unwrap();
    let Waiting::Finished(AttemptOutcome::Stopped { boundary, cause }) = attempt.waiting() else {
        panic!()
    };
    assert_eq!(boundary, "registry_read");
    let mut error: &(dyn std::error::Error + 'static) = cause.as_ref();
    let mut utf8 = false;
    loop {
        utf8 |= error.downcast_ref::<std::str::Utf8Error>().is_some();
        match error.source() {
            Some(source) => error = source,
            None => break,
        }
    }
    assert!(utf8, "original decoder error was flattened");
    assert!(observer.observations.len() > 1);
    let last = observer.observations.last().unwrap();
    assert!(last.decoded.is_err() && last.output.as_ref().unwrap().complete());
    assert!(last.output.as_ref().unwrap().stdout.contains(&0xff));

    executable(
        Path::new(&ssh.program),
        "head -c 65536 /dev/zero\nfor arg do last=$arg; done\nexec /bin/sh -c \"$last\"",
    );
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: scope.clone(),
        overrides,
        intent: EntryIntent::selector("work"),
        default_scaffold: "false".into(),
    })
    .unwrap();
    let mut observer = observe::ssh::Ssh::acquire(
        ssh,
        scope,
        tta_runner::capture::Limits {
            per_stream: 64,
            ..Default::default()
        },
    )
    .unwrap();
    observe::drive_reads(&mut attempt, &mut observer).unwrap();
    assert!(
        matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Stopped { boundary, cause }) if boundary == "context_read" && cause.source().is_some())
    );
    assert_eq!(observer.observations.len(), 1);
    let output = observer.observations[0].output.as_ref().unwrap();
    assert!(!output.complete() && output.stdout.len() <= 64 && output.omitted_stdout > 0);
}
