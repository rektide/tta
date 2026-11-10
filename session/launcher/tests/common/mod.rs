#![allow(dead_code)]
use std::path::Path;
use tta_launcher::{
    entry::{EntryAttempt, protocol::*},
    scaffold, socket,
};
pub fn input(selector: &str) -> Invocation {
    Invocation {
        context: SuppliedContext {
            host: "supplied".into(),
            cwd: "/target".into(),
            socket: socket::Resolved::path("/target/socket"),
            program: "tmux".into(),
            config: None,
            client: ClientEntry::AttachTerminal,
        },
        intent: EntryIntent::selector(selector),
        registry: scaffold::Config::parse(None, None).unwrap(),
        generator: scaffold::Choice::Off,
    }
}
pub fn answer(attempt: &mut EntryAttempt, observation: Observation) {
    let Waiting::Read(request) = attempt.waiting() else {
        panic!("{:?}", attempt.snapshot());
    };
    attempt
        .advance(Event::Observed(ReadCompletion {
            request: request.clone(),
            result: Ok(observation),
        }))
        .unwrap();
}
pub fn seed() -> Observation {
    Observation::Sessions(SessionInventory::Present(vec![tta_launcher::Session {
        id: "$7".parse().unwrap(),
        name: "seed".into(),
        group: None,
    }]))
}
pub fn project(attempt: &mut EntryAttempt) {
    answer(attempt, seed());
    answer(
        attempt,
        Observation::Projects(tta_launcher::ProjectInventory {
            requested_root: "/root".into(),
            root: "/root".into(),
            entries: vec![tta_launcher::ProjectEntry {
                name: "project".into(),
                directory: "/root/project".into(),
            }],
        }),
    );
}
pub fn bind(attempt: &mut EntryAttempt, delivery: Delivery) {
    attempt.advance(Event::Deliver(delivery)).unwrap();
    while let Waiting::Read(request) = attempt.waiting() {
        let observed = match &request.kind {
            ReadKind::Sessions { .. } => seed(),
            ReadKind::ProjectIdentity { .. } => Observation::ProjectIdentity(Path::new("/root/project").into()),
            other => panic!("{other:?}"),
        };
        answer(attempt, observed);
    }
}
