//! Bind the chosen operation, never rerank, fall back or replace its anchor.
use super::{
    action::{BoundAction, BoundOperation, EntryDecision, EntryPlan},
    protocol::EntryError,
    select::{ViewFingerprint, WorkspaceCatalog},
};

pub(super) fn check(plan: &EntryPlan, catalog: &WorkspaceCatalog) -> Result<(), EntryError> {
    assert_eq!(catalog.request.endpoint(), &plan.target.endpoint(), "ingress must correlate the exact endpoint");
    let expected = match &plan.decision {
        EntryDecision::OpenView { work, .. } => &work.anchor,
        EntryDecision::EnterView { view, .. } => &view.fingerprint,
    };
    let observed = catalog.inventory.sessions().iter().find(|s| s.id == expected.id).map(ViewFingerprint::from);
    if observed.as_ref() != Some(expected) {
        return Err(EntryError::AnchorChanged {
            expected: expected.clone(),
            observed,
        });
    }
    Ok(())
}

pub(super) fn operation(plan: &EntryPlan, catalog: &WorkspaceCatalog) -> Result<BoundAction, EntryError> {
    let operation = match &plan.decision {
        EntryDecision::EnterView { view, via } => BoundOperation::EnterView {
            view: view.fingerprint.clone(),
            via: *via,
        },
        EntryDecision::OpenView { work, naming, via } => {
            let anchor = catalog.inventory.sessions().iter().find(|s| s.id == work.anchor.id).expect("checked anchor");
            let name = crate::satellite_name(anchor, catalog.inventory.sessions(), &naming.prefix);
            crate::resolve::validate_name(&name).map_err(super::action::Error::from)?;
            BoundOperation::OpenView {
                anchor: work.anchor.clone(),
                name,
                directory: naming.directory.clone(),
                via: *via,
            }
        }
    };
    Ok(BoundAction {
        target: plan.target.clone(),
        operation,
        witness: catalog.request.clone(),
    })
}
