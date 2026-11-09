use super::{
    action::{
        CreationBasis, DecisionBasis, EntryDecision, EntryPlan, GeneratorProgram, NewSessionSpec, PaneDirectory, ProjectBasis, ViewDirectory, ViewNaming,
    },
    discover,
    protocol::{BoundTarget, ClientEntry, EntryError, EntryIntent, GeneratorDefaults, ViewPreference},
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

/// StartProject plan: chosen physical identity, selected route, all routes and
/// a once-settled new-session spec. Names, paths, startup and window policy
/// are constructor-checked here, once.
pub(super) fn start_project(
    target: &BoundTarget,
    intent: &EntryIntent,
    selection: &super::select::WorkspaceCatalog,
    task: &discover::Task,
    project: &super::action::ProjectRef,
) -> Result<EntryPlan, EntryError> {
    let name = intent.session_name.as_deref().unwrap_or(&project.name);
    discover::validate_new_session_name(selection, name)?;
    super::action::validate_startup(&intent.startup)?;
    Ok(EntryPlan {
        target: target.clone(),
        decision: EntryDecision::StartProject {
            project: project.clone(),
            session: NewSessionSpec {
                name: name.to_owned(),
                startup: intent.startup.clone(),
                directory: PaneDirectory::new(intent.directory.as_deref().unwrap_or(&project.entry))?,
                window: intent.window.clone(),
            },
            via: target.client,
        },
        basis: DecisionBasis::Project(Box::new(ProjectBasis {
            selection: selection.request.clone(),
            roots: task.roots_requests().to_vec(),
            routes: project.routes.clone(),
        })),
    })
}

/// ScaffoldAndStart plan: fixed creation site and the generator program
/// materialized exactly once with the actually observed defaults. No
/// preliminary expression with fake empty defaults is ever built.
pub(super) fn scaffold_and_start(
    target: &BoundTarget,
    intent: &EntryIntent,
    selection: &super::select::WorkspaceCatalog,
    task: &discover::Task,
    creation: &discover::Creation,
    generator: &crate::scaffold::Entry,
    defaults: &GeneratorDefaults,
) -> Result<EntryPlan, EntryError> {
    let program = generator
        .scaffold(
            &creation.site.name,
            &creation.site.root,
            &crate::scaffold::Defaults {
                author: defaults.author.clone(),
                license: defaults.license.clone(),
            },
        )
        .map_err(EntryError::Generator)?;
    super::action::validate_startup(&intent.startup)?;
    Ok(EntryPlan {
        target: target.clone(),
        decision: EntryDecision::ScaffoldAndStart {
            destination: creation.site.clone(),
            generator: GeneratorProgram {
                generator: generator.name.clone(),
                program,
            },
            session: NewSessionSpec {
                name: creation.session_name.clone(),
                startup: intent.startup.clone(),
                directory: PaneDirectory::new(intent.directory.as_deref().unwrap_or(&creation.site.path))?,
                window: intent.window.clone(),
            },
            via: target.client,
        },
        basis: DecisionBasis::Creation(Box::new(CreationBasis {
            selection: selection.request.clone(),
            roots: task.roots_requests().to_vec(),
            destination: creation.destination.as_ref().map(|(request, _)| request.clone()),
            defaults: creation.defaults.as_ref().map(|(request, _)| request.clone()),
            generator: generator.name.clone(),
        })),
    })
}

fn basis(found: &WorkspaceMatch) -> DecisionBasis {
    DecisionBasis::Workspace {
        selection: found.request.clone(),
        workspace: found.work.key.clone(),
        matched_by: found.work.matched_by.clone(),
    }
}
