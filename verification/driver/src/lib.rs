//! Runtime acceptance, shared by the command and integration tests. Every run
//! owns a private socket and project fixture; the user's server is never queried.
//!
//! ```no_run
//! use tta_verify::{Backend, Config, Mode, Scenario, exercise};
//! let config = Config::builder()
//!     .tta("target/debug/tta".into())
//!     .backend(Backend::Native)
//!     .build();
//! let report = exercise(&config, Mode::Direct, Scenario::New);
//! assert!(report.success, "{report:#?}");
//! ```
#![forbid(unsafe_code)]

pub mod isolation;
pub mod ssh;
pub mod views;

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use bon::Builder;
use clap::ValueEnum;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tta_command::{Command as PlannedCommand, Handoff, Word};

const OUTPUT_LIMIT: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    Native,
    Cli,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Cli => "cli",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Direct,
    Shell,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    New,
    Existing,
}

#[derive(Clone, Debug, Builder)]
pub struct Config {
    pub tta: PathBuf,
    pub backend: Backend,
    /// Invoke tta from a real pane beneath an already attached client.
    #[builder(default)]
    pub inside: bool,
    /// Each external query and lifecycle phase is separately bounded.
    #[builder(default = Duration::from_secs(10))]
    pub timeout: Duration,
}

#[derive(Clone, Debug, Serialize)]
pub struct State {
    pub session_id: String,
    pub session_name: String,
    pub attached: u32,
    pub pane_id: String,
    pub pane_pid: u32,
    pub cwd: PathBuf,
    pub client_pids: Vec<u32>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ProcessOutput {
    pub status: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
    pub omitted_bytes: usize,
    pub complete: bool,
    pub capture_stop: String,
    pub cleanup: Vec<String>,
}

impl From<tta_runner::capture::Output> for ProcessOutput {
    fn from(output: tta_runner::capture::Output) -> Self {
        Self {
            status: output.status.and_then(|s| s.code()),
            success: output.success(),
            timed_out: output.stop == tta_runner::capture::Stop::Deadline,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            omitted_bytes: output.omitted_stdout + output.omitted_stderr,
            complete: output.complete(),
            capture_stop: format!("{:?}", output.stop),
            cleanup: output.cleanup,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub elapsed_ms: u128,
    pub event: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub backend: Backend,
    pub mode: Mode,
    pub scenario: Scenario,
    pub inside: bool,
    pub success: bool,
    pub failure: Option<String>,
    pub socket: PathBuf,
    pub project: PathBuf,
    pub events: Vec<Event>,
    pub initial: Option<State>,
    pub origin_before: Option<State>,
    pub origin_after: Option<State>,
    pub attached: Option<State>,
    pub detached: Option<State>,
    pub emission: Option<ProcessOutput>,
    pub terminal_pid: Option<u32>,
    pub terminal_exit: Option<u32>,
    pub transcript: String,
    pub omitted_transcript_bytes: usize,
    pub trace_events: Vec<serde_json::Value>,
    pub inside_trace: Option<String>,
    pub cleanup: Option<ProcessOutput>,
    pub endpoint_preflight: Option<ProcessOutput>,
}

/// Exercise the real executable, observe its server independently, detach via
/// its terminal, and return causal diagnostics even when an assertion fails.
pub fn exercise(config: &Config, mode: Mode, scenario: Scenario) -> Report {
    let start = Instant::now();
    let mut report = Report {
        backend: config.backend,
        mode,
        scenario,
        inside: config.inside,
        success: false,
        failure: None,
        socket: PathBuf::new(),
        project: PathBuf::new(),
        events: Vec::new(),
        initial: None,
        origin_before: None,
        origin_after: None,
        attached: None,
        detached: None,
        emission: None,
        terminal_pid: None,
        terminal_exit: None,
        transcript: String::new(),
        omitted_transcript_bytes: 0,
        trace_events: Vec::new(),
        inside_trace: None,
        cleanup: None,
        endpoint_preflight: None,
    };
    let mut terminal = None;
    let fixture = Fixture::new(config.timeout);
    let mut result = match &fixture {
        Ok(fixture) => {
            report.socket = fixture.socket.clone();
            report.project = fixture.project.clone();
            run(config, fixture, &mut report, &mut terminal, start)
        }
        Err(error) => Err(anyhow::anyhow!("create owned fixture: {error:#}")),
    };
    if let Some(mut terminal) = terminal {
        if terminal.exit.is_none() {
            report.events.push(Event {
                elapsed_ms: start.elapsed().as_millis(),
                event: "terminating terminal after failed exercise".to_owned(),
            });
        }
        if let Err(error) = terminal.stop(config.timeout) {
            report.events.push(Event {
                elapsed_ms: start.elapsed().as_millis(),
                event: format!("terminal cleanup: {error:#}"),
            });
        }
        let capture = terminal.capture.lock().unwrap();
        report.transcript = capture.text();
        report.omitted_transcript_bytes = capture.omitted;
        report.terminal_exit = terminal.exit;
    }
    if config.inside
        && let Ok(fixture) = &fixture
    {
        match read_text(&fixture.directory.path().join("inside-trace.jsonl")) {
            Ok(trace) => report.inside_trace = Some(trace),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) if result.is_ok() => result = Err(error.into()),
            Err(error) => report.events.push(Event {
                elapsed_ms: start.elapsed().as_millis(),
                event: format!("read inside trace: {error}"),
            }),
        }
    }
    let source = report
        .inside_trace
        .as_deref()
        .or_else(|| report.emission.as_ref().map(|output| output.stderr.as_str()))
        .unwrap_or(&report.transcript);
    report.trace_events = source.lines().filter_map(|line| serde_json::from_str(line.trim()).ok()).collect();
    if let Err(error) = result {
        report.failure = Some(format!("{error:#}"));
    } else if let Err(error) = check_trace(&report) {
        report.failure = Some(format!("{error:#}"));
    }
    if let Ok(fixture) = &fixture
        && fixture.proved.get()
        && fixture.socket.exists()
    {
        fixture.cleanup_attempted.set(true);
        match fixture.tmux(&["kill-server"]) {
            Ok(cleanup) => {
                if !cleanup.success && report.failure.is_none() {
                    report.failure = Some(format!("owned server cleanup failed: {cleanup:?}"));
                }
                report.cleanup = Some(cleanup);
            }
            Err(error) if report.failure.is_none() => report.failure = Some(format!("owned server cleanup: {error:#}")),
            Err(error) => report.events.push(Event {
                elapsed_ms: start.elapsed().as_millis(),
                event: format!("owned server cleanup failed: {error:#}"),
            }),
        }
    }
    report.success = report.failure.is_none();
    report.events.push(Event {
        elapsed_ms: start.elapsed().as_millis(),
        event: if report.success { "verified" } else { "failed" }.to_owned(),
    });
    report
}

fn run(config: &Config, fixture: &Fixture, report: &mut Report, terminal: &mut Option<Terminal>, start: Instant) -> Result<()> {
    let executable = std::path::absolute(&config.tta)?;
    let record = |report: &mut Report, event: &str| {
        report.events.push(Event {
            elapsed_ms: start.elapsed().as_millis(),
            event: event.to_owned(),
        })
    };
    ensure!(!fixture.socket.exists(), "fixture socket already existed");
    let preflight = fixture.tmux(&["list-sessions"])?;
    let proof = isolation::verify_absence(&fixture.socket, &preflight);
    report.endpoint_preflight = Some(preflight);
    proof?;
    fixture.proved.set(true);
    if report.scenario == Scenario::Existing {
        let output = fixture.tmux(&[
            "new-session",
            "-d",
            "-s",
            "verify-project",
            "-c",
            fixture.project.to_str().unwrap(),
            "sleep",
            "120",
        ])?;
        ensure!(output.success, "bootstrap existing session: {output:?}");
        report.initial = Some(fixture.state()?);
        record(report, "existing detached session seeded");
    } else {
        record(report, "cold server: socket absent");
    }
    let args = vec![
        "--backend".into(),
        config.backend.as_str().into(),
        "--socket-path".into(),
        fixture.socket.clone().into_os_string(),
        "--root".into(),
        fixture.root.clone().into_os_string(),
        "--tmux-config".into(),
        "/dev/null".into(),
        "--trace".into(),
        "verify-project".into(),
    ];
    let mut command = if config.inside {
        let output = fixture.tmux(&["new-session", "-d", "-s", "verify-origin", "-c", fixture.directory.path().to_str().unwrap()])?;
        ensure!(output.success, "bootstrap origin pane: {output:?}");
        let mut command = CommandBuilder::new("tmux");
        command.args(["-u", "-f", "/dev/null", "-S"]);
        command.arg(&fixture.socket);
        command.args(["attach-session", "-t", "=verify-origin"]);
        command
    } else if report.mode == Mode::Shell {
        let mut emit = Command::new(&executable);
        emit.args(&args).arg("--print-shell");
        fixture.environment(&mut emit);
        let output = capture(emit, config.timeout)?;
        let source = output.stdout.clone();
        let success = output.success;
        report.emission = Some(output);
        ensure!(success, "shell emission failed: {:?}", report.emission);
        ensure!(!source.trim().is_empty(), "shell emission was empty");
        ensure!(!source.contains(" exec "), "ordinary shell emission must not replace its caller");
        if report.scenario == Scenario::New {
            ensure!(!fixture.socket.exists(), "printing mutated a cold server");
        }
        record(report, "shell plan emitted without mutation");
        let mut command = CommandBuilder::new("/bin/sh");
        command.args([
            "-c",
            "eval \"$1\"; status=$?; printf '\\nTTA_VERIFY_CALLER_SURVIVED:%s\\n' \"$status\"; exit \"$status\"",
            "tta-verify-shell",
            &source,
        ]);
        command
    } else {
        let mut command = CommandBuilder::new(&executable);
        command.args(&args);
        command
    };
    fixture.terminal_environment(&mut command);
    command.cwd(fixture.directory.path());
    let spawned = Terminal::spawn(command)?;
    report.terminal_pid = spawned.child.process_id();
    *terminal = Some(spawned);
    record(report, "terminal process spawned");
    let terminal = terminal.as_mut().unwrap();
    if config.inside {
        let deadline = Instant::now() + config.timeout;
        loop {
            let origin = fixture.state_for("verify-origin")?;
            if origin.attached == 1 && origin.client_pids.len() == 1 {
                ensure!(origin.client_pids.first().copied() == report.terminal_pid, "origin client identity mismatch");
                let invocation = PlannedCommand::builder()
                    .program(Word::new(executable.as_os_str())?)
                    .args(args.iter().map(|arg| Word::new(arg.clone())).collect::<std::result::Result<Vec<_>, _>>()?)
                    .build();
                let raw = tta_command::Expression::from(invocation).shell(Handoff::Run)?;
                let trace = Word::new(fixture.directory.path().join("inside-trace.jsonl"))?.shell()?;
                let result = Word::new(fixture.directory.path().join("inside-status"))?.shell()?;
                let source = if report.mode == Mode::Shell {
                    let plan = Word::new(fixture.directory.path().join("inside-plan"))?.shell()?;
                    format!(
                        "tta_plan=$({raw} --print-shell 2>{trace}); tta_status=$?; if [ \"$tta_status\" -eq 0 ]; then printf '%s' \"$tta_plan\" >{plan}; eval \"$tta_plan\"; tta_status=$?; fi; printf '%s' \"$tta_status\" >{result}"
                    )
                } else {
                    format!("{raw} 2>{trace}; tta_status=$?; printf '%s' \"$tta_status\" >{result}")
                };
                let sent = fixture.tmux(&["send-keys", "-t", &origin.pane_id, "-l", &source])?;
                ensure!(sent.success, "invoke tta in origin pane: {sent:?}");
                ensure!(
                    fixture.tmux(&["send-keys", "-t", &origin.pane_id, "Enter"])?.success,
                    "send origin Enter failed"
                );
                report.origin_before = Some(origin);
                record(report, "invoked tta beneath the real attached origin client");
                break;
            }
            ensure!(Instant::now() < deadline, "origin attach timed out: {origin:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let deadline = Instant::now() + config.timeout;
    let mut last_observation = String::new();
    let independent = report.scenario == Scenario::Existing && !config.inside;
    let expected_name = if independent { "_verify-project-1" } else { "verify-project" };
    loop {
        if let Some(exit) = terminal.child.try_wait()? {
            terminal.exit = Some(exit.exit_code());
            bail!(
                "terminal exited before attach (code {}); last observation: {last_observation}",
                exit.exit_code()
            );
        }
        match fixture.state_for(expected_name) {
            Ok(state) if state.attached == 1 && state.client_pids.len() == 1 => {
                ensure!(state.cwd == fixture.project, "attached pane cwd mismatch: {state:?}");
                ensure!(state.session_name == expected_name, "wrong session: {state:?}");
                if report.mode == Mode::Direct {
                    ensure!(
                        report.terminal_pid == state.client_pids.first().copied(),
                        "direct handoff did not preserve the process identity: terminal={:?}, state={state:?}",
                        report.terminal_pid
                    );
                }
                if let Some(initial) = &report.initial {
                    ensure!((state.session_id != initial.session_id) == independent, "wrong entry independence policy");
                    ensure!(
                        state.pane_id == initial.pane_id && state.pane_pid == initial.pane_pid,
                        "satellite replaced the seeded work"
                    );
                }
                if let Some(origin) = &report.origin_before {
                    ensure!(state.client_pids == origin.client_pids, "switch replaced the client or nested another client");
                }
                report.attached = Some(state);
                break;
            }
            Ok(state) => last_observation = format!("{state:?}"),
            Err(error) => last_observation = format!("{error:#}"),
        }
        ensure!(Instant::now() < deadline, "attach timed out; last observation: {last_observation}");
        std::thread::sleep(Duration::from_millis(10));
    }
    record(report, "observed expected session, cwd and attached client");
    if config.inside {
        let deadline = Instant::now() + config.timeout;
        let result = fixture.directory.path().join("inside-status");
        loop {
            match read_text(&result) {
                Ok(status) => {
                    ensure!(status == "0", "inside tta/eval failed with status {status:?}");
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            ensure!(Instant::now() < deadline, "inside tta did not return to its original shell");
            std::thread::sleep(Duration::from_millis(5));
        }
        let origin = fixture.state_for("verify-origin")?;
        let before = report.origin_before.as_ref().unwrap();
        ensure!(
            origin.attached == 0 && origin.pane_id == before.pane_id && origin.pane_pid == before.pane_pid,
            "origin pane/shell did not survive switch: {origin:?}"
        );
        report.origin_after = Some(origin);
        if report.mode == Mode::Shell {
            report.emission = Some(ProcessOutput {
                status: Some(0),
                success: true,
                stdout: read_text(&fixture.directory.path().join("inside-plan"))?,
                stderr: read_text(&fixture.directory.path().join("inside-trace.jsonl"))?,
                ..ProcessOutput::default()
            });
        }
        record(report, "same client switched; original pane and evaluating shell survived");
    }
    terminal.writer.write_all(b"\x02d")?;
    terminal.writer.flush()?;
    record(report, "sent tmux detach key through PTY");
    terminal.wait(config.timeout)?;
    ensure!(terminal.exit == Some(0), "terminal process exited with {:?}", terminal.exit);
    let detached = fixture.state()?;
    ensure!(
        detached.attached == 0 && detached.client_pids.is_empty(),
        "client remained attached: {detached:?}"
    );
    let attached = report.attached.as_ref().unwrap();
    ensure!(
        detached.session_id == report.initial.as_ref().unwrap_or(attached).session_id
            && detached.pane_id == attached.pane_id
            && detached.pane_pid == attached.pane_pid,
        "detach replaced or destroyed session/pane state"
    );
    ensure!(detached.cwd == fixture.project, "detach changed cwd");
    report.detached = Some(detached);
    if independent {
        ensure!(
            fixture.state_for("_verify-project-1").is_err(),
            "disposable view survived detach alongside base"
        );
    }
    record(
        report,
        "process exited cleanly; workspace pane survived and disposable view followed lifecycle policy",
    );
    terminal.settle_reader(config.timeout);
    if report.mode == Mode::Shell && !config.inside {
        ensure!(
            terminal.capture.lock().unwrap().text().contains("TTA_VERIFY_CALLER_SURVIVED:0"),
            "evaluating shell did not survive tmux detach"
        );
    }
    ensure!(terminal.capture.lock().unwrap().omitted == 0, "PTY transcript exceeded its diagnostic bound");
    ensure!(terminal.capture.lock().unwrap().done, "PTY transcript did not reach EOF");
    Ok(())
}

fn check_trace(report: &Report) -> Result<()> {
    let expected = if report.mode == Mode::Shell { "emission" } else { "handoff" };
    for event in ["lookup.start", "resolution", "plan", expected] {
        ensure!(
            report.trace_events.iter().any(|value| value["fields"]["event"] == event),
            "missing tta trace event {event:?}"
        );
    }
    let decision = if report.scenario == Scenario::New { "new" } else { "existing" };
    ensure!(
        report
            .trace_events
            .iter()
            .any(|value| value["fields"]["event"] == "resolution" && value["fields"]["decision"] == decision),
        "trace decision disagrees with externally observed scenario"
    );
    ensure!(
        report.trace_events.iter().any(|value| value["fields"]["event"] == "lookup.start"
            && value["fields"]["backend"]
                .as_str()
                .is_some_and(|backend| backend.eq_ignore_ascii_case(report.backend.as_str()))),
        "trace backend disagrees with the selected backend"
    );
    if let Some(attached) = &report.attached {
        let resolved_name = report.initial.as_ref().unwrap_or(attached).session_name.as_str();
        ensure!(
            report
                .trace_events
                .iter()
                .any(|value| value["fields"]["event"] == "resolution" && value["fields"]["session_name"] == resolved_name),
            "trace session name disagrees with observed session"
        );
    }
    Ok(())
}

struct Fixture {
    directory: tempfile::TempDir,
    socket: PathBuf,
    root: PathBuf,
    project: PathBuf,
    timeout: Duration,
    proved: std::cell::Cell<bool>,
    cleanup_attempted: std::cell::Cell<bool>,
}

impl Fixture {
    fn new(timeout: Duration) -> Result<Self> {
        ensure!(!timeout.is_zero(), "timeout must be positive");
        let directory = tempfile::Builder::new().prefix("tta-accept-").tempdir_in(isolation::scratch_root())?;
        let socket = directory.path().join("socket");
        let root = directory.path().join("projects ' #[literal];");
        let project = root.join("verify-project");
        std::fs::create_dir_all(&project)?;
        std::fs::create_dir(directory.path().join("home"))?;
        std::fs::create_dir(directory.path().join("config"))?;
        Ok(Self {
            directory,
            socket,
            root,
            project,
            timeout,
            proved: std::cell::Cell::new(false),
            cleanup_attempted: std::cell::Cell::new(false),
        })
    }

    fn environment(&self, command: &mut Command) {
        for (key, _) in std::env::vars_os() {
            if key.as_encoded_bytes().starts_with(b"TTA_") || key == "PROJECTS_DIR" {
                command.env_remove(key);
            }
        }
        command
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .env("TERM", "xterm-256color")
            .env("HOME", self.directory.path().join("home"))
            .env("XDG_CONFIG_HOME", self.directory.path().join("config"))
            .env("SHELL", "/bin/sh")
            .env("ENV", "/dev/null");
    }

    fn terminal_environment(&self, command: &mut CommandBuilder) {
        for (key, _) in std::env::vars_os() {
            if key.as_encoded_bytes().starts_with(b"TTA_") || key == "PROJECTS_DIR" {
                command.env_remove(key);
            }
        }
        command.env("TMUX", "");
        command.env("TMUX_PANE", "");
        command.env("TERM", "xterm-256color");
        command.env("HOME", self.directory.path().join("home"));
        command.env("XDG_CONFIG_HOME", self.directory.path().join("config"));
        command.env("SHELL", "/bin/sh");
        command.env("ENV", "/dev/null");
    }

    fn tmux(&self, args: &[&str]) -> Result<ProcessOutput> {
        let mut command = Command::new("tmux");
        command.args(["-u", "-f", "/dev/null", "-S"]).arg(&self.socket).args(args);
        command.env("LC_ALL", "C");
        self.environment(&mut command);
        capture(command, self.timeout)
    }

    fn state(&self) -> Result<State> {
        self.state_for("verify-project")
    }

    fn state_for(&self, name: &str) -> Result<State> {
        let output = self.tmux(&["list-sessions", "-F", "#{session_id}\t#{session_name}\t#{session_attached}"])?;
        ensure!(output.success, "tmux state query: {output:?}");
        let rows: Vec<Vec<_>> = output.stdout.lines().map(|line| line.split('\t').collect()).collect();
        ensure!(rows.iter().all(|row| row.len() == 3), "malformed owned-session state: {output:?}");
        let fields = rows
            .iter()
            .find(|row| row[1] == name)
            .with_context(|| format!("owned session {name:?} not found: {output:?}"))?;
        let pane = self.tmux(&["display-message", "-p", "-t", fields[0], "#{pane_current_path}\n#{pane_id}\n#{pane_pid}"])?;
        ensure!(pane.success, "tmux pane query: {pane:?}");
        let pane_fields: Vec<_> = pane.stdout.trim_end_matches('\n').split('\n').collect();
        ensure!(pane_fields.len() == 3, "malformed owned-pane state: {pane:?}");
        let clients = self.tmux(&["list-clients", "-F", "#{client_pid}\t#{session_name}"])?;
        ensure!(clients.success, "tmux client query: {clients:?}");
        let mut client_pids = Vec::new();
        for line in clients.stdout.lines() {
            let (pid, session) = line.split_once('\t').context("malformed owned client row")?;
            if session == name {
                client_pids.push(pid.parse()?);
            }
        }
        Ok(State {
            session_id: fields[0].to_owned(),
            session_name: fields[1].to_owned(),
            attached: fields[2].parse()?,
            cwd: pane_fields[0].into(),
            pane_id: pane_fields[1].to_owned(),
            pane_pid: pane_fields[2].parse()?,
            client_pids,
        })
    }
}

// Last-resort cleanup for early returns/panics. The endpoint is exclusively
// inside this owned TempDir; no default endpoint or broad process kill is used.
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.proved.get() && !self.cleanup_attempted.get() && self.socket.exists() {
            match self.tmux(&["kill-server"]) {
                Ok(output) => eprintln!("tta verification fallback cleanup for {:?}: {output:?}", self.socket),
                Err(error) => eprintln!("tta verification fallback cleanup for {:?}: {error:#}", self.socket),
            }
        }
    }
}

#[derive(Default)]
struct Buffer {
    bytes: VecDeque<u8>,
    omitted: usize,
    done: bool,
}

fn read_text(path: &Path) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.take((OUTPUT_LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > OUTPUT_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("retained runtime artifact exceeds {OUTPUT_LIMIT} bytes: {path:?}"),
        ));
    }
    String::from_utf8(bytes).map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

impl Buffer {
    fn push(&mut self, bytes: &[u8]) {
        for byte in bytes {
            if self.bytes.len() == OUTPUT_LIMIT {
                self.bytes.pop_front();
                self.omitted += 1;
            }
            self.bytes.push_back(*byte);
        }
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

fn reader(mut input: Box<dyn Read + Send>) -> Arc<Mutex<Buffer>> {
    let capture = Arc::new(Mutex::new(Buffer::default()));
    let output = capture.clone();
    std::thread::spawn(move || {
        let mut bytes = [0_u8; 4096];
        loop {
            match input.read(&mut bytes) {
                Ok(0) => break,
                Ok(count) => output.lock().unwrap().push(&bytes[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break, // PTY EOF commonly arrives as EIO.
            }
        }
        output.lock().unwrap().done = true;
    });
    capture
}

fn capture(mut command: Command, timeout: Duration) -> Result<ProcessOutput> {
    let description = format!("{command:?}");
    let limits = tta_runner::capture::Limits {
        timeout,
        per_stream: OUTPUT_LIMIT,
        ..Default::default()
    };
    Ok(tta_runner::capture::run(&mut command, limits)
        .with_context(|| format!("capture {description}"))?
        .into())
}

fn settle(capture: &Arc<Mutex<Buffer>>, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !capture.lock().unwrap().done && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
}

struct Terminal {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
    capture: Arc<Mutex<Buffer>>,
    exit: Option<u32>,
}

impl Terminal {
    fn spawn(command: CommandBuilder) -> Result<Self> {
        let pair = portable_pty::native_pty_system().openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let capture = reader(pair.master.try_clone_reader()?);
        let writer = pair.master.take_writer()?;
        let child = pair.slave.spawn_command(command)?;
        drop(pair.slave);
        Ok(Self {
            child,
            writer,
            _master: pair.master,
            capture,
            exit: None,
        })
    }

    fn wait(&mut self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.exit = Some(status.exit_code());
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "terminal exit timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn stop(&mut self, timeout: Duration) -> Result<()> {
        if self.exit.is_none() {
            self.child.kill()?;
            self.wait(timeout)?;
        }
        self.settle_reader(Duration::from_millis(100));
        Ok(())
    }

    fn settle_reader(&self, timeout: Duration) {
        settle(&self.capture, timeout);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.exit.is_none() {
            let _ = self.child.kill();
        }
    }
}
