use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

#[derive(Parser)]
#[command(about = "Exercise actual SSH against an exclusively owned loopback sshd, without target tta")]
struct Args {
    #[arg(long)]
    tta: PathBuf,
    #[arg(long)]
    term_preserver: Option<PathBuf>,
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,
    #[arg(long)]
    report: Option<PathBuf>,
}

fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();
    let report = tta_verify::ssh::exercise(&tta_verify::ssh::Config {
        tta: args.tta,
        consumer: args.term_preserver,
        timeout: Duration::from_secs(args.timeout),
    });
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(path) = args.report {
        std::fs::write(path, &text)?;
    }
    println!("{text}");
    eprintln!(
        "actual SSH verification: {}; {} checks; {} probes",
        if report.success { "passed" } else { "FAILED" },
        report.checks.len(),
        report.observations.len()
    );
    if let Some(error) = &report.failure {
        eprintln!("{error}");
    }
    Ok(if report.success { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}
