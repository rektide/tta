//! Test hosts for the public protocol, not a production compatibility surface.
#![allow(dead_code)]
use std::path::Path;
use tta_launcher::{
    Session,
    entry::{EntryAttempt, action::EntryPlan, protocol::*},
};
use tta_runner::observe::{
    self,
    local::{Local, Sessions},
};

pub struct Inventory(pub Vec<Session>);
impl Sessions for Inventory {
    fn inventory(&mut self, _: &std::ffi::OsStr, _: &Path) -> anyhow::Result<SessionInventory> {
        Ok(SessionInventory::Present(self.0.clone()))
    }
}
pub fn sessions(names: &[&str]) -> Vec<Session> {
    names
        .iter()
        .enumerate()
        .map(|(n, name)| Session {
            id: format!("${n}").parse().unwrap(),
            name: (*name).into(),
            group: None,
        })
        .collect()
}
pub fn input(cwd: &Path, selector: &str) -> Invocation {
    Invocation {
        context: SuppliedContext {
            host: "test".into(),
            cwd: cwd.into(),
            socket: tta_launcher::socket::Resolved::path(cwd.join("socket")),
            program: "tmux".into(),
            config: None,
            client: ClientEntry::AttachTerminal,
        },
        intent: EntryIntent::selector(selector),
        registry: tta_launcher::scaffold::Config::parse(None, None).unwrap(),
        generator: tta_launcher::scaffold::Choice::Off,
    }
}
pub fn read(attempt: &mut EntryAttempt, sessions: Vec<Session>) {
    let target = attempt.snapshot().target.unwrap();
    let mut inventory = Inventory(sessions);
    let mut observer = Local::supplied(&target, &mut inventory).unwrap();
    observe::drive_reads(attempt, &mut observer).unwrap();
}
pub fn prepare(input: Invocation, sessions: Vec<Session>) -> EntryAttempt {
    let mut attempt = EntryAttempt::start(input).unwrap();
    read(&mut attempt, sessions);
    attempt
}
pub fn plan(attempt: &EntryAttempt) -> &EntryPlan {
    let Waiting::Planned(plan) = attempt.waiting() else {
        panic!("{:?}", attempt.snapshot());
    };
    plan
}
pub fn cause(attempt: &EntryAttempt) -> &EntryError {
    let Waiting::Finished(AttemptOutcome::Stopped { cause, .. }) = attempt.waiting() else {
        panic!("{:?}", attempt.snapshot());
    };
    cause
}
pub fn export(mut attempt: EntryAttempt, sessions: Vec<Session>) -> tta_command::Expression {
    attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
    read(&mut attempt, sessions);
    let Waiting::Finished(AttemptOutcome::Exported(action)) = attempt.waiting() else {
        panic!("{:?}", attempt.snapshot());
    };
    action.expression().clone()
}

pub fn terminal(mut attempt: EntryAttempt, sessions: Vec<Session>) -> tta_command::Expression {
    attempt.advance(Event::Deliver(Delivery::Execute { native: false })).unwrap();
    read(&mut attempt, sessions);
    let Waiting::Effect(EffectRequest {
        kind: EffectKind::Terminal { expression },
        ..
    }) = attempt.waiting()
    else {
        panic!("{:?}", attempt.snapshot());
    };
    let expression = expression.clone();
    attempt.advance(Event::Cancel).unwrap(); // No EffectStarted: no mutation.
    expression
}
pub fn enable_creation(input: &mut Invocation) {
    let _ = &input;
    #[cfg(feature = "scaffold-rekon")]
    let adapter = "rekon";
    #[cfg(all(not(feature = "scaffold-rekon"), feature = "scaffold-rekon-native"))]
    let adapter = "rekon-native";
    #[cfg(any(feature = "scaffold-rekon", feature = "scaffold-rekon-native"))]
    {
        input.registry = tta_launcher::scaffold::Config::parse(None, Some(&format!("[scaffold.generators.test]\nadapter='{adapter}'\n"))).unwrap();
        input.generator = tta_launcher::scaffold::Choice::Default;
    }
    #[cfg(not(any(feature = "scaffold-rekon", feature = "scaffold-rekon-native")))]
    panic!("creation test requires an adapter");
}
