use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use bon::builder;

use crate::backend::SessionLookup;
use tta_launcher::{DestinationState, ProjectEntry, ProjectInventory, Resolution};
pub use tta_launcher::{ResolveError, Selection};

#[derive(Debug)]
struct Project {
    name: String,
    directory: PathBuf,
}

/// Inspect once, prefer existing sessions, and only then enumerate the project
/// root. This is observation, not authorization to create or mutate a session.
/// With `create` set, an otherwise-clean miss becomes a `Create` selection
/// after validating the fragment as a fresh, nonexisting project name.
#[builder]
pub fn resolve(
    lookup: &dyn SessionLookup,
    argument: &str,
    #[builder(default = "_")] satellite_prefix: &str,
    #[builder(default = &[])] roots: &[PathBuf],
    session_name: Option<&str>,
    #[builder(default)] create: bool,
) -> Result<Selection, ResolveError> {
    if argument.is_empty() {
        return Err(ResolveError::Empty);
    }
    let sessions = lookup.sessions().map_err(|source| ResolveError::Observation(Box::new(source)))?;
    let roots: Vec<_> = roots
        .iter()
        .map(|root| {
            if root.is_absolute() {
                Ok(root.clone())
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(root))
                    .map_err(|source| ResolveError::Directory { path: root.clone(), source })
            }
        })
        .collect::<Result<_, _>>()?;
    let mut inventories = Vec::new();
    let mut destination = None;
    loop {
        match tta_launcher::resolve()
            .sessions(&sessions)
            .argument(argument)
            .satellite_prefix(satellite_prefix)
            .roots(&roots)
            .maybe_session_name(session_name)
            .create(create)
            .projects(&inventories)
            .maybe_destination(destination)
            .call()?
        {
            Resolution::Selected(selection) => return Ok(selection),
            Resolution::NeedProjects(root) => inventories.push(project_inventory(&root, tta_launcher::projects::observation_fragment(argument))?),
            Resolution::NeedDestination(path) => destination = Some(destination_state(&path)?),
        }
    }
}

/// A dangling symlink is present. Only NotFound authorizes fresh creation;
/// inaccessible paths and every other inspection failure stop the workflow.
pub fn destination_state(path: &Path) -> Result<DestinationState, ResolveError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(DestinationState::Present),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DestinationState::Absent),
        Err(source) => Err(ResolveError::Directory { path: path.to_owned(), source }),
    }
}

pub fn project_inventory(root: &Path, argument: &str) -> Result<ProjectInventory, ResolveError> {
    let canonical = root
        .canonicalize()
        .map_err(|source| ResolveError::Directory { path: root.to_owned(), source })?;
    Ok(ProjectInventory {
        requested_root: root.to_owned(),
        root: canonical.clone(),
        entries: projects(&canonical, argument)?
            .into_iter()
            .map(|project| ProjectEntry {
                name: project.name,
                directory: project.directory,
            })
            .collect(),
    })
}

fn projects(root: &Path, argument: &str) -> Result<Vec<Project>, ResolveError> {
    let entries = fs::read_dir(root).map_err(|source| ResolveError::Directory { path: root.to_owned(), source })?;
    let mut projects = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ResolveError::Directory { path: root.to_owned(), source })?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            // A UTF-8 CLI fragment cannot name this directory faithfully.
            continue;
        };
        if !name.contains(argument) {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type().map_err(|source| ResolveError::Directory { path: path.clone(), source })?;
        let is_directory = if kind.is_symlink() {
            match fs::metadata(&path) {
                Ok(metadata) => metadata.is_dir(),
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(source) => return Err(ResolveError::Directory { path, source }),
            }
        } else {
            kind.is_dir()
        };
        if is_directory {
            let directory = path.canonicalize().map_err(|source| ResolveError::Directory { path, source })?;
            projects.push(Project { name, directory });
        }
    }
    Ok(projects)
}
