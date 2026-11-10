use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tta_launcher::{ProjectEntry, ProjectInventory, ResolveError, entry::protocol::DestinationEvidence};

#[derive(Debug)]
struct Project {
    name: String,
    directory: PathBuf,
}

/// A dangling symlink is present. Only NotFound authorizes fresh creation;
/// inaccessible paths and every other inspection failure stop the attempt.
pub fn destination_state(path: &Path) -> Result<DestinationEvidence, ResolveError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(DestinationEvidence::Present),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DestinationEvidence::Absent),
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
