//! Runner-free supplied-facts exercises of the retained public actor. Each run
//! creates a fresh attempt; no transcript is a resume token or effect receipt.
use anyhow::{Result, bail, ensure};
use serde::Serialize;
use tta_command::{Expression, Handoff};
use tta_launcher::{
    Session,
    entry::{EntryAttempt, action::EntryDecision, protocol::*},
    scaffold, socket,
};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Scenario {
    Existing,
    Project,
    Create,
    Grouped,
}
const NAME: &str = "work' $; 雪";
const ROOT: &str = "/target/physical/projects";

fn invocation(scenario: Scenario) -> Result<Invocation> {
    Ok(Invocation {
        context: SuppliedContext {
            host: "supplied-target".into(),
            cwd: "/target/invocation".into(),
            socket: socket::Resolved::path("/target/private/tmux.sock"),
            program: "/target/bin/tmux".into(),
            config: None,
            client: ClientEntry::AttachTerminal,
        },
        intent: EntryIntent {
            roots: vec!["/target/projects".into()],
            view: if matches!(scenario, Scenario::Grouped) {
                ViewPreference::Independent
            } else {
                ViewPreference::Shared
            },
            ..EntryIntent::selector(NAME)
        },
        registry: scaffold::Config::parse(
            Some(std::path::Path::new("/target/config.toml")),
            Some(
                "[scaffold.generators.native]\nadapter='rekon-native'\n[scaffold.generators.native.options]\ninstall=false\n[scaffold.generators.native.options.remotes]\norigin=''\ntngl=''\nvoodoo=''\n",
            ),
        )?,
        generator: scaffold::Choice::Default,
    })
}
fn session() -> Session {
    Session {
        id: "$7".parse().unwrap(),
        name: NAME.into(),
        group: None,
    }
}

fn observation(request: &ReadRequest, scenario: Scenario) -> Result<Observation> {
    Ok(match &request.kind {
        ReadKind::Sessions { purpose, endpoint, program } => {
            ensure!(
                endpoint.socket == std::path::Path::new("/target/private/tmux.sock") && program == "/target/bin/tmux",
                "wrong target session read"
            );
            if matches!(scenario, Scenario::Existing | Scenario::Grouped) {
                Observation::Sessions(SessionInventory::Present(vec![session()]))
            } else {
                ensure!(
                    matches!(purpose, SessionReadPurpose::Selection | SessionReadPurpose::BindAction),
                    "wrong read purpose"
                );
                Observation::Sessions(SessionInventory::Absent)
            }
        }
        ReadKind::Projects { slot, root, filter } => {
            ensure!(
                slot.0 == 1 && root == std::path::Path::new("/target/projects") && filter == NAME,
                "wrong project read"
            );
            Observation::Projects(tta_launcher::ProjectInventory {
                requested_root: root.clone(),
                root: ROOT.into(),
                entries: if matches!(scenario, Scenario::Project) {
                    vec![tta_launcher::ProjectEntry {
                        name: NAME.into(),
                        directory: std::path::Path::new(ROOT).join(NAME),
                    }]
                } else {
                    vec![]
                },
            })
        }
        ReadKind::Destination { path, .. } => {
            ensure!(path == &std::path::Path::new(ROOT).join(NAME), "wrong destination");
            Observation::Destination(DestinationEvidence::Absent)
        }
        ReadKind::GeneratorDefaults { cwd, needs } => {
            ensure!(
                cwd == std::path::Path::new("/target/invocation") && needs.author && needs.license,
                "wrong defaults"
            );
            Observation::Defaults(GeneratorDefaults {
                author: Some("Target author ' 雪".into()),
                license: Some("Apache-2.0".into()),
            })
        }
        ReadKind::ProjectIdentity { route } => {
            ensure!(route.requested_root == std::path::Path::new("/target/projects"), "wrong selected route");
            Observation::ProjectIdentity(std::path::Path::new(ROOT).join(NAME))
        }
        ReadKind::CreationRootIdentity { root, expected } => {
            ensure!(
                root == std::path::Path::new("/target/projects") && expected == std::path::Path::new(ROOT),
                "wrong root binding"
            );
            Observation::ProjectIdentity(ROOT.into())
        }
        other => bail!("unexpected supplied read: {other:?}"),
    })
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub kind: &'static str,
    pub scenario: String,
    pub execution_attempted: bool,
    pub observations: Vec<RecordedExchange>,
    pub selection: String,
    pub shell: String,
    pub checks: Vec<&'static str>,
    pub emitted_probes: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct RecordedExchange {
    pub request: String,
    pub reply: String,
}

fn drive(attempt: &mut EntryAttempt, scenario: Scenario, records: &mut Vec<RecordedExchange>, probes: &mut Vec<String>) -> Result<()> {
    while let Waiting::Read(request) = attempt.waiting() {
        let request = request.clone();
        let observation = observation(&request, scenario)?;
        // Construction only: these scripts are NOT run by this no-I/O witness.
        probes.push(tta_launcher::entry::probe::Probe::new(request.clone(), format!("witness-token-{}", request.id.sequence))?.script()?);
        records.push(RecordedExchange {
            request: format!("{request:?}"),
            reply: format!("{observation:?}"),
        });
        attempt.advance(Event::Observed(ReadCompletion {
            request,
            result: Ok(observation),
        }))?;
    }
    Ok(())
}

pub fn exercise(scenario: Scenario) -> Result<Report> {
    let mut attempt = EntryAttempt::start(invocation(scenario)?)?;
    let mut observations = Vec::new();
    let mut emitted_probes = Vec::new();
    drive(&mut attempt, scenario, &mut observations, &mut emitted_probes)?;
    let Waiting::Planned(plan) = attempt.waiting() else {
        bail!("not planned: {:?}", attempt.snapshot());
    };
    let decision = plan.decision().clone();
    ensure!(
        matches!(decision, EntryDecision::ScaffoldAndStart { .. }) == matches!(scenario, Scenario::Create),
        "wrong creation decision"
    );
    ensure!(
        matches!(decision, EntryDecision::EnterView { .. } | EntryDecision::OpenView { .. }) == matches!(scenario, Scenario::Existing | Scenario::Grouped),
        "wrong work domain"
    );
    attempt.advance(Event::Deliver(Delivery::Export))?;
    drive(&mut attempt, scenario, &mut observations, &mut emitted_probes)?;
    let Waiting::Finished(AttemptOutcome::Exported(action)) = attempt.waiting() else {
        bail!("not exported: {:?}", attempt.snapshot());
    };
    let steps = action.expression().steps();
    let commands: Vec<_> = steps
        .iter()
        .filter_map(|s| if let Expression::Command(c) = s { Some(c) } else { None })
        .collect();
    let tmux = commands.last().context("no tmux action")?;
    ensure!(tmux.program.as_os_str() == "/target/bin/tmux", "invoking-host program leaked");
    let args: Vec<_> = tmux.args.iter().map(|w| w.as_os_str().to_str().unwrap()).collect();
    let mut checks = vec![
        "correlated reads and native supplied facts",
        "semantic decision domain",
        "fresh operation-specific binding before observation-only export",
    ];
    match scenario {
        Scenario::Existing => {
            ensure!(args == ["-S", "/target/private/tmux.sock", "attach-session", "-t", "$7"], "canonical ID lost");
        }
        Scenario::Grouped => {
            ensure!(steps.len() == 1 && args.iter().filter(|v| **v == ";").count() == 2, "coherent queue split");
            let attach = args.iter().position(|a| *a == "attach-session").unwrap();
            let keep = args.iter().position(|a| *a == "set-option").unwrap();
            ensure!(attach < keep && args.last() == Some(&"keep-last"), "wrong lifecycle order");
            checks.push("one create/attach/keep-last queue");
        }
        Scenario::Project | Scenario::Create => {
            let expected = std::path::Path::new(ROOT).join(NAME);
            ensure!(
                steps
                    .iter()
                    .rev()
                    .find_map(|s| if let Expression::ChangeDir(cd) = s { Some(cd.path.as_os_str()) } else { None })
                    == Some(expected.as_os_str()),
                "physical root ignored"
            );
            checks.push("selected physical route retained into ChangeDir");
            if matches!(scenario, Scenario::Create) {
                let writes: Vec<_> = steps.iter().filter_map(|s| if let Expression::Write(w) = s { Some(w) } else { None }).collect();
                ensure!(writes.len() == 1, "generator materialized more than once");
                let package: serde_json::Value = serde_json::from_str(writes[0].contents.as_os_str().to_str().unwrap())?;
                ensure!(
                    package["author"] == "Target author ' 雪" && package["license"] == "Apache-2.0",
                    "target defaults lost"
                );
                checks.push("actual supplied defaults in one generated package");
            }
        }
    }
    ensure!(
        !attempt.snapshot().journal.iter().any(|r| matches!(r, AttemptRecord::EffectIssued(_))),
        "export attempted execution"
    );
    Ok(Report {
        kind: "supplied-facts retained entry attempt; no host I/O",
        scenario: format!("{scenario:?}"),
        execution_attempted: false,
        observations,
        selection: format!("{decision:?}"),
        shell: action.expression().shell(Handoff::Run)?,
        checks,
        emitted_probes,
    })
}
use anyhow::Context;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_scenarios_reach_export_without_a_runner() {
        for scenario in [Scenario::Existing, Scenario::Project, Scenario::Create, Scenario::Grouped] {
            exercise(scenario).unwrap();
        }
    }
    #[test]
    fn inspection_wrong_shape_stale_and_duplicate_replies_do_not_advance() {
        let mut attempt = EntryAttempt::start(invocation(Scenario::Existing).unwrap()).unwrap();
        let Waiting::Read(request) = attempt.waiting() else { panic!() };
        let request = request.clone();
        let before = attempt.snapshot();
        for _ in 0..3 {
            assert_eq!(attempt.snapshot().journal, before.journal);
        }
        assert!(
            attempt
                .advance(Event::Observed(ReadCompletion {
                    request: request.clone(),
                    result: Ok(Observation::Destination(DestinationEvidence::Absent))
                }))
                .is_err()
        );
        let mut wrong = request.clone();
        wrong.id.sequence += 1;
        assert!(
            attempt
                .advance(Event::Observed(ReadCompletion {
                    request: wrong,
                    result: Ok(Observation::Sessions(SessionInventory::Absent))
                }))
                .is_err()
        );
        assert_eq!(attempt.snapshot().journal, before.journal);
        attempt
            .advance(Event::Observed(ReadCompletion {
                request: request.clone(),
                result: Ok(observation(&request, Scenario::Existing).unwrap()),
            }))
            .unwrap();
        assert!(matches!(attempt.waiting(), Waiting::Planned(_)));
        assert!(
            attempt
                .advance(Event::Observed(ReadCompletion {
                    request,
                    result: Ok(Observation::Sessions(SessionInventory::Absent))
                }))
                .is_err()
        );
        // EnterView, not just grouped entry, must stop on changed fresh anchor.
        attempt.advance(Event::Deliver(Delivery::Export)).unwrap();
        let Waiting::Read(request) = attempt.waiting() else { panic!() };
        let request = request.clone();
        attempt
            .advance(Event::Observed(ReadCompletion {
                request,
                result: Ok(Observation::Sessions(SessionInventory::Absent)),
            }))
            .unwrap();
        assert!(matches!(attempt.waiting(), Waiting::Finished(AttemptOutcome::Stopped { .. })));
    }
}
