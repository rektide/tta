use crate::{Session, matching};
use bon::builder;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selection {
    Existing(Session),
    Project { directory: PathBuf, session_name: String },
    Create { directory: PathBuf, session_name: String },
}

/// Observed canonical root and immediate directory/symlink basenames. The
/// observer must fail on inspection errors, not manufacture an empty inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectInventory {
    pub root: PathBuf,
    pub names: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationState {
    Absent,
    Present,
}

/// A resolution step is either a selection or the next required fact. No I/O.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Resolution {
    Selected(Selection),
    NeedProjects(PathBuf),
    NeedDestination(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("provide a nonempty session/project name fragment")]
    Empty,
    #[error("cannot determine the project root: set HOME or pass --root")]
    MissingRoot,
    #[error("observation failed: {0}")]
    Observation(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("ambiguous session fragment {argument:?}: {candidates:?}; provide a more specific name")]
    Sessions { argument: String, candidates: Vec<String> },
    #[error("ambiguous project fragment {argument:?} under {root:?}: {candidates:?}; provide a more specific name")]
    Projects { argument: String, root: PathBuf, candidates: Vec<String> },
    #[error("no session or immediate project directory matches {argument:?} under {root:?}")]
    Missing { argument: String, root: PathBuf },
    #[error("cannot scaffold {argument:?}: a new project name must be a single path component, not '.', '..', or a path containing '/'")]
    UnsafeName { argument: String },
    #[error("refusing to scaffold {path:?}: something already exists there")]
    Exists { path: PathBuf },
    #[error("cannot inspect project directory {path:?}: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("project inventory requires an absolute canonical root and immediate basenames")]
    InvalidInventory,
    #[error("session name {0:?} is empty or contains '.', ':', or control characters; pass --session-name with a distinct tmux-safe name")]
    InvalidName(String),
    #[error("satellite prefix {0:?} must be nonempty and contain no '.', ':', or control characters; set --satellite-prefix or TTA_SATELLITE_PREFIX")]
    InvalidSatellitePrefix(String),
    #[error("session name {name:?} already belongs to session {id}; choose another --session-name")]
    Collision { name: String, id: crate::SessionId },
}

/// Prefer sessions before requiring any filesystem facts. Call again with the
/// requested observations; supplied facts never imply filesystem access.
#[builder]
pub fn resolve(
    sessions: &[Session],
    argument: &str,
    #[builder(default = "_")] satellite_prefix: &str,
    root: Option<&Path>,
    session_name: Option<&str>,
    #[builder(default)] create: bool,
    projects: Option<&ProjectInventory>,
    destination: Option<DestinationState>,
) -> Result<Resolution, ResolveError> {
    let _span = tracing::info_span!("resolve", argument).entered();
    if argument.is_empty() {
        return Err(ResolveError::Empty);
    }
    crate::validate_satellite_prefix(satellite_prefix)?;
    tracing::info!(event = "lookup.complete", count = sessions.len());
    if let Some(selection) = resolve_unique(sessions, argument, satellite_prefix)? {
        return Ok(Resolution::Selected(selection));
    }
    let Some(projects) = projects else {
        return Ok(Resolution::NeedProjects(root.ok_or(ResolveError::MissingRoot)?.to_owned()));
    };
    if !projects.root.is_absolute() || projects.names.iter().any(|name| !component(name)) {
        return Err(ResolveError::InvalidInventory);
    }
    let root = &projects.root;
    let (project_name, created) = match matching::select(projects.names.clone(), argument, |name| name).map_err(|candidates| {
        tracing::warn!(event = "match.ambiguous", domain = "project", count = candidates.len());
        ResolveError::Projects {
            argument: argument.to_owned(),
            root: root.clone(),
            candidates,
        }
    })? {
        Some(name) => (name, false),
        None if create => {
            if !component(argument) {
                return Err(ResolveError::UnsafeName { argument: argument.to_owned() });
            }
            let path = root.join(argument);
            match destination {
                None => return Ok(Resolution::NeedDestination(path)),
                Some(DestinationState::Present) => return Err(ResolveError::Exists { path }),
                Some(DestinationState::Absent) => (argument.to_owned(), true),
            }
        }
        None => {
            return Err(ResolveError::Missing {
                argument: argument.to_owned(),
                root: root.clone(),
            });
        }
    };
    let name = session_name.unwrap_or(&project_name);
    validate_name(name)?;
    if let Some(session) = sessions.iter().find(|session| session.name == name) {
        return Err(ResolveError::Collision {
            name: name.to_owned(),
            id: session.id,
        });
    }
    let directory = root.join(&project_name);
    let selection = if created {
        tracing::info!(event = "match.create", ?directory, session_name = name);
        Selection::Create {
            directory,
            session_name: name.to_owned(),
        }
    } else {
        tracing::info!(event = "match.project", ?directory, session_name = name);
        Selection::Project {
            directory,
            session_name: name.to_owned(),
        }
    };
    Ok(Resolution::Selected(selection))
}

/// Resolve only the existing-work branch. Ambiguity is an error guard, never a
/// selected/missing state and never permission to consult projects or scaffold.
pub fn resolve_unique(sessions: &[Session], argument: &str, satellite_prefix: &str) -> Result<Option<Selection>, ResolveError> {
    if argument.is_empty() {
        return Err(ResolveError::Empty);
    }
    crate::validate_satellite_prefix(satellite_prefix)?;
    let selected = crate::workspace::select(sessions, argument, satellite_prefix).map_err(|candidates| {
        tracing::warn!(event = "match.ambiguous", domain = "session", count = candidates.len());
        ResolveError::Sessions {
            argument: argument.to_owned(),
            candidates: candidates.into_iter().map(|session| format!("{} ({})", session.name, session.id)).collect(),
        }
    })?;
    Ok(selected.map(|session| {
        tracing::info!(event = "match.existing", session_id = %session.id, session_name = %session.name);
        Selection::Existing(session)
    }))
}

fn component(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\0'])
}

pub(crate) fn validate_name(name: &str) -> Result<(), ResolveError> {
    if name.is_empty() || name.contains(['.', ':']) || name.chars().any(char::is_control) {
        return Err(ResolveError::InvalidName(name.to_owned()));
    }
    Ok(())
}
