//! Scoped structural mocks do not share tracing-test's global subscriber.
use tracing::Level;
use tracing_mock::{expect, subscriber};
use tta_launcher::{Selection, Session};
use tta_runner::{
    backend::{LookupError, SessionLookup},
    resolve::resolve,
};

struct Inventory;
impl SessionLookup for Inventory {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok(vec![Session {
            id: "$2".parse().unwrap(),
            name: "project".into(),
            group: None,
        }])
    }
}

#[test]
fn lookup_then_exact_match_events_remain_inside_the_resolve_span() {
    let span = expect::span().named("resolve");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone().with_fields(expect::field("argument").with_value(&"project").only()))
        .enter(span.clone())
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_ancestry(expect::has_contextual_parent("resolve"))
                .with_fields(
                    expect::field("event")
                        .with_value(&"lookup.complete")
                        .and(expect::field("count").with_value(&1usize))
                        .only(),
                ),
        )
        .event(
            expect::event()
                .at_level(Level::INFO)
                .with_ancestry(expect::has_contextual_parent("resolve"))
                .with_fields(
                    expect::field("event")
                        .with_value(&"match.existing")
                        .and(expect::field("session_id"))
                        .and(expect::field("session_name"))
                        .only(),
                ),
        )
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    tracing::subscriber::with_default(subscriber, || {
        assert!(matches!(
            resolve().lookup(&Inventory).argument("project").call().unwrap(),
            Selection::Existing(_)
        ));
    });
    handle.assert_finished();
}
