use anyhow::{Result, ensure};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Execute controlled runner-free probe round-trips (local processes, not SSH)")]
struct Args {
    #[arg(long)]
    report: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let report = tta_probe_witness::exercise();
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = args.report {
        std::fs::write(path, &json)?;
    }
    println!("{json}");
    ensure!(
        report.success,
        "probe round-trip failed: {}",
        report.failure.as_deref().unwrap_or("unknown failure")
    );
    Ok(())
}
