//! Bind the chosen operation, never rerank, fall back or replace its anchor.
use std::path::PathBuf;

use super::{
    action::{BoundAction, BoundOperation, BoundSession, EntryDecision, EntryPlan},
    protocol::{DestinationEvidence, EntryError, ReadRequest},
    select::{ViewFingerprint, WorkspaceCatalog},
};

/// Fresh non-session evidence a new-work binding consumed, with the request
/// that produced it.
#[derive(Clone, Debug)]
pub(super) enum Evidence {
    /// Observed canonical directory the chosen route resolves to now.
    Route(PathBuf),
    Destination(DestinationEvidence),
}

pub(super) fn check(plan: &EntryPlan, sessions: &WorkspaceCatalog, evidence: Option<&(ReadRequest, Evidence)>) -> Result<(), EntryError> {
    assert_eq!(
        sessions.request.endpoint(),
        &plan.target.endpoint(),
        "ingress must correlate the exact endpoint"
    );
    match &plan.decision {
        EntryDecision::OpenView { work, .. } => {
            assert!(evidence.is_none(), "existing-work bindings read no extra evidence");
            anchor_check(&work.anchor, sessions)
        }
        EntryDecision::EnterView { view, .. } => {
            assert!(evidence.is_none(), "existing-work bindings read no extra evidence");
            anchor_check(&view.fingerprint, sessions)
        }
        EntryDecision::StartProject { project, session, .. } => {
            fresh_name_check(&session.name, sessions)?;
            let (_, Evidence::Route(observed)) = evidence.expect("chart reads route identity for StartProject") else {
                unreachable!("route evidence answers a route identity request");
            };
            if observed != &project.route.directory {
                return Err(EntryError::RouteChanged {
                    route: Box::new(project.route.clone()),
                    expected: project.route.directory.clone(),
                    observed: observed.clone(),
                });
            }
            Ok(())
        }
        EntryDecision::ScaffoldAndStart { destination, session, .. } => {
            fresh_name_check(&session.name, sessions)?;
            let (_, Evidence::Destination(evidence)) = evidence.expect("chart reads destination absence for ScaffoldAndStart") else {
                unreachable!("destination evidence answers a destination request");
            };
            match evidence {
                DestinationEvidence::Absent => Ok(()),
                DestinationEvidence::Present => Err(EntryError::DestinationChanged {
                    path: destination.path.clone(),
                }),
            }
        }
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

pub(super) fn operation(plan: &EntryPlan, sessions: &WorkspaceCatalog, evidence: Option<&(ReadRequest, Evidence)>) -> Result<BoundAction, EntryError> {
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
    Ok(BoundAction {
        target: plan.target.clone(),
        operation,
        sessions_witness: sessions.request.clone(),
        evidence_witness: evidence.map(|(request, _)| request.clone()),
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
