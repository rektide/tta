//! Bind the chosen operation, never rerank, fall back or replace its anchor.
use std::path::PathBuf;

use super::{
    action::{BoundAction, BoundOperation, BoundSession, EntryDecision, EntryPlan, ExecutionShape},
    protocol::{ClientEntry, DestinationEvidence, EntryError, ReadRequest, SessionInventory},
    select::{ViewFingerprint, WorkspaceCatalog},
};

/// Fresh non-session evidence a new-work binding consumed, with the request
/// that produced it.
#[derive(Clone, Debug)]
pub(super) enum Evidence {
    /// Observed canonical directory the chosen route resolves to now.
    Route(PathBuf),
    /// Observed canonical root of the chosen creation site.
    CreationRoot(PathBuf),
    Destination(DestinationEvidence),
}

/// Pure execution-shape choice from the decision, bound target, fresh
/// binding inventory and the caller's native capability — made before any
/// generation or mutation, never as an error fallback afterwards. Native
/// detached creation is eligible only for the local, live nonempty server,
/// inside-switch, representable-startup case on new work.
pub(super) fn shape(plan: &EntryPlan, inventory: &SessionInventory, execution: Option<ExecutionChoice>) -> ExecutionShape {
    let eligible = execution.is_some_and(|choice| choice.native)
        && matches!(plan.target().scope().origin, super::context::Origin::Local { .. })
        && plan.target().client() == ClientEntry::SwitchWithinServer
        && matches!(inventory, SessionInventory::Present(sessions) if !sessions.is_empty())
        && match plan.decision() {
            EntryDecision::StartProject { session, .. } | EntryDecision::ScaffoldAndStart { session, .. } => {
                // The native tmux invocation is UTF-8-only; raw Unix startup
                // argv stays on the complete CLI queue.
                session.startup.iter().all(|word| word.to_str().is_some())
            }
            _ => false,
        };
    if eligible {
        ExecutionShape::NativeCreateThenEnter
    } else {
        ExecutionShape::TerminalBlock
    }
}

/// The delivery the caller chose at `planned`: export (always a portable
/// terminal block) or execute with the host's native capability stated.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExecutionChoice {
    pub native: bool,
}

pub(super) fn check(plan: &EntryPlan, sessions: &WorkspaceCatalog, evidence: &CollectedEvidence) -> Result<(), EntryError> {
    assert_eq!(
        sessions.request.endpoint(),
        &plan.target.endpoint(),
        "ingress must correlate the exact endpoint"
    );
    match &plan.decision {
        EntryDecision::OpenView { work, .. } => {
            assert!(evidence.0.is_empty(), "existing-work bindings read no extra evidence");
            anchor_check(&work.anchor, sessions)
        }
        EntryDecision::EnterView { view, .. } => {
            assert!(evidence.0.is_empty(), "existing-work bindings read no extra evidence");
            anchor_check(&view.fingerprint, sessions)
        }
        EntryDecision::StartProject { project, session, .. } => {
            fresh_name_check(&session.name, sessions)?;
            let observed = evidence.route().expect("chart reads route identity for StartProject");
            if observed != &project.route.directory {
                return Err(EntryError::RouteChanged {
                    route: Box::new(project.route.clone()),
                    expected: project.route.directory.clone(),
                    observed: observed.clone(),
                });
            }
            Ok(())
        }
        // The creation-root identity was already verified when its observation
        // was accepted, BEFORE the destination lstat was issued; here the
        // collected witnesses must show that order.
        EntryDecision::ScaffoldAndStart { destination, session, .. } => {
            fresh_name_check(&session.name, sessions)?;
            assert!(evidence.creation_root().is_some(), "creation-root witness precedes destination binding");
            let evidence = evidence.destination().expect("chart reads destination absence after the creation root");
            match evidence {
                DestinationEvidence::Absent => Ok(()),
                DestinationEvidence::Present => Err(EntryError::DestinationChanged {
                    path: destination.path.clone(),
                }),
            }
        }
    }
}

/// Fresh non-session evidence a new-work binding collected, in request order.
#[derive(Clone, Debug, Default)]
pub(super) struct CollectedEvidence(Vec<(ReadRequest, Evidence)>);

impl CollectedEvidence {
    pub(super) fn push(&mut self, request: ReadRequest, evidence: Evidence) {
        self.0.push((request, evidence));
    }
    fn route(&self) -> Option<&PathBuf> {
        self.0.iter().find_map(|(_, evidence)| match evidence {
            Evidence::Route(path) => Some(path),
            _ => None,
        })
    }
    fn creation_root(&self) -> Option<&PathBuf> {
        self.0.iter().find_map(|(_, evidence)| match evidence {
            Evidence::CreationRoot(path) => Some(path),
            _ => None,
        })
    }
    fn destination(&self) -> Option<&DestinationEvidence> {
        self.0.iter().find_map(|(_, evidence)| match evidence {
            Evidence::Destination(evidence) => Some(evidence),
            _ => None,
        })
    }
    pub(super) fn requests(&self) -> Vec<ReadRequest> {
        self.0.iter().map(|(request, _)| request.clone()).collect()
    }
}

fn anchor_check(expected: &ViewFingerprint, catalog: &WorkspaceCatalog) -> Result<(), EntryError> {
    let observed = catalog.inventory.sessions().iter().find(|s| s.id == expected.id).map(ViewFingerprint::from);
    if observed.as_ref() != Some(expected) {
        return Err(EntryError::AnchorChanged {
            expected: expected.clone(),
            observed,
        });
    }
    Ok(())
}

fn fresh_name_check(name: &str, catalog: &WorkspaceCatalog) -> Result<(), EntryError> {
    if let Some(session) = catalog.inventory.sessions().iter().find(|session| session.name == name) {
        return Err(EntryError::NameCollision {
            name: name.to_owned(),
            id: session.id,
        });
    }
    Ok(())
}

pub(super) fn operation(
    plan: &EntryPlan,
    sessions: &WorkspaceCatalog,
    evidence: &CollectedEvidence,
    execution: Option<ExecutionChoice>,
) -> Result<BoundAction, EntryError> {
    let operation = match &plan.decision {
        EntryDecision::EnterView { view, via } => BoundOperation::EnterView {
            view: view.fingerprint.clone(),
            via: *via,
        },
        EntryDecision::OpenView { work, naming, via } => {
            let anchor = sessions.inventory.sessions().iter().find(|s| s.id == work.anchor.id).expect("checked anchor");
            let name = crate::satellite_name(anchor, sessions.inventory.sessions(), &naming.prefix);
            crate::resolve::validate_name(&name).map_err(super::action::Error::from)?;
            BoundOperation::OpenView {
                anchor: work.anchor.clone(),
                name,
                directory: naming.directory.clone(),
                via: *via,
            }
        }
        EntryDecision::StartProject { session, via, .. } => BoundOperation::StartProject {
            session: bound_session(session),
            via: *via,
        },
        EntryDecision::ScaffoldAndStart { generator, session, via, .. } => BoundOperation::ScaffoldAndStart {
            generator: generator.program.clone(),
            session: bound_session(session),
            via: *via,
        },
    };
    let shape = shape(plan, &sessions.inventory, execution);
    Ok(BoundAction {
        target: plan.target.clone(),
        operation,
        shape,
        sessions_witness: sessions.request.clone(),
        evidence_witnesses: evidence.requests(),
    })
}

fn bound_session(session: &super::action::NewSessionSpec) -> BoundSession {
    BoundSession {
        name: session.name.clone(),
        directory: session.directory.clone(),
        startup: session.startup.clone(),
        window: session.window.clone(),
    }
}
