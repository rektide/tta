#![forbid(unsafe_code)]

#[cfg(not(unix))]
compile_error!("tta requires a Unix terminal and tmux");

#[cfg(not(any(feature = "native", feature = "cli")))]
compile_error!("enable at least one lookup backend: native or cli");

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tta_command::Handoff;
use tta_launcher::{backend, launch, resolve, socket};

mod environment;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum Backend {
    #[cfg(feature = "native")]
    #[default]
    Native,
    #[cfg(feature = "cli")]
    #[cfg_attr(not(feature = "native"), default)]
    Cli,
}

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    after_help = "Matching is case-sensitive and literal: exact name, otherwise a unique substring.\nExisting sessions always take priority. Quote shell metacharacters.\nExample: tta_plan=$(tta --print-shell project) && eval \"$tta_plan\""
)]
struct Args {
    /// Session or immediate project directory name fragment (not a glob)
    argument: String,
    /// Project root [env: TTA_ROOT, then home-relative PROJECTS_DIR; default: $HOME/src]
    #[arg(long)]
    root: Option<PathBuf>,
    /// Override the name of a new session (does not change session matching)
    #[arg(long)]
    session_name: Option<String>,
    /// Emit POSIX shell commands for eval; do not create or attach
    #[arg(long)]
    print_shell: bool,
    /// Include exec in shell output, explicitly replacing the evaluating shell
    #[arg(long, requires = "print_shell")]
    exec_shell: bool,
    /// Lookup backend [env: TTA_BACKEND; default: native if compiled, otherwise cli]
    #[arg(long, value_enum)]
    backend: Option<Backend>,
    /// Select a tmux socket label, as with tmux -L [env: TTA_SOCKET_NAME]
    #[arg(long, conflicts_with = "socket_path")]
    socket_name: Option<OsString>,
    /// Select a tmux socket path, as with tmux -S [env: TTA_SOCKET_PATH]
    #[arg(long, conflicts_with = "socket_name")]
    socket_path: Option<PathBuf>,
    /// Final client executable, also used by CLI lookup [env: TTA_TMUX; default: tmux]
    #[arg(long)]
    tmux: Option<OsString>,
    /// tmux startup configuration (-f) [env: TTA_TMUX_CONFIG]
    #[arg(long)]
    tmux_config: Option<PathBuf>,
    /// Structured events on stderr [env: TTA_TRACE; --trace=false disables]
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    trace: Option<bool>,
}

fn main() -> Result<ExitCode> {
    let args = environment::configure(Args::parse())?;
    if args.trace.unwrap_or(false) {
        tracing_subscriber::fmt()
            .json()
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .without_time()
            .init();
    }
    let result = run(args);
    if let Err(error) = &result {
        tracing::error!(event = "failure", error = %error, cause = ?error);
    }
    result
}

fn run(mut args: Args) -> Result<ExitCode> {
    let context = socket::Context::from_environment()?;
    let backend = args.backend.unwrap_or_default();
    let mut tmux = args.tmux.unwrap_or_else(|| OsString::from("tmux"));
    // Explicit relative executable/config paths belong to the invocation cwd,
    // not the project cwd used later by the launch plan. Bare names keep PATH.
    if tmux.as_encoded_bytes().contains(&b'/') {
        tmux = context.cwd.join(&tmux).into_os_string();
    }
    args.tmux_config = args.tmux_config.map(|path| context.cwd.join(path));
    let selection = match (args.socket_name, args.socket_path) {
        (Some(name), _) => socket::Selection::Name(name),
        (_, Some(path)) => socket::Selection::Path(path),
        _ => socket::Selection::Default,
    };
    let socket = context.resolve(&selection)?;
    let mode = context.client_mode(&socket.path)?;
    tracing::info!(event = "lookup.start", backend = ?backend, socket = ?socket.path, argument = %args.argument);
    let lookup: Box<dyn backend::SessionLookup> = match backend {
        #[cfg(feature = "native")]
        Backend::Native => Box::new(backend::Native {
            endpoint: tmax::endpoint::Tmux::new().socket_path(&socket.path),
        }),
        #[cfg(feature = "cli")]
        Backend::Cli => Box::new(backend::Cli {
            program: tmux.clone(),
            socket: socket.path.clone(),
        }),
    };
    // The configured root is inspected only after session matching fails.
    let resolved = resolve()
        .lookup(lookup.as_ref())
        .argument(&args.argument)
        .maybe_root(args.root.as_deref())
        .maybe_session_name(args.session_name.as_deref())
        .call()?;
    match &resolved {
        tta_launcher::Selection::Existing(session) => {
            tracing::info!(event = "resolution", decision = "existing", session_id = %session.id, session_name = %session.name)
        }
        tta_launcher::Selection::Project { directory, session_name } => {
            tracing::info!(event = "resolution", decision = "new", directory = ?directory, session_name = %session_name)
        }
    }
    let expression = launch()
        .selection(&resolved)
        .socket(&socket)
        .client_mode(mode)
        .program(&tmux)
        .maybe_config(args.tmux_config.as_deref())
        .call()?;
    let operation = match (&resolved, mode) {
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Attach) => "attach-session",
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Switch) => "switch-client",
        (tta_launcher::Selection::Project { .. }, tta_launcher::ClientMode::Attach) => "new-session+attach",
        (tta_launcher::Selection::Project { .. }, tta_launcher::ClientMode::Switch) => "new-session-detached+switch-client",
    };
    tracing::info!(event = "plan", operation, client_mode = ?mode);
    if args.print_shell {
        let handoff = if args.exec_shell { Handoff::Exec } else { Handoff::Run };
        tracing::info!(event = "emission", handoff = ?handoff);
        println!("{}", expression.shell(handoff)?);
        return Ok(ExitCode::SUCCESS);
    }
    // Reuse the caller's terminal. Successful exec never returns to Rust.
    tracing::info!(event = "handoff", program = ?tmux, operation);
    let status = expression.execute(Handoff::Exec).context("tmux client handoff failed")?;
    Ok(match status {
        Some(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        None => ExitCode::SUCCESS,
    })
}
