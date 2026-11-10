//! SSH terminal-effect interpretation under the frozen connection/setup used
//! for every read. Only terminal effects exist on this transport: native
//! detached creation is local by policy, and generation rides inside the
//! aggregate block the target shell short-circuits.
use anyhow::{Context, Result, bail};
use tta_command::{Expression, Handoff};
use tta_launcher::entry::{
    EntryAttempt,
    protocol::{
        AttemptSnapshot, BoundTarget, Delivery, EffectCompletion, EffectFailure, EffectKind, EffectReceipt, EffectRequest, Event, MutationKnowledge, Waiting,
    },
};

use super::{Executor, failure};
use crate::command::Execute;

/// A failed terminal transport keeps its native cause; the mutation may have
/// reached the target before the connection reported the failure.
#[derive(Debug, thiserror::Error)]
pub enum EffectError {
    #[error("cannot construct the terminal command under the frozen SSH setup: {0}")]
    Construct(#[source] anyhow::Error),
    #[error("terminal block exited with {code:?}")]
    Terminal { code: Option<i32> },
    #[error(transparent)]
    Command(#[from] crate::command::Error),
}

pub struct Ssh<'a> {
    observer: &'a mut crate::observe::ssh::Ssh,
}

impl<'a> Ssh<'a> {
    pub fn new(observer: &'a mut crate::observe::ssh::Ssh) -> Self {
        Self { observer }
    }
}

/// Reads, delivery, binding and the terminal effect over one frozen SSH
/// observer. `print` stops at the exported target-side block; execution
/// dispatches the terminal request through the same connection/setup that
/// answered every read. The executor is created per effect so the observer
/// stays borrowed by exactly one party at a time.
pub fn drive(attempt: &mut EntryAttempt, observer: &mut crate::observe::ssh::Ssh, print: bool, handoff: Handoff) -> Result<AttemptSnapshot> {
    let delivery = if print { Delivery::Export } else { Delivery::Execute { native: false } };
    let terminal_handoff = if print { handoff } else { Handoff::Exec };
    crate::observe::drive_reads(attempt, observer).context("entry read driver")?;
    if matches!(attempt.waiting(), Waiting::Planned(_)) {
        attempt
            .advance(Event::Deliver(delivery))
            .context("entry delivery violated the attempt protocol")?;
    }
    crate::observe::drive_reads(attempt, observer).context("entry read driver")?;
    loop {
        let effect = match attempt.waiting() {
            Waiting::Effect(effect) => effect.clone(),
            Waiting::Finished(_) => break,
            Waiting::Read(_) | Waiting::InFlight(_) => bail!("attempt suspended at an impossible boundary mid-execution"),
            Waiting::Planned(_) => bail!("attempt returned to planned after delivery"),
        };
        let target = attempt.snapshot().target.clone().context("executing attempt has a frozen target")?;
        attempt
            .advance(Event::EffectStarted(effect.id))
            .context("effect start violated the attempt protocol")?;
        let result = Ssh::new(observer).perform(&effect, &target, terminal_handoff);
        attempt
            .advance(Event::EffectFinished(EffectCompletion { id: effect.id, result }))
            .context("effect completion violated the attempt protocol")?;
    }
    Ok(attempt.snapshot())
}

impl Executor for Ssh<'_> {
    fn perform(&mut self, effect: &EffectRequest, target: &BoundTarget, handoff: Handoff) -> std::result::Result<EffectReceipt, EffectFailure> {
        let EffectKind::Terminal { expression } = &effect.kind else {
            return Err(failure(
                EffectError::Construct(anyhow::anyhow!("SSH attempts dispatch one terminal block, not {effect:?}")),
                MutationKnowledge::NotStarted,
            ));
        };
        // Constructing the command is read-only and reuses the frozen
        // connection/setup; it is not a dispatch claim.
        let command = self
            .observer
            .terminal_command(target, expression)
            .map_err(|source| failure(EffectError::Construct(source), MutationKnowledge::NotStarted))?;
        match Expression::from(command).execute(handoff) {
            Ok(Some(status)) if status.success() => Ok(EffectReceipt::Terminal { code: status.code() }),
            Ok(Some(status)) => Err(failure(EffectError::Terminal { code: status.code() }, MutationKnowledge::MayHaveOccurred)),
            Ok(None) => Ok(EffectReceipt::Terminal { code: None }),
            Err(source) => Err(failure(EffectError::Command(source), MutationKnowledge::MayHaveOccurred)),
        }
    }
}
