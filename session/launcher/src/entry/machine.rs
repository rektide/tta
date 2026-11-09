//! Only control discriminants cross serde. Native directives cannot assign a
//! successor state: the retained actor is the sole transition owner.
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use xmachines_runtime::{Actor, Ev, EventType, Machine, Registry};

pub(super) const DEFINITION: &str = include_str!("entry.machine.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Directive {
    ReadContext,
    ApplyContextPolicy,
    ReadEndpointIdentity,
    ReadRegistry,
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
    ReadBindingDestination,
    CheckBinding,
    BindOperation,
    ExportAction,
    DispatchExecution,
    RequestGenerate,
    RequestDirectory,
    RequestNative,
    RequestTerminal,
}

/// Which fresh non-session evidence a decision still needs at binding, as a
/// serializable summary computed by the ingress — exactly like `ViewPolicy`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BindingEvidence {
    None,
    Route,
    RootThenDestination,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Supplied,
    ContextPolicy {
        needs_identity: bool,
    },
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
        evidence: BindingEvidence,
    },
    /// The ingress-computed summary of an accepted binding-evidence read:
    /// a verified creation root still needs its destination lstat; a route
    /// identity does not.
    ObservedEvidence {
        destination_needed: bool,
    },
    ViewPolicy {
        independent: bool,
    },
    Prepared,
    Export,
    Execute,
    Checked,
    Bound {
        exported: bool,
    },
    Exported,
    EffectTerminal,
    EffectGenerate,
    EffectDirectory,
    EffectStarted,
    EffectFinished,
    Delegated,
    Stop,
    Cancel,
}

impl EventType for Event {
    fn event_type(&self) -> &str {
        match self {
            Self::Supplied => "supplied",
            Self::ContextPolicy { .. } => "context_policy",
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
            Self::ObservedEvidence { .. } => "observed_evidence",
            Self::ViewPolicy { .. } => "view_policy",
            Self::Prepared => "prepared",
            Self::Export => "export",
            Self::Execute => "execute",
            Self::Checked => "checked",
            Self::Bound { .. } => "bound",
            Self::Exported => "exported",
            Self::EffectTerminal => "effect_terminal",
            Self::EffectGenerate => "effect_generate",
            Self::EffectDirectory => "effect_directory",
            Self::EffectStarted => "effect_started",
            Self::EffectFinished => "effect_finished",
            Self::Delegated => "delegated",
            Self::Stop => "stop",
            Self::Cancel => "cancel",
        }
    }
}

static MACHINE: LazyLock<Arc<Machine<(), Event>>> = LazyLock::new(|| {
    let mut registry = Registry::new();
    registry.guard("needs_endpoint_identity", |args| {
        matches!(args.event, Ev::User(Event::ContextPolicy { needs_identity: true }))
    });
    registry.guard("independent", |args| matches!(args.event, Ev::User(Event::ViewPolicy { independent: true })));
    registry.guard("more_roots", |args| {
        matches!(args.event, Ev::User(Event::ObservedProjects { more_roots: true }))
    });
    registry.guard("needs_defaults", |args| {
        matches!(args.event, Ev::User(Event::ValidatedGenerator { needs_defaults: true }))
    });
    registry.guard("needs_binding_evidence", |args| {
        matches!(
            args.event,
            Ev::User(Event::BindingObserved {
                evidence: BindingEvidence::Route | BindingEvidence::RootThenDestination
            })
        )
    });
    registry.guard("needs_destination_evidence", |args| {
        matches!(args.event, Ev::User(Event::ObservedEvidence { destination_needed: true }))
    });
    registry.guard("export_delivery", |args| matches!(args.event, Ev::User(Event::Bound { exported: true })));
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

    fn supplied_control() -> Control {
        let mut control = Control::start();
        assert_eq!(control.take_directives(), [Directive::ReadContext]);
        control.send(Event::Supplied);
        control
    }

    #[test]
    fn actual_entry_emission_is_once_ordered_and_guarded_on_one_retained_actor() {
        for independent in [false, true] {
            let mut control = supplied_control();
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
                (
                    Event::BindingObserved {
                        evidence: BindingEvidence::None,
                    },
                    Directive::CheckBinding,
                ),
                (Event::Checked, Directive::BindOperation),
                (Event::Bound { exported: true }, Directive::ExportAction),
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
        let mut control = supplied_control();
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
            (
                Event::BindingObserved {
                    evidence: BindingEvidence::RootThenDestination,
                },
                Directive::ReadBindingEvidence,
            ),
            (Event::ObservedEvidence { destination_needed: true }, Directive::ReadBindingDestination),
            (Event::Observed, Directive::CheckBinding),
            (Event::Checked, Directive::BindOperation),
            (Event::Bound { exported: true }, Directive::ExportAction),
        ] {
            control.send(event);
            assert_eq!(control.take_directives(), [directive]);
        }
        control.send(Event::Exported);
        assert_eq!(control.leaf(), "exported");
        // A route-evidence binding goes straight from the identity read to
        // the check state, with no destination recheck.
        let mut control = supplied_control();
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
        control.take_directives();
        control.send(Event::Prepared);
        control.send(Event::Export);
        control.take_directives();
        control.send(Event::BindingObserved {
            evidence: BindingEvidence::Route,
        });
        assert_eq!(control.take_directives(), [Directive::ReadBindingEvidence]);
        control.send(Event::Observed);
        assert_eq!(control.take_directives(), [Directive::CheckBinding]);
        // A generator needing no defaults skips the defaults read entirely.
        let mut control = supplied_control();
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
        let mut control = supplied_control();
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
                let mut control = supplied_control();
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
            Directive::ReadContext,
            Directive::ApplyContextPolicy,
            Directive::ReadEndpointIdentity,
            Directive::ReadRegistry,
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
            Directive::ReadBindingDestination,
            Directive::CheckBinding,
            Directive::BindOperation,
            Directive::ExportAction,
            Directive::DispatchExecution,
            Directive::RequestGenerate,
            Directive::RequestDirectory,
            Directive::RequestNative,
            Directive::RequestTerminal,
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
        guards.dedup();
        assert_eq!(
            guards,
            [
                "export_delivery",
                "independent",
                "more_roots",
                "needs_binding_evidence",
                "needs_defaults",
                "needs_destination_evidence",
                "needs_endpoint_identity"
            ]
        );
        assert!(references.actions.is_empty() && references.actors.is_empty() && references.delays.is_empty());
    }

    // The execution course on the real actor: pure dispatch chooses the first
    // effect; every effect walks requested → in-flight → finished exactly once,
    // and only a terminal effect can delegate.
    #[test]
    fn execution_emission_walks_requested_inflight_finished_once() {
        let mut control = supplied_control();
        control.take_directives();
        for event in [Event::Validated, Event::Observed, Event::Unique] {
            control.send(event);
            control.take_directives();
        }
        control.send(Event::ViewPolicy { independent: false });
        control.take_directives();
        control.send(Event::Prepared);
        assert_eq!(control.leaf(), "planned");
        // An existing-work decision always dispatches one terminal block.
        control.send(Event::Execute);
        assert_eq!(control.take_directives(), [Directive::ReadBindingSessions]);
        control.send(Event::BindingObserved {
            evidence: BindingEvidence::None,
        });
        assert_eq!(control.take_directives(), [Directive::CheckBinding]);
        control.send(Event::Checked);
        assert_eq!(control.take_directives(), [Directive::BindOperation]);
        control.send(Event::Bound { exported: false });
        assert_eq!(control.take_directives(), [Directive::DispatchExecution]);
        control.send(Event::EffectTerminal);
        assert_eq!(control.take_directives(), [Directive::RequestTerminal]);
        assert_eq!(control.leaf(), "run_terminal_requested");
        control.send(Event::Delegated);
        assert_eq!(control.leaf(), "delegated");
        // The native ladder: generate → directory → native → terminal.
        let mut control = supplied_control();
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
        for event in [Event::Eligible, Event::Observed, Event::ValidatedGenerator { needs_defaults: false }] {
            control.send(event);
            control.take_directives();
        }
        control.send(Event::Prepared);
        control.send(Event::Execute);
        control.take_directives();
        control.send(Event::BindingObserved {
            evidence: BindingEvidence::RootThenDestination,
        });
        control.take_directives();
        control.send(Event::ObservedEvidence { destination_needed: true });
        control.take_directives();
        control.send(Event::Observed);
        control.take_directives();
        control.send(Event::Checked);
        control.take_directives();
        control.send(Event::Bound { exported: false });
        assert_eq!(control.take_directives(), [Directive::DispatchExecution]);
        control.send(Event::EffectGenerate);
        assert_eq!(control.take_directives(), [Directive::RequestGenerate]);
        assert_eq!(control.leaf(), "run_generate_requested");
        control.send(Event::EffectStarted);
        assert_eq!(control.leaf(), "run_generate_in_flight");
        assert!(control.take_directives().is_empty());
        control.send(Event::EffectFinished);
        assert_eq!(control.take_directives(), [Directive::RequestDirectory]);
        control.send(Event::EffectStarted);
        control.send(Event::EffectFinished);
        assert_eq!(control.take_directives(), [Directive::RequestNative]);
        control.send(Event::EffectStarted);
        control.send(Event::EffectFinished);
        assert_eq!(control.take_directives(), [Directive::RequestTerminal]);
        assert_eq!(control.leaf(), "run_terminal_requested");
        control.send(Event::EffectStarted);
        assert_eq!(control.leaf(), "run_terminal_in_flight");
        control.send(Event::EffectFinished);
        assert_eq!(control.leaf(), "completed");
        assert_eq!(control.0.status(), xmachines_runtime::Status::Done);
        // A start-project native shape skips generation entirely.
        let mut control = supplied_control();
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
        control.take_directives();
        control.send(Event::Prepared);
        control.send(Event::Execute);
        control.take_directives();
        control.send(Event::BindingObserved {
            evidence: BindingEvidence::Route,
        });
        control.take_directives();
        control.send(Event::Observed);
        control.take_directives();
        control.send(Event::Checked);
        control.take_directives();
        control.send(Event::Bound { exported: false });
        assert_eq!(control.take_directives(), [Directive::DispatchExecution]);
        control.send(Event::EffectDirectory);
        assert_eq!(control.take_directives(), [Directive::RequestDirectory]);
    }
}
