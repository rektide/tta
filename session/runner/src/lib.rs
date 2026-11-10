//! Optional runtime adapters for the runner-free `tta-launcher` and `tta-command`.
#![forbid(unsafe_code)]

pub mod backend;
pub mod capture;
pub mod command;
pub mod driver;
pub mod execute;
pub mod execution;
pub mod observe;
pub mod resolve;
pub mod scaffold;
pub mod socket;
pub mod ssh;

pub use driver::{drive, prepare};
