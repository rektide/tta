//! The thin effect driver against actual local interpreters: fake CLI
//! programs and worker-free observers. Real tmux/terminal and authenticated
//! SSH acceptance live in the commissioned verification matrix, not here.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
#[cfg(feature = "scaffold-rekon")]
use std::path::PathBuf;

use anyhow::Result;
use tta_command::Handoff;
#[cfg(feature = "scaffold-rekon")]
use tta_command::{Assignment, Word};
use tta_launcher::entry::{EntryAttempt, protocol::*};
use tta_runner::execute::{self, local};
use tta_runner::observe::local as read_local;

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TTA_TEST_TMPDIR").unwrap_or_else(|| "/tmp/opencode".into())).unwrap()
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(any(feature = "native", feature = "scaffold-rekon"))]
fn effects_journal(snapshot: &AttemptSnapshot) -> Vec<&EffectRequest> {
    snapshot
        .journal
        .iter()
        .filter_map(|record| match record {
            AttemptRecord::EffectIssued(effect) => Some(effect),
            _ => None,
        })
        .collect()
}

fn recorded(path: &Path) -> Vec<String> {
    std::fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).split_terminator('\0').map(str::to_owned).collect())
        .unwrap_or_default()
}

struct FakeSessions(Vec<crate_session::Session>);
mod crate_session {
    pub type Session = tta_launcher::Session;
    pub fn session(id: u32, name: &str, group: Option<&str>) -> Session {
        Session {
            id: format!("${id}").parse().unwrap(),
            name: name.into(),
            group: group.map(str::to_owned),
        }
    }
}

impl read_local::Sessions for FakeSessions {
    fn inventory(&mut self, _: &std::ffi::OsStr, _: &Path) -> Result<SessionInventory> {
        Ok(SessionInventory::Present(self.0.clone()))
    }
}

fn orphan() -> tta_launcher::Session {
    crate_session::session(42, "view-project-42", Some("project"))
}

fn supplied(cwd: &Path, program: &Path, client: ClientEntry) -> EntryAttempt {
    EntryAttempt::start(Invocation {
        context: SuppliedContext {
            host: "local-fixture".into(),
            cwd: cwd.into(),
            socket: tta_launcher::socket::Resolved::path(cwd.join("dead-socket")),
            program: program.as_os_str().to_owned(),
            config: None,
            client,
        },
        intent: EntryIntent::selector("project"),
        registry: tta_launcher::scaffold::Config::parse(None, None).unwrap(),
        generator: tta_launcher::scaffold::Choice::Off,
    })
    .unwrap()
}

fn drive_local(attempt: &mut EntryAttempt, sessions: Vec<tta_launcher::Session>, delivery: Delivery) -> AttemptSnapshot {
    let target = attempt.snapshot().target.clone().expect("supplied target");
    let mut lookup = FakeSessions(sessions);
    let mut observer = read_local::Local::supplied(&target, &mut lookup).unwrap();
    let mut executor = local::Local::default();
    execute::drive(attempt, &mut observer, &mut executor, delivery, Handoff::Run).unwrap()
}

#[test]
fn terminal_block_execution_runs_the_complete_cli_queue_once() {
    let temp = scratch();
    let tmux = temp.path().join("tmux");
    let events = temp.path().join("events");
    executable(&tmux, &format!("printf '%s\\0' \"$@\" >> {}", events.as_os_str().to_str().unwrap()));
    let mut attempt = supplied(temp.path(), &tmux, ClientEntry::AttachTerminal);
    let snapshot = drive_local(&mut attempt, vec![orphan()], Delivery::Execute { native: true });
    // An outside client opens an independent view: one coherent CLI queue.
    let AttemptOutcome::Completed { effects } = snapshot.outcome.as_ref().expect("completed") else {
        panic!("outcome: {:?}", snapshot.outcome)
    };
    assert!(matches!(effects.finished.as_slice(), [(_, EffectReceipt::Terminal { code: Some(0) })]));
    let calls = recorded(&events);
    assert!(calls.iter().any(|arg| arg == "new-session"), "{calls:?}");
    assert!(calls.iter().any(|arg| arg == "attach-session"), "{calls:?}");
    assert!(calls.iter().any(|arg| arg == "keep-last"), "{calls:?}");
    // One tmux invocation carries the whole queue.
    assert_eq!(calls.iter().filter(|arg| *arg == "new-session").count(), 1);
}

#[test]
#[cfg(feature = "native")]
fn failed_native_create_keeps_earlier_receipts_and_never_falls_back() {
    let temp = scratch();
    let tmux = temp.path().join("tmux");
    executable(&tmux, "exit 0");
    // A supplied context whose socket has no server: the native creation
    // dispatch fails with connection uncertainty after the directory check.
    let mut input = Invocation {
        context: SuppliedContext {
            host: "local-fixture".into(),
            cwd: temp.path().into(),
            socket: tta_launcher::socket::Resolved::path(temp.path().join("dead-socket")),
            program: tmux.as_os_str().to_owned(),
            config: None,
            client: ClientEntry::SwitchWithinServer,
        },
        intent: EntryIntent {
            selector: "@1/foo".into(),
            roots: vec![temp.path().join("root")],
            ..EntryIntent::selector("placeholder-unused")
        },
        registry: tta_launcher::scaffold::Config::parse(None, None).unwrap(),
        generator: tta_launcher::scaffold::Choice::Off,
    };
    input.intent.selector = "@1/foo".into();
    std::fs::create_dir(temp.path().join("root")).unwrap();
    std::fs::create_dir(temp.path().join("root/foo")).unwrap();
    let mut attempt = EntryAttempt::start(input).unwrap();
    let snapshot = drive_local(&mut attempt, vec![crate_session::session(7, "other", None)], Delivery::Execute { native: true });
    let AttemptOutcome::Stopped { cause, effects, .. } = snapshot.outcome.as_ref().expect("stopped") else {
        panic!("outcome: {:?}", snapshot.outcome)
    };
    let EntryError::Effect { effect, failure } = &**cause else {
        panic!("cause: {cause:?}")
    };
    assert!(matches!(effect.kind, EffectKind::CreateNative { .. }));
    assert_eq!(failure.mutation, MutationKnowledge::MayHaveOccurred);
    // The directory check finished before the failed creation; no terminal
    // CLI block was ever dispatched as a fallback.
    assert!(matches!(effects.finished.as_slice(), [(_, EffectReceipt::Directory)]));
    assert!(!effects_journal(&snapshot).iter().any(|e| e.is_terminal()));
}

/// A generator-hostile fixture bin and registry-free scaffold input builder.
#[cfg(feature = "scaffold-rekon")]
fn scaffold_fixture(fail: bool) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let temp = scratch();
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let tmux = bin.join("tmux");
    let events = temp.path().join("events");
    let rekon = bin.join("rekon");
    let fail_text = if fail { "exit 29" } else { "" };
    executable(
        &rekon,
        &format!(
            "printf 'rekon %s\\0' \"$1\" >> {events}\n{fail_text}\n/bin/mkdir \"$TTA_TEST_ROOT/$1\"",
            events = events.as_os_str().to_str().unwrap(),
        ),
    );
    executable(&tmux, &format!("printf '%s\\0' \"$@\" >> {}", events.as_os_str().to_str().unwrap()));
    (temp, bin, root, events)
}

#[cfg(feature = "scaffold-rekon")]
fn scaffold_attempt(temp: &Path, bin: &Path, root: &Path, client: ClientEntry) -> (EntryAttempt, Vec<Assignment>) {
    let attempt = EntryAttempt::start(Invocation {
        context: SuppliedContext {
            host: "local-fixture".into(),
            cwd: temp.into(),
            socket: tta_launcher::socket::Resolved::path(temp.join("dead-socket")),
            program: bin.join("tmux").as_os_str().to_owned(),
            config: None,
            client,
        },
        intent: EntryIntent {
            selector: "fresh".into(),
            roots: vec![root.to_owned()],
            ..EntryIntent::selector("fresh")
        },
        registry: tta_launcher::scaffold::Config::parse(None, Some("[scaffold]\ndefault = \"rk\"\n\n[scaffold.generators.rk]\nadapter = \"rekon\"\n")).unwrap(),
        generator: tta_launcher::scaffold::Choice::Default,
    })
    .unwrap();
    let environment = vec![
        Assignment::new("PATH", Word::new(bin.as_os_str()).unwrap()).unwrap(),
        Assignment::new("TTA_TEST_ROOT", Word::new(root.as_os_str()).unwrap()).unwrap(),
    ];
    (attempt, environment)
}

#[cfg(feature = "scaffold-rekon")]
fn drive_scaffold(attempt: &mut EntryAttempt, environment: &[Assignment], sessions: Vec<tta_launcher::Session>, delivery: Delivery) -> AttemptSnapshot {
    let target = attempt.snapshot().target.clone().expect("supplied target");
    let mut lookup = FakeSessions(sessions);
    let mut observer = read_local::Local::supplied(&target, &mut lookup).unwrap();
    let mut executor = local::Local {
        program_environment: environment.to_vec(),
    };
    execute::drive(attempt, &mut observer, &mut executor, delivery, Handoff::Run).unwrap()
}

#[cfg(all(feature = "scaffold-rekon", feature = "native"))]
#[test]
fn native_scaffold_generates_then_checks_before_one_native_create() {
    let (temp, bin, root, events) = scaffold_fixture(false);
    let (mut attempt, environment) = scaffold_attempt(temp.path(), &bin, &root, ClientEntry::SwitchWithinServer);
    let snapshot = drive_scaffold(
        &mut attempt,
        &environment,
        vec![crate_session::session(7, "other", None)],
        Delivery::Execute { native: true },
    );
    // The generator ran and produced the directory; the native create then
    // failed against the dead socket without any CLI fallback.
    let AttemptOutcome::Stopped { cause, effects, .. } = snapshot.outcome.as_ref().expect("stopped") else {
        panic!("outcome: {:?}", snapshot.outcome)
    };
    assert!(
        matches!(&**cause, EntryError::Effect { effect, .. } if matches!(effect.kind, EffectKind::CreateNative { .. })),
        "cause: {cause:?} effects: {effects:?} journal: {:?}",
        effects_journal(&snapshot).iter().map(|e| &e.kind).collect::<Vec<_>>()
    );
    assert_eq!(
        effects.finished,
        vec![
            (effects.finished[0].0, EffectReceipt::Generated),
            (effects.finished[1].0, EffectReceipt::Directory),
        ]
    );
    assert_eq!(recorded(&events), ["rekon fresh"]);
    assert!(root.join("fresh").is_dir());
    assert!(!effects_journal(&snapshot).iter().any(|e| e.is_terminal()));
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn generator_failure_prevents_directory_native_and_terminal_effects() {
    let (temp, bin, root, events) = scaffold_fixture(true);
    let (mut attempt, environment) = scaffold_attempt(temp.path(), &bin, &root, ClientEntry::SwitchWithinServer);
    let snapshot = drive_scaffold(
        &mut attempt,
        &environment,
        vec![crate_session::session(7, "other", None)],
        Delivery::Execute { native: true },
    );
    let AttemptOutcome::Stopped { cause, effects, .. } = snapshot.outcome.as_ref().expect("stopped") else {
        panic!("outcome: {:?}", snapshot.outcome)
    };
    let EntryError::Effect { effect, failure, .. } = &**cause else {
        panic!("cause: {cause:?}")
    };
    assert!(matches!(effect.kind, EffectKind::Generate { .. }), "cause: {cause:?}");
    assert_eq!(failure.mutation, MutationKnowledge::MayHaveOccurred);
    assert!(effects.finished.is_empty());
    assert!(!root.join("fresh").exists());
    assert_eq!(recorded(&events), ["rekon fresh"]);
    // No directory check, native create or terminal block was dispatched.
    assert!(effects_journal(&snapshot).iter().all(|e| matches!(e.kind, EffectKind::Generate { .. })));
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn cold_server_scaffold_uses_the_complete_terminal_block_by_policy() {
    let (temp, bin, root, events) = scaffold_fixture(false);
    let (mut attempt, environment) = scaffold_attempt(temp.path(), &bin, &root, ClientEntry::SwitchWithinServer);
    // No live sessions: creation stays on the complete CLI block, which
    // short-circuits generation before creation and entry.
    let snapshot = drive_scaffold(&mut attempt, &environment, Vec::new(), Delivery::Execute { native: true });
    let AttemptOutcome::Completed { effects } = snapshot.outcome.as_ref().expect("completed") else {
        panic!("outcome: {:?}", snapshot.outcome)
    };
    assert!(
        matches!(effects.finished.as_slice(), [(_, EffectReceipt::Terminal { code: Some(0) })]),
        "effects: {effects:?}"
    );
    let calls = recorded(&events);
    assert_eq!(&calls[..1], &["rekon fresh"]);
    assert!(calls.iter().any(|arg| arg == "new-session"), "{calls:?}");
    assert!(calls.iter().any(|arg| arg == "switch-client"), "{calls:?}");
}
