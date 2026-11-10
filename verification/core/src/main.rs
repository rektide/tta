use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tta_core_witness::{Scenario, exercise};

#[derive(Parser)]
#[command(about = "Drive fresh entry attempts with supplied facts, without a runner or tmux")]
struct Args {
    #[arg(long, value_enum)]
    scenario: Option<Scenario>,
    #[arg(long)]
    report: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let scenarios = args.scenario.map_or_else(
        || vec![Scenario::Existing, Scenario::Project, Scenario::Create, Scenario::Grouped],
        |scenario| vec![scenario],
    );
    let reports = scenarios.into_iter().map(exercise).collect::<Result<Vec<_>>>()?;
    let json = serde_json::to_string_pretty(&reports)?;
    if let Some(path) = args.report {
        std::fs::write(path, &json)?;
    }
    println!("{json}");
    Ok(())
}
