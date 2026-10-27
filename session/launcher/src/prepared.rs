//! Semantic launch data, independent of a runner or backend feature.
use std::path::{Path, PathBuf};
use tmax::operation::Invocation;
use tta_command::Expression;

/// Complete portable expression plus the semantic native-creation candidate.
/// Eligibility still needs a fresh runner preflight; this is never an outcome.
#[derive(Clone, Debug)]
pub struct LaunchPlan {
    pub(crate) expression: Expression,
    pub(crate) socket: PathBuf,
    pub(crate) native_creation: Option<NativeCreation>,
}

#[derive(Clone, Debug)]
pub struct NativeCreation {
    pub invocation: Invocation,
    pub prefix: Option<Expression>,
    pub directory: PathBuf,
    pub handoff: Expression,
}

impl LaunchPlan {
    pub fn expression(&self) -> &Expression {
        &self.expression
    }
    pub fn into_expression(self) -> Expression {
        self.expression
    }
    pub fn socket(&self) -> &Path {
        &self.socket
    }
    pub fn native_creation(&self) -> Option<&NativeCreation> {
        self.native_creation.as_ref()
    }
}
