//! Effect interpretation for the owned entry protocol. Executors perform one
//! requested effect; the driver owns started/finished correlation. Neither
//! chooses work, reranks a selection or dispatches a second sequence.
pub mod local;

use anyhow::{Context, Result, bail};
use tta_command::Handoff;
use tta_launcher::entry::{
    EntryAttempt,
    protocol::{AttemptSnapshot, BoundTarget, Delivery, EffectCompletion, EffectFailure, EffectReceipt, EffectRequest, Event, MutationKnowledge, Waiting},
};

use crate::observe::{Observer, drive_reads};

pub trait Executor {
    /// Perform one requested effect to completion. Called once per effect,
    /// only after `EffectStarted`. A successful terminal effect under
    /// `Handoff::Exec` replaces this process and never returns.
    fn perform(&mut self, effect: &EffectRequest, target: &BoundTarget, handoff: Handoff) -> std::result::Result<EffectReceipt, EffectFailure>;
}

/// Wrap a typed native cause with its honest mutation knowledge.
pub fn failure(source: impl Into<Box<dyn std::error::Error + Send + Sync>>, mutation: MutationKnowledge) -> EffectFailure {
    EffectFailure {
        source: source.into(),
        mutation,
    }
}

/// One thin interpreter: reads through `planned`, the caller's delivery,
/// binding reads, then the bound effect sequence once. There is no second
/// orchestration after the plan and no fallback between effects.
pub fn drive(
    attempt: &mut EntryAttempt,
    observer: &mut dyn Observer,
    executor: &mut dyn Executor,
    delivery: Delivery,
    handoff: Handoff,
) -> Result<AttemptSnapshot> {
    drive_reads(attempt, observer).context("entry read driver")?;
    if matches!(attempt.waiting(), Waiting::Planned(_)) {
        attempt
            .advance(Event::Deliver(delivery))
            .context("entry delivery violated the attempt protocol")?;
    }
    drive_reads(attempt, observer).context("entry read driver")?;
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
        let result = executor.perform(&effect, &target, handoff);
        attempt
            .advance(Event::EffectFinished(EffectCompletion { id: effect.id, result }))
            .context("effect completion violated the attempt protocol")?;
    }
    Ok(attempt.snapshot())
}
