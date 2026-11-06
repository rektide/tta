use std::fs;
use std::io;
use std::path::Path;

use bon::builder;

use crate::backend::SessionLookup;
use tta_launcher::{DestinationState, ProjectInventory, Resolution};
pub use tta_launcher::{ResolveError, Selection};

#[derive(Debug)]
struct Project {
    name: String,
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
    root: Option<&Path>,
    session_name: Option<&str>,
    #[builder(default)] create: bool,
) -> Result<Selection, ResolveError> {
    if argument.is_empty() {
        return Err(ResolveError::Empty);
    }
    let sessions = lookup.sessions().map_err(|source| ResolveError::Observation(Box::new(source)))?;
    let mut inventory = None;
    let mut destination = None;
    loop {
        match tta_launcher::resolve()
            .sessions(&sessions)
            .argument(argument)
            .satellite_prefix(satellite_prefix)
            .maybe_root(root)
            .maybe_session_name(session_name)
            .create(create)
            .maybe_projects(inventory.as_ref())
            .maybe_destination(destination)
            .call()?
        {
            Resolution::Selected(selection) => return Ok(selection),
            Resolution::NeedProjects(root) => inventory = Some(project_inventory(&root, argument)?),
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
        root: canonical.clone(),
        names: projects(&canonical, argument)?.into_iter().map(|project| project.name).collect(),
    })
}

/// Sorted immediate project basenames, using the same directory/symlink policy
/// as resolution. Inventory only: no tmux connection or session-name validation.
pub fn project_names(root: &Path) -> Result<Vec<String>, ResolveError> {
    let mut names: Vec<_> = projects(root, "")?.into_iter().map(|project| project.name).collect();
    names.sort();
    Ok(names)
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
            projects.push(Project { name });
        }
    }
    Ok(projects)
}
