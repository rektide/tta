//! Positive two-real-terminal acceptance for ordinary independent entry. This
//! does not invoke inside switching or assert input-origin inference is fixed.
use super::*;

#[derive(Debug, Serialize)]
pub struct ViewReport {
    pub backend: Backend,
    pub mode: Mode,
    pub prefix: String,
    pub tmux_version: String,
    pub checks: Vec<String>,
    pub observations: Vec<String>,
    pub completion: String,
    pub success: bool,
    pub failure: Option<String>,
    pub cleanup: Option<ProcessOutput>,
    pub transcripts: Vec<String>,
}

/// Owns a private server, invokes the built tta in real PTYs, and records the
/// independently observed client/window/pane/process identities. No control
/// clients stand in for terminal clients. Both entry and cleanup are bounded.
pub fn exercise(config: &Config, mode: Mode, prefix: &str) -> ViewReport {
    let mut report = ViewReport {
        backend: config.backend,
        mode,
        prefix: prefix.into(),
        tmux_version: String::new(),
        checks: Vec::new(),
        observations: Vec::new(),
        completion: String::new(),
        success: false,
        failure: None,
        cleanup: None,
        transcripts: Vec::new(),
    };
    let mut terminals = Vec::new();
    let fixture = Fixture::new(config.timeout);
    let result = (|| -> Result<()> {
        let fixture = fixture.as_ref().map_err(|error| anyhow::anyhow!("fixture: {error:#}"))?;
        let mut version = Command::new("tmux");
        version.arg("-V");
        let version = capture(version, config.timeout)?;
        ensure!(version.success, "version query failed: {version:?}");
        report.tmux_version = version.stdout.trim().into();
        let preflight = fixture.tmux(&["list-sessions"])?;
        isolation::verify_absence(&fixture.socket, &preflight)?;
        fixture.proved.set(true);
        run(config, fixture, mode, prefix, &mut terminals, &mut report)
    })();
    if let Err(error) = result {
        report.failure = Some(format!("{error:#}"));
    }
    for terminal in &mut terminals {
        if let Err(error) = terminal.stop(config.timeout) {
            report.failure.get_or_insert_with(|| format!("terminal cleanup: {error:#}"));
        }
        report.transcripts.push(terminal.capture.lock().unwrap().text());
    }
    if let Ok(fixture) = fixture
        && fixture.proved.get()
        && fixture.socket.exists()
    {
        fixture.cleanup_attempted.set(true);
        match fixture.tmux(&["kill-server"]) {
            Ok(cleanup) => {
                if !cleanup.success {
                    report.failure.get_or_insert_with(|| format!("cleanup: {cleanup:?}"));
                }
                report.cleanup = Some(cleanup);
            }
            Err(error) => {
                report.failure.get_or_insert_with(|| format!("cleanup: {error:#}"));
            }
        }
    }
    report.success = report.failure.is_none();
    report
}

fn output(fixture: &Fixture, args: &[&str]) -> Result<String> {
    let result = fixture.tmux(args)?;
    ensure!(result.success, "tmux {args:?}: {result:?}");
    Ok(result.stdout.trim_end_matches('\n').into())
}

fn await_state(fixture: &Fixture, name: &str, attached: u32) -> Result<State> {
    let deadline = Instant::now() + fixture.timeout;
    loop {
        let state = fixture.state_for(name);
        if let Ok(state) = &state
            && state.attached == attached
        {
            return Ok(state.clone());
        }
        ensure!(Instant::now() < deadline, "waiting for {name} with {attached} clients: {state:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn query(config: &Config, fixture: &Fixture, prefix: &str) -> Command {
    let mut command = Command::new(&config.tta);
    fixture.environment(&mut command);
    command
        .args(["--backend", config.backend.as_str(), "--socket-path"])
        .arg(&fixture.socket)
        .args(["--tmux-config", "/dev/null"]);
    if prefix != "_" {
        command.args(["--satellite-prefix", prefix]);
    }
    command
}

fn launch(config: &Config, fixture: &Fixture, mode: Mode, prefix: &str, shared: bool, orphan: bool) -> Result<Terminal> {
    let mut query = query(config, fixture, prefix);
    if shared {
        query.arg("--group=false");
    }
    query
        .arg("--root")
        .arg(if orphan { Path::new("/tta-nonexistent-orphan-root") } else { &fixture.root });
    query.args(["verify-project", "--", "sleep", "120"]);
    let mut command = match mode {
        Mode::Direct => {
            let mut command = CommandBuilder::new(&config.tta);
            command.args(query.get_args());
            command
        }
        Mode::Shell => {
            // --print-shell is a launcher option, not part of startup argv.
            let mut emit = Command::new(&config.tta);
            fixture.environment(&mut emit);
            emit.arg("--print-shell").args(query.get_args());
            let result = capture(emit, config.timeout)?;
            ensure!(result.success, "emission: {result:?}");
            let mut command = CommandBuilder::new("/bin/sh");
            command.args(["-c", "eval \"$1\"", "tta-view-evaluator", &result.stdout]);
            command
        }
    };
    fixture.terminal_environment(&mut command);
    command.cwd(fixture.directory.path());
    Terminal::spawn(command)
}

fn panes(fixture: &Fixture, session: &str) -> Result<Vec<String>> {
    let mut rows: Vec<_> = output(
        fixture,
        &["list-panes", "-s", "-t", &format!("={session}"), "-F", "#{window_id} #{pane_id} #{pane_pid}"],
    )?
    .lines()
    .map(str::to_owned)
    .collect();
    rows.sort();
    Ok(rows)
}

fn window(fixture: &Fixture, session: &str) -> Result<String> {
    output(fixture, &["display-message", "-p", "-t", &format!("={session}:"), "#{window_id}"])
}

fn select(fixture: &Fixture, session: &str, window: &str) -> Result<()> {
    output(fixture, &["select-window", "-t", &format!("={session}:{window}")])?;
    Ok(())
}

fn detach(terminal: &mut Terminal, timeout: Duration) -> Result<()> {
    terminal.writer.write_all(b"\x02d")?;
    terminal.writer.flush()?;
    terminal.wait(timeout)?;
    ensure!(terminal.exit == Some(0), "detach exit: {:?}", terminal.exit);
    Ok(())
}

fn run(config: &Config, fixture: &Fixture, mode: Mode, prefix: &str, terminals: &mut Vec<Terminal>, report: &mut ViewReport) -> Result<()> {
    let base = "verify-project";
    let satellite = format!("{prefix}{base}-1");
    terminals.push(launch(config, fixture, mode, prefix, false, false)?);
    let first = await_state(fixture, base, 1)?;
    let original = window(fixture, base)?;
    let other = output(
        fixture,
        &["new-window", "-d", "-P", "-F", "#{window_id}", "-t", "=verify-project:", "sleep", "120"],
    )?;
    let initial_panes = panes(fixture, base)?;
    terminals.push(launch(config, fixture, mode, prefix, false, false)?);
    let second = await_state(fixture, &satellite, 1)?;
    ensure!(
        first.session_id != second.session_id && first.client_pids != second.client_pids,
        "not distinct terminal views"
    );
    ensure!(
        first.pane_id == second.pane_id && first.pane_pid == second.pane_pid,
        "initial process not shared"
    );
    ensure!(panes(fixture, &satellite)? == initial_panes, "window/pane/process IDs differ");
    report.observations.push(format!("cold={first:?}; later={second:?}; shared={initial_panes:?}"));
    select(fixture, &satellite, &other)?;
    ensure!(window(fixture, &satellite)? == other && window(fixture, base)? == original, "B moved A");
    select(fixture, base, &other)?;
    select(fixture, &satellite, &original)?;
    ensure!(
        window(fixture, base)? == other && window(fixture, &satellite)? == original,
        "B's return moved A"
    );
    select(fixture, base, &original)?;
    ensure!(window(fixture, &satellite)? == original, "A moved B");
    select(fixture, base, &other)?;
    ensure!(
        window(fixture, base)? == other && window(fixture, &satellite)? == original,
        "A's selection moved B"
    );
    select(fixture, base, &original)?;
    report
        .observations
        .push(output(fixture, &["list-clients", "-F", "#{client_pid} #{session_id} #{window_id} #{pane_id}"])?);
    report
        .checks
        .push("plain cold/later real-terminal entry shares all window/pane/process IDs with independent selection both ways".into());

    terminals.push(launch(config, fixture, mode, prefix, true, false)?);
    await_state(fixture, base, 2)?;
    select(fixture, base, &other)?;
    let clients = output(fixture, &["list-clients", "-F", "#{session_id} #{window_id}"])?;
    ensure!(
        clients.lines().filter(|row| *row == format!("{} {other}", first.session_id)).count() == 2,
        "shared escape hatch failed: {clients}"
    );
    ensure!(window(fixture, &satellite)? == original, "shared escape hatch changed independent view");
    detach(&mut terminals[2], config.timeout)?;
    await_state(fixture, base, 1)?;
    report
        .checks
        .push("--group=false shares the base's selection without affecting the independent satellite".into());

    let orphan = format!("{prefix}{base}-42");
    output(fixture, &["rename-session", "-t", &second.session_id, &orphan])?;
    output(fixture, &["kill-session", "-t", &first.session_id])?;
    terminals[0].wait(config.timeout)?;
    detach(&mut terminals[1], config.timeout)?;
    let survivor = await_state(fixture, &orphan, 0)?;
    ensure!(panes(fixture, &orphan)? == initial_panes, "base loss or final detach destroyed work");
    let mut completion = query(config, fixture, prefix);
    completion.args(["--completion-candidates", "sessions"]);
    let completion = capture(completion, config.timeout)?;
    ensure!(completion.success, "orphan completion: {completion:?}");
    ensure!(
        completion.stdout.lines().any(|name| name == base) && completion.stdout.lines().any(|name| name == orphan),
        "no usable orphan alias: {completion:?}"
    );
    report.completion = completion.stdout;
    ensure!(
        output(fixture, &["list-sessions", "-F", "#{session_name}"])? == orphan,
        "completion mutated sessions"
    );
    terminals.push(launch(config, fixture, mode, prefix, false, true)?);
    let resumed = await_state(fixture, &satellite, 1)?;
    ensure!(panes(fixture, &satellite)? == initial_panes, "orphan alias created new processes");
    ensure!(resumed.session_id != survivor.session_id, "orphan entry did not create independent view");
    detach(&mut terminals[3], config.timeout)?;
    ensure!(
        output(fixture, &["list-sessions", "-F", "#{session_id}"])?.lines().count() == 1,
        "final view not preserved"
    );
    report.observations.push(format!("orphan={survivor:?}; resumed={resumed:?}"));
    report.checks.push("only numeric satellite survives base loss and final detach; alias lookup/completion rendezvous without a root or new processes, without recursive decoration".into());
    Ok(())
}
