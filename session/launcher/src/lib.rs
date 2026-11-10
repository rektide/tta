//! Runner-free retained entry and catalog protocols, semantic actions and pure
//! matching, context policy, registry parsing and worker-free probe codecs.
#![forbid(unsafe_code)]

pub mod entry;
mod matching;
pub mod probe;
pub mod projects;
pub mod scaffold;
pub mod socket;
pub mod target;
mod validation;
mod workspace;

pub use projects::{ProjectEntry, ProjectInventory, ProjectSource};
pub use tmax::operation::SessionId;
pub use validation::ResolveError;
pub use workspace::{satellite_alias, satellite_name, session_names, validate_satellite_prefix};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    /// Actual server-local tmux group identity; `None` means ungrouped. Never
    /// infer membership from a satellite-shaped name.
    pub group: Option<String>,
}

impl From<tmax::query::Session> for Session {
    fn from(session: tmax::query::Session) -> Self {
        Self {
            id: session.session_id,
            name: session.name,
            group: session.grouped.then_some(session.group),
        }
    }
}
