//! Only control discriminants cross serde. Native directives cannot assign a
//! successor state: the retained actor is the sole transition owner.
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use xmachines_runtime::{Actor, Ev, EventType, Machine, Registry};

pub(super) const DEFINITION: &str = include_str!("entry.machine.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Directive {
    ValidateRegistry,
    ReadSelectionSessions,
    RankSessions,
    ScopeProjects,
    ReadProjects,
    RankProjects,
    PrepareStart,
    CreatePolicy,
    ReadDestination,
    ValidateGenerator,
    ReadDefaults,
    PrepareCreation,
    SettleView,
    PlanOpenView,
    PlanEnterView,
    ReadBindingSessions,
    ReadBindingEvidence,
    CheckBinding,
    BindOperation,
    ExportAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Event {
    Validated,
    Observed,
    Unique,
    Empty,
    Scoped,
    /// The discovery task's remaining-root summary rides the transition; the
    /// chart's `more_roots` guard re-enters the read state on it.
    ObservedProjects {
        more_roots: bool,
    },
    ProjectUnique,
    ProjectEmpty,
    Eligible,
    ValidatedGenerator {
        needs_defaults: bool,
    },
    BindingObserved {
        evidence_needed: bool,
    },
    ViewPolicy {
        independent: bool,
    },
    Prepared,
    Export,
    Checked,
    Bound,
    Exported,
    Stop,
    Cancel,
}

impl EventType for Event {
    fn event_type(&self) -> &str {
        match self {
            Self::Validated => "validated",
            Self::Observed => "observed",
            Self::Unique => "unique",
            Self::Empty => "empty",
            Self::Scoped => "scoped",
            Self::ObservedProjects { .. } => "observed_projects",
            Self::ProjectUnique => "project_unique",
            Self::ProjectEmpty => "project_empty",
            Self::Eligible => "eligible",
            Self::ValidatedGenerator { .. } => "validated_generator",
            Self::BindingObserved { .. } => "binding_observed",
            Self::ViewPolicy { .. } => "view_policy",
            Self::Prepared => "prepared",
            Self::Export => "export",
            Self::Checked => "checked",
            Self::Bound => "bound",
            Self::Exported => "exported",
            Self::Stop => "stop",
            Self::Cancel => "cancel",
        }
    }
}

static MACHINE: LazyLock<Arc<Machine<(), Event>>> = LazyLock::new(|| {
    let mut registry = Registry::new();
    registry.guard("independent", |args| matches!(args.event, Ev::User(Event::ViewPolicy { independent: true })));
    registry.guard("more_roots", |args| {
        matches!(args.event, Ev::User(Event::ObservedProjects { more_roots: true }))
    });
    registry.guard("needs_defaults", |args| {
        matches!(args.event, Ev::User(Event::ValidatedGenerator { needs_defaults: true }))
    });
    registry.guard("needs_binding_evidence", |args| {
        matches!(args.event, Ev::User(Event::BindingObserved { evidence_needed: true }))
    });
    Arc::new(Machine::from_json_str(DEFINITION, &registry).expect("valid embedded entry-attempt machine"))
});

pub(super) struct Control(Actor<(), Event>);

impl Control {
    pub(super) fn start() -> Self {
        Self(Actor::start(Arc::clone(&MACHINE), 0).expect("entry-attempt initialization"))
    }

    pub(super) fn leaf(&self) -> &str {
        let leaves = self.0.active_leaf_ids();
        assert_eq!(leaves.len(), 1, "entry attempt has one active leaf");
        leaves[0]
    }

    pub(super) fn send(&mut self, event: Event) {
        self.0.send(event, 0).expect("embedded control event must succeed");
    }

    pub(super) fn take_directives(&mut self) -> Vec<Directive> {
        self.0
            .take_emitted()
            .into_iter()
            .map(|value| serde_json::from_value(value).expect("embedded chart emitted an unknown native directive"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn actual_entry_emission_is_once_ordered_and_guarded_on_one_retained_actor() {
        for independent in [false, true] {
            let mut control = Control::start();
            assert_eq!(control.take_directives(), [Directive::ValidateRegistry]);
            assert!(control.take_directives().is_empty());
            for (event, directive) in [
                (Event::Validated, Directive::ReadSelectionSessions),
                (Event::Observed, Directive::RankSessions),
                (Event::Unique, Directive::SettleView),
                (
                    Event::ViewPolicy { independent },
                    if independent { Directive::PlanOpenView } else { Directive::PlanEnterView },
                ),
            ] {
                control.send(event);
                assert_eq!(control.take_directives(), [directive]);
                assert!(control.take_directives().is_empty());
            }
            control.send(Event::Prepared);
            assert_eq!(control.leaf(), "planned");
            assert!(control.take_directives().is_empty());
            for (event, directive) in [
                (Event::Export, Directive::ReadBindingSessions),
                (Event::BindingObserved { evidence_needed: false }, Directive::CheckBinding),
                (Event::Checked, Directive::BindOperation),
                (Event::Bound, Directive::ExportAction),
            ] {
                control.send(event);
                assert_eq!(control.take_directives(), [directive]);
            }
            control.send(Event::Exported);
            assert_eq!(control.0.status(), xmachines_runtime::Status::Done);
            assert_eq!(control.leaf(), "exported");
            assert!(control.0.pending_effects().is_empty());
            assert_eq!(control.0.next_deadline(), None);
        }
    }

    // The machine-owned discovery course on the real actor: session miss,
    // scope, per-root read reentry driven by the remaining-root summary,
    // guarded physical rank, then the creation preparation ladder.
    #[test]
    fn discovery_emission_reenters_the_root_read_and_walks_the_creation_ladder() {
        let mut control = Control::start();
        control.take_directives();
        for (event, directive) in [
            (Event::Validated, Directive::ReadSelectionSessions),
            (Event::Observed, Directive::RankSessions),
            (Event::Empty, Directive::ScopeProjects),
            (Event::Scoped, Directive::ReadProjects),
            // Two configured roots: the first accept re-enters the read state.
            (Event::ObservedProjects { more_roots: true }, Directive::ReadProjects),
            (Event::ObservedProjects { more_roots: false }, Directive::RankProjects),
        ] {
            control.send(event);
            assert_eq!(control.take_directives(), [directive]);
            assert!(control.take_directives().is_empty());
        }
        control.send(Event::ProjectEmpty);
        assert_eq!(control.take_directives(), [Directive::CreatePolicy]);
        for (event, directive) in [
            (Event::Eligible, Directive::ReadDestination),
            (Event::Observed, Directive::ValidateGenerator),
            (Event::ValidatedGenerator { needs_defaults: true }, Directive::ReadDefaults),
            (Event::Observed, Directive::PrepareCreation),
        ] {
            control.send(event);
            assert_eq!(control.take_directives(), [directive]);
            assert!(control.take_directives().is_empty());
        }
        control.send(Event::Prepared);
        assert_eq!(control.leaf(), "planned");
        for (event, directive) in [
            (Event::Export, Directive::ReadBindingSessions),
            (Event::BindingObserved { evidence_needed: true }, Directive::ReadBindingEvidence),
            (Event::Observed, Directive::CheckBinding),
            (Event::Checked, Directive::BindOperation),
            (Event::Bound, Directive::ExportAction),
        ] {
            control.send(event);
            assert_eq!(control.take_directives(), [directive]);
        }
        control.send(Event::Exported);
        assert_eq!(control.leaf(), "exported");
        // A generator needing no defaults skips the defaults read entirely.
        let mut control = Control::start();
        control.take_directives();
        for event in [
            Event::Validated,
            Event::Observed,
            Event::Empty,
            Event::Scoped,
            Event::ObservedProjects { more_roots: false },
        ] {
            control.send(event);
            control.take_directives();
        }
        control.send(Event::ProjectEmpty);
        control.take_directives();
        for event in [Event::Eligible, Event::Observed] {
            control.send(event);
            control.take_directives();
        }
        control.send(Event::ValidatedGenerator { needs_defaults: false });
        assert_eq!(control.take_directives(), [Directive::PrepareCreation]);
        // A unique physical project skips creation and prepares new work.
        let mut control = Control::start();
        control.take_directives();
        for event in [
            Event::Validated,
            Event::Observed,
            Event::Empty,
            Event::Scoped,
            Event::ObservedProjects { more_roots: false },
        ] {
            control.send(event);
            control.take_directives();
        }
        control.send(Event::ProjectUnique);
        assert_eq!(control.take_directives(), [Directive::PrepareStart]);
        control.send(Event::Prepared);
        assert_eq!(control.leaf(), "planned");
    }

    #[test]
    fn parent_stop_and_cancel_cover_nested_read_and_pure_states() {
        for event in [Event::Stop, Event::Cancel] {
            for count in 0..12 {
                let mut control = Control::start();
                control.take_directives();
                for event in [
                    Event::Validated,
                    Event::Observed,
                    Event::Empty,
                    Event::Scoped,
                    Event::ObservedProjects { more_roots: false },
                    Event::ProjectEmpty,
                    Event::Eligible,
                    Event::Observed,
                    Event::ValidatedGenerator { needs_defaults: true },
                    Event::Observed,
                    Event::Prepared,
                    Event::Export,
                ]
                .into_iter()
                .take(count)
                {
                    control.send(event);
                    control.take_directives();
                }
                control.send(event);
                assert_eq!(control.leaf(), "stopped");
                assert!(control.take_directives().is_empty());
            }
        }
    }

    // Engine-fit witness for the next stage's per-root read reentry. Native data
    // remains outside this synthetic control-only chart, just as in production.
    #[test]
    fn reenter_emits_parent_child_order_without_restarting_the_actor() {
        let definition = json!({
            "id": "fit", "initial": "collect", "states": {
                "collect": {
                    "entry": [{ "type": "@xstate.emit", "event": { "type": "parent" } }],
                    "initial": "root", "states": {
                        "root": {
                            "entry": [{ "type": "@xstate.emit", "event": { "type": "child" } }],
                            "on": { "observed": { "target": "root", "reenter": true } }
                        }
                    }
                }
            }
        });
        let machine = Arc::new(Machine::<(), Event>::from_json_str(&definition.to_string(), &Registry::new()).unwrap());
        let mut actor = Actor::start(machine, 0).unwrap();
        assert_eq!(actor.take_emitted(), [json!({ "type": "parent" }), json!({ "type": "child" })]);
        for _ in 0..3 {
            actor.send(Event::Observed, 0).unwrap();
            assert_eq!(actor.take_emitted(), [json!({ "type": "child" })]);
            assert!(actor.take_emitted().is_empty());
        }
    }

    #[test]
    fn every_embedded_directive_decodes_and_only_control_guard_is_registered() {
        fn walk(state: &serde_json::Value, directives: &mut Vec<Directive>) {
            if let Some(actions) = state["entry"].as_array() {
                for action in actions {
                    assert_eq!(action["type"], "@xstate.emit");
                    directives.push(serde_json::from_value(action["event"].clone()).unwrap());
                }
            }
            if let Some(states) = state["states"].as_object() {
                for state in states.values() {
                    walk(state, directives);
                }
            }
        }
        let value = serde_json::from_str(DEFINITION).unwrap();
        let mut directives = Vec::new();
        walk(&value, &mut directives);
        let expected = [
            Directive::ValidateRegistry,
            Directive::ReadSelectionSessions,
            Directive::RankSessions,
            Directive::ScopeProjects,
            Directive::ReadProjects,
            Directive::RankProjects,
            Directive::PrepareStart,
            Directive::CreatePolicy,
            Directive::ReadDestination,
            Directive::ValidateGenerator,
            Directive::ReadDefaults,
            Directive::PrepareCreation,
            Directive::SettleView,
            Directive::PlanOpenView,
            Directive::PlanEnterView,
            Directive::ReadBindingSessions,
            Directive::ReadBindingEvidence,
            Directive::CheckBinding,
            Directive::BindOperation,
            Directive::ExportAction,
        ];
        assert_eq!(directives.len(), expected.len());
        for directive in expected {
            assert_eq!(
                directives.iter().filter(|d| **d == directive).count(),
                1,
                "one chart entry for each exhaustive native handler"
            );
        }
        let references = Machine::<(), Event>::references(&value).unwrap();
        let mut guards = references.guards.iter().map(|r| r.name.as_str()).collect::<Vec<_>>();
        guards.sort_unstable();
        assert_eq!(guards, ["independent", "more_roots", "needs_binding_evidence", "needs_defaults"]);
        assert!(references.actions.is_empty() && references.actors.is_empty() && references.delays.is_empty());
    }
}
