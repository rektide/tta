//! Opt-in actual OpenSSH exercise. Private loopback sshd, private keys/config,
//! explicit owned tmux socket, selected target setup, and independent queries.
use std::ffi::OsString;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use portable_pty::CommandBuilder;
use serde::Serialize;
use serde_json::{Value, json};
use tta_command::{Expression, Handoff, Word};
use tta_launcher::{Selection, target};
use tta_runner::{capture, ssh as runner};

use super::{ProcessOutput, Terminal, isolation};

pub struct Config {
    pub tta: PathBuf,
    pub consumer: Option<PathBuf>,
    pub timeout: Duration,
}

#[derive(Default, Serialize)]
pub struct Report {
    pub kind: String,
    pub success: bool,
    pub failure: Option<String>,
    pub fixture: Value,
    pub tools: Vec<Value>,
    pub checks: Vec<Value>,
    pub observations: Vec<Value>,
    pub terminals: Vec<Value>,
    pub cleanup: Vec<Value>,
    pub sshd_log: String,
}

fn quote(value: impl Into<OsString>) -> Result<String> {
    Ok(Word::new(value)?.shell()?)
}

fn checked(command: &mut Command, timeout: Duration) -> Result<ProcessOutput> {
    let output: ProcessOutput = capture::run(command, capture::Limits { timeout, ..Default::default() })?.into();
    ensure!(output.success, "command {command:?}: {output:?}");
    Ok(output)
}

const NO_TTA: &str =
    "command -v tta; tta_status=$?; printf '\\ncommand-v-tta-status=%s\\n' \"$tta_status\"; [ \"$tta_status\" -eq 1 ] || [ \"$tta_status\" -eq 127 ]";

fn verify_no_tta(output: &ProcessOutput) -> Result<()> {
    // Require exactly the controlled selected-shell startup line, then EMPTY
    // command stdout and a shell-appropriate absence status. Extra output,
    // command-not-found errors from setup, and incomplete capture cannot pass.
    ensure!(
        output.success
            && output.complete
            && matches!(
                output.stdout.strip_prefix("selected target startup\n"),
                Some("\ncommand-v-tta-status=1\n" | "\ncommand-v-tta-status=127\n")
            ),
        "target tta absence not proved after selected shell setup: {output:?}"
    );
    Ok(())
}

fn executable(path: &Path, source: &str) -> Result<()> {
    std::fs::write(path, source)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

struct Fixture {
    directory: tempfile::TempDir,
    ssh: runner::Ssh,
    setup: target::TargetSetup,
    socket: PathBuf,
    root: PathBuf,
    invocation: PathBuf,
    bin: PathBuf,
    local_bin: PathBuf,
    log: PathBuf,
    daemon: Option<Child>,
    proved: bool,
    cleanup_attempted: bool,
    timeout: Duration,
}

impl Fixture {
    fn new(timeout: Duration) -> Result<Self> {
        ensure!(!timeout.is_zero(), "positive timeout required");
        let directory = tempfile::Builder::new().prefix("tta-ssh-").tempdir_in(super::isolation::scratch_root())?;
        let base = directory.path();
        let socket = base.join("tmux.sock");
        let root = base.join("target projects ' #;");
        let invocation = base.join("target invocation");
        let bin = base.join("target-bin");
        let local_bin = base.join("local-bin");
        for path in [&root, &invocation, &bin, &local_bin, &base.join("home/.config/jj"), &base.join("runtime")] {
            std::fs::create_dir_all(path)?;
        }
        let log = base.join("sshd.log");
        let client_config = base.join("ssh_config");
        let shell = base.join("selected-target-shell");
        let ssh = runner::Ssh::builder()
            .program("/usr/bin/ssh".into())
            .destination("tta-owned".into())
            .options(vec!["-F".into(), client_config.as_os_str().into()])
            .build();
        let setup = target::TargetSetup::builder()
            .shell(shell.to_str().context("UTF-8 fixture path")?.into())
            .login(true)
            .interactive(true)
            .build();
        let mut fixture = Self {
            directory,
            ssh,
            setup,
            socket,
            root,
            invocation,
            bin,
            local_bin,
            log,
            daemon: None,
            proved: false,
            cleanup_attempted: false,
            timeout,
        };
        fixture.configure(&client_config, &shell)?;
        Ok(fixture)
    }

    fn configure(&mut self, client_config: &Path, shell: &Path) -> Result<()> {
        let base = self.directory.path();
        let host_key = base.join("host-key");
        let client_key = base.join("client-key");
        for path in [&host_key, &client_key] {
            checked(
                Command::new("/usr/bin/ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-f"]).arg(path),
                self.timeout,
            )?;
        }
        let authorized = base.join("authorized_keys");
        std::fs::copy(client_key.with_extension("pub"), &authorized)?;
        std::fs::set_permissions(&authorized, std::fs::Permissions::from_mode(0o600))?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let user = checked(Command::new("id").arg("-un"), self.timeout)?.stdout.trim().to_owned();
        ensure!(!user.is_empty() && !user.chars().any(char::is_whitespace), "invalid fixture username");
        let config = base.join("sshd_config");
        std::fs::write(
            &config,
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nStrictModes no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPubkeyAuthentication yes\nAuthenticationMethods publickey\nAllowUsers {user}\nPermitRootLogin no\nUseDNS no\nPrintMotd no\nPrintLastLog no\nLogLevel VERBOSE\n",
                host_key.display(),
                base.join("sshd.pid").display(),
                authorized.display()
            ),
        )?;
        let public = std::fs::read_to_string(host_key.with_extension("pub"))?;
        let public: Vec<_> = public.split_whitespace().collect();
        ensure!(public.len() >= 2, "malformed generated public key");
        let known_hosts = base.join("known_hosts");
        std::fs::write(&known_hosts, format!("[127.0.0.1]:{port} {} {}\n", public[0], public[1]))?;
        std::fs::write(
            client_config,
            format!(
                "Host tta-owned\n HostName 127.0.0.1\n Port {port}\n User {user}\n IdentityFile {}\n IdentitiesOnly yes\n UserKnownHostsFile {}\n GlobalKnownHostsFile /dev/null\n StrictHostKeyChecking yes\n BatchMode yes\n ControlMaster no\n ConnectTimeout 3\n LogLevel ERROR\n",
                client_key.display(),
                known_hosts.display()
            ),
        )?;
        let home = base.join("home");
        std::fs::write(home.join(".gitconfig"), "[user]\n name = Target Git Author\n email = target@example.invalid\n")?;
        std::fs::write(
            home.join(".config/jj/config.toml"),
            "[user]\nname = 'Target JJ Author'\nemail = 'target@example.invalid'\n",
        )?;
        std::fs::write(
            base.join("registry.toml"),
            "[scaffold]\ndefault = 'native'\n[scaffold.generators.native]\nadapter = 'rekon-native'\n[scaffold.generators.native.options]\ninstall = false\n[scaffold.generators.native.options.remotes]\norigin = ''\ntngl = ''\nvoodoo = ''\n",
        )?;
        // Only this one tool is admitted from /usr/local/bin. tta in that
        // directory is deliberately unreachable in the post-startup PATH.
        symlink("/usr/local/bin/jj", self.bin.join("jj"))?;
        executable(
            &self.bin.join("tmux"),
            &format!(
                "#!/bin/sh\ntta_seen=no; tta_next=no\nfor tta_arg do\n if [ \"$tta_next\" = yes ]; then\n  [ \"$tta_arg\" = {socket} ] || {{ printf 'fixture refused another endpoint\\n' >&2; exit 92; }}\n  tta_seen=yes; tta_next=no\n else\n  case \"$tta_arg\" in -S) tta_next=yes ;; -L) printf 'fixture refused label endpoint\\n' >&2; exit 93 ;; esac\n fi\ndone\n[ \"$tta_seen\" = yes ] && [ \"$tta_next\" = no ] || {{ printf 'fixture requires its exact -S endpoint\\n' >&2; exit 94; }}\nprintf '%s\\000' TTA-CALL-v1 \"$@\" TTA-END-v1 >> {log}\nexec /usr/bin/tmux \"$@\"\n",
                socket = quote(self.socket.clone().into_os_string())?,
                log = quote(base.join("tmux-argv").into_os_string())?
            ),
        )?;
        executable(
            shell,
            &format!(
                "#!/bin/sh\nprintf '%s\\000' \"$@\" >> {shell_log}\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != -c ]; do shift; done\n[ \"$#\" -eq 2 ] || exit 91\nshift\nprintf 'selected target startup\\n'\nprintf 'selected target diagnostic\\n' >&2\nexport HOME={home} XDG_CONFIG_HOME={config} PATH={path}\nexport TTA_ROOT={root} TTA_SOCKET_PATH={socket} TTA_CONFIG={registry} TTA_TMUX=tmux TTA_TMUX_CONFIG=/dev/null LICENSE=Apache-2.0\nunset AUTHOR TMUX TMUX_PANE TTA_SOCKET_NAME\ncd -P {invocation} || exit $?\nexec /bin/sh -c \"$1\"\n",
                shell_log = quote(base.join("shell-argv").into_os_string())?,
                home = quote(home.clone().into_os_string())?,
                config = quote(home.join(".config").into_os_string())?,
                path = quote(format!("{}:/usr/bin:/bin", self.bin.display()))?,
                root = quote(self.root.clone().into_os_string())?,
                socket = quote(self.socket.clone().into_os_string())?,
                registry = quote(base.join("registry.toml").into_os_string())?,
                invocation = quote(self.invocation.clone().into_os_string())?
            ),
        )?;
        executable(
            &self.local_bin.join("ssh"),
            &format!("#!/bin/sh\nexec /usr/bin/ssh -F {} \"$@\"\n", quote(client_config.as_os_str())?),
        )?;
        checked(Command::new("/usr/sbin/sshd").args(["-t", "-f"]).arg(&config), self.timeout)?;
        drop(listener); // The only race is an owned loopback ephemeral port.
        let log = std::fs::File::create(&self.log)?;
        self.daemon = Some(
            Command::new("/usr/sbin/sshd")
                .args(["-D", "-e", "-f"])
                .arg(&config)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()?,
        );
        let start = Instant::now();
        loop {
            if let Some(status) = self.daemon.as_mut().unwrap().try_wait()? {
                bail!("owned sshd exited {status}: {}", super::read_text(&self.log)?);
            }
            if TcpStream::connect_timeout(&SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into(), Duration::from_millis(50)).is_ok() {
                break;
            }
            ensure!(start.elapsed() < self.timeout, "owned sshd startup timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    fn remote(&self, source: &str) -> Result<ProcessOutput> {
        let command = self.ssh.command(source, false)?;
        Ok(capture::run(
            &mut tta_runner::command::process(&command),
            capture::Limits {
                timeout: self.timeout,
                ..Default::default()
            },
        )?
        .into())
    }

    fn tmux(&self, args: &[&str]) -> Result<ProcessOutput> {
        let mut words = vec![
            "/usr/bin/tmux".to_owned(),
            "-u".into(),
            "-f".into(),
            "/dev/null".into(),
            "-S".into(),
            self.socket.to_str().context("UTF-8 socket")?.into(),
        ];
        words.extend(args.iter().map(|arg| (*arg).into()));
        let source = words.into_iter().map(quote).collect::<Result<Vec<_>>>()?.join(" ");
        self.remote(&format!("LC_ALL=C TMUX= TMUX_PANE= {source}"))
    }

    fn prepare(&self, options: target::Options, report: &mut Report) -> Result<runner::PreparedRemote> {
        self.prepare_with(self.setup.clone(), options, report)
    }

    fn prepare_with(&self, setup: target::TargetSetup, options: target::Options, report: &mut Report) -> Result<runner::PreparedRemote> {
        match runner::prepare(
            self.ssh.clone(),
            setup,
            options,
            capture::Limits {
                timeout: self.timeout,
                ..Default::default()
            },
        ) {
            Ok(prepared) => {
                record_observations(&prepared.observations, report);
                Ok(prepared)
            }
            Err(error) => {
                record_observations(&error.observations, report);
                Err(error.into())
            }
        }
    }

    fn command_builder(&self, command: &tta_command::Command) -> CommandBuilder {
        let mut builder = CommandBuilder::new(command.program.as_os_str());
        builder.args(command.args.iter().map(|arg| arg.as_os_str()));
        self.local_environment(&mut builder);
        builder
    }

    fn local_environment(&self, builder: &mut CommandBuilder) {
        builder.cwd(self.directory.path());
        builder.env("TERM", "xterm-256color");
        builder.env("PATH", format!("{}:/usr/bin:/bin", self.local_bin.display()));
        builder.env("HOME", self.directory.path().join("local-wrong-home"));
        builder.env("XDG_CONFIG_HOME", self.directory.path().join("local-wrong-config"));
        builder.env("XDG_RUNTIME_DIR", self.directory.path().join("runtime"));
        builder.env("TTA_ROOT", "/local/wrong/projects");
        builder.env("TTA_CONFIG", "/local/wrong/config.toml");
        builder.env("TTA_TMUX", "/local/wrong/tmux");
        builder.env("TTA_SOCKET_PATH", "/local/wrong/socket");
        builder.env("TMUX", "/local/wrong/socket,1,0");
        builder.env("AUTHOR", "Local Wrong Author");
        builder.env("LICENSE", "Local-Wrong-License");
    }

    fn inventory(&self) -> Result<String> {
        let output = self.tmux(&["list-sessions", "-F", "#{session_name}\t#{session_attached}"])?;
        ensure!(output.success, "independent inventory failed: {output:?}");
        Ok(output.stdout)
    }

    fn attached(&self, name: &str) -> Result<String> {
        let start = Instant::now();
        loop {
            let output = self.tmux(&["list-sessions", "-F", "#{session_name}\t#{session_attached}"])?;
            if output.success && output.stdout.lines().any(|line| line == format!("{name}\t1")) {
                return Ok(output.stdout);
            }
            ensure!(start.elapsed() < self.timeout, "session {name:?} never attached: {output:?}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn terminal(&self, command: CommandBuilder, name: &str, report: &mut Report, consumer: bool, loss: bool) -> Result<()> {
        let mut terminal = Terminal::spawn(command)?;
        let mut run = || -> Result<Value> {
            let state = self.attached(name)?;
            let group = if let Some(base) = name.strip_prefix('_').and_then(|value| value.strip_suffix("-1")) {
                let primary = self.tmux(&["display-message", "-p", "-t", &format!("={base}"), "#{window_id}"])?;
                let satellite = self.tmux(&["display-message", "-p", "-t", &format!("={name}"), "#{window_id}"])?;
                let keep = self.tmux(&["show-options", "-v", "-t", &format!("={name}:"), "destroy-unattached"])?;
                ensure!(
                    primary.success && satellite.success && primary.stdout == satellite.stdout,
                    "group did not share its window"
                );
                ensure!(keep.success && keep.stdout == "keep-last\n", "group did not retain keep-last: {keep:?}");
                Some(json!({"primary": primary, "satellite": satellite, "keep_last": keep}))
            } else {
                None
            };
            let mut retry = None;
            if loss {
                terminal.stop(self.timeout)?;
            } else {
                terminal.writer.write_all(b"\x02d")?;
                terminal.writer.flush()?;
                if consumer {
                    wait_text(&terminal, "space=retry q=quit", self.timeout)?;
                    // The user, not the library, authorizes a fresh attempt.
                    terminal.writer.write_all(b" ")?;
                    terminal.writer.flush()?;
                    retry = Some(self.attached(&format!("_{name}-1"))?);
                    terminal.writer.write_all(b"\x02d")?;
                    terminal.writer.flush()?;
                    wait_text_count(&terminal, "space=retry q=quit", 2, self.timeout)?;
                    terminal.writer.write_all(b"q")?;
                    terminal.writer.flush()?;
                }
                terminal.wait(self.timeout)?;
                ensure!(terminal.exit == Some(0), "terminal failed: {:?}", terminal.exit);
            }
            Ok(json!({"attached_inventory": state, "group": group, "user_authorized_retry_inventory": retry, "transport_loss_injected": loss}))
        };
        let result = run();
        let stopped = terminal.stop(self.timeout);
        let capture = terminal.capture.lock().unwrap();
        report.terminals.push(json!({"name": name, "consumer": consumer, "exit": terminal.exit, "transcript": capture.text(), "omitted_bytes": capture.omitted, "evidence": result.as_ref().ok(), "failure": result.as_ref().err().map(|e| format!("{e:#}")), "cleanup_failure": stopped.as_ref().err().map(|e| format!("{e:#}"))}));
        result?;
        stopped?;
        ensure!(capture.omitted == 0, "PTY transcript was capped");
        Ok(())
    }

    fn cleanup(&mut self, report: &mut Report) -> Result<()> {
        self.cleanup_attempted = true;
        let mut result = Ok(());
        if self.proved {
            if self.socket.exists() {
                match self.tmux(&["kill-server"]) {
                    Ok(output) => {
                        if !output.success {
                            result = Err(anyhow::anyhow!("owned tmux cleanup failed: {output:?}"));
                        }
                        report
                            .cleanup
                            .push(json!({"operation": "kill exclusively owned server", "socket": self.socket, "output": output}));
                    }
                    Err(error) => result = Err(error),
                }
            }
            match self.tmux(&["list-sessions"]) {
                Ok(output) => {
                    if let Err(error) = isolation::verify_absence(&self.socket, &output) {
                        result = Err(error);
                    }
                    report.cleanup.push(json!({"operation": "post-cleanup exact absence", "output": output}));
                }
                Err(error) => result = Err(error),
            }
        }
        if let Some(mut daemon) = self.daemon.take() {
            daemon.kill()?;
            let start = Instant::now();
            loop {
                if let Some(status) = daemon.try_wait()? {
                    report
                        .cleanup
                        .push(json!({"operation": "stop owned sshd", "pid": daemon.id(), "status": status.to_string()}));
                    break;
                }
                ensure!(start.elapsed() < self.timeout, "owned sshd termination timed out");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        match super::read_text(&self.log) {
            Ok(log) => report.sshd_log = log,
            Err(error) => {
                report.cleanup.push(json!({"operation": "read bounded sshd log", "failure": error.to_string()}));
                result = Err(error.into());
            }
        }
        result
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            let mut report = Report::default();
            let result = self.cleanup(&mut report);
            eprintln!(
                "owned SSH fixture fallback cleanup: {result:?}; evidence={}",
                serde_json::to_string(&report.cleanup).unwrap_or_default()
            );
        }
    }
}

fn record_observations(records: &[runner::Record], report: &mut Report) {
    for record in records {
        let decoded = record
            .decoded
            .as_ref()
            .map(|value| {
                json!({
                    "reply": format!("{:?}", value.reply), "command_status": value.diagnostics.command_status,
                    "command_stderr": String::from_utf8_lossy(&value.diagnostics.command_stderr),
                    "startup_stdout": String::from_utf8_lossy(&value.diagnostics.startup_stdout),
                    "transport_stderr": String::from_utf8_lossy(&value.diagnostics.transport_stderr),
                })
            })
            .map_err(Clone::clone);
        report.observations.push(json!({"request": format!("{:?}", record.request), "status": record.output.status.map(|s| s.to_string()), "complete": record.output.complete(), "stop": format!("{:?}", record.output.stop), "stdout": String::from_utf8_lossy(&record.output.stdout), "stderr": String::from_utf8_lossy(&record.output.stderr), "decoded": decoded}));
    }
}

fn wait_text(terminal: &Terminal, text: &str, timeout: Duration) -> Result<()> {
    wait_text_count(terminal, text, 1, timeout)
}

fn wait_text_count(terminal: &Terminal, text: &str, count: usize, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        let capture = terminal.capture.lock().unwrap();
        if capture.text().matches(text).count() >= count {
            return Ok(());
        }
        ensure!(start.elapsed() < timeout, "PTY never produced {text:?}: {}", capture.text());
        drop(capture);
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Missing prerequisites produce a failing report, never a successful skip.
pub fn exercise(config: &Config) -> Report {
    let mut report = Report {
        kind: "actual OpenSSH loopback; no target tta; independent target queries".into(),
        ..Default::default()
    };
    let mut fixture = match Fixture::new(config.timeout) {
        Ok(fixture) => fixture,
        Err(error) => {
            report.failure = Some(format!("create owned SSH fixture: {error:#}"));
            return report;
        }
    };
    report.fixture = json!({"root": fixture.directory.path(), "socket": fixture.socket, "destination": fixture.ssh.destination, "ssh_options": fixture.ssh.options, "target_setup": format!("{:?}", fixture.setup)});
    if let Err(error) = run(config, &mut fixture, &mut report) {
        report.failure = Some(format!("{error:#}"));
    }
    if let Err(error) = fixture.cleanup(&mut report) {
        let previous = report.failure.take().unwrap_or_default();
        report.failure = Some(format!("{previous}\ncleanup: {error:#}"));
    }
    report.success = report.failure.is_none();
    report
}

fn run(config: &Config, fixture: &mut Fixture, report: &mut Report) -> Result<()> {
    for (program, args) in [
        ("/usr/bin/ssh", vec!["-V"]),
        ("/usr/sbin/sshd", vec!["-V"]),
        ("/usr/bin/tmux", vec!["-V"]),
        ("/usr/bin/find", vec!["--version"]),
        ("/usr/bin/stat", vec!["--version"]),
    ] {
        let output = checked(Command::new(program).args(args), fixture.timeout)?;
        report.tools.push(json!({"program": program, "output": output}));
    }
    let proof = fixture.tmux(&["list-sessions"])?;
    isolation::verify_absence(&fixture.socket, &proof)?;
    fixture.proved = true;
    report
        .checks
        .push(json!({"check": "exact owned endpoint absence before any mutation", "output": proof}));
    let no_tta = fixture.remote(&fixture.setup.shell_command(NO_TTA)?)?;
    verify_no_tta(&no_tta)?;
    report
        .checks
        .push(json!({"check": "no target tta after selected shell setup and restricted PATH", "output": no_tta}));

    let name = "cold' $; 雪";
    std::fs::create_dir(fixture.root.join(name))?;
    let initial = fixture.invocation.join("initial ' $; directory");
    std::fs::create_dir(&initial)?;
    let argv_file = fixture.directory.path().join("startup-argv");
    let recorder = fixture.bin.join("startup-recorder");
    executable(
        &recorder,
        &format!(
            "#!/bin/sh\nprintf '%s\\000' \"$PWD\" \"$@\" > {}\nexec /usr/bin/sleep 180\n",
            quote(argv_file.clone().into_os_string())?
        ),
    )?;
    let startup: Vec<OsString> = [recorder.as_os_str().to_owned(), "arg with spaces".into(), "quote' $; # 雪".into(), "".into()].into();
    let prepared = fixture.prepare(
        target::Options::builder()
            .argument(name.into())
            .default_scaffold("default".into())
            .directory(PathBuf::from("initial ' $; directory"))
            .startup(startup)
            .build(),
        report,
    )?;
    ensure!(matches!(prepared.launch.selection, Selection::Project { .. }), "cold project selection failed");
    ensure!(
        prepared.facts.cwd == fixture.invocation && prepared.facts.value("HOME") == fixture.directory.path().join("home").to_str(),
        "target context disagreement lost"
    );
    fixture.terminal(fixture.command_builder(&prepared.command()?), name, report, false, false)?;
    let argv = fixture.remote(&format!("cat -- {}", quote(argv_file.into_os_string())?))?;
    ensure!(argv.success, "startup artifact: {argv:?}");
    let expected = format!("{}\0arg with spaces\0quote' $; # 雪\0\0", initial.display());
    ensure!(argv.stdout == expected, "literal argv/cwd changed: {argv:?}");
    report.checks.push(json!({"check": "cold/project; target defaults; literal startup argv and relative target cwd", "selection": format!("{:?}", prepared.launch.selection), "argv": argv}));

    let grouped = fixture.prepare(
        target::Options::builder()
            .argument(name.into())
            .roots(vec!["/unusable/project/root".into()])
            .build(),
        report,
    )?;
    let leaves = grouped.launch.plan.expression().steps();
    ensure!(leaves.len() == 1, "grouped tmux queue split");
    let Expression::Command(command) = leaves[0] else {
        bail!("missing grouped command");
    };
    ensure!(
        command.args.iter().filter(|arg| arg.as_os_str() == ";").count() == 2,
        "grouped queue ordering missing"
    );
    let satellite = format!("_{name}-1");
    fixture.terminal(fixture.command_builder(&grouped.command()?), &satellite, report, false, false)?;
    let inventory = fixture.inventory()?;
    ensure!(
        !inventory.lines().any(|line| line.starts_with(&format!("{satellite}\t"))),
        "detached satellite was not removed"
    );
    report.checks.push(json!({"check": "existing/group; one create-attach-keep-last queue; detach lifecycle", "shell": grouped.launch.plan.expression().shell(Handoff::Run)?, "inventory_after_detach": inventory}));

    fixture.setup.environment.push(target::Environment {
        name: "TTA_SATELLITE_PREFIX".into(),
        value: Some("remote-".into()),
    });
    for (explicit, prefix) in [(None, "remote-"), (Some("chosen-"), "chosen-")] {
        let prepared = fixture.prepare(
            target::Options::builder()
                .argument(name.into())
                .maybe_satellite_prefix(explicit.map(str::to_owned))
                .build(),
            report,
        )?;
        let satellite = format!("{prefix}{name}-1");
        fixture.terminal(fixture.command_builder(&prepared.command()?), &satellite, report, false, false)?;
        report
            .checks
            .push(json!({"check": "SSH default independent entry honors target prefix and explicit override", "prefix": prefix, "explicit": explicit}));
    }
    fixture.setup.environment.pop();

    let generated = fixture.prepare(
        target::Options::builder()
            .argument("generated".into())
            .default_scaffold("default".into())
            .startup(vec!["/usr/bin/sleep".into(), "180".into()])
            .build(),
        report,
    )?;
    ensure!(
        matches!(generated.launch.selection, Selection::Create { .. }),
        "registry did not activate native scaffold"
    );
    ensure!(!fixture.root.join("generated").exists(), "planning mutated target files");
    fixture.terminal(fixture.command_builder(&generated.command()?), "generated", report, false, false)?;
    let package = fixture.remote(&format!("cat -- {}", quote(fixture.root.join("generated/package.json").into_os_string())?))?;
    ensure!(package.success, "generated package unavailable: {package:?}");
    let parsed: Value = serde_json::from_str(&package.stdout)?;
    ensure!(
        parsed["author"] == "Target Git Author" && parsed["license"] == "Apache-2.0",
        "local defaults leaked: {parsed}"
    );
    report
        .checks
        .push(json!({"check": "target registry scaffold and target Git/license defaults", "package": parsed}));

    // A failed native generator stops before any tmux command or handoff. The
    // wrapper counts exact invocations; no inference from trace wording.
    let failed_bin = fixture.directory.path().join("failed-bin");
    std::fs::create_dir(&failed_bin)?;
    let failures = fixture.directory.path().join("generator-attempts");
    executable(
        &failed_bin.join("jj"),
        &format!(
            "#!/bin/sh\nprintf 'attempt\\n' >> {}\nprintf 'injected generator failure\\n' >&2\nexit 43\n",
            quote(failures.clone().into_os_string())?
        ),
    )?;
    let mut setup = fixture.setup.clone();
    setup.environment.push(target::Environment {
        name: "PATH".into(),
        value: Some(format!("{}:{}:/usr/bin:/bin", failed_bin.display(), fixture.bin.display())),
    });
    let failed = fixture.prepare_with(
        setup,
        target::Options::builder()
            .argument("failed-generation".into())
            .default_scaffold("default".into())
            .build(),
        report,
    )?;
    let before = fixture.inventory()?;
    let target = failed.setup.launch(&failed.launch, Handoff::Exec)?;
    let output = fixture.remote(&target)?;
    ensure!(!output.success && output.status == Some(43), "failed generator was hidden: {output:?}");
    ensure!(fixture.inventory()? == before, "failed generator mutated tmux");
    ensure!(std::fs::read_to_string(failures)? == "attempt\n", "generator was blindly replayed");
    report
        .checks
        .push(json!({"check": "failed scaffold prevents tmux/handoff; no replay", "output": output}));

    std::fs::write(fixture.root.join("file-blocker"), "x")?;
    symlink("file-blocker/child", fixture.root.join("blocked-link"))?;
    let before = fixture.inventory()?;
    let blocked = fixture.prepare(
        target::Options::builder()
            .argument("blocked-link".into())
            .default_scaffold("default".into())
            .build(),
        report,
    );
    ensure!(blocked.is_err(), "a symlink target's ENOTDIR was treated as a missing project");
    ensure!(fixture.inventory()? == before, "symlink inspection error authorized mutation");
    report.checks.push(json!({"check": "real SSH GNU-stat distinction: symlink ENOTDIR stops before scaffold, not find-%Y-N absence", "error": format!("{:#}", blocked.unwrap_err())}));

    for (label, source) in [
        ("malformed", "printf 'not a tmax reply\\n'\n"),
        ("failed", "printf 'denied observation\\n' >&2\nexit 41\n"),
    ] {
        let program = fixture.bin.join(format!("{label}-tmux"));
        executable(&program, &format!("#!/bin/sh\n{source}"))?;
        let before = fixture.inventory()?;
        let result = fixture.prepare(
            target::Options::builder()
                .argument("must-not-create".into())
                .program(program.into_os_string())
                .build(),
            report,
        );
        ensure!(result.is_err(), "{label} observation was accepted");
        ensure!(
            fixture.inventory()? == before && !fixture.root.join("must-not-create").exists(),
            "failed read permitted mutation"
        );
        report
            .checks
            .push(json!({"check": format!("real SSH {label} typed probe; no mutation or fallback"), "error": format!("{:#}", result.unwrap_err())}));
    }

    let tta = std::path::absolute(&config.tta)?;
    let mut cli = Command::new(&tta);
    cli.arg("--ssh")
        .arg(&fixture.ssh.destination)
        .arg("--ssh-program")
        .arg("/usr/bin/ssh")
        .arg("--ssh-option=-F")
        .arg("--ssh-option")
        .arg(fixture.directory.path().join("ssh_config"))
        .arg("--target-shell")
        .arg(fixture.setup.shell.as_ref().unwrap())
        .arg("--target-interactive")
        .arg("--print-shell")
        .arg("--")
        .arg(name)
        .env("TTA_ROOT", "/local/wrong")
        .env("TTA_CONFIG", "/local/wrong")
        .env("TTA_TMUX", "/local/wrong")
        .env("TTA_SATELLITE_PREFIX", "local-wrong-")
        .env("TMUX", "/local/wrong,1,0");
    let emission = checked(&mut cli, fixture.timeout)?;
    ensure!(
        emission.stdout.starts_with("'/usr/bin/ssh' ") && !emission.stdout.contains("selected target startup"),
        "SSH print-shell stdout was not a local invocation"
    );
    let mut eval = CommandBuilder::new("/bin/sh");
    eval.args(["-c", emission.stdout.trim_end_matches('\n')]);
    fixture.local_environment(&mut eval);
    fixture.terminal(eval, &format!("_{name}-1"), report, false, false)?;
    report
        .checks
        .push(json!({"check": "CLI SSH print-shell discovers read-only and emits an evaluable LOCAL SSH invocation", "output": emission}));

    std::fs::create_dir(fixture.root.join("cli-direct"))?;
    let mut direct = CommandBuilder::new(&tta);
    direct.args(["--ssh", "tta-owned", "--ssh-program", "/usr/bin/ssh", "--ssh-option=-F", "--ssh-option"]);
    direct.arg(fixture.directory.path().join("ssh_config"));
    direct.arg("--target-shell");
    direct.arg(fixture.setup.shell.as_ref().unwrap());
    direct.args(["--target-interactive", "--", "cli-direct", "/usr/bin/sleep", "180"]);
    fixture.local_environment(&mut direct);
    fixture.terminal(direct, "cli-direct", report, false, false)?;
    report
        .checks
        .push(json!({"check": "CLI direct SSH cold project launch under contradictory local defaults"}));

    std::fs::create_dir(fixture.root.join("loss-project"))?;
    let lost = fixture.prepare(
        target::Options::builder()
            .argument("loss-project".into())
            .startup(vec!["/usr/bin/sleep".into(), "180".into()])
            .build(),
        report,
    )?;
    fixture.terminal(fixture.command_builder(&lost.command()?), "loss-project", report, false, true)?;
    ensure!(
        fixture.inventory()?.contains("loss-project\t0"),
        "target mutation did not survive transport loss"
    );
    report
        .checks
        .push(json!({"check": "actual SSH carrier loss after observed target creation; no automatic replay/rollback", "inventory": fixture.inventory()?}));

    if let Some(consumer) = &config.consumer {
        std::fs::create_dir(fixture.root.join("consumer-project"))?;
        let mut command = CommandBuilder::new(std::path::absolute(consumer)?);
        command.args([
            "--ssh",
            "tta-owned",
            "--tta",
            "consumer-project",
            "--session-backend",
            "tta-lib",
            "--no-ssh-master",
            "--shell",
        ]);
        command.arg(fixture.setup.shell.as_ref().unwrap());
        command.args(["--", "/usr/bin/sleep", "180"]);
        fixture.local_environment(&mut command);
        fixture.terminal(command, "consumer-project", report, true, false)?;
        report.checks.push(json!({"check": "term-preserver local embedded SSH attempts; real proxy PTYs; detach, user-authorized retry/fresh grouping, detach and quit; no installed target tta", "consumer": consumer, "managed_master": "disabled explicitly for exclusively owned fixture lifecycle"}));
    }
    let calls = std::fs::read(fixture.directory.path().join("tmux-argv"))?;
    let new_sessions = calls.split(|b| *b == 0).filter(|word| *word == b"new-session").count();
    let expected = if config.consumer.is_some() { 10 } else { 8 };
    ensure!(new_sessions == expected, "unexpected tmux creation replay: {new_sessions}, expected {expected}");
    report.checks.push(json!({"check": "recorded target tmux argv: exactly one mutation invocation per authorized attempt", "new_session_invocations": new_sessions, "argv_bytes": calls}));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_target_tta_accepts_posix_absence_only_after_startup_with_empty_output() {
        for status in [1, 127] {
            let absent = ProcessOutput {
                success: true,
                complete: true,
                stdout: format!("selected target startup\n\ncommand-v-tta-status={status}\n"),
                ..Default::default()
            };
            verify_no_tta(&absent).unwrap();
            for stdout in [
                format!("selected target startup\n/usr/local/bin/tta\n\ncommand-v-tta-status={status}\n"),
                format!("\ncommand-v-tta-status={status}\n"),
                "selected target startup\n\ncommand-v-tta-status=2\n".into(),
            ] {
                assert!(verify_no_tta(&ProcessOutput { stdout, ..absent.clone() }).is_err());
            }
            assert!(verify_no_tta(&ProcessOutput { complete: false, ..absent }).is_err());
        }
        // Actual installed POSIX shells, no SSH and no external command lookup:
        // command is a builtin and PATH excludes every executable, including tta.
        for shell in ["/bin/sh", "/bin/bash"] {
            let output = checked(
                Command::new(shell).args(["-c", &format!("printf 'selected target startup\\n'; PATH=/nonexistent; {NO_TTA}")]),
                Duration::from_secs(1),
            )
            .unwrap();
            verify_no_tta(&output).unwrap();
        }
    }
}
