//! Optional runtime adapters for the runner-free `tta-launcher` and `tta-command`.
#![forbid(unsafe_code)]

pub mod backend;
pub mod command;
pub mod driver;
pub mod execution;
pub mod resolve;
pub mod scaffold;
pub mod socket;

pub use driver::{drive, prepare};
