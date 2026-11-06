//! Isolated global text-diagnostic test; never install this subscriber in the
//! scoped-mock binary. Producers have returned before taking the log-buffer lock.
use tta_launcher::{ResolveError, Session};
use tta_runner::{
    backend::{LookupError, SessionLookup},
    resolve::resolve,
};

struct Ambiguous;
impl SessionLookup for Ambiguous {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok([("$1", "a-project"), ("$2", "z-project")]
            .into_iter()
            .map(|(id, name)| Session {
                id: id.parse().unwrap(),
                name: name.to_owned(),
                group: None,
            })
            .collect())
    }
}

#[test]
#[tracing_test::traced_test]
fn ambiguity_has_one_stable_same_line_diagnostic() {
    let result = resolve().lookup(&Ambiguous).argument("project").call();
    assert!(matches!(result, Err(ResolveError::Sessions { .. })));
    // No logging, producer waits or other tracing inside logs_assert's lock.
    logs_assert(|lines| {
        let count = lines
            .iter()
            .filter(|line| line.contains("event=\"match.ambiguous\"") && line.contains("domain=\"session\"") && line.contains("count=2"))
            .count();
        if count == 1 {
            Ok(())
        } else {
            Err(format!("expected one session ambiguity diagnostic, got {count}: {lines:#?}"))
        }
    });
}
