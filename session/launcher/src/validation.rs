//! Domain validation shared by discovery and entry; no scheduling or I/O.
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("cannot determine the project root: set HOME or pass --root")]
    MissingRoot,
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
}

pub(crate) fn validate_name(name: &str) -> Result<(), ResolveError> {
    if name.is_empty() || name.contains(['.', ':']) || name.chars().any(char::is_control) {
        return Err(ResolveError::InvalidName(name.to_owned()));
    }
    Ok(())
}
