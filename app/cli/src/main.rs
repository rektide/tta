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
    /// Project root [default: $HOME/src]
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
    /// Session lookup implementation (native never invokes a client process)
    #[arg(long, value_enum, default_value_t)]
    backend: Backend,
    /// Select a tmux socket label, as with tmux -L
    #[arg(long, conflicts_with = "socket_path")]
    socket_name: Option<OsString>,
    /// Select a tmux socket path, as with tmux -S
    #[arg(long, conflicts_with = "socket_name")]
    socket_path: Option<PathBuf>,
    /// Executable for the final tmux client (and CLI lookup, if selected)
    #[arg(long, default_value = "tmux")]
    tmux: OsString,
    /// tmux startup configuration (-f); mainly useful with /dev/null in tests
    #[arg(long)]
    tmux_config: Option<PathBuf>,
    /// Write structured runtime events to stderr (never shell-output stdout)
    #[arg(long)]
    trace: bool,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    if args.trace {
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
    // Explicit relative executable/config paths belong to the invocation cwd,
    // not the project cwd used later by the launch plan. Bare names keep PATH.
    if args.tmux.as_encoded_bytes().contains(&b'/') {
        args.tmux = context.cwd.join(&args.tmux).into_os_string();
    }
    args.tmux_config = args.tmux_config.map(|path| context.cwd.join(path));
    let selection = match (args.socket_name, args.socket_path) {
        (Some(name), _) => socket::Selection::Name(name),
        (_, Some(path)) => socket::Selection::Path(path),
        _ => socket::Selection::Default,
    };
    let socket = context.resolve(&selection)?;
    let mode = context.client_mode(&socket.path)?;
    tracing::info!(event = "lookup.start", backend = ?args.backend, socket = ?socket.path, argument = %args.argument);
    let lookup: Box<dyn backend::SessionLookup> = match args.backend {
        #[cfg(feature = "native")]
        Backend::Native => Box::new(backend::Native {
            endpoint: tmax::endpoint::Tmux::new().socket_path(&socket.path),
        }),
        #[cfg(feature = "cli")]
        Backend::Cli => Box::new(backend::Cli {
            program: args.tmux.clone(),
            socket: socket.path.clone(),
        }),
    };
    // No HOME is necessary when an existing session matches or --root is given.
    // Resolve checks sessions before trying this root.
    let root = args.root.or_else(|| {
        std::env::var_os("HOME")
            .filter(|home| !home.is_empty())
            .map(|home| PathBuf::from(home).join("src"))
    });
    let resolved = resolve()
        .lookup(lookup.as_ref())
        .argument(&args.argument)
        .maybe_root(root.as_deref())
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
        .program(&args.tmux)
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
    tracing::info!(event = "handoff", program = ?args.tmux, operation);
    let status = expression.execute(Handoff::Exec).context("tmux client handoff failed")?;
    Ok(match status {
        Some(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        None => ExitCode::SUCCESS,
    })
}
