//! Informational pure-planning cost probe, not an I/O or timing-assertion test.
use std::hint::black_box;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use clap::Parser;
use tta_core_witness::{Scenario, fixture, replay};
use tta_launcher::Session;
use tta_launcher::workflow::{ObservationReply, Progress, SessionInventory, Workflow};

#[derive(Parser)]
#[command(about = "Measure immutable ready pulls and complete supplied-facts replay; never observe or execute a target")]
struct Options {
    #[arg(long, default_value = "1000")]
    iterations: NonZeroUsize,
    #[arg(long, default_value = "5")]
    samples: NonZeroUsize,
    /// Add nonmatching sessions to each supplied session inventory.
    #[arg(long, default_value_t = 0)]
    extra_sessions: u32,
    /// Add nonmatching directories to each supplied project inventory.
    #[arg(long, default_value_t = 0)]
    extra_projects: u32,
}

fn median_cost(options: &Options, mut operation: impl FnMut()) -> Duration {
    // Warm the shared machine and allocation paths outside measurement. Each
    // measured operation still starts its own actor and drops the returned plan.
    for _ in 0..100 {
        operation();
    }
    let mut samples: Vec<_> = (0..options.samples.get())
        .map(|_| {
            let start = Instant::now();
            for _ in 0..options.iterations.get() {
                operation();
            }
            start.elapsed().div_f64(options.iterations.get() as f64)
        })
        .collect();
    samples.sort();
    samples[samples.len() / 2]
}

fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    let extra_sessions: Vec<_> = (0..options.extra_sessions)
        .map(|index| {
            Ok(Session {
                id: format!("${}", 1000_u32.checked_add(index).expect("too many fixture sessions")).parse()?,
                name: format!("unrelated-session-{index}"),
                group: None,
            })
        })
        .collect::<anyhow::Result<_>>()?;
    println!("scenario,extra_sessions,extra_projects,iterations,samples,ready_next_median_ns,transcript_median_ns");
    for scenario in [Scenario::Existing, Scenario::Project, Scenario::Create, Scenario::Grouped] {
        let (input, mut exchanges) = fixture(scenario)?;
        for exchange in &mut exchanges {
            match &mut exchange.reply {
                ObservationReply::Sessions(inventory) if !extra_sessions.is_empty() => {
                    let mut sessions = inventory.sessions().to_vec();
                    sessions.extend_from_slice(&extra_sessions);
                    *inventory = SessionInventory::Present(sessions);
                }
                ObservationReply::Projects(inventory) => {
                    inventory
                        .names
                        .extend((0..options.extra_projects).map(|index| format!("unrelated-project-{index}")));
                }
                _ => {}
            }
        }
        let mut ready = Workflow::new(input.clone())?;
        for exchange in &exchanges {
            ready.supply(exchange.reply.clone())?;
        }
        assert!(matches!(ready.next()?, Progress::Ready(_)));
        let next = median_cost(&options, || {
            black_box(black_box(&ready).next().unwrap());
        });
        let transcript = median_cost(&options, || {
            // Includes Workflow construction, input/reply cloning, every caller
            // pull and supply's own pending-request validation, and plan drop.
            black_box(replay(black_box(&input).clone(), black_box(&exchanges)).unwrap());
        });
        println!(
            "{scenario:?},{},{},{},{},{},{}",
            options.extra_sessions,
            options.extra_projects,
            options.iterations,
            options.samples,
            next.as_nanos(),
            transcript.as_nanos(),
        );
    }
    Ok(())
}
