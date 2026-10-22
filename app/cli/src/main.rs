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
use tta_launcher::{backend, launch, scaffold, socket};

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
    #[arg(required_unless_present = "completion_candidates")]
    argument: Option<String>,
    /// Read-only shell completion inventory (one name per line)
    #[arg(long, hide = true, value_enum, conflicts_with_all = ["argument", "print_shell", "exec_shell", "session_name", "scaffold"])]
    completion_candidates: Option<CompletionCandidates>,
    /// Project root [env: TTA_ROOT, then home-relative PROJECTS_DIR; default: $HOME/src]
    #[arg(long)]
    root: Option<PathBuf>,
    /// Override the name of a new session (does not change session matching)
    #[arg(long)]
    session_name: Option<String>,
    /// Scaffold a missing project before creating its session
    /// [env: TTA_SCAFFOLD; --scaffold=NAME picks a configured generator, a bare --scaffold uses the configured default, --scaffold=false disables]
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "default")]
    scaffold: Option<String>,
    /// Create a grouped satellite when an existing session matches
    #[arg(long)]
    group: bool,
    /// Initial directory for new projects or grouped satellites
    #[arg(long)]
    dir: Option<PathBuf>,
    /// Startup executable and arguments for a new project session only
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<OsString>,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CompletionCandidates {
    Backends,
    Projects,
    Sessions,
    Scaffolds,
}

fn print_candidates(mut names: Vec<String>) {
    names.sort();
    names.dedup();
    for name in names {
        // The line-oriented shell protocol cannot represent control characters.
        if !name.chars().any(char::is_control) {
            println!("{name}");
        }
    }
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
    if matches!(args.completion_candidates, Some(CompletionCandidates::Backends)) {
        print_candidates(
            Backend::value_variants()
                .iter()
                .map(|backend| backend.to_possible_value().unwrap().get_name().to_owned())
                .collect(),
        );
        return Ok(ExitCode::SUCCESS);
    }
    if matches!(args.completion_candidates, Some(CompletionCandidates::Projects)) {
        let root = args.root.as_deref().ok_or(tta_launcher::ResolveError::MissingRoot)?;
        print_candidates(tta_launcher::project_names(root)?);
        return Ok(ExitCode::SUCCESS);
    }
    let config = scaffold::Config::load()?;
    if matches!(args.completion_candidates, Some(CompletionCandidates::Scaffolds)) {
        print_candidates(config.names());
        return Ok(ExitCode::SUCCESS);
    }
    let context = socket::Context::from_environment()?;
    let backend = args.backend.unwrap_or_default();
    let mut tmux = args.tmux.unwrap_or_else(|| OsString::from("tmux"));
    // Explicit relative executable/config paths belong to the invocation cwd,
    // not the project cwd used later by the launch plan. Bare names keep PATH.
    if tmux.as_encoded_bytes().contains(&b'/') {
        tmux = context.cwd.join(&tmux).into_os_string();
    }
    args.tmux_config = args.tmux_config.map(|path| context.cwd.join(path));
    args.dir = args.dir.map(|path| context.cwd.join(path));
    let selection = match (args.socket_name, args.socket_path) {
        (Some(name), _) => socket::Selection::Name(name),
        (_, Some(path)) => socket::Selection::Path(path),
        _ => socket::Selection::Default,
    };
    let socket = context.resolve(&selection)?;
    // Inventory is allowed for another server: it never switches this client.
    let mode = if args.completion_candidates.is_none() {
        Some(context.client_mode(&socket.path)?)
    } else {
        None
    };
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
    if matches!(args.completion_candidates, Some(CompletionCandidates::Sessions)) {
        print_candidates(lookup.sessions()?.into_iter().map(|session| session.name).collect());
        return Ok(ExitCode::SUCCESS);
    }
    let argument = args.argument.as_deref().expect("clap requires a fragment outside completion mode");
    let mode = mode.expect("launch mode was checked before lookup");
    tracing::info!(event = "lookup.start", backend = ?backend, socket = ?socket.path, argument);
    // Scaffold selection: CLI > environment > compiled default posture.
    let choice = args
        .scaffold
        .as_deref()
        .unwrap_or(if cfg!(feature = "scaffold-by-default") { "default" } else { "false" });
    // The configured root is inspected only after session matching fails.
    let scaffold::Resolved {
        selection: resolved,
        prefix,
        generator,
    } = scaffold::resolve()
        .lookup(lookup.as_ref())
        .argument(argument)
        .maybe_root(args.root.as_deref())
        .maybe_session_name(args.session_name.as_deref())
        .choice(choice)
        .config(&config)
        .call()?;
    match &resolved {
        tta_launcher::Selection::Existing(session) => {
            tracing::info!(event = "resolution", decision = "existing", session_id = %session.id, session_name = %session.name)
        }
        tta_launcher::Selection::Project { directory, session_name } => {
            tracing::info!(event = "resolution", decision = "new", directory = ?directory, session_name = %session_name)
        }
        tta_launcher::Selection::Create { directory, session_name } => {
            let entry = generator.as_ref().expect("create selections require a chosen generator");
            tracing::info!(event = "resolution", decision = "create", directory = ?directory, session_name = session_name, generator = %entry.name, adapter = %entry.adapter);
        }
    }
    let expression = launch()
        .selection(&resolved)
        .socket(&socket)
        .client_mode(mode)
        .program(&tmux)
        .maybe_config(args.tmux_config.as_deref())
        .maybe_prefix(prefix.as_ref())
        .grouped(args.group)
        .startup(&args.command)
        .maybe_directory(args.dir.as_deref())
        .call()?;
    let operation = match (&resolved, mode) {
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Attach) if args.group => "new-grouped-session+attach",
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Switch) if args.group => "new-grouped-session-detached+switch-client",
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Attach) => "attach-session",
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Switch) => "switch-client",
        (tta_launcher::Selection::Project { .. }, tta_launcher::ClientMode::Attach) => "new-session+attach",
        (tta_launcher::Selection::Project { .. }, tta_launcher::ClientMode::Switch) => "new-session-detached+switch-client",
        (tta_launcher::Selection::Create { .. }, tta_launcher::ClientMode::Attach) => "scaffold+new-session+attach",
        (tta_launcher::Selection::Create { .. }, tta_launcher::ClientMode::Switch) => "scaffold+new-session-detached+switch-client",
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
