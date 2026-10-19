use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use bon::builder;

use crate::backend::{LookupError, SessionLookup};
use crate::{Session, matching};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selection {
    Existing(Session),
    Project { directory: PathBuf, session_name: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("provide a nonempty session/project name fragment")]
    Empty,
    #[error("cannot determine the project root: set HOME or pass --root")]
    MissingRoot,
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error("ambiguous session fragment {argument:?}: {candidates:?}; provide a more specific name")]
    Sessions { argument: String, candidates: Vec<String> },
    #[error("ambiguous project fragment {argument:?} under {root:?}: {candidates:?}; provide a more specific name")]
    Projects { argument: String, root: PathBuf, candidates: Vec<String> },
    #[error("no session or immediate project directory matches {argument:?} under {root:?}")]
    Missing { argument: String, root: PathBuf },
    #[error("cannot inspect project directory {path:?}: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("session name {0:?} is empty or contains '.', ':', or control characters; pass --session-name with a distinct tmux-safe name")]
    InvalidName(String),
    #[error("session name {name:?} already belongs to session {id}; choose another --session-name")]
    Collision { name: String, id: crate::SessionId },
}

#[derive(Debug)]
struct Project {
    name: String,
    path: PathBuf,
}

/// Inspect once, prefer existing sessions, and only then enumerate the project
/// root. This is observation, not authorization to create or mutate a session.
#[builder]
pub fn resolve(lookup: &dyn SessionLookup, argument: &str, root: Option<&Path>, session_name: Option<&str>) -> Result<Selection, ResolveError> {
    let _span = tracing::info_span!("resolve", argument).entered();
    if argument.is_empty() {
        return Err(ResolveError::Empty);
    }
    let sessions = lookup.sessions()?;
    tracing::info!(event = "lookup.complete", count = sessions.len());
    match matching::select(sessions.clone(), argument, |session| &session.name) {
        Ok(Some(session)) => {
            tracing::info!(event = "match.existing", session_id = %session.id, session_name = %session.name);
            return Ok(Selection::Existing(session));
        }
        Err(candidates) => {
            tracing::warn!(event = "match.ambiguous", domain = "session", count = candidates.len());
            return Err(ResolveError::Sessions {
                argument: argument.to_owned(),
                candidates: candidates.into_iter().map(|session| format!("{} ({})", session.name, session.id)).collect(),
            });
        }
        Ok(None) => {}
    }

    let root = root.ok_or(ResolveError::MissingRoot)?;
    let directory_error = |source| ResolveError::Directory { path: root.to_owned(), source };
    let root = root.canonicalize().map_err(directory_error)?;
    let entries = fs::read_dir(&root).map_err(|source| ResolveError::Directory { path: root.clone(), source })?;
    let mut projects = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ResolveError::Directory { path: root.clone(), source })?;
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
            projects.push(Project { name, path });
        }
    }
    let project = matching::select(projects, argument, |project| &project.name)
        .map_err(|projects| {
            tracing::warn!(event = "match.ambiguous", domain = "project", count = projects.len());
            ResolveError::Projects {
                argument: argument.to_owned(),
                root: root.clone(),
                candidates: projects.into_iter().map(|project| project.name).collect(),
            }
        })?
        .ok_or_else(|| ResolveError::Missing {
            argument: argument.to_owned(),
            root,
        })?;
    let name = session_name.unwrap_or(&project.name);
    validate_name(name)?;
    if let Some(session) = sessions.iter().find(|session| session.name == name) {
        return Err(ResolveError::Collision {
            name: name.to_owned(),
            id: session.id,
        });
    }
    tracing::info!(event = "match.project", directory = ?project.path, session_name = name);
    Ok(Selection::Project {
        directory: project.path,
        session_name: name.to_owned(),
    })
}

pub(crate) fn validate_name(name: &str) -> Result<(), ResolveError> {
    if name.is_empty() || name.contains(['.', ':']) || name.chars().any(char::is_control) {
        return Err(ResolveError::InvalidName(name.to_owned()));
    }
    Ok(())
}
