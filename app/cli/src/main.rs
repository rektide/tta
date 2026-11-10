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
use tta_launcher::entry::{
    EntryAttempt,
    catalog::{self, Candidate, CatalogQuery, Input as CatalogInput},
    context,
    protocol::*,
};
use tta_runner::execute::local as execute_local;
use tta_runner::observe::{self, local as read_local};

mod environment;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
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
    after_help = "Matching is case-sensitive and literal: exact session name, then workspace alias, then shortest substring.\nTies across distinct workspaces are ambiguous. Sessions always take priority.\nOutside tmux, plain entry opens an independent view; --group=false shares window selection.\nExample: tta_plan=$(tta --print-shell project) && eval \"$tta_plan\""
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
    /// Ordered project roots, repeatable
    /// [env: TTA_ROOTS as a JSON array, then TTA_ROOT, then home-relative PROJECTS_DIR; default: $HOME/src]
    #[arg(long)]
    root: Vec<PathBuf>,
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
    SessionAliases,
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

/// Explicit CLI values as overrides: environment layering, whole-layer
/// selection and defaults belong to the shared context policy alone.
fn overrides(args: &Args) -> context::Overrides {
    let socket = match (&args.socket_name, &args.socket_path) {
        (Some(name), _) => Some(tta_launcher::socket::Selection::Name(name.clone())),
        (_, Some(path)) => Some(tta_launcher::socket::Selection::Path(path.clone())),
        _ => None,
    };
    context::Overrides {
        roots: (!args.root.is_empty()).then(|| args.root.clone()),
        socket,
        program: args.tmux.clone(),
        config: args.tmux_config.clone(),
        registry: args.scaffold_config.clone(),
        scaffold: args.scaffold.clone(),
        satellite_prefix: args.satellite_prefix.clone(),
    }
}

fn intent(args: &Args) -> EntryIntent {
    EntryIntent {
        selector: args.argument.as_deref().unwrap_or("catalog").to_owned(),
        view: match args.group {
            Some(true) => ViewPreference::Independent,
            Some(false) => ViewPreference::Shared,
            None => ViewPreference::Auto,
        },
        satellite_prefix: "_".into(),
        view_directory: args.dir.clone(),
        roots: Vec::new(),
        session_name: args.session_name.clone(),
        startup: args.command.clone(),
        directory: args.dir.clone(),
        window: match args.window_name.as_deref() {
            Some(value) => window(value),
            None => InitialWindow::AfterSession,
        },
    }
}

fn window(value: &str) -> InitialWindow {
    match value {
        "default" | "true" | "1" => InitialWindow::AfterSession,
        "false" | "0" | "off" | "none" => InitialWindow::Unnamed,
        name => InitialWindow::Named(name.to_owned()),
    }
}

fn default_scaffold() -> String {
    if cfg!(feature = "scaffold-by-default") { "default" } else { "false" }.into()
}

fn native_capable(backend: Backend) -> bool {
    #[cfg(feature = "native")]
    {
        backend == Backend::Native
    }
    #[cfg(not(feature = "native"))]
    {
        false
    }
}

fn session_backend(backend: Backend) -> read_local::SessionBackend {
    match backend {
        #[cfg(feature = "native")]
        Backend::Native => read_local::SessionBackend::Native,
        #[cfg(feature = "cli")]
        Backend::Cli => read_local::SessionBackend::Cli,
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

fn run(args: Args) -> Result<ExitCode> {
    if args.ssh.is_some() {
        return run_ssh(args);
    }
    if matches!(args.completion_candidates, Some(CompletionCandidates::Backends)) {
        print_candidates(
            Backend::value_variants()
                .iter()
                .map(|backend| backend.to_possible_value().unwrap().get_name().to_owned())
                .collect(),
        );
        return Ok(ExitCode::SUCCESS);
    }
    if args.completion_candidates.is_some() {
        return run_completion(args);
    }
    let backend = args.backend.unwrap_or_default();
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: context::Scope {
            origin: context::Origin::Local { host: "local".into() },
            cwd: None,
        },
        overrides: overrides(&args),
        intent: intent(&args),
        default_scaffold: default_scaffold(),
    })
    .map_err(|error| anyhow::anyhow!("invalid entry invocation: {error}"))?;
    tracing::info!(event = "lookup.start", backend = ?backend, argument = %attempt_intent_selector(&args));
    let mut sessions = session_backend(backend);
    let scope = attempt_scope(&attempt)?;
    let mut observer = read_local::Local::acquire(scope, &mut sessions).expect("local acquisition for a local attempt");
    let delivery = if args.print_shell {
        Delivery::Export
    } else {
        Delivery::Execute {
            native: native_capable(backend),
        }
    };
    // Successful exec replaces this process with the real tmux client; only
    // shell emission distinguishes Run from Exec.
    let handoff = if args.print_shell && !args.exec_shell { Handoff::Run } else { Handoff::Exec };
    let mut executor = execute_local::Local::default();
    let snapshot = tta_runner::execute::drive(&mut attempt, &mut observer, &mut executor, delivery, handoff).context("tmux launch failed")?;
    finish(snapshot, handoff)
}

fn attempt_intent_selector(args: &Args) -> &str {
    args.argument.as_deref().unwrap_or("")
}

/// Report an attempt outcome: exported shell, completed receipts (an
/// aggregate terminal status becomes this process's exit status), or the
/// typed stopped cause with its honest mutation knowledge. Labels come from
/// the semantic decision, never a second classification matrix.
fn finish(snapshot: AttemptSnapshot, handoff: Handoff) -> Result<ExitCode> {
    let Some(outcome) = snapshot.outcome else {
        anyhow::bail!("entry attempt ended without an outcome at {}", snapshot.leaf);
    };
    match outcome {
        AttemptOutcome::Exported(exported) => {
            println!("{}", exported.expression().shell(handoff)?);
            Ok(ExitCode::SUCCESS)
        }
        AttemptOutcome::Completed { effects } => Ok(match effects.finished.last() {
            Some((_, EffectReceipt::Terminal { code: Some(code) })) => ExitCode::from(*code as u8),
            _ => ExitCode::SUCCESS,
        }),
        AttemptOutcome::Delegated { .. } => Ok(ExitCode::SUCCESS),
        AttemptOutcome::Cancelled { boundary, .. } => {
            anyhow::bail!("entry attempt cancelled at {boundary}");
        }
        AttemptOutcome::Stopped { cause, .. } => {
            Err(anyhow::Error::new(Caused(cause)).context("tmux launch failed; do not replay a possibly executed mutation"))
        }
    }
}

/// Keep the full typed source chain in the reported error.
struct Caused(std::sync::Arc<EntryError>);

impl std::fmt::Debug for Caused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl std::fmt::Display for Caused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Caused {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

fn run_completion(args: Args) -> Result<ExitCode> {
    let backend = args.backend.unwrap_or_default();
    let mut sessions = session_backend(backend);
    let mut query = CatalogQuery::acquire(CatalogInput {
        scope: context::Scope {
            origin: context::Origin::Local { host: "local".into() },
            cwd: None,
        },
        overrides: overrides(&args),
    });
    {
        let scope = match query.waiting() {
            catalog::Waiting::Read(request) => (*request.scope).clone(),
            _ => anyhow::bail!("catalog must start with a context read"),
        };
        let mut observer = read_local::Local::acquire(scope, &mut sessions).expect("local acquisition for a catalog query");
        observe::drive_reads(&mut query, &mut observer).context("completion read driver")?;
    }
    let report = match query.waiting() {
        catalog::Waiting::Finished(catalog::Outcome::Report(report)) => report,
        catalog::Waiting::Finished(catalog::Outcome::Cancelled { boundary }) => anyhow::bail!("catalog cancelled at {boundary}"),
        _ => anyhow::bail!("catalog ended without a report"),
    };
    // Policy/binding errors invalidate the whole query; per-source read
    // failures stay visible as diagnostics while successful sources still
    // project their candidates. A failed registry source is always fatal:
    // every completion mode shares it.
    let registry_failed = report
        .sources
        .iter()
        .any(|source| matches!(source.request.kind, ReadKind::Registry { .. }) && source.result.is_err());
    if !report.errors.is_empty() || registry_failed {
        for error in &report.errors {
            eprintln!("{error:#}");
        }
        for source in &report.sources {
            if let Err(error) = &source.result {
                eprintln!("completion source failed: {error:#}");
            }
        }
        anyhow::bail!("completion policy failed");
    }
    for source in &report.sources {
        if let Err(error) = &source.result {
            eprintln!("completion source failed: {error:#}");
        }
    }
    match args.completion_candidates {
        Some(CompletionCandidates::Projects) => {
            // A bare basename is offered only when complete root evidence
            // establishes its unique physical identity; every ambiguous name
            // stays reachable through its qualified @N/name selector.
            let texts: Vec<String> = report
                .candidates
                .iter()
                .filter(|candidate| matches!(candidate, Candidate::Project { .. }))
                .map(Candidate::text)
                .collect();
            if !report.sources.iter().any(|source| matches!(source.request.kind, ReadKind::Projects { .. })) {
                return Err(tta_launcher::ResolveError::MissingRoot.into());
            }
            let bare: std::collections::BTreeSet<String> = texts.iter().filter(|text| !text.starts_with('@')).cloned().collect();
            print_candidates(
                texts
                    .into_iter()
                    .filter(|text| match text.strip_prefix('@').and_then(|selector| selector.split_once('/')) {
                        Some((_, name)) => !bare.contains(name),
                        None => true,
                    })
                    .collect(),
            );
        }
        // Aliases only: the primary completion surface, so a generated
        // satellite name never hides the workspace alias on Tab.
        Some(CompletionCandidates::SessionAliases) => {
            print_candidates(
                report
                    .workspaces
                    .iter()
                    .flat_map(|workspace| &workspace.views)
                    .map(|view| tta_launcher::satellite_alias(&view.fingerprint.name, &report.satellite_prefix).to_owned())
                    .collect(),
            );
        }
        // Raw names plus aliases, matching the literal session-name surface.
        Some(CompletionCandidates::Sessions) => {
            print_candidates(
                report
                    .candidates
                    .iter()
                    .filter(|candidate| matches!(candidate, Candidate::View { .. } | Candidate::WorkspaceAlias { .. }))
                    .map(Candidate::text)
                    .collect(),
            );
        }
        Some(CompletionCandidates::Scaffolds) => print_candidates(report.generators.clone()),
        _ => unreachable!("completion dispatch checked the mode"),
    }
    Ok(ExitCode::SUCCESS)
}

fn run_ssh(args: Args) -> Result<ExitCode> {
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
        environment.push(tta_launcher::target::Environment {
            name: name.into(),
            value: Some(value.into()),
        });
    }
    for name in &args.target_unset {
        environment.push(tta_launcher::target::Environment {
            name: name.clone(),
            value: None,
        });
    }
    let setup = tta_launcher::target::TargetSetup::builder()
        .maybe_shell(args.target_shell.clone())
        .login(args.target_login.unwrap_or(true))
        .interactive(args.target_interactive)
        .environment(environment)
        .maybe_cwd(args.target_cwd.clone())
        .build();
    let ssh = tta_runner::ssh::Ssh::builder()
        .destination(args.ssh.clone().expect("SSH branch"))
        .maybe_program(args.ssh_program.clone())
        .options(args.ssh_option.clone())
        .build();
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: context::Scope {
            origin: context::Origin::Ssh {
                host: args.ssh.clone().expect("SSH branch"),
                setup,
            },
            cwd: None,
        },
        // Target-local defaults come only from target facts; pre-defaulted
        // local CLI values never enter remote overrides.
        overrides: overrides(&args),
        intent: intent(&args),
        default_scaffold: default_scaffold(),
    })
    .map_err(|error| anyhow::anyhow!("invalid entry invocation: {error}"))?;
    let mut observer = observe::ssh::Ssh::acquire(ssh, attempt_scope(&attempt)?, limits)?;
    let handoff = if args.exec_shell { Handoff::Exec } else { Handoff::Run };
    let snapshot = tta_runner::execute::ssh::drive(&mut attempt, &mut observer, args.print_shell, handoff).map_err(|error| {
        error.context(if args.print_shell {
            "SSH planning failed"
        } else {
            "SSH terminal handoff failed; do not replay a possibly executed target mutation"
        })
    })?;
    observe::ssh::write_diagnostics(std::io::stderr().lock(), &observer.observations)?;
    match snapshot.outcome {
        Some(AttemptOutcome::Exported(exported)) => {
            // The exported artifact is the TARGET-side block; the printed
            // command is the resolved local SSH invocation carrying it.
            let target = snapshot.target.as_ref().expect("bound target");
            let command = observer.terminal_command(target, exported.expression())?;
            println!("{}", tta_command::Expression::from(command).shell(handoff)?);
            Ok(ExitCode::SUCCESS)
        }
        Some(AttemptOutcome::Completed { effects }) => Ok(match effects.finished.last() {
            Some((_, EffectReceipt::Terminal { code: Some(code) })) => ExitCode::from(*code as u8),
            _ => ExitCode::SUCCESS,
        }),
        Some(AttemptOutcome::Stopped { boundary, effects, cause }) => {
            Err(anyhow::Error::new(Caused(cause)).context(format!("SSH entry stopped at {boundary}; retained effect knowledge {effects:?}")))
        }
        Some(AttemptOutcome::Cancelled { boundary, .. }) => anyhow::bail!("entry attempt cancelled at {boundary}"),
        Some(AttemptOutcome::Delegated { .. }) => Ok(ExitCode::SUCCESS),
        None => anyhow::bail!("SSH attempt ended without an outcome at {}", snapshot.leaf),
    }
}

fn attempt_scope(attempt: &EntryAttempt) -> Result<context::Scope> {
    match attempt.waiting() {
        Waiting::Read(request) => Ok((*request.scope).clone()),
        _ => anyhow::bail!("acquisition must start with a context read"),
    }
}
