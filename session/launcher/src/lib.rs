//! Runner-free observation contracts, matching, registry parsing and launch planning.
#![forbid(unsafe_code)]

mod matching;
mod plan;
mod prepared;
pub mod probe;
mod resolve;
pub mod scaffold;
pub mod socket;
pub mod target;
pub mod workflow;

pub use plan::{ClientMode, PlanError, WindowName, launch, prepare_launch, satellite_name};
pub use prepared::{LaunchPlan, NativeCreation};
pub use resolve::{DestinationState, ProjectInventory, Resolution, ResolveError, Selection, resolve};
pub use tmax::operation::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
}

impl From<tmax::query::Session> for Session {
    fn from(session: tmax::query::Session) -> Self {
        Self {
            id: session.session_id,
            name: session.name,
        }
    }
}
