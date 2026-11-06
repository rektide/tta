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

/// The native handler vocabulary, not another transition table. The graph alone
/// chooses the successor; Rust exhaustively implements each resulting stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Stage {
    Sessions,
    Resolve,
    Projects,
    Destination,
    Defaults,
    FreshSessions,
    Prepare,
    Ready,
}

impl Stage {
    fn from_id(id: &str) -> Self {
        match id {
            "sessions" => Self::Sessions,
            "resolve" => Self::Resolve,
            "projects" => Self::Projects,
            "destination" => Self::Destination,
            "defaults" => Self::Defaults,
            "fresh_sessions" => Self::FreshSessions,
            "prepare" => Self::Prepare,
            "ready" => Self::Ready,
            _ => panic!("unhandled embedded entry machine stage {id}"),
        }
    }
}

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

    pub(super) fn stage(&self) -> Stage {
        let states = self.0.active_leaf_ids();
        assert_eq!(states.len(), 1, "entry machine must have one active leaf");
        Stage::from_id(states[0])
    }

    pub(super) fn send(&mut self, event: Event) {
        let before = self.stage();
        self.0.send(event, 0).expect("entry control events have no fallible actions");
        assert_ne!(self.stage(), before, "entry event must advance the embedded graph");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const DOCUMENT: &str = include_str!("../../../../doc/entry/README.md");

    fn overall_diagram() -> &'static str {
        DOCUMENT.split("```mermaid\nflowchart TD\n").nth(1).unwrap().split("```").next().unwrap()
    }

    // Only the machine's subgraph belongs here. Prelude/completion, native error
    // exits and emission/execution are intentionally outside its control graph.
    // Preserve per-event arrow order: matching an unordered edge set would miss
    // a diagram that reverses the runtime's ordered policy guards.
    fn diagram_agrees(diagram: &str) -> bool {
        type Routes = BTreeMap<(String, String), Vec<String>>;
        let definition: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        let states = definition["states"].as_object().unwrap();
        let mut expected = Routes::new();
        for (source, state) in states {
            if !diagram.lines().any(|line| line.trim_start().starts_with(&format!("{source}["))) {
                return false;
            }
            let Some(events) = state.get("on").and_then(serde_json::Value::as_object) else {
                continue;
            };
            for (event, transitions) in events {
                let transitions = transitions.as_array().cloned().unwrap_or_else(|| vec![transitions.clone()]);
                for transition in &transitions {
                    let target = transition["target"].as_str().unwrap();
                    let label = match transition["guard"]["type"].as_str() {
                        Some(guard) => format!("{event}, {guard}"),
                        None if transitions.len() > 1 => format!("{event}, otherwise"),
                        None => event.clone(),
                    };
                    expected.entry((source.clone(), event.clone())).or_default().push(format!("{label}| {target}"));
                }
            }
        }
        let mut actual = Routes::new();
        for line in diagram.lines() {
            let Some((source, arrow)) = line.trim().split_once(" -->") else {
                continue;
            };
            if !states.contains_key(source) {
                continue;
            }
            let (label, target) = match arrow.strip_prefix('|').and_then(|arrow| arrow.split_once("| ")) {
                Some(parts) => parts,
                None => ("", arrow.trim()),
            };
            if !states.contains_key(target) {
                continue;
            }
            let event = label.split(", ").next().unwrap();
            actual.entry((source.into(), event.into())).or_default().push(format!("{label}| {target}"));
        }
        actual == expected
    }

    #[test]
    fn executable_graph_routes_every_observation_and_guard_in_priority_order() {
        for (defaults, satellite, expected) in [
            (true, true, Stage::Defaults),
            (true, false, Stage::Defaults),
            (false, true, Stage::FreshSessions),
            (false, false, Stage::Prepare),
        ] {
            let mut control = Control::new();
            assert_eq!(control.stage(), Stage::Sessions);
            control.send(Event::Observed);
            for (event, pending) in [(Event::ProjectsNeeded, Stage::Projects), (Event::DestinationNeeded, Stage::Destination)] {
                control.send(event);
                assert_eq!(control.stage(), pending);
                control.send(Event::Observed);
                assert_eq!(control.stage(), Stage::Resolve);
            }
            control.send(Event::Resolved {
                needs_defaults: defaults,
                needs_satellite: satellite,
            });
            assert_eq!(control.stage(), expected);
            if defaults {
                control.send(Event::Observed);
                control.send(Event::Resolved {
                    needs_defaults: false,
                    needs_satellite: satellite,
                });
            }
            if satellite {
                assert_eq!(control.stage(), Stage::FreshSessions);
                control.send(Event::Observed);
            }
            assert_eq!(control.stage(), Stage::Prepare);
            control.send(Event::Prepared);
            assert_eq!(control.stage(), Stage::Ready);
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
        for id in states.keys() {
            Stage::from_id(id);
        }
        for state in states.values() {
            let fields = state.as_object().unwrap();
            assert!(fields.keys().all(|key| matches!(key.as_str(), "id" | "on" | "type")));
        }
    }

    #[test]
    fn maintained_overall_diagram_matches_exact_routes_and_guard_priority() {
        assert!(diagram_agrees(overall_diagram()));
    }

    #[test]
    fn diagram_check_rejects_extra_missing_misnamed_and_reordered_routes() {
        let diagram = overall_diagram();
        let first = "resolve -->|resolved, needs_defaults| defaults";
        let second = "resolve -->|resolved, needs_satellite| fresh_sessions";
        for changed in [
            format!("{diagram}\n    sessions -->|observed| prepare"),
            format!("{diagram}\n    sessions --> prepare"),
            diagram.replace(first, ""),
            diagram.replace("projects_needed|", "projects_needed_typo|"),
            diagram.replace(first, "TEMPORARY").replace(second, first).replace("TEMPORARY", second),
        ] {
            assert!(!diagram_agrees(&changed), "accepted diagram drift: {changed}");
        }
    }
}
