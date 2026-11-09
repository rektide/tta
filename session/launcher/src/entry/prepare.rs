use super::{
    action::{DecisionBasis, EntryDecision, EntryPlan, ViewDirectory, ViewNaming},
    protocol::{BoundTarget, ClientEntry, EntryError, EntryIntent, ViewPreference},
    select::WorkspaceMatch,
};

pub(super) fn generator(config: &crate::scaffold::Config, choice: &crate::scaffold::Choice) -> Result<Option<crate::scaffold::Entry>, EntryError> {
    match choice {
        crate::scaffold::Choice::Off => Ok(None),
        crate::scaffold::Choice::Default => config.default_entry().map_err(EntryError::Registry),
        crate::scaffold::Choice::Named(name) => config.entry(name).map(Some).map_err(EntryError::Registry),
    }
}

pub(super) fn independent(preference: ViewPreference, client: ClientEntry) -> bool {
    match preference {
        ViewPreference::Independent => true,
        ViewPreference::Shared => false,
        ViewPreference::Auto => client == ClientEntry::AttachTerminal,
    }
}

pub(super) fn open_view(target: &BoundTarget, intent: &EntryIntent, found: &WorkspaceMatch) -> Result<EntryPlan, EntryError> {
    let directory = intent.view_directory.as_deref().map(ViewDirectory::new).transpose()?;
    Ok(EntryPlan {
        target: target.clone(),
        decision: EntryDecision::OpenView {
            work: found.work.clone(),
            naming: ViewNaming {
                prefix: intent.satellite_prefix.clone(),
                directory,
            },
            via: target.client,
        },
        basis: basis(found),
    })
}

pub(super) fn enter_view(target: &BoundTarget, found: &WorkspaceMatch) -> EntryPlan {
    EntryPlan {
        target: target.clone(),
        decision: EntryDecision::EnterView {
            view: found.view.clone(),
            via: target.client,
        },
        basis: basis(found),
    }
}

fn basis(found: &WorkspaceMatch) -> DecisionBasis {
    DecisionBasis {
        selection: found.request.clone(),
        workspace: found.work.key.clone(),
        matched_by: found.work.matched_by.clone(),
    }
}
