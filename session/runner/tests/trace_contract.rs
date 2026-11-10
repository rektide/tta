//! Scoped production trace contract derives labels from semantic decisions.
mod common;
use tracing::Level;
use tracing_mock::{expect, subscriber};
use tta_command::Handoff;
use tta_launcher::entry::protocol::*;

#[test]
fn exported_entry_emits_resolution_plan_and_emission_once_in_order() {
    let tmp = common::scratch_dir("tta-trace-");
    let mut i = common::entry::input(tmp.path(), "project");
    i.intent.view = ViewPreference::Shared;
    let mut attempt = common::entry::prepare(i, common::entry::sessions(&["project"]));
    let mut inventory = common::entry::Inventory(common::entry::sessions(&["project"]));
    let target = attempt.snapshot().target.unwrap();
    let mut observer = tta_runner::observe::local::Local::supplied(&target, &mut inventory).unwrap();
    let field = |value| expect::field("event").with_value(value);
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event().at_level(Level::INFO).with_fields(
                field(&"resolution")
                    .and(expect::field("decision").with_value(&"existing"))
                    .and(expect::field("session_name").with_value(&tracing::field::display("project")))
                    .only(),
            ),
        )
        .event(
            expect::event().at_level(Level::INFO).with_fields(
                field(&"plan")
                    .and(expect::field("operation").with_value(&"attach-session"))
                    .and(expect::field("client"))
                    .only(),
            ),
        )
        .event(
            expect::event().at_level(Level::INFO).with_fields(
                field(&"emission")
                    .and(expect::field("operation").with_value(&"attach-session"))
                    .and(expect::field("handoff"))
                    .only(),
            ),
        )
        .only()
        .run_with_handle();
    tracing::subscriber::with_default(subscriber, || {
        let snapshot = tta_runner::execute::drive(
            &mut attempt,
            &mut observer,
            &mut tta_runner::execute::local::Local::default(),
            Delivery::Export,
            Handoff::Run,
        )
        .unwrap();
        assert!(matches!(snapshot.outcome, Some(AttemptOutcome::Exported(_))));
    });
    handle.assert_finished();
}
