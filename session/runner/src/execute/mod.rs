//! Effect interpretation for the owned entry protocol. Executors perform one
//! requested effect; the driver owns started/finished correlation. Neither
//! chooses work, reranks a selection or dispatches a second sequence.
pub mod local;
pub mod ssh;

use anyhow::{Context, Result, bail};
use tta_command::Handoff;
use tta_launcher::entry::{
    EntryAttempt,
    action::EntryDecision,
    protocol::{
        AttemptSnapshot, BoundTarget, ClientEntry, Delivery, EffectCompletion, EffectFailure, EffectReceipt, EffectRequest, Event, MutationKnowledge, Waiting,
    },
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
    run(attempt, observer, executor, handoff)
}

/// Binding reads and the bound effect sequence after the caller delivered.
/// Emits tta's diagnostic trace events (resolution/plan and emission or
/// handoff) after the plan is bound and before the first dispatch: a
/// successful exec never returns to emit them afterwards.
pub fn run(attempt: &mut EntryAttempt, observer: &mut dyn Observer, executor: &mut dyn Executor, handoff: Handoff) -> Result<AttemptSnapshot> {
    drive_reads(attempt, observer).context("entry read driver")?;
    if let Some(plan) = attempt.snapshot().plan {
        trace_plan(&plan, handoff);
    }
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

/// The production diagnostic contract, derived from the semantic decision —
/// never a second classification matrix in a frontend.
pub fn operation_label(decision: &EntryDecision, via: ClientEntry) -> &'static str {
    let inside = via == ClientEntry::SwitchWithinServer;
    match decision {
        EntryDecision::EnterView { .. } => {
            if inside {
                "switch-client"
            } else {
                "attach-session"
            }
        }
        EntryDecision::OpenView { .. } => {
            if inside {
                "new-grouped-session-detached+switch-client"
            } else {
                "new-grouped-session-detached+attach"
            }
        }
        EntryDecision::StartProject { .. } => {
            if inside {
                "new-session-detached+switch-client"
            } else {
                "new-session+attach"
            }
        }
        EntryDecision::ScaffoldAndStart { .. } => {
            if inside {
                "scaffold+new-session-detached+switch-client"
            } else {
                "scaffold+new-session+attach"
            }
        }
    }
}

pub(crate) fn trace_plan(plan: &tta_launcher::entry::action::EntryPlan, handoff: Handoff) {
    let (decision, session_name) = match plan.decision() {
        EntryDecision::EnterView { view, .. } => ("existing", view.fingerprint.name.clone()),
        EntryDecision::OpenView { work, .. } => ("existing", work.anchor.name.clone()),
        EntryDecision::StartProject { session, .. } => ("new", session.name.clone()),
        EntryDecision::ScaffoldAndStart { session, .. } => ("create", session.name.clone()),
    };
    tracing::info!(event = "resolution", decision, session_name = %session_name);
    tracing::info!(event = "plan", operation = operation_label(plan.decision(), plan.target().client()), client = ?plan.target().client());
    tracing::info!(
        event = if handoff == Handoff::Exec { "handoff" } else { "emission" },
        operation = operation_label(plan.decision(), plan.target().client()),
        handoff = ?handoff
    );
}
