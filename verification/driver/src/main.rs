#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use tta_verify::{Backend, Config, Mode, Scenario, exercise};

#[derive(Parser)]
#[command(version, about = "Exercise and instrument real tta attach/detach on exclusively owned private tmux servers")]
struct Args {
    /// Executable under assessment [default: tta beside this helper]
    #[arg(long)]
    tta: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "native")]
    backend: Backend,
    /// By default exercise direct and evaluated-shell execution
    #[arg(long, value_enum)]
    mode: Option<Mode>,
    /// By default exercise both cold creation and existing-session attachment
    #[arg(long, value_enum)]
    scenario: Option<Scenario>,
    /// Per-phase/query timeout in seconds
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,
    /// Also save the complete JSON report (including traces and PTY transcript)
    #[arg(long)]
    report: Option<PathBuf>,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    let tta = args.tta.unwrap_or(std::env::current_exe()?.with_file_name("tta"));
    let config = Config::builder()
        .tta(tta)
        .backend(args.backend)
        .timeout(Duration::from_secs(args.timeout))
        .build();
    let modes = args.mode.map_or_else(|| vec![Mode::Direct, Mode::Shell], |mode| vec![mode]);
    let scenarios = args.scenario.map_or_else(|| vec![Scenario::New, Scenario::Existing], |scenario| vec![scenario]);
    let mut reports = Vec::new();
    for mode in modes {
        for scenario in &scenarios {
            reports.push(exercise(&config, mode, *scenario));
        }
    }
    let success = reports.iter().all(|report| report.success);
    let text = serde_json::to_string_pretty(&serde_json::json!({ "success": success, "runs": reports }))?;
    if let Some(path) = args.report {
        std::fs::write(path, &text)?;
    }
    println!("{text}");
    Ok(if success { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}
