//! Ambiguity is structured evidence, not a historical resolver log string.
mod common;
use tta_launcher::entry::protocol::*;
#[test]
fn ambiguity_retains_candidates_without_a_plan_or_mutation_request() {
    let tmp = common::scratch_dir("tta-ambiguity-");
    let attempt = common::entry::prepare(
        common::entry::input(tmp.path(), "project"),
        common::entry::sessions(&["a-project", "z-project"]),
    );
    assert!(matches!(common::entry::cause(&attempt), EntryError::Ambiguous { candidates, .. } if candidates.len() == 2));
    let snapshot = attempt.snapshot();
    assert!(snapshot.plan.is_none());
    assert!(!snapshot.journal.iter().any(|r| matches!(r, AttemptRecord::EffectIssued(_))));
}
