//! Live tests own a unique path socket. They never query or kill the default
//! server. Requires tmux >= 3.2; native tests additionally require supported Linux.
#![cfg(all(unix, any(feature = "cli", all(feature = "native", target_os = "linux", target_endian = "little"))))]

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

mod common;

use tta_command::{Expression, Handoff, Word};
use tta_launcher::{Session, entry::protocol::ClientEntry};
use tta_runner::command::{Execute, ProcessCommand};
use tta_runner::observe::local::{SessionBackend, Sessions};

struct Server {
    directory: tempfile::TempDir,
    socket: PathBuf,
    started: bool,
}

impl Server {
    fn cold() -> Self {
        let directory = common::scratch_dir("tta-server-");
        let socket = directory.path().join("s");
        Self {
            directory,
            socket,
            started: false,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new("tmux");
        command
            .args(["-f", "/dev/null", "-S"])
            .arg(&self.socket)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .env("TERM", "xterm-256color")
            .stdin(Stdio::null());
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        let output = self.command().args(args).output().expect("live tests require tmux on PATH");
        assert!(output.status.success(), "tmux {args:?}: {}", String::from_utf8_lossy(&output.stderr));
        output
    }

    fn start(&mut self) {
        let missing = self.command().arg("ls").output().unwrap();
        assert!(!missing.status.success());
        let stderr = String::from_utf8(missing.stderr).unwrap();
        assert!(stderr.contains(self.socket.to_str().unwrap()), "{stderr}");
        assert!(stderr.contains("no server running") || stderr.contains("No such file or directory"), "{stderr}");
        self.run(&["new-session", "-d", "-s", "seed", "sleep", "120"]);
        self.started = true;
        self.run(&["set-option", "-g", "default-shell", "/bin/sh"]);
        self.run(&["set-option", "-g", "default-command", "sleep 120"]);
    }

    fn inventories(&self) -> Vec<Vec<Session>> {
        vec![
            #[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
            SessionBackend::Native.inventory(OsStr::new("tmux"), &self.socket).unwrap().sessions().to_vec(),
            #[cfg(feature = "cli")]
            SessionBackend::Cli.inventory(OsStr::new("tmux"), &self.socket).unwrap().sessions().to_vec(),
        ]
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if self.started {
            match self.command().arg("kill-server").stdout(Stdio::null()).status() {
                Ok(status) if !status.success() => eprintln!("owned tmux cleanup {}: {status}", self.socket.display()),
                Err(error) => eprintln!("owned tmux cleanup {}: {error}", self.socket.display()),
                _ => {}
            }
        }
    }
}

#[test]
fn cold_and_stale_servers_are_empty_without_starting_a_server() {
    let server = Server::cold();
    for sessions in server.inventories() {
        assert!(sessions.is_empty());
    }
    assert!(!server.socket.exists());
    let listener = std::os::unix::net::UnixListener::bind(&server.socket).unwrap();
    drop(listener);
    for sessions in server.inventories() {
        assert!(sessions.is_empty());
    }
}

#[test]
fn native_and_cli_decode_the_same_hostile_names() {
    let mut server = Server::cold();
    server.start();
    let marker = server.directory.path().join("job-marker");
    let name = format!("quote' space %25 #[x] #{{session_name}} #(touch {});", marker.display());
    let format_name = name.replace('#', "#{a:35}");
    // tmax's argv adapter also protects tmux's trailing-semicolon command parser.
    let invocation = tmax::operation::Invocation::new("new-session")
        .unwrap()
        .args(["-d", "-s", &format_name, "sleep", "120"]);
    let output = server.command().args(invocation.argv().unwrap()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let inventories = server.inventories();
    for inventory in &inventories {
        assert!(inventory.iter().any(|session| session.name == name), "{inventory:?}");
    }
    if inventories.len() > 1 {
        assert_eq!(inventories[0], inventories[1]);
    }
    assert!(!marker.exists(), "a literal tmux format operand executed a shell job");
}

#[test]
fn kept_alive_server_without_sessions_has_an_empty_inventory() {
    let mut server = Server::cold();
    server.start();
    server.run(&["set-option", "-s", "exit-empty", "off"]);
    server.run(&["kill-session", "-t", "=seed"]);
    for inventory in server.inventories() {
        assert!(inventory.is_empty());
    }
}

#[test]
fn planned_creation_preserves_literal_directory_and_name_and_refuses_races() {
    let mut server = Server::cold();
    server.start();
    let directory = server.directory.path().join("space ' #[x] #{session_name} %;");
    fs::create_dir(&directory).unwrap();
    let name = "new#[x] #{session_name} %;";
    let mut i = common::entry::input(server.directory.path(), directory.file_name().unwrap().to_str().unwrap());
    i.context.socket = tta_launcher::socket::Resolved::path(&server.socket);
    i.context.client = ClientEntry::SwitchWithinServer;
    i.intent.roots = vec![server.directory.path().into()];
    i.intent.session_name = Some(name.into());
    let sessions = server.inventories()[0].clone();
    let plan = common::entry::export(common::entry::prepare(i, sessions.clone()), sessions);
    let Expression::And(and) = &plan else { panic!("expected sequence") };
    // Execute only creation: switching real terminal clients belongs to PTY
    // acceptance, not this no-terminal test. No native mutation API is used.
    assert!(and.steps[1].execute(Handoff::Run).unwrap().unwrap().success());
    let inventories = server.inventories();
    let session = inventories[0].iter().find(|session| session.name == name).unwrap();
    let path = server.run(&["display-message", "-p", "-t", &session.id.to_string(), "#{pane_current_path}"]);
    assert_eq!(String::from_utf8(path.stdout).unwrap().trim_end_matches('\n'), directory.to_str().unwrap());
    // No -A: another creator winning the race is a visible duplicate-name
    // failure, never an implicit attach that could change an existing cwd.
    let Expression::Command(create) = &and.steps[1] else {
        panic!("expected command")
    };
    let raced = create.process().output().unwrap();
    assert!(!raced.status.success());
    assert!(String::from_utf8_lossy(&raced.stderr).contains("duplicate session"));
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn native_protocol_failures_are_not_misreported_as_cold_servers() {
    let server = Server::cold();
    let listener = std::os::unix::net::UnixListener::bind(&server.socket).unwrap();
    let peer = std::thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });
    let result = SessionBackend::Native.inventory(OsStr::new("tmux"), &server.socket);
    assert!(result.is_err(), "a peer closing during the protocol must be an error");
    peer.join().unwrap();
}

#[test]
fn inaccessible_sockets_are_errors_not_cold_servers() {
    use std::os::unix::fs::PermissionsExt;
    // Root bypasses Unix socket permissions; pure classification tests still
    // exercise PermissionDenied in that environment.
    if rustix::process::geteuid().is_root() {
        return;
    }
    let mut server = Server::cold();
    server.start();
    fs::set_permissions(&server.socket, fs::Permissions::from_mode(0o000)).unwrap();
    #[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
    assert!(SessionBackend::Native.inventory(OsStr::new("tmux"), &server.socket).is_err());
    #[cfg(feature = "cli")]
    assert!(SessionBackend::Cli.inventory(OsStr::new("tmux"), &server.socket).is_err());
    fs::set_permissions(&server.socket, fs::Permissions::from_mode(0o600)).unwrap();
}

#[cfg(feature = "cli")]
#[test]
fn cli_missing_executable_and_malformed_reply_are_errors() {
    let server = Server::cold();
    assert!(
        SessionBackend::Cli
            .inventory(server.directory.path().join("missing").as_os_str(), &server.socket)
            .is_err()
    );
    // printf emits a malformed typed row; capture retains its original cause.
    let error = SessionBackend::Cli.inventory(OsStr::new("printf"), &server.socket).unwrap_err();
    assert!(error.downcast_ref::<tta_runner::observe::local::CaptureFailure>().is_some());
}

#[test]
fn label_policy_is_frozen_in_direct_and_shell_modes_with_an_explicit_socket_carrier() {
    for shell in [false, true] {
        let mut server = Server::cold();
        let context = tta_launcher::socket::Context::builder()
            .cwd(server.directory.path().to_owned())
            .uid(rustix::process::geteuid().as_raw())
            // Relative TMUX_TMPDIR is frozen against this cwd, not project cwd.
            .tmpdir("relative-tmp".into())
            .build();
        fs::create_dir(server.directory.path().join("relative-tmp")).unwrap();
        let endpoint = context.resolve(&tta_launcher::socket::Selection::Name("owned".into())).unwrap();
        server.socket = endpoint.path.clone();
        assert!(!server.socket.parent().unwrap().exists());
        // A label-policy carrier, not evidence for this tmux build's -L rules.
        // The real server ALWAYS receives -S; this cannot reach a default server
        // even on builds which ignore TMUX_TMPDIR.
        use std::os::unix::fs::PermissionsExt;
        let carrier = server.directory.path().join("label-carrier");
        let quote = |path: &std::path::Path| Word::new(path.as_os_str()).unwrap().shell().unwrap();
        fs::write(&carrier, format!("#!/bin/sh\n[ \"$1\" = -L ] && [ \"$2\" = owned ] || exit 94\n[ \"$TMUX_TMPDIR\" = {} ] || exit 95\nshift 2\nmkdir -m 700 {} || exit\nexec tmux -S {} \"$@\"\n",
            quote(&endpoint.label.as_ref().unwrap().directory), quote(server.socket.parent().unwrap()), quote(&server.socket))).unwrap();
        fs::set_permissions(&carrier, fs::Permissions::from_mode(0o700)).unwrap();
        for inventory in server.inventories() {
            assert!(inventory.is_empty());
        }
        assert!(!server.socket.parent().unwrap().exists());
        let directory = server.directory.path().join("project");
        fs::create_dir(&directory).unwrap();
        let mut i = common::entry::input(server.directory.path(), "project");
        i.context.socket = endpoint;
        i.context.client = ClientEntry::SwitchWithinServer;
        i.context.program = carrier.into_os_string();
        i.context.config = Some("/dev/null".into());
        i.intent.roots = vec![server.directory.path().into()];
        let mut plan = common::entry::export(common::entry::prepare(i, vec![]), vec![]);
        let Expression::And(and) = &mut plan else { panic!("expected sequence") };
        let Expression::Command(create) = &mut and.steps[1] else {
            panic!("expected create")
        };
        // Test-only isolation of tmux configuration and initial pane command.
        create.args.extend([Word::new("sleep").unwrap(), Word::new("120").unwrap()]);
        let create = &and.steps[1];
        let success = if shell {
            Command::new("sh")
                .args(["-c", &create.shell(Handoff::Run).unwrap()])
                .status()
                .unwrap()
                .success()
        } else {
            create.execute(Handoff::Run).unwrap().unwrap().success()
        };
        assert!(success);
        server.started = true;
        assert!(server.socket.exists());
        for inventory in server.inventories() {
            assert_eq!(inventory[0].name, "project");
        }
    }
}
