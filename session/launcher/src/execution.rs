//! tmux-specific execution policy, kept out of the generic command model.

#[cfg(feature = "native")]
use std::path::PathBuf;
use std::process::ExitStatus;

#[cfg(feature = "native")]
use tmax::operation::Invocation;
use tta_command::{Expression, Handoff};

/// Execution selection, independently reusable from session discovery. Native
/// currently owns grouped detached creation only; terminal operations, cold
/// project creation and raw startup argv remain on the CLI path by policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchBackend {
    #[cfg(feature = "native")]
    Native,
    Cli,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error(transparent)]
    Command(#[from] tta_command::Error),
    #[error("native satellite creation failed; it may already have taken effect; no client handoff or retry was attempted: {0}")]
    Native(#[from] tmax::error::OperationError),
    #[error("cannot forward native tmux output: {0}")]
    Output(#[from] std::io::Error),
}

/// A pure prepared launch, retaining the complete CLI expression for inspection
/// and shell emission. Native execution may create the detached satellite before
/// handing the terminal to the remaining CLI command. Failures short-circuit;
/// already-created sessions are never automatically removed or recreated.
#[derive(Clone, Debug)]
pub struct LaunchPlan {
    pub(crate) expression: Expression,
    #[cfg(feature = "native")]
    pub(crate) socket: PathBuf,
    #[cfg(feature = "native")]
    pub(crate) native_creation: Option<NativeCreation>,
}

#[cfg(feature = "native")]
#[derive(Clone, Debug)]
pub(crate) struct NativeCreation {
    pub invocation: Invocation,
    pub handoff: Expression,
}

impl LaunchPlan {
    pub fn expression(&self) -> &Expression {
        &self.expression
    }

    pub fn into_expression(self) -> Expression {
        self.expression
    }

    /// Execute eligible mutations natively, or the complete CLI expression.
    /// `Handoff::Exec` still replaces the caller with the real tmux client.
    /// There is no native-error fallback, retry, server startup or rollback.
    pub fn execute(&self, backend: LaunchBackend, handoff: Handoff) -> Result<Option<ExitStatus>, ExecuteError> {
        #[cfg(feature = "native")]
        if backend == LaunchBackend::Native
            && let Some(creation) = &self.native_creation
        {
            use std::io::Write;

            tracing::info!(event = "launch.native", operation = "new-session-detached", socket = ?self.socket);
            let outcome = tmax::endpoint::Tmux::new().socket_path(&self.socket).run(creation.invocation.clone())?;
            // Usually empty; preserve successful server diagnostics/hooks too.
            let mut stdout = std::io::stdout().lock();
            for line in outcome.body {
                writeln!(stdout, "{line}")?;
            }
            stdout.flush()?;
            std::io::stderr().write_all(&outcome.stderr)?;
            return Ok(creation.handoff.execute(handoff)?);
        }
        #[cfg(not(feature = "native"))]
        let _ = backend;
        Ok(self.expression.execute(handoff)?)
    }
}
