//! Reusable supplied-facts transcript/replay exercises. This crate intentionally
//! has no runner, tmux process, native client or transport dependency.
use anyhow::{Result, bail, ensure};
use serde::Serialize;
use tta_command::{Expression, Handoff};
use tta_launcher::workflow::{Input, ObservationReply, ObservationRequest, PreparedLaunch, Progress, SessionInventory, Workflow};
use tta_launcher::{ClientMode, DestinationState, ProjectInventory, Selection, Session, scaffold, socket};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Scenario {
    Existing,
    Project,
    Create,
    Grouped,
}

#[derive(Clone, Debug)]
pub struct Exchange {
    pub request: ObservationRequest,
    pub reply: ObservationReply,
}

/// Replay checks request identity, reply type, exhaustion, and absence of extra
/// replies. It cannot contact a host and does not report execution success.
pub fn replay(input: Input, transcript: &[Exchange]) -> Result<PreparedLaunch> {
    let mut workflow = Workflow::new(input)?;
    for exchange in transcript {
        let Progress::Observe(request) = workflow.next()? else {
            bail!("transcript has extra replies after readiness");
        };
        ensure!(
            request == exchange.request,
            "transcript request mismatch: expected {request:?}, got {:?}",
            exchange.request
        );
        workflow.supply(exchange.reply.clone())?;
    }
    match workflow.next()? {
        Progress::Ready(ready) => Ok(*ready),
        Progress::Observe(request) => bail!("incomplete transcript; still needs {request:?}"),
    }
}

pub fn fixture(scenario: Scenario) -> Result<(Input, Vec<Exchange>)> {
    let name = "work' $; 雪";
    let socket_path = "/target/private/tmux.sock";
    let registry = scaffold::Config::parse(
        Some(std::path::Path::new("/target/config.toml")),
        Some(
            "[scaffold.generators.native]\nadapter = 'rekon-native'\n[scaffold.generators.native.options]\ninstall = false\n[scaffold.generators.native.options.remotes]\norigin = ''\ntngl = ''\nvoodoo = ''\n",
        ),
    )?;
    let input = Input::builder()
        .argument(name.into())
        .cwd("/target/invocation".into())
        .socket(socket::Resolved::path(socket_path))
        .client_mode(ClientMode::Attach)
        .program("/target/bin/tmux".into())
        .registry(registry)
        .scaffold("default".into())
        .root("/target/projects".into())
        .grouped(matches!(scenario, Scenario::Grouped))
        .build();
    let sessions = || ObservationRequest::Sessions {
        socket: socket_path.into(),
        program: "/target/bin/tmux".into(),
    };
    let session = Session {
        id: "$7".parse()?,
        name: name.into(),
    };
    let mut exchanges = vec![Exchange {
        request: sessions(),
        reply: ObservationReply::Sessions(match scenario {
            Scenario::Existing | Scenario::Grouped => SessionInventory::Present(vec![session.clone()]),
            Scenario::Project | Scenario::Create => SessionInventory::Absent,
        }),
    }];
    match scenario {
        Scenario::Existing => {}
        Scenario::Grouped => exchanges.push(Exchange {
            request: sessions(),
            reply: ObservationReply::Sessions(SessionInventory::Present(vec![session])),
        }),
        Scenario::Project | Scenario::Create => {
            exchanges.push(Exchange {
                request: ObservationRequest::Projects {
                    root: "/target/projects".into(),
                    argument: name.into(),
                },
                reply: ObservationReply::Projects(ProjectInventory {
                    root: "/target/physical/projects".into(),
                    names: if matches!(scenario, Scenario::Project) { vec![name.into()] } else { vec![] },
                }),
            });
            if matches!(scenario, Scenario::Create) {
                exchanges.push(Exchange {
                    request: ObservationRequest::Destination {
                        path: std::path::Path::new("/target/physical/projects").join(name),
                    },
                    reply: ObservationReply::Destination(DestinationState::Absent),
                });
                exchanges.push(Exchange {
                    request: ObservationRequest::GeneratorDefaults {
                        directory: "/target/invocation".into(),
                        author: true,
                        license: true,
                    },
                    reply: ObservationReply::GeneratorDefaults(scaffold::Defaults {
                        author: Some("Target author ' 雪".into()),
                        license: Some("Apache-2.0".into()),
                    }),
                });
            }
        }
    }
    Ok((input, exchanges))
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
}

#[derive(Debug, Serialize)]
pub struct RecordedExchange {
    pub request: String,
    pub reply: String,
}

pub fn exercise(scenario: Scenario) -> Result<Report> {
    let (input, transcript) = fixture(scenario)?;
    let prepared = replay(input.clone(), &transcript)?;
    let expected_existing = matches!(scenario, Scenario::Existing | Scenario::Grouped);
    ensure!(
        matches!(&prepared.selection, Selection::Existing(_)) == expected_existing,
        "wrong selection domain"
    );
    ensure!(
        matches!(&prepared.selection, Selection::Create { .. }) == matches!(scenario, Scenario::Create),
        "wrong creation decision"
    );
    let mut checks = vec![
        "exact observation sequence and reply types",
        "selection domain",
        "target-local paths from supplied facts",
    ];
    let commands: Vec<_> = prepared
        .plan
        .expression()
        .steps()
        .into_iter()
        .filter_map(|step| if let Expression::Command(command) = step { Some(command) } else { None })
        .collect();
    let tmux = commands.last().ok_or_else(|| anyhow::anyhow!("no tmux command"))?;
    ensure!(tmux.program.as_os_str() == "/target/bin/tmux", "local program leaked into target plan");
    let args: Vec<_> = tmux.args.iter().map(|word| word.as_os_str().to_str().unwrap()).collect();
    match scenario {
        Scenario::Grouped => {
            ensure!(commands.len() == 1, "grouped queue split across processes");
            ensure!(args[2] == "new-session" && args[3] == "-d", "satellite must be detached");
            let attach = args.iter().position(|arg| *arg == "attach-session").unwrap();
            let keep = args.iter().position(|arg| *arg == "set-option").unwrap();
            ensure!(attach < keep && args.last() == Some(&"keep-last"), "wrong satellite lifecycle ordering");
            ensure!(args.iter().filter(|arg| **arg == ";").count() == 2, "not one tmux queue");
            checks.push("one grouped create/attach/keep-last tmux queue");
        }
        Scenario::Existing => ensure!(
            args == ["-S", "/target/private/tmux.sock", "attach-session", "-t", "$7"],
            "existing selection did not retain canonical ID"
        ),
        Scenario::Create => {
            let writes: Vec<_> = prepared
                .plan
                .expression()
                .steps()
                .into_iter()
                .filter_map(|step| if let Expression::Write(write) = step { Some(write) } else { None })
                .collect();
            ensure!(writes.len() == 1, "expected one package.json write");
            let package: serde_json::Value = serde_json::from_str(writes[0].contents.as_os_str().to_str().unwrap())?;
            ensure!(
                package["author"] == "Target author ' 雪" && package["license"] == "Apache-2.0",
                "target defaults not reflected in literal package content"
            );
            checks.push("supplied target author/license in package.json");
        }
        Scenario::Project => ensure!(commands.len() == 1, "existing directory unexpectedly scaffolded"),
    }
    ensure!(replay(input, &transcript[..transcript.len() - 1]).is_err(), "incomplete transcript accepted");
    checks.push("truncated replay rejected");
    Ok(Report {
        kind: "supplied-facts runner-free replay",
        scenario: format!("{scenario:?}"),
        execution_attempted: false,
        observations: transcript
            .iter()
            .map(|exchange| RecordedExchange {
                request: format!("{:?}", exchange.request),
                reply: format!("{:?}", exchange.reply),
            })
            .collect(),
        selection: format!("{:?}", prepared.selection),
        shell: prepared.plan.expression().shell(Handoff::Run)?,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_scenarios_pass_semantic_checks_without_a_runner() {
        for scenario in [Scenario::Existing, Scenario::Project, Scenario::Create, Scenario::Grouped] {
            exercise(scenario).unwrap();
        }
    }
    #[test]
    fn mismatched_and_extra_replies_fail_closed() {
        let (input, mut transcript) = fixture(Scenario::Existing).unwrap();
        let mut workflow = Workflow::new(input.clone()).unwrap();
        assert!(workflow.supply(ObservationReply::Destination(DestinationState::Absent)).is_err());
        let request = workflow.next().unwrap();
        assert!(matches!(request, Progress::Observe(ObservationRequest::Sessions { .. })));
        transcript.push(transcript[0].clone());
        assert!(replay(input, &transcript).is_err());
    }
}
