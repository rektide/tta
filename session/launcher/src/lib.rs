//! Session discovery is replaceable; matching and launch planning are shared.
#![forbid(unsafe_code)]

pub mod backend;
pub mod execution;
mod matching;
mod plan;
mod resolve;
pub mod scaffold;
pub mod socket;

pub use plan::{ClientMode, PlanError, launch, prepare_launch, satellite_name};
pub use resolve::{ResolveError, Selection, project_names, resolve};
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
