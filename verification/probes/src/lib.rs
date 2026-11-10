//! Independent runner-free public protocol witness. Controlled stdlib process
//! carriers execute probes, never the exported action. Not authenticated SSH.
#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tta_command::{Expression, Handoff, Word};
use tta_launcher::{
    entry::{
        EntryAttempt,
        action::{EntryDecision, ExportedAction},
        context, probe,
        protocol::*,
        registry,
    },
    target,
};

fn quote(value: impl Into<OsString>) -> Result<String> {
    Ok(Word::new(value)?.shell()?)
}
fn executable(path: &Path, source: &str) -> Result<()> {
    std::fs::write(path, source)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}
const POISON: &[(&str, &str)] = &[
    ("HOME", "/local/wrong/home"),
    ("TTA_ROOT", "/local/wrong/root"),
    ("TTA_ROOTS", "not-json"),
    ("TTA_CONFIG", "/local/wrong/config"),
    ("TTA_TMUX", "/local/wrong/tmux"),
    ("TTA_SOCKET_PATH", "/local/wrong/socket"),
    ("TMUX", "/local/wrong/socket,1,0"),
    ("AUTHOR", "Local Wrong Author"),
    ("LICENSE", "Local-Wrong-License"),
];
const REGISTRY: &[u8] = b"[scaffold.generators.native]\nadapter='rekon-native'\n[scaffold.generators.native.options]\ninstall=false\n[scaffold.generators.native.options.remotes]\norigin=''\ntngl=''\nvoodoo=''\n";

struct Fixture {
    directory: tempfile::TempDir,
    setup: target::TargetSetup,
    root: PathBuf,
    invocation: PathBuf,
    tmux: PathBuf,
    log: PathBuf,
}
impl Fixture {
    fn new(bytes: &[u8]) -> Result<Self> {
        let root = std::env::var_os("TTA_TEST_TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let directory = tempfile::Builder::new().prefix("tta-runner-free-").tempdir_in(root)?;
        let base = directory.path();
        let root = base.join("projects ' #;");
        let invocation = base.join("invocation");
        let home = base.join("home");
        for path in [&root, &invocation, &home.join(".config"), &home.join("tta")] {
            std::fs::create_dir_all(path)?;
        }
        std::fs::write(
            home.join(".gitconfig"),
            "[user]\n name = Witness Git Author\n email = witness@example.invalid\n",
        )?;
        std::os::unix::fs::symlink("home/.config", base.join("cfg-link"))?;
        let registry = base.join("cfg-link/../tta/config.toml");
        std::fs::write(home.join("tta/config.toml"), bytes)?;
        let tmux = base.join("fake-tmux");
        executable(
            &tmux,
            "#!/bin/sh\nprev=; socket=\nfor arg do [ \"$prev\" != -S ] || socket=$arg; prev=$arg; done\nprintf 'no server running on %s\\n' \"$socket\" >&2\nexit 1\n",
        )?;
        let shell = base.join("selected-shell");
        let log = base.join("shell-argv");
        executable(
            &shell,
            &format!(
                "#!/bin/sh\nprintf '%s\\000' \"$@\" >> {log}\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != -c ]; do shift; done\n[ \"$#\" -eq 2 ] || exit 91\nshift\nprintf 'startup chatter\\n'\nprintf 'startup diagnostic\\n' >&2\nexport HOME={home} TTA_ROOT={root} TTA_SOCKET_PATH={socket} TTA_CONFIG={registry} TTA_TMUX={tmux} LICENSE=Apache-2.0\nunset TTA_ROOTS TMUX TMUX_PANE TTA_SOCKET_NAME TTA_SCAFFOLD TTA_SATELLITE_PREFIX AUTHOR XDG_CONFIG_HOME\ncd -P {invocation} || exit $?\nexec /bin/sh -c \"$1\"\n",
                log = quote(&log)?,
                home = quote(&home)?,
                root = quote(&root)?,
                socket = quote(base.join("socket").into_os_string())?,
                registry = quote(registry.into_os_string())?,
                tmux = quote(&tmux)?,
                invocation = quote(&invocation)?
            ),
        )?;
        let setup = target::TargetSetup::builder()
            .shell(shell.to_str().context("UTF-8 fixture")?.into())
            .login(true)
            .interactive(true)
            .build();
        Ok(Self {
            directory,
            setup,
            root,
            invocation,
            tmux,
            log,
        })
    }
}

/// Only finite controlled fixture programs use this stdlib carrier. Readers
/// reach actual EOF; no deadline/cap is promised (arbitrary hosts use runner).
fn observe(request: &ReadRequest, step: Option<registry::Step>, records: &mut Vec<Value>) -> Result<probe::Reply> {
    let mut nonce = [0; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
    let token = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let probe = match step.clone() {
        Some(step) => probe::Probe::registry(request.clone(), step, token)?,
        None => probe::Probe::new(request.clone(), token)?,
    };
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", &probe.shell()?])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in POISON {
        command.env(name, value);
    }
    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut b = Vec::new();
        stdout.read_to_end(&mut b).map(|_| b)
    });
    let err = std::thread::spawn(move || {
        let mut b = Vec::new();
        stderr.read_to_end(&mut b).map(|_| b)
    });
    let stdout = out.join().unwrap()?;
    let stderr = err.join().unwrap()?;
    let status = child.wait()?.code();
    let decoded = probe.decode(tta_launcher::probe::Output {
        stdout: &stdout,
        stderr: &stderr,
        status,
        complete: status.is_some(),
    });
    records.push(json!({"request": format!("{request:?}"), "step": format!("{step:?}"), "carrier_status": status, "complete": status.is_some(), "stdout_bytes": stdout.len(), "stderr_bytes": stderr.len(), "decoded": decoded.as_ref().map(|d| json!({"reply": format!("{:?}", d.reply), "command_status": d.diagnostics.command_status, "startup_stdout_bytes": d.diagnostics.startup_stdout.len(), "transport_stderr_bytes": d.diagnostics.transport_stderr.len()})).map_err(|e| format!("{e:#}"))}));
    Ok(decoded?.reply)
}

#[derive(Debug)]
struct ObservationCause(anyhow::Error);
impl std::fmt::Display for ObservationCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.0)
    }
}
impl std::error::Error for ObservationCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

fn drive(fixture: &Fixture, selector: &str, records: &mut Vec<Value>) -> Result<ExportedAction> {
    let mut attempt = EntryAttempt::acquire(context::Acquisition {
        scope: context::Scope {
            origin: context::Origin::Ssh {
                host: "runner-free-target".into(),
                setup: fixture.setup.clone(),
            },
            cwd: None,
        },
        overrides: context::Overrides::default(),
        intent: EntryIntent::selector(selector),
        default_scaffold: "default".into(),
    })?;
    loop {
        match attempt.waiting() {
            Waiting::Read(request) => {
                let request = request.clone();
                let result = (|| -> Result<Observation> {
                    if let ReadKind::Registry { path } = &request.kind {
                        let mut task = registry::Task::new(path.clone())?;
                        while let Some(step) = task.step().cloned() {
                            let probe::Reply::Registry(reply) = observe(&request, Some(step.clone()), records)? else {
                                bail!("wrong registry decode")
                            };
                            task.accept(&step, reply)?;
                        }
                        Ok(Observation::Registry(task.result().context("registry unfinished")?.clone()))
                    } else {
                        let probe::Reply::Observation(observation) = observe(&request, None, records)? else {
                            bail!("wrong observation decode")
                        };
                        Ok(observation)
                    }
                })()
                .map_err(|e| ObservationFailure {
                    source: Box::new(ObservationCause(e)),
                });
                attempt.advance(Event::Observed(ReadCompletion { request, result }))?;
            }
            Waiting::Planned(plan) => {
                ensure!(
                    plan.target().cwd() == fixture.invocation && plan.target().program() == fixture.tmux,
                    "local target values leaked"
                );
                ensure!(
                    matches!(plan.decision(), EntryDecision::ScaffoldAndStart { destination, .. } if destination.path == fixture.root.join(selector)),
                    "wrong destination"
                );
                attempt.advance(Event::Deliver(Delivery::Export))?;
            }
            Waiting::Finished(AttemptOutcome::Exported(action)) => return Ok(*action.clone()),
            Waiting::Finished(AttemptOutcome::Stopped { cause, .. }) => return Err(anyhow::Error::new(cause.clone())),
            _ => bail!("unexpected attempt boundary: {}", attempt.snapshot().leaf),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub kind: &'static str,
    pub success: bool,
    pub failure: Option<String>,
    pub action_executed: bool,
    pub tools: Vec<Value>,
    pub checks: Vec<Value>,
    pub observations: Vec<Value>,
}
pub fn exercise() -> Report {
    let mut report = Report {
        kind: "runner-free retained actor and emitted probe round-trip; process carrier, not SSH",
        success: false,
        failure: None,
        action_executed: false,
        tools: vec![],
        checks: vec![],
        observations: vec![],
    };
    let result = (|| -> Result<()> {
        for program in ["/usr/bin/find", "/usr/bin/stat", "/usr/bin/git"] {
            let out = Command::new(program).arg("--version").output()?;
            ensure!(out.status.success(), "required tool {program}");
            report.tools.push(json!({"program": program, "stdout": String::from_utf8_lossy(&out.stdout)}));
        }
        let fixture = Fixture::new(REGISTRY)?;
        let action = drive(&fixture, "witness-create", &mut report.observations)?;
        report
            .checks
            .push(json!({"check": "one retained actor owns context, registry through symlink/interior '..', selection and fresh binding"}));
        let argv = std::fs::read(&fixture.log)?;
        let words: Vec<_> = argv.split(|b| *b == 0).filter(|b| !b.is_empty()).collect();
        ensure!(
            words.len() == report.observations.len() * 4 && words.chunks(4).all(|w| w[0] == b"-l" && w[1] == b"-i" && w[2] == b"-c"),
            "selected shell not applied once per probe"
        );
        report
            .checks
            .push(json!({"check": "each probe executes selected login/interactive shell exactly once", "probe_count": report.observations.len()}));
        let decoded = &report.observations[0]["decoded"]["Ok"];
        ensure!(
            decoded["startup_stdout_bytes"] == 16 && decoded["transport_stderr_bytes"] == 19,
            "startup channels lost"
        );
        report
            .checks
            .push(json!({"check": "startup stdout/stderr stay distinct from framed payload and command diagnostics"}));
        let steps = action.expression().steps();
        let writes: Vec<_> = steps.iter().filter_map(|s| if let Expression::Write(w) = s { Some(w) } else { None }).collect();
        ensure!(writes.len() == 1, "generator materialization duplicated");
        let package: Value = serde_json::from_str(writes[0].contents.as_os_str().to_str().unwrap())?;
        ensure!(
            package["author"] == "Witness Git Author" && package["license"] == "Apache-2.0",
            "local defaults leaked"
        );
        ensure!(!fixture.root.join("witness-create").exists(), "export mutated project");
        report
            .checks
            .push(json!({"check": "target physical destination and actual Git/license defaults materialize once; export remains observation-only"}));
        let context::Origin::Ssh { setup, .. } = &action.action().binding_requests()[0].scope.origin else {
            unreachable!()
        };
        ensure!(
            setup.shell_command(&action.expression().shell(Handoff::Exec)?)?.contains("new-session"),
            "frozen handoff lost action"
        );
        report
            .checks
            .push(json!({"check": "bound target setup constructs one frozen handoff, without execution"}));
        std::fs::write(fixture.root.join("blocker"), "x")?;
        std::os::unix::fs::symlink("blocker/child", fixture.root.join("blocked"))?;
        std::fs::create_dir(fixture.root.join("decoy ' $;\n wit"))?;
        let error = drive(&fixture, "blocked", &mut report.observations).unwrap_err();
        ensure!(format!("{error:#}").contains("cannot inspect symlink"), "ENOTDIR hidden: {error:#}");
        report
            .checks
            .push(json!({"check": "symlink ENOTDIR stops actor before scaffold, not absence", "error": format!("{error:#}")}));
        let bad = Fixture::new(b"\xff\xfe[scaffold]\n")?;
        let error = drive(&bad, "anything", &mut report.observations).unwrap_err();
        ensure!(format!("{error:#}").contains("utf-8"), "registry decode cause lost: {error:#}");
        report
            .checks
            .push(json!({"check": "non-UTF-8 registry fails the bounded acquisition before session selection", "error": format!("{error:#}")}));
        ensure!(fixture.directory.path().exists(), "fixture unexpectedly gone");
        Ok(())
    })();
    report.success = result.is_ok();
    report.failure = result.err().map(|e| format!("{e:#}"));
    report
}

#[cfg(test)]
mod tests {
    #[test]
    fn public_core_reaches_freshly_bound_export_without_runner() {
        let report = super::exercise();
        assert!(report.success, "{report:#?}");
        assert_eq!(report.checks.len(), 7);
        assert!(!report.action_executed);
        for cause in ["cannot inspect symlink", "utf-8"] {
            assert!(
                report
                    .observations
                    .iter()
                    .any(|r| r["decoded"]["Err"].as_str().is_some_and(|e| e.contains(cause)))
            );
        }
    }
}
