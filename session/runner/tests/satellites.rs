//! Real control clients on exclusively owned, preflight-verified absolute sockets.
//! Cleanup kills individual sessions only. No default endpoint is ever used.
#![cfg(unix)]

mod common;

use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use tta_command::{Expression, Word};
use tta_launcher::entry::protocol::{ClientEntry, ViewPreference};
use tta_runner::command::ProcessCommand;

struct Server {
    directory: tempfile::TempDir,
    socket: PathBuf,
}

impl Server {
    fn new() -> Self {
        let directory = common::scratch_dir("tta-satellites-");
        let socket = directory.path().join("sock");
        let server = Self { directory, socket };
        let missing = server.command().arg("ls").output().unwrap();
        assert!(!missing.status.success());
        let stderr = String::from_utf8(missing.stderr).unwrap();
        assert!(stderr.contains(server.socket.to_str().unwrap()), "{stderr}");
        assert!(stderr.contains("no server running") || stderr.contains("No such file or directory"), "{stderr}");
        assert!(!server.socket.exists());
        server.run(&["new-session", "-d", "-s", "seed", "sleep", "120"]);
        server
    }

    fn command(&self) -> Command {
        let mut command = Command::new("tmux");
        command
            .args(["-S"])
            .arg(&self.socket)
            .args(["-f", "/dev/null"])
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .env("LC_ALL", "C");
        command
    }

    fn run(&self, args: &[&str]) -> String {
        text(self.command().args(args).output().unwrap())
    }

    fn field(&self, target: &str, format: &str) -> String {
        self.run(&["display-message", "-p", "-t", &format!("{target}:"), format])
    }

    fn plan(&self, selector: &str, mode: ClientEntry, startup: &[OsString], directory: Option<&std::path::Path>) -> Expression {
        let mut i = common::entry::input(self.directory.path(), selector);
        i.context.socket = tta_launcher::socket::Resolved::path(&self.socket);
        i.context.client = mode;
        i.context.config = Some("/dev/null".into());
        i.intent.view = ViewPreference::Independent;
        i.intent.startup = startup.to_vec();
        i.intent.directory = directory.map(Into::into);
        i.intent.view_directory = directory.map(Into::into);
        i.intent.roots = vec![self.directory.path().into()];
        if selector != "seed" {
            i.intent.session_name = Some("created".into());
        }
        let group = self.field("=seed", "#{session_group}");
        let sessions = vec![tta_launcher::Session {
            id: self.field("=seed", "#{session_id}").parse().unwrap(),
            name: "seed".into(),
            group: (!group.is_empty()).then_some(group),
        }];
        if startup.iter().any(|arg| arg.to_str().is_none()) {
            common::entry::terminal(common::entry::prepare(i, sessions.clone()), sessions)
        } else {
            common::entry::export(common::entry::prepare(i, sessions.clone()), sessions)
        }
    }

    fn clients(&self) -> String {
        self.run(&["list-clients", "-F", "#{client_pid} #{session_id}"])
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let output = self.command().args(["list-sessions", "-F", "#{session_id}"]).output().unwrap();
        if output.status.success() {
            for id in String::from_utf8(output.stdout).unwrap().lines() {
                // Every session on this verified, exclusively owned server was
                // created by this fixture. Never issue a server-wide kill.
                let output = self.command().args(["kill-session", "-t", id]).output().unwrap();
                if !output.status.success() {
                    eprintln!("cleanup {id}: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
    }
}

fn text(output: Output) -> String {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap().trim_end_matches('\n').to_owned()
}

fn wait(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for tmux observation");
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct Client(Child);
impl Client {
    fn start(command: &mut Command) -> Self {
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            let _ = io::copy(&mut stdout, &mut io::sink());
        });
        Self(child)
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            self.0.kill().unwrap();
        }
        self.0.wait().unwrap();
    }
}

#[test]
fn outside_satellite_has_independent_window_and_disappears_on_detach() {
    for shell in [false, true] {
        let server = Server::new();
        server.run(&["new-window", "-d", "-t", "=seed", "sleep", "120"]);
        let seed_window = server.field("=seed", "#{window_id}");
        let initial = server.directory.path().join("initial #{session_name}");
        fs::create_dir(&initial).unwrap();
        let mut expression = server.plan("seed", ClientEntry::AttachTerminal, &["must-not-run".into()], Some(&initial));
        let Expression::Command(command) = &mut expression else {
            panic!("expected grouped command")
        };
        command.args.insert(0, Word::new("-C").unwrap());
        let mut process = if shell {
            let mut process = Command::new("sh");
            process.args(["-c", &expression.shell(tta_command::Handoff::Exec).unwrap()]);
            process
        } else {
            let Expression::Command(command) = &expression else { unreachable!() };
            command.process()
        };
        process.env_remove("TMUX").env_remove("TMUX_PANE");
        let client = Client::start(&mut process);
        wait(|| !server.clients().is_empty());
        let satellite = server.clients().split_whitespace().nth(1).unwrap().to_owned();
        assert_ne!(satellite, server.field("=seed", "#{session_id}"));
        assert_eq!(server.field(&satellite, "#{session_name}"), "_seed-1");
        assert_eq!(server.run(&["show-options", "-v", "-t", &satellite, "destroy-unattached"]), "keep-last");
        assert_eq!(server.field(&satellite, "#{session_path}"), initial.to_str().unwrap());
        assert_eq!(server.field(&satellite, "#{session_group}"), server.field("=seed", "#{session_group}"));
        server.run(&["select-window", "-t", &format!("{satellite}:1")]);
        assert_ne!(server.field(&satellite, "#{window_id}"), seed_window);
        assert_eq!(server.field("=seed", "#{window_id}"), seed_window);
        server.run(&["detach-client", "-s", &satellite]);
        wait(|| server.clients().is_empty());
        drop(client);
        wait(|| server.run(&["list-sessions", "-F", "#{session_id}"]).lines().count() == 1);
        assert_eq!(server.field("=seed", "#{session_windows}"), "2");
    }
}

#[test]
fn inside_satellite_switches_same_client_and_keep_last_preserves_final_session() {
    let server = Server::new();
    let client = Client::start(server.command().args(["-C", "attach-session", "-t", "=seed"]));
    wait(|| !server.clients().is_empty());
    let before = server.clients();
    let expression = server.plan("seed", ClientEntry::SwitchWithinServer, &[], None);
    let Expression::Command(command) = expression else {
        panic!("expected command")
    };
    text(
        command
            .process()
            .env("TMUX", format!("{},0,0", server.socket.display()))
            .env("TMUX_PANE", server.field("=seed", "#{pane_id}"))
            .output()
            .unwrap(),
    );
    let after = server.clients();
    assert_eq!(before.split_whitespace().next(), after.split_whitespace().next());
    let satellite = after.split_whitespace().nth(1).unwrap();
    assert_ne!(before, after);
    assert_eq!(server.field(satellite, "#{session_name}"), "_seed-1");
    assert_eq!(server.run(&["show-options", "-v", "-t", satellite, "destroy-unattached"]), "keep-last");
    server.run(&["kill-session", "-t", "=seed"]);
    server.run(&["detach-client", "-s", satellite]);
    wait(|| server.clients().is_empty());
    drop(client);
    assert_eq!(server.run(&["list-sessions", "-F", "#{session_id}"]), satellite);
}

#[test]
fn startup_preserves_raw_arguments_and_explicit_cwd() {
    let server = Server::new();
    let directory = server.directory.path().join("chosen #{session_name};");
    fs::create_dir(&directory).unwrap();
    let record = directory.join("argv");
    let startup = vec![
        "/bin/sh".into(),
        "-c".into(),
        "printf '%s\\0' \"$@\" > argv; pwd > cwd; exec sleep 120".into(),
        "recorder".into(),
        "".into(),
        "space ' $(false) #{session_name};".into(),
        ";".into(),
        OsString::from_vec(b"raw\xff\\;".to_vec()),
    ];
    fs::create_dir(server.directory.path().join("not-used")).unwrap();
    let expression = server.plan("not-used", ClientEntry::SwitchWithinServer, &startup, Some(&directory));
    let Expression::And(and) = expression else { panic!("expected sequence") };
    let Expression::Command(create) = &and.steps[1] else {
        panic!("expected creation")
    };
    text(create.process().env_remove("TMUX").env_remove("TMUX_PANE").output().unwrap());
    wait(|| record.exists() && directory.join("cwd").exists());
    assert_eq!(fs::read(&record).unwrap(), b"\0space ' $(false) #{session_name};\0;\0raw\xff\\;\0");
    assert_eq!(fs::read_to_string(directory.join("cwd")).unwrap().trim_end(), directory.to_str().unwrap());
}

#[test]
fn lone_startup_executable_with_spaces_is_not_shell_source() {
    let server = Server::new();
    let executable = server.directory.path().join("my executable; literal");
    fs::write(&executable, "#!/bin/sh\nprintf done > marker\nexec sleep 120\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(server.directory.path().join("project")).unwrap();
    let expression = server.plan(
        "project",
        ClientEntry::SwitchWithinServer,
        &[executable.into_os_string()],
        Some(server.directory.path()),
    );
    let Expression::And(and) = expression else { panic!("expected sequence") };
    let Expression::Command(create) = &and.steps[1] else {
        panic!("expected creation")
    };
    text(create.process().env_remove("TMUX").env_remove("TMUX_PANE").output().unwrap());
    wait(|| server.directory.path().join("marker").exists());
    assert_eq!(fs::read_to_string(server.directory.path().join("marker")).unwrap(), "done");
}
