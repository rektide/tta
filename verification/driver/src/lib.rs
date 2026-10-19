//! Runtime acceptance, shared by the command and integration tests. Every run
//! owns a private socket and project fixture; the user's server is never queried.
#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use bon::Builder;
use clap::ValueEnum;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;

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
    pub success: bool,
    pub failure: Option<String>,
    pub socket: PathBuf,
    pub project: PathBuf,
    pub events: Vec<Event>,
    pub initial: Option<State>,
    pub attached: Option<State>,
    pub detached: Option<State>,
    pub emission: Option<ProcessOutput>,
    pub terminal_pid: Option<u32>,
    pub terminal_exit: Option<u32>,
    pub transcript: String,
    pub omitted_transcript_bytes: usize,
    pub trace_events: Vec<serde_json::Value>,
    pub cleanup: Option<ProcessOutput>,
}

/// Exercise the real executable, observe its server independently, detach via
/// its terminal, and return causal diagnostics even when an assertion fails.
pub fn exercise(config: &Config, mode: Mode, scenario: Scenario) -> Report {
    let start = Instant::now();
    let mut report = Report {
        backend: config.backend,
        mode,
        scenario,
        success: false,
        failure: None,
        socket: PathBuf::new(),
        project: PathBuf::new(),
        events: Vec::new(),
        initial: None,
        attached: None,
        detached: None,
        emission: None,
        terminal_pid: None,
        terminal_exit: None,
        transcript: String::new(),
        omitted_transcript_bytes: 0,
        trace_events: Vec::new(),
        cleanup: None,
    };
    let mut terminal = None;
    let fixture = Fixture::new(config.timeout);
    let result = match &fixture {
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
    let source = report.emission.as_ref().map(|output| output.stderr.as_str()).unwrap_or(&report.transcript);
    report.trace_events = source.lines().filter_map(|line| serde_json::from_str(line.trim()).ok()).collect();
    if let Err(error) = result {
        report.failure = Some(format!("{error:#}"));
    } else if let Err(error) = check_trace(&report) {
        report.failure = Some(format!("{error:#}"));
    }
    if let Ok(fixture) = &fixture {
        if fixture.socket.exists() {
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
    let mut command = if report.mode == Mode::Shell {
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
    let deadline = Instant::now() + config.timeout;
    let mut last_observation = String::new();
    loop {
        if let Some(exit) = terminal.child.try_wait()? {
            terminal.exit = Some(exit.exit_code());
            bail!(
                "terminal exited before attach (code {}); last observation: {last_observation}",
                exit.exit_code()
            );
        }
        match fixture.state() {
            Ok(state) if state.attached == 1 && state.client_pids.len() == 1 => {
                ensure!(state.cwd == fixture.project, "attached pane cwd mismatch: {state:?}");
                ensure!(state.session_name == "verify-project", "wrong session: {state:?}");
                if report.mode == Mode::Direct {
                    ensure!(
                        report.terminal_pid == state.client_pids.first().copied(),
                        "direct handoff did not preserve the process identity: terminal={:?}, state={state:?}",
                        report.terminal_pid
                    );
                }
                if let Some(initial) = &report.initial {
                    ensure!(state.session_id == initial.session_id, "attach replaced the seeded session");
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
        detached.session_id == attached.session_id && detached.pane_id == attached.pane_id && detached.pane_pid == attached.pane_pid,
        "detach replaced or destroyed session/pane state"
    );
    ensure!(detached.cwd == fixture.project, "detach changed cwd");
    report.detached = Some(detached);
    record(report, "process exited cleanly; detached session and pane survived");
    terminal.settle_reader(config.timeout);
    if report.mode == Mode::Shell {
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
        ensure!(
            report
                .trace_events
                .iter()
                .any(|value| value["fields"]["event"] == "resolution" && value["fields"]["session_name"] == attached.session_name),
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
}

impl Fixture {
    fn new(timeout: Duration) -> Result<Self> {
        ensure!(!timeout.is_zero(), "timeout must be positive");
        let parent = if Path::new("/tmp/opencode").is_dir() {
            PathBuf::from("/tmp/opencode")
        } else {
            std::env::temp_dir()
        };
        let directory = tempfile::Builder::new().prefix("tta-accept-").tempdir_in(parent)?;
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
        })
    }

    fn environment(&self, command: &mut Command) {
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
        self.environment(&mut command);
        capture(command, self.timeout)
    }

    fn state(&self) -> Result<State> {
        let output = self.tmux(&["list-sessions", "-F", "#{session_id}\t#{session_name}\t#{session_attached}"])?;
        ensure!(output.success, "tmux state query: {output:?}");
        let fields: Vec<_> = output.stdout.trim_end_matches('\n').split('\t').collect();
        ensure!(fields.len() == 3 && fields[1] == "verify-project", "malformed owned-session state: {output:?}");
        let pane = self.tmux(&["display-message", "-p", "-t", fields[0], "#{pane_current_path}\n#{pane_id}\n#{pane_pid}"])?;
        ensure!(pane.success, "tmux pane query: {pane:?}");
        let pane_fields: Vec<_> = pane.stdout.trim_end_matches('\n').split('\n').collect();
        ensure!(pane_fields.len() == 3, "malformed owned-pane state: {pane:?}");
        let clients = self.tmux(&["list-clients", "-F", "#{client_pid}"])?;
        ensure!(clients.success, "tmux client query: {clients:?}");
        Ok(State {
            session_id: fields[0].to_owned(),
            session_name: fields[1].to_owned(),
            attached: fields[2].parse()?,
            cwd: pane_fields[0].into(),
            pane_id: pane_fields[1].to_owned(),
            pane_pid: pane_fields[2].parse()?,
            client_pids: clients.stdout.lines().map(str::parse).collect::<std::result::Result<_, _>>()?,
        })
    }
}

// Last-resort cleanup for early returns/panics. The endpoint is exclusively
// inside this owned TempDir; no default endpoint or broad process kill is used.
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.socket.exists() {
            let _ = self.tmux(&["kill-server"]);
        }
    }
}

#[derive(Default)]
struct Buffer {
    bytes: VecDeque<u8>,
    omitted: usize,
    done: bool,
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
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {description}"))?;
    let stdout = reader(Box::new(child.stdout.take().unwrap()));
    let stderr = reader(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + timeout;
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status, false);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break (child.wait()?, true);
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    settle(&stdout, Duration::from_millis(100));
    settle(&stderr, Duration::from_millis(100));
    let stdout = stdout.lock().unwrap();
    let stderr = stderr.lock().unwrap();
    let output = ProcessOutput {
        status: status.code(),
        success: status.success() && !timed_out,
        timed_out,
        stdout: stdout.text(),
        stderr: stderr.text(),
        omitted_bytes: stdout.omitted + stderr.omitted,
    };
    ensure!(output.omitted_bytes == 0, "process output exceeded its bound ({description}): {output:?}");
    Ok(output)
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
