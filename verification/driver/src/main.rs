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
    /// Run tta within a real attached tmux pane and verify client switching
    #[arg(long)]
    inside: bool,
    /// Prove default independent views, shared opt-out and orphan resumption with two real terminals
    #[arg(long, conflicts_with_all = ["inside", "scenario"])]
    independent_views: bool,
    /// Prefix for the independent-view regression (omitted exercises the launcher default)
    #[arg(long, requires = "independent_views", default_value = "_")]
    satellite_prefix: String,
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
        .inside(args.inside)
        .timeout(Duration::from_secs(args.timeout))
        .build();
    let modes = args.mode.map_or_else(|| vec![Mode::Direct, Mode::Shell], |mode| vec![mode]);
    if args.independent_views {
        let reports: Vec<_> = modes
            .into_iter()
            .map(|mode| tta_verify::views::exercise(&config, mode, &args.satellite_prefix))
            .collect();
        let success = reports.iter().all(|report| report.success);
        for report in &reports {
            if let Some(failure) = &report.failure {
                eprintln!("{:?}: {failure}", report.mode);
            }
        }
        let text = serde_json::to_string_pretty(&serde_json::json!({"success": success, "runs": reports}))?;
        if let Some(path) = args.report {
            std::fs::write(path, &text)?;
        }
        println!("{text}");
        return Ok(if success { ExitCode::SUCCESS } else { ExitCode::FAILURE });
    }
    let scenarios = args.scenario.map_or_else(|| vec![Scenario::New, Scenario::Existing], |scenario| vec![scenario]);
    let mut reports = Vec::new();
    for mode in modes {
        for scenario in &scenarios {
            reports.push(exercise(&config, mode, *scenario));
        }
    }
    let success = reports.iter().all(|report| report.success);
    let passed = reports.iter().filter(|report| report.success).count();
    eprintln!(
        "tta runtime verification: {passed}/{} passed ({} backend, inside={})",
        reports.len(),
        args.backend.as_str(),
        args.inside
    );
    for report in &reports {
        if let Some(failure) = &report.failure {
            eprintln!("  {:?}/{:?}: {failure}", report.mode, report.scenario);
        }
    }
    let text = serde_json::to_string_pretty(&serde_json::json!({ "success": success, "runs": reports }))?;
    if let Some(path) = args.report {
        std::fs::write(path, &text)?;
    }
    println!("{text}");
    Ok(if success { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}
