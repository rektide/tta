//! Runner-free observation contracts, matching, registry parsing and launch planning.
#![forbid(unsafe_code)]

pub mod entry;
mod matching;
mod plan;
mod prepared;
pub mod probe;
pub mod projects;
mod resolve;
pub mod scaffold;
pub mod socket;
pub mod target;
pub mod workflow;
mod workspace;

pub use plan::{ClientMode, PlanError, WindowName, launch, prepare_launch};
pub use prepared::{LaunchPlan, NativeCreation};
pub use projects::{ProjectEntry, ProjectInventory, ProjectSource};
pub use resolve::{DestinationState, Resolution, ResolveError, Selection, resolve, resolve_unique};
pub use tmax::operation::SessionId;
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
