//! Independent review witness for the SSH milestone: drive the *public* core
//! API through a complete runner-free request -> probe -> decode -> resume
//! loop using only std process execution. This package deliberately has no
//! `tta-runner` dependency; if the loop needs the runner, it must fail here.
//!
//! Covered concerns that no existing checked-in witness covers runner-free:
//! - `target::Bootstrap` walked from "/" with a symlinked component and an
//!   interior "..", through real GNU find/stat listings;
//! - the framed `probe::Probe` script executed and decoded via the public
//!   `probe::Output` contract (stdlib capture, honest completeness);
//! - `Registry` and `GeneratorDefaults` probes (no process-carrier coverage
//!   existed; only the real SSH fixture exercised them);
//! - contradictory local environment values overridden by target facts;
//! - symlink ENOTDIR and non-UTF-8 registry content failing closed inside
//!   the same loop.
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as _, Result, bail, ensure};
use serde_json::{Value, json};
use tta_command::{Expression, Handoff};
use tta_launcher::{Selection, probe, target, workflow};

struct Ran {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    complete: bool,
}

/// Stdlib-only bounded carrier. Both pipes are read to real EOF and the exit
/// code is required; anything else is reported incomplete, never assumed.
fn run_script(script: &str, poison: &[(&str, &str)]) -> Result<Ran> {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(script).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (name, value) in poison {
        command.env(name, value);
    }
    let mut child = command.spawn().context("spawn stdlib probe carrier")?;
    let mut out_pipe = child.stdout.take().expect("piped stdout");
    let mut err_pipe = child.stderr.take().expect("piped stderr");
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        out_pipe.read_to_end(&mut bytes).map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        err_pipe.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stdout = out.join().expect("stdout reader")?;
    let stderr = err.join().expect("stderr reader")?;
    let status = child.wait()?;
    Ok(Ran { stdout, stderr, status: status.code(), complete: status.code().is_some() })
}

fn token() -> Result<String> {
    let mut nonce = [0u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
    Ok(nonce.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// One public-API observation: construct, emit, execute, decode. The setup
/// is shared by probes and the final action, mirroring the runner contract.
fn observe(
    setup: &target::TargetSetup,
    request: probe::Request,
    poison: &[(&str, &str)],
    records: &mut Vec<Value>,
) -> Result<probe::Decoded> {
    let probe = probe::Probe::new(request.clone(), token()?)?;
    let ran = run_script(&probe.shell(setup)?, poison).context("execute emitted probe")?;
    let decoded = probe.decode(probe::Output {
        stdout: &ran.stdout,
        stderr: &ran.stderr,
        status: ran.status,
        complete: ran.complete,
    });
    records.push(json!({
        "request": format!("{request:?}"),
        "carrier_status": ran.status,
        "complete": ran.complete,
        "stdout_bytes": ran.stdout.len(),
        "stderr_bytes": ran.stderr.len(),
        "decoded": decoded.as_ref().map(|d| json!({
            "reply": format!("{:?}", d.reply),
            "command_status": d.diagnostics.command_status,
            "startup_stdout_bytes": d.diagnostics.startup_stdout.len(),
            "trailing_stdout_bytes": d.diagnostics.trailing_stdout.len(),
            "transport_stderr_bytes": d.diagnostics.transport_stderr.len(),
            "command_stderr_bytes": d.diagnostics.command_stderr.len(),
        })).map_err(|e: &anyhow::Error| format!("{e:#}")),
    }));
    decoded
}

fn quote(value: impl Into<OsString>) -> Result<String> {
    Ok(tta_command::Word::new(value)?.shell()?)
}

fn executable(path: &Path, source: &str) -> Result<()> {
    std::fs::write(path, source)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

struct Fixture {
    directory: tempfile::TempDir,
    setup: target::TargetSetup,
    root: PathBuf,
    invocation: PathBuf,
    tmux: PathBuf,
    registry: PathBuf,
    shell_argv_log: PathBuf,
}

/// The registry is reached through `<base>/cfg-link/../tta/<name>` where
/// `cfg-link` is a symlink to `home/.config`: the walk must cross a symlinked
/// component, an interior "..", and ordinary components from "/" down.
fn shell_source(log: &Path, home: &Path, root: &Path, socket: &Path, registry: &Path, tmux: &Path, invocation: &Path) -> Result<String> {
    Ok(format!(
        "#!/bin/sh\nprintf '%s\\000' \"$@\" >> {log}\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != -c ]; do shift; done\n[ \"$#\" -eq 2 ] || exit 91\nshift\nprintf 'startup chatter\\n'\nprintf 'startup diagnostic\\n' >&2\nexport HOME={home} TTA_ROOT={root} TTA_SOCKET_PATH={socket} TTA_CONFIG={registry} TTA_TMUX={tmux} LICENSE=Apache-2.0\nunset TMUX TMUX_PANE TTA_SOCKET_NAME TTA_SCAFFOLD AUTHOR XDG_CONFIG_HOME\ncd -P {invocation} || exit $?\nexec /bin/sh -c \"$1\"\n",
        log = quote(log)?,
        home = quote(home)?,
        root = quote(root)?,
        socket = quote(socket)?,
        registry = quote(registry)?,
        tmux = quote(tmux)?,
        invocation = quote(invocation)?,
    ))
}

impl Fixture {
    fn new(shell_name: &str, registry_name: &str, registry_bytes: &[u8]) -> Result<Self> {
        let directory = tempfile::Builder::new().prefix("tta-ssh-regress-").tempdir_in("/tmp/opencode")?;
        let base = directory.path();
        let root = base.join("target projects ' #;");
        let invocation = base.join("invocation dir");
        let home = base.join("home");
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(&invocation)?;
        std::fs::create_dir_all(home.join(".config/tta"))?;
        std::fs::write(home.join(".gitconfig"), "[user]\n name = Witness Git Author\n email = witness@example.invalid\n")?;
        // cfg-link -> home/.config, so cfg-link/.. resolves to home and the
        // registry physically lives at home/tta/<name>.
        std::os::unix::fs::symlink("home/.config", base.join("cfg-link"))?;
        std::fs::create_dir_all(home.join("tta"))?;
        let registry = base.join("cfg-link/../tta").join(registry_name);
        std::fs::write(home.join("tta").join(registry_name), registry_bytes)?;
        let tmux = base.join("fake-tmux");
        let socket = base.join("tmux.sock");
        let shell_argv_log = base.join("shell-argv");
        executable(
            &tmux,
            "#!/bin/sh\nsocket=\nprev=\nfor arg in \"$@\"; do\n if [ \"$prev\" = -S ]; then socket=$arg; fi\n prev=$arg\ndone\nprintf 'no server running on %s\\n' \"$socket\" >&2\nexit 1\n",
        )?;
        let shell = base.join(shell_name);
        executable(&shell, &shell_source(&shell_argv_log, &home, &root, &socket, &registry, &tmux, &invocation)?)?;
        let setup = target::TargetSetup::builder()
            .shell(shell.to_str().context("UTF-8 fixture path")?.into())
            .login(true)
            .interactive(true)
            .build();
        Ok(Self { directory, setup, root, invocation, tmux, registry, shell_argv_log })
    }

    /// Records of the selected shell's argv, NUL-delimited, four words each
    /// (flags, -c, carrier). The shell appends one record per invocation.
    fn shell_records(&self) -> Result<usize> {
        let bytes = std::fs::read(&self.shell_argv_log).unwrap_or_default();
        let words = bytes.split(|b| *b == 0).filter(|word| !word.is_empty()).count();
        ensure!(words % 4 == 0, "shell argv log is not four words per invocation");
        Ok(words / 4)
    }

    fn shell_argv_flat(&self) -> Result<Vec<String>> {
        let bytes = std::fs::read(&self.shell_argv_log).unwrap_or_default();
        Ok(bytes
            .split(|b| *b == 0)
            .filter(|word| !word.is_empty())
            .map(|word| String::from_utf8_lossy(word).into_owned())
            .collect())
    }
}

/// Contradictory local values that must never become target facts.
const POISON: &[(&str, &str)] = &[
    ("HOME", "/local/wrong/home"),
    ("XDG_CONFIG_HOME", "/local/wrong/xdg"),
    ("TTA_ROOT", "/local/wrong/root"),
    ("TTA_CONFIG", "/local/wrong/config.toml"),
    ("TTA_TMUX", "/local/wrong/tmux"),
    ("TTA_SOCKET_PATH", "/local/wrong/socket"),
    ("TTA_SOCKET_NAME", "/local/wrong/name"),
    ("TTA_SCAFFOLD", "local-wrong-generator"),
    ("TMUX", "/local/wrong/socket,1,0"),
    ("AUTHOR", "Local Wrong Author"),
    ("LICENSE", "Local-Wrong-License"),
];

const REGISTRY: &[u8] = b"[scaffold]\ndefault = 'native'\n[scaffold.generators.native]\nadapter = 'rekon-native'\n[scaffold.generators.native.options]\ninstall = false\n[scaffold.generators.native.options.remotes]\norigin = ''\ntngl = ''\nvoodoo = ''\n";

fn main() -> Result<()> {
    let report_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/opencode/tta-runner-free-review.json".into());
    let mut report = serde_json::Map::new();
    report.insert("kind".into(), json!("runner-free public-API probe round-trip (review witness; stdlib carrier only)"));
    let mut checks: Vec<Value> = Vec::new();
    let result = run(&mut checks);
    for check in &checks {
        println!("check: {}", check["check"].as_str().unwrap_or_default());
    }
    report.insert("success".into(), json!(result.is_ok()));
    report.insert(
        "failure".into(),
        result.as_ref().err().map(|e| json!(format!("{e:#}"))).unwrap_or(Value::Null),
    );
    report.insert("checks".into(), Value::Array(checks));
    std::fs::write(&report_path, serde_json::to_string_pretty(&Value::Object(report))?)?;
    println!("report: {report_path}");
    result.map(|()| ())
}

fn drive_setup(
    setup: &mut target::TargetSetup,
    options: target::Options,
    poison: &[(&str, &str)],
    records: &mut Vec<Value>,
) -> Result<(workflow::Input, target::Facts)> {
    let mut bootstrap = target::Bootstrap::new(options);
    let input = loop {
        match bootstrap.next()? {
            target::Progress::Ready(input) => break *input,
            target::Progress::Observe(request) => {
                let reply = match observe(setup, probe::Request::Setup(request), poison, records)?.reply {
                    probe::Reply::Setup(reply) => reply,
                    probe::Reply::Workflow(_) => bail!("setup probe decoded a workflow reply"),
                };
                bootstrap.supply(reply)?;
                if let Some(facts) = bootstrap.facts() {
                    // Freeze the observed physical cwd for every later probe
                    // and the final action, as the runner contract requires.
                    setup.cwd = Some(facts.cwd.clone());
                }
            }
        }
    };
    Ok((input, bootstrap.facts().expect("ready bootstrap").clone()))
}

fn run(checks: &mut Vec<Value>) -> Result<()> {
    let fixture = Fixture::new("selected-shell", "config.toml", REGISTRY)?;
    let records = &mut Vec::new();
    let mut setup = fixture.setup.clone();
    let (input, facts) = drive_setup(
        &mut setup,
        target::Options::builder().argument("witness-create".into()).default_scaffold("default".into()).build(),
        POISON,
        records,
    )?;
    ensure!(facts.cwd == fixture.invocation, "target cwd disagreement: {:?}", facts.cwd);
    let home = fixture.directory.path().join("home");
    ensure!(facts.value("HOME") == home.to_str(), "local HOME leaked: {:?}", facts.value("HOME"));
    ensure!(facts.value("TTA_ROOT") == Some(fixture.root.to_str().expect("UTF-8")), "local TTA_ROOT leaked");
    ensure!(facts.value("TTA_CONFIG") == Some(fixture.registry.to_str().expect("UTF-8")), "registry path disagreement: {:?}", facts.value("TTA_CONFIG"));
    ensure!(facts.value("TTA_TMUX") == Some(fixture.tmux.to_str().expect("UTF-8")), "local TTA_TMUX leaked");
    checks.push(json!({"check": "runner-free Bootstrap: context probe and registry walk through a symlinked component and interior '..'; contradictory local values never became facts"}));

    let mut workflow = workflow::Workflow::new(input.clone())?;
    let prepared = loop {
        match workflow.next()? {
            workflow::Progress::Ready(prepared) => break *prepared,
            workflow::Progress::Observe(request) => {
                let reply = match observe(&setup, probe::Request::Workflow(request), POISON, records)?.reply {
                    probe::Reply::Workflow(reply) => reply,
                    probe::Reply::Setup(_) => bail!("workflow probe decoded a setup reply"),
                };
                workflow.supply(reply)?;
            }
        }
    };
    let records_after_happy = records.len();
    ensure!(records_after_happy == 12, "unexpected probe count: {records_after_happy}");
    ensure!(fixture.shell_records()? == 12, "unexpected selected-shell invocations: {}", fixture.shell_records()?);
    let argv = fixture.shell_argv_flat()?;
    ensure!(
        argv.chunks(4).all(|record| record.first().is_some_and(|w| w == "-l") && record.get(1).is_some_and(|w| w == "-i") && record.get(2).is_some_and(|w| w == "-c")),
        "selected-shell login/interactive argv not applied to every probe"
    );
    checks.push(json!({"check": "every probe ran the selected shell with -l -i -c; exactly 12 probes (setup 8, workflow 4)"}));

    let first = &records[0];
    let decoded = &first["decoded"]["Ok"];
    ensure!(
        !decoded.is_null() && decoded["startup_stdout_bytes"] == json!(16) && decoded["transport_stderr_bytes"] == json!(19),
        "framing lost the deliberate startup chatter: {first}"
    );
    checks.push(json!({"check": "startup stdout and startup stderr stayed distinct diagnostic channels through the public decode"}));

    let expected_directory = fixture.root.join("witness-create");
    let Selection::Create { directory, .. } = &prepared.selection else {
        bail!("wrong selection: {:?}", prepared.selection)
    };
    ensure!(directory == &expected_directory, "create directory: {directory:?}");
    let steps = prepared.plan.expression().steps();
    let writes: Vec<_> = steps.iter().filter_map(|s| if let Expression::Write(w) = s { Some(w) } else { None }).collect();
    ensure!(writes.len() == 1, "expected one package.json write");
    let package: Value = serde_json::from_str(writes[0].contents.as_os_str().to_str().expect("UTF-8 package"))?;
    ensure!(package["author"] == "Witness Git Author" && package["license"] == "Apache-2.0", "local author/license leaked: {package}");
    let commands: Vec<_> = steps.iter().filter_map(|s| if let Expression::Command(c) = s { Some(c) } else { None }).collect();
    let tmux = commands.last().expect("tmux command");
    ensure!(tmux.program.as_os_str() == fixture.tmux, "tmux program disagreement: {:?}", tmux.program);
    let args: Vec<_> = tmux.args.iter().map(|a| a.as_os_str().to_str().expect("UTF-8 arg")).collect();
    let socket = fixture.directory.path().join("tmux.sock").to_str().expect("UTF-8").to_owned();
    ensure!(args.contains(&"-S") && args.contains(&socket.as_str()), "socket disagreement: {args:?}");
    ensure!(args.contains(&"new-session") && args.contains(&"witness-create"), "cold creation command missing: {args:?}");
    let last_cd = steps.iter().rev().find_map(|s| if let Expression::ChangeDir(cd) = s { Some(cd) } else { None });
    ensure!(last_cd.is_some_and(|cd| cd.path.as_os_str() == expected_directory.as_os_str()), "ChangeDir ignored the target project directory");
    checks.push(json!({"check": "runner-free Create plan: target registry generator, target Git/license defaults, target socket/program and literal directory"}));

    let handoff = setup.launch(&prepared, Handoff::Exec)?;
    ensure!(handoff.starts_with("/bin/sh -c "), "prepared launch is not a single selected-shell command");
    ensure!(handoff.contains("new-session"), "prepared launch lost the tmux action");
    checks.push(json!({"check": "TargetSetup::launch renders the prepared plan as one selected-shell handoff"}));

    // ENOTDIR: a symlink whose referent has a non-directory ancestor must
    // stop the same loop; find's %Y=N cannot masquerade as absence here.
    std::fs::write(fixture.root.join("blocker"), "x")?;
    std::os::unix::fs::symlink("blocker/child", fixture.root.join("blocked"))?;
    std::fs::create_dir(fixture.root.join("decoy ' $;\n wit"))?;
    let mut blocked_input = input.clone();
    blocked_input.argument = "blocked".into();
    let mut workflow = workflow::Workflow::new(blocked_input)?;
    let blocked = loop {
        match workflow.next()? {
            workflow::Progress::Ready(_) => bail!("blocked selection became ready"),
            workflow::Progress::Observe(request) => match observe(&setup, probe::Request::Workflow(request), POISON, records) {
                Ok(decoded) => match decoded.reply {
                    probe::Reply::Workflow(reply) => workflow.supply(reply)?,
                    probe::Reply::Setup(_) => bail!("workflow probe decoded a setup reply"),
                },
                Err(error) => break error,
            },
        }
    };
    ensure!(blocked.to_string().contains("cannot inspect symlink"), "ENOTDIR classification missing: {blocked:#}");
    ensure!(fixture.shell_records()? == 14, "blocked run used unexpected probes: {}", fixture.shell_records()?);
    checks.push(json!({"check": "symlink ENOTDIR fails the runner-free loop closed inside Projects decode; hostile decoy names parsed without poisoning", "error": format!("{blocked:#}")}));

    // Non-UTF-8 registry content must fail the Registry decode, not parse.
    let bad = Fixture::new("selected-shell-badcfg", "bad-config.toml", b"\xff\xfe[scaffold]\ndefault = 'native'\n")?;
    let mut bad_setup = bad.setup.clone();
    let mut bad_records = Vec::new();
    let bad_error = drive_setup(
        &mut bad_setup,
        target::Options::builder().argument("anything".into()).default_scaffold("default".into()).build(),
        POISON,
        &mut bad_records,
    )
    .expect_err("non-UTF-8 registry became ready");
    ensure!(bad_error.to_string().contains("utf-8"), "non-UTF-8 registry failed without a decode reason: {bad_error:#}");
    checks.push(json!({"check": "non-UTF-8 registry text fails the Registry decode inside the loop", "error": format!("{bad_error:#}")}));
    for record in bad_records.drain(..) {
        records.push(record);
    }
    Ok(())
}
