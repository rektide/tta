//! tmux-specific execution policy, kept out of the generic command model.

use std::process::ExitStatus;

use crate::command::Execute;
#[cfg(feature = "native")]
use tta_command::Expression;
use tta_command::Handoff;
#[cfg(feature = "native")]
use tta_command::{And, ChangeDir, Word};
use tta_launcher::LaunchPlan;

/// Execution selection, independently reusable from session discovery. Native
/// currently owns ordinary detached project creation inside a live server;
/// terminal operations, grouped queues, cold creation and non-UTF-8 startup
/// argv remain on the CLI path by policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LaunchBackend {
    #[cfg(feature = "native")]
    #[default]
    Native,
    #[cfg_attr(not(feature = "native"), default)]
    Cli,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error(transparent)]
    Command(#[from] crate::command::Error),
    #[error(transparent)]
    Build(#[from] tta_command::Error),
    #[cfg(feature = "native")]
    #[error("native session creation failed; it may already have taken effect; no client handoff or retry was attempted: {0}")]
    Native(#[from] tmax::error::OperationError),
    #[error("native launch preflight failed: {0}")]
    Preflight(#[from] crate::backend::LookupError),
    #[error("cannot forward native tmux output: {0}")]
    Output(#[from] std::io::Error),
}

pub trait ExecuteLaunch {
    /// Execute eligible mutations natively, or the complete CLI expression.
    /// `Handoff::Exec` still replaces the caller with the real tmux client.
    /// There is no native-error fallback, retry, server startup or rollback.
    fn execute(&self, backend: LaunchBackend, handoff: Handoff) -> Result<Option<ExitStatus>, ExecuteError>;
}

impl ExecuteLaunch for LaunchPlan {
    fn execute(&self, backend: LaunchBackend, handoff: Handoff) -> Result<Option<ExitStatus>, ExecuteError> {
        #[cfg(feature = "native")]
        if backend == LaunchBackend::Native
            && let Some(creation) = self.native_creation()
        {
            use std::io::Write;

            use crate::backend::SessionLookup;

            let endpoint = tmax::endpoint::Tmux::new().socket_path(self.socket());
            // Cold creation belongs to the real client (server startup and -f).
            // Classify with a read before any generator or mutation, never retry
            // a failed/possibly executed native create through another backend.
            if (crate::backend::Native { endpoint: endpoint.clone() }).sessions()?.is_empty() {
                return Ok(self.expression().execute(handoff)?);
            }
            if let Some(prefix) = &creation.prefix
                && let Some(status) = prefix.execute(Handoff::Run)?
                && !status.success()
            {
                return Ok(Some(status));
            }
            let directory = creation.directory.canonicalize().map_err(|source| crate::command::Error::Directory {
                path: creation.directory.clone(),
                source,
            })?;
            if !directory.is_dir() {
                return Err(crate::command::Error::Directory {
                    path: creation.directory.clone(),
                    source: std::io::ErrorKind::NotADirectory.into(),
                }
                .into());
            }
            tracing::info!(event = "launch.native", operation = "new-session-detached", socket = ?self.socket());
            let outcome = endpoint.client_directory(&directory).run(creation.invocation.clone())?;
            // Usually empty; preserve successful server diagnostics/hooks too.
            let mut stdout = std::io::stdout().lock();
            for line in outcome.body {
                writeln!(stdout, "{line}")?;
            }
            stdout.flush()?;
            std::io::stderr().write_all(&outcome.stderr)?;
            let remaining = Expression::from(
                And::builder()
                    .steps(vec![
                        ChangeDir::builder().path(Word::new(directory.as_os_str())?).build().into(),
                        creation.handoff.clone(),
                    ])
                    .build(),
            );
            return Ok(remaining.execute(handoff)?);
        }
        #[cfg(not(feature = "native"))]
        let _ = backend;
        Ok(self.expression().execute(handoff)?)
    }
}
