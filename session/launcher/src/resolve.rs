use crate::{ProjectInventory, ProjectSource, Session, projects};
use bon::builder;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selection {
    Existing(Session),
    Project {
        directory: PathBuf,
        session_name: String,
        sources: Vec<ProjectSource>,
    },
    Create {
        directory: PathBuf,
        session_name: String,
        sources: Vec<ProjectSource>,
    },
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
    #[error("ambiguous project fragment {argument:?}: {candidates:?}; use a qualified @N/name selector")]
    Projects { argument: String, candidates: Vec<String> },
    #[error("no session or immediate project directory matches {argument:?} under {roots:?}")]
    Missing { argument: String, roots: Vec<PathBuf> },
    #[error("cannot choose a scaffold destination for {argument:?}; use one of {candidates:?}")]
    Destinations { argument: String, candidates: Vec<String> },
    #[error("invalid project selector {argument:?}; use @N/name with N in 1..={roots}")]
    Selector { argument: String, roots: usize },
    #[error("invalid project roots: {0}")]
    RootConfiguration(String),
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
    #[error("project inventory must answer the requested root with absolute physical paths and unique immediate basenames")]
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
    #[builder(default = &[])] roots: &[PathBuf],
    session_name: Option<&str>,
    #[builder(default)] create: bool,
    #[builder(default = &[])] projects: &[ProjectInventory],
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
    let scope = projects::scope(argument, roots)?;
    if roots.iter().any(|root| !root.is_absolute()) {
        return Err(ResolveError::RootConfiguration("project roots must be absolute target-local paths".into()));
    }
    let sources = projects::sources(roots, &scope.indices, projects)?;
    if let Some(&index) = scope.indices.get(projects.len()) {
        return Ok(Resolution::NeedProjects(roots[index].clone()));
    }
    let (directory, project_name, created, sources) = match projects::select(&sources, argument, scope.fragment)? {
        Some(source) => (
            source.entry_path(),
            source.name.clone(),
            false,
            sources.iter().filter(|route| route.directory == source.directory).cloned().collect(),
        ),
        None if create => {
            if !projects::component(scope.fragment) {
                return Err(ResolveError::UnsafeName { argument: argument.to_owned() });
            }
            let physical: std::collections::BTreeSet<_> = projects.iter().map(|inventory| &inventory.root).collect();
            if !scope.qualified && physical.len() > 1 {
                return Err(ResolveError::Destinations {
                    argument: argument.into(),
                    candidates: scope.indices.iter().map(|index| format!("@{}/{}", index + 1, scope.fragment)).collect(),
                });
            }
            let inventory = &projects[0];
            let path = inventory.root.join(scope.fragment);
            let name = session_name.unwrap_or(scope.fragment);
            validate_creation_name(sessions, name)?;
            match destination {
                None => return Ok(Resolution::NeedDestination(path)),
                Some(DestinationState::Present) => return Err(ResolveError::Exists { path }),
                Some(DestinationState::Absent) => (
                    path.clone(),
                    scope.fragment.to_owned(),
                    true,
                    vec![ProjectSource {
                        root_index: scope.indices[0] + 1,
                        requested_root: inventory.requested_root.clone(),
                        root: inventory.root.clone(),
                        name: scope.fragment.into(),
                        directory: path,
                    }],
                ),
            }
        }
        None => {
            return Err(ResolveError::Missing {
                argument: argument.to_owned(),
                roots: scope.indices.iter().map(|index| roots[*index].clone()).collect(),
            });
        }
    };
    let name = session_name.unwrap_or(&project_name);
    validate_creation_name(sessions, name)?;
    let selection = if created {
        tracing::info!(event = "match.create", ?directory, session_name = name);
        Selection::Create {
            directory,
            session_name: name.to_owned(),
            sources,
        }
    } else {
        tracing::info!(event = "match.project", ?directory, session_name = name);
        Selection::Project {
            directory,
            session_name: name.to_owned(),
            sources,
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

fn validate_creation_name(sessions: &[Session], name: &str) -> Result<(), ResolveError> {
    validate_name(name)?;
    if let Some(session) = sessions.iter().find(|session| session.name == name) {
        return Err(ResolveError::Collision {
            name: name.into(),
            id: session.id,
        });
    }
    Ok(())
}

pub(crate) fn validate_name(name: &str) -> Result<(), ResolveError> {
    if name.is_empty() || name.contains(['.', ':']) || name.chars().any(char::is_control) {
        return Err(ResolveError::InvalidName(name.to_owned()));
    }
    Ok(())
}
