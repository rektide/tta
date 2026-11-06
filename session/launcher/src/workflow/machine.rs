//! The executable control graph. Only event kinds and two policy bits cross
//! its serde boundary; domain inputs, observations, results and errors do not.
//!
//! Each pull replays the supplied facts through a fresh actor. This preserves
//! `next(&self)` and Workflow's native Clone + Send + Sync without locks, actor
//! persistence, or a second hand-written transition table. The compiled machine
//! is shared; actors have no actions, clock, I/O, child actors or pending effects.
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use xmachines_runtime::{Actor, Ev, EventType, Machine, Registry};

pub(super) const DEFINITION: &str = include_str!("entry.machine.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Event {
    Observed,
    ProjectsNeeded,
    DestinationNeeded,
    Resolved { needs_defaults: bool, needs_satellite: bool },
    Prepared,
}

impl EventType for Event {
    fn event_type(&self) -> &str {
        match self {
            Self::Observed => "observed",
            Self::ProjectsNeeded => "projects_needed",
            Self::DestinationNeeded => "destination_needed",
            Self::Resolved { .. } => "resolved",
            Self::Prepared => "prepared",
        }
    }
}

static MACHINE: LazyLock<Arc<Machine<(), Event>>> = LazyLock::new(|| {
    let mut registry = Registry::new();
    registry
        .guard("needs_defaults", |args| {
            matches!(args.event, Ev::User(Event::Resolved { needs_defaults: true, .. }))
        })
        .guard("needs_satellite", |args| {
            matches!(args.event, Ev::User(Event::Resolved { needs_satellite: true, .. }))
        });
    // Embedded program, not user configuration. Failure is a developer error;
    // graph and protocol tests cover it without adding public runtime errors.
    Arc::new(Machine::from_json_str(DEFINITION, &registry).expect("valid embedded entry machine"))
});

pub(super) struct Control(Actor<(), Event>);

impl Control {
    pub(super) fn new() -> Self {
        Self(Actor::start(Arc::clone(&MACHINE), 0).expect("entry machine has no fallible initialization"))
    }

    pub(super) fn state(&self) -> &str {
        let states = self.0.active_leaf_ids();
        assert_eq!(states.len(), 1, "entry machine must have one active leaf");
        states[0]
    }

    pub(super) fn send(&mut self, event: Event) {
        let before = self.state().to_owned();
        self.0.send(event, 0).expect("entry control events have no fallible actions");
        assert_ne!(self.state(), before, "entry event must advance the embedded graph");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_graph_routes_every_observation_and_guard_in_priority_order() {
        for (defaults, satellite, expected) in [
            (true, true, "defaults"),
            (true, false, "defaults"),
            (false, true, "fresh_sessions"),
            (false, false, "prepare"),
        ] {
            let mut control = Control::new();
            assert_eq!(control.state(), "sessions");
            control.send(Event::Observed);
            for (event, pending) in [(Event::ProjectsNeeded, "projects"), (Event::DestinationNeeded, "destination")] {
                control.send(event);
                assert_eq!(control.state(), pending);
                control.send(Event::Observed);
                assert_eq!(control.state(), "resolve");
            }
            control.send(Event::Resolved {
                needs_defaults: defaults,
                needs_satellite: satellite,
            });
            assert_eq!(control.state(), expected);
            if defaults {
                control.send(Event::Observed);
                control.send(Event::Resolved {
                    needs_defaults: false,
                    needs_satellite: satellite,
                });
            }
            if satellite {
                assert_eq!(control.state(), "fresh_sessions");
                control.send(Event::Observed);
            }
            assert_eq!(control.state(), "prepare");
            control.send(Event::Prepared);
            assert_eq!(control.state(), "ready");
            assert_eq!(control.0.status(), xmachines_runtime::Status::Done);
            assert!(control.0.pending_effects().is_empty());
            assert_eq!(control.0.next_deadline(), None);
        }
    }

    #[test]
    fn graph_has_only_the_two_native_policy_guards_and_no_effect_implementations() {
        let references = Machine::<(), Event>::references(&serde_json::from_str(DEFINITION).unwrap()).unwrap();
        assert_eq!(
            references.guards.iter().map(|guard| guard.name.as_str()).collect::<Vec<_>>(),
            ["needs_defaults", "needs_satellite"]
        );
        assert!(references.actions.is_empty() && references.delays.is_empty() && references.actors.is_empty());
        assert!(references.exprs.is_empty() && references.contexts.is_empty() && references.languages.is_empty());
        let definition: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        let states = definition["states"].as_object().unwrap();
        assert_eq!(states.len(), 8);
        for state in states.values() {
            let fields = state.as_object().unwrap();
            assert!(fields.keys().all(|key| matches!(key.as_str(), "id" | "on" | "type")));
        }
    }
}
