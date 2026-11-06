#![forbid(unsafe_code)]

#[cfg(not(unix))]
compile_error!("tta requires a Unix terminal and tmux");

#[cfg(not(any(feature = "native", feature = "cli")))]
compile_error!("enable at least one backend: native or cli");

use std::ffi::OsString;
use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tta_command::Handoff;
use tta_launcher::workflow::PreparedLaunch;
use tta_runner::execution::{ExecuteLaunch, LaunchBackend};
use tta_runner::{backend, scaffold, socket};

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
    /// Observe and launch on an SSH target; the target needs no installed tta
    #[arg(long, conflicts_with = "completion_candidates")]
    ssh: Option<String>,
    /// Literal OpenSSH option/argument, repeated (use = for leading '-')
    #[arg(long, requires = "ssh", allow_hyphen_values = true)]
    ssh_option: Vec<OsString>,
    /// Local SSH executable [default: ssh]
    #[arg(long, requires = "ssh")]
    ssh_program: Option<OsString>,
    /// Per-probe local capture deadline in milliseconds [env: TTA_SSH_PROBE_TIMEOUT; default: 10000]
    #[arg(long, requires = "ssh", value_name = "MS")]
    ssh_probe_timeout: Option<NonZeroU64>,
    /// Retained bytes per channel stream [env: TTA_SSH_PROBE_RETENTION; default: 262144]
    #[arg(long, requires = "ssh", value_name = "BYTES")]
    ssh_probe_retention: Option<NonZeroUsize>,
    /// Selected target shell (default: target SHELL, then /bin/sh)
    #[arg(long, requires = "ssh")]
    target_shell: Option<String>,
    /// Run target shell login startup before every probe and handoff
    #[arg(long, requires = "ssh", num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    target_login: Option<bool>,
    /// Run target shell interactive startup too (probes still have no PTY)
    #[arg(long, requires = "ssh")]
    target_interactive: bool,
    /// Target invocation cwd after shell startup (never resolved locally)
    #[arg(long, requires = "ssh")]
    target_cwd: Option<PathBuf>,
    /// Literal target NAME=VALUE override after selected shell startup
    #[arg(long, requires = "ssh")]
    target_env: Vec<String>,
    /// Unset a target variable after selected shell startup
    #[arg(long, requires = "ssh")]
    target_unset: Vec<String>,
    /// Session or immediate project directory name fragment (not a glob)
    #[arg(required_unless_present = "completion_candidates")]
    argument: Option<String>,
    /// Read-only shell completion inventory (one name per line)
    #[arg(long, hide = true, value_enum, conflicts_with_all = ["argument", "print_shell", "exec_shell", "session_name", "window_name", "scaffold"])]
    completion_candidates: Option<CompletionCandidates>,
    /// Project root [env: TTA_ROOT, then home-relative PROJECTS_DIR; default: $HOME/src]
    #[arg(long)]
    root: Option<PathBuf>,
    /// Override the name of a new session (does not change session matching)
    #[arg(long)]
    session_name: Option<String>,
    /// Name a new session's initial window (naming disables automatic-rename for it)
    /// [default: the session name; --window-name=NAME renames it, --window-name=false leaves it unnamed]
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "default")]
    window_name: Option<String>,
    /// Scaffold a missing project before creating its session
    /// [env: TTA_SCAFFOLD; --scaffold=NAME picks a configured generator, a bare --scaffold uses the configured default, --scaffold=false disables]
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "default")]
    scaffold: Option<String>,
    /// Explicit scaffold registry path (on the target for --ssh)
    #[arg(long)]
    scaffold_config: Option<PathBuf>,
    /// Independent view (default outside tmux); --group=false shares window selection
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    group: Option<bool>,
    /// Satellite name prefix [env: TTA_SATELLITE_PREFIX; default: _]
    #[arg(long)]
    satellite_prefix: Option<String>,
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
    /// Lookup and eligible launch backend [env: TTA_BACKEND; default: native if compiled, otherwise cli]
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
    if args.ssh.is_some() {
        return run_ssh(args);
    }
    tta_launcher::validate_satellite_prefix(args.satellite_prefix.as_deref().unwrap_or("_"))?;
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
        print_candidates(tta_runner::resolve::project_names(root)?);
        return Ok(ExitCode::SUCCESS);
    }
    let config = match &args.scaffold_config {
        Some(path) => scaffold::load_config_from(Some(path))?,
        None => scaffold::load_config()?,
    };
    if matches!(args.completion_candidates, Some(CompletionCandidates::Scaffolds)) {
        print_candidates(config.names());
        return Ok(ExitCode::SUCCESS);
    }
    let context = socket::context()?;
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
        Some(socket::client_mode(&context, &socket.path)?)
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
        let prefix = args.satellite_prefix.as_deref().unwrap_or("_");
        print_candidates(tta_launcher::session_names(&lookup.sessions()?, prefix));
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
    let PreparedLaunch {
        selection: resolved,
        generator,
        plan,
    } = tta_runner::prepare()
        .lookup(lookup.as_ref())
        .argument(argument)
        .socket(&socket)
        .client_mode(mode)
        .program(&tmux)
        .maybe_root(args.root.as_deref())
        .maybe_session_name(args.session_name.as_deref())
        .choice(choice)
        .config(&config)
        .maybe_tmux_config(args.tmux_config.as_deref())
        .maybe_grouped(args.group)
        .satellite_prefix(args.satellite_prefix.as_deref().unwrap_or("_"))
        .startup(&args.command)
        .maybe_directory(args.dir.as_deref())
        .window_name(match args.window_name.as_deref() {
            Some(value) => tta_launcher::WindowName::parse(value)?,
            None => tta_launcher::WindowName::Session,
        })
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
    let operation = match (&resolved, mode) {
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Attach) if args.group.unwrap_or(true) => "new-grouped-session-detached+attach",
        (tta_launcher::Selection::Existing(_), tta_launcher::ClientMode::Switch) if args.group.unwrap_or(false) => "new-grouped-session-detached+switch-client",
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
        println!("{}", plan.expression().shell(handoff)?);
        return Ok(ExitCode::SUCCESS);
    }
    // Reuse the caller's terminal. Successful exec never returns to Rust.
    tracing::info!(event = "handoff", program = ?tmux, operation);
    let launch_backend = match backend {
        #[cfg(feature = "native")]
        Backend::Native => LaunchBackend::Native,
        #[cfg(feature = "cli")]
        Backend::Cli => LaunchBackend::Cli,
    };
    let status = plan.execute(launch_backend, Handoff::Exec).context("tmux launch failed")?;
    Ok(match status {
        Some(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        None => ExitCode::SUCCESS,
    })
}

fn run_ssh(args: Args) -> Result<ExitCode> {
    use tta_launcher::target;
    use tta_runner::command::Execute;
    #[cfg(feature = "native")]
    if matches!(args.backend, Some(Backend::Native)) {
        anyhow::bail!("--ssh uses target tmux CLI probes and handoff, not the local native backend");
    }
    let defaults = tta_runner::capture::Limits::default();
    let limits = tta_runner::capture::Limits {
        timeout: args
            .ssh_probe_timeout
            .map(|ms| std::time::Duration::from_millis(ms.get()))
            .unwrap_or(defaults.timeout),
        per_stream: args.ssh_probe_retention.map(NonZeroUsize::get).unwrap_or(defaults.per_stream),
        ..defaults
    };
    let mut environment = Vec::new();
    for value in &args.target_env {
        let (name, value) = value.split_once('=').context("--target-env requires NAME=VALUE")?;
        environment.push(target::Environment {
            name: name.into(),
            value: Some(value.into()),
        });
    }
    for name in &args.target_unset {
        environment.push(target::Environment {
            name: name.clone(),
            value: None,
        });
    }
    let setup = target::TargetSetup::builder()
        .maybe_shell(args.target_shell)
        .login(args.target_login.unwrap_or(true))
        .interactive(args.target_interactive)
        .environment(environment)
        .maybe_cwd(args.target_cwd)
        .build();
    let ssh = tta_runner::ssh::Ssh::builder()
        .destination(args.ssh.expect("SSH branch"))
        .maybe_program(args.ssh_program)
        .options(args.ssh_option)
        .build();
    let socket = match (args.socket_name, args.socket_path) {
        (Some(name), _) => Some(socket::Selection::Name(name)),
        (_, Some(path)) => Some(socket::Selection::Path(path)),
        _ => None,
    };
    let options = target::Options::builder()
        .argument(args.argument.expect("fragment required"))
        .maybe_root(args.root)
        .maybe_socket(socket)
        .maybe_program(args.tmux)
        .maybe_registry_path(args.scaffold_config)
        .maybe_scaffold(args.scaffold)
        .default_scaffold(if cfg!(feature = "scaffold-by-default") { "default" } else { "false" }.into())
        .maybe_config(args.tmux_config)
        .maybe_session_name(args.session_name)
        .maybe_grouped(args.group)
        .maybe_satellite_prefix(args.satellite_prefix)
        .startup(args.command)
        .maybe_directory(args.dir)
        .window_name(match args.window_name.as_deref() {
            Some(value) => tta_launcher::WindowName::parse(value)?,
            None => tta_launcher::WindowName::Session,
        })
        .build();
    let prepared = match tta_runner::ssh::prepare(ssh, setup, options, limits) {
        Ok(prepared) => prepared,
        Err(error) => {
            if let Err(write_error) = tta_runner::ssh::write_diagnostics(std::io::stderr().lock(), &error.observations) {
                // Reporting failure must not replace the original planning cause.
                eprintln!("cannot write SSH probe diagnostics: {write_error}");
            }
            return Err(error.into());
        }
    };
    tta_runner::ssh::write_diagnostics(std::io::stderr().lock(), &prepared.observations)?;
    if args.print_shell {
        println!("{}", prepared.shell(if args.exec_shell { Handoff::Exec } else { Handoff::Run })?);
        return Ok(ExitCode::SUCCESS);
    }
    let status = tta_command::Expression::from(prepared.command()?)
        .execute(Handoff::Exec)
        .context("SSH terminal handoff failed; do not replay a possibly executed target mutation")?;
    Ok(status
        .map(|status| ExitCode::from(status.code().unwrap_or(1) as u8))
        .unwrap_or(ExitCode::SUCCESS))
}
