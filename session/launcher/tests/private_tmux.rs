//! Live tests own a unique path socket. They never query or kill the default
//! server. Requires tmux >= 3.2; native tests additionally require supported Linux.
#![cfg(all(unix, any(feature = "cli", all(feature = "native", target_os = "linux", target_endian = "little"))))]

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use tta_command::{Expression, Handoff};
use tta_launcher::{
    ClientMode, Selection, Session,
    backend::{self, SessionLookup},
    launch,
};

struct Server {
    directory: tempfile::TempDir,
    socket: PathBuf,
    started: bool,
}

impl Server {
    fn cold() -> Self {
        let directory = tempfile::Builder::new().prefix("tta-server-").tempdir().unwrap();
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
        self.run(&["new-session", "-d", "-s", "seed", "sleep", "120"]);
        self.started = true;
        self.run(&["set-option", "-g", "default-shell", "/bin/sh"]);
        self.run(&["set-option", "-g", "default-command", "sleep 120"]);
    }

    fn inventories(&self) -> Vec<Vec<Session>> {
        vec![
            #[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
            backend::Native {
                endpoint: tmax::endpoint::Tmux::new().socket_path(&self.socket),
            }
            .sessions()
            .unwrap(),
            #[cfg(feature = "cli")]
            backend::Cli {
                program: "tmux".into(),
                socket: self.socket.clone(),
            }
            .sessions()
            .unwrap(),
        ]
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if self.started {
            let _ = self.command().arg("kill-server").stdout(Stdio::null()).stderr(Stdio::null()).status();
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
    let name = "quote' space %25 #[x] #{session_name};";
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
    let selection = Selection::Project {
        directory: directory.clone(),
        session_name: name.to_owned(),
    };
    let plan = launch()
        .selection(&selection)
        .socket(&server.socket)
        .client_mode(ClientMode::Switch)
        .program(OsStr::new("tmux"))
        .call()
        .unwrap();
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
    let result = backend::Native {
        endpoint: tmax::endpoint::Tmux::new().socket_path(&server.socket),
    }
    .sessions();
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
    assert!(
        backend::Native {
            endpoint: tmax::endpoint::Tmux::new().socket_path(&server.socket)
        }
        .sessions()
        .is_err()
    );
    #[cfg(feature = "cli")]
    assert!(
        backend::Cli {
            program: "tmux".into(),
            socket: server.socket.clone()
        }
        .sessions()
        .is_err()
    );
    fs::set_permissions(&server.socket, fs::Permissions::from_mode(0o600)).unwrap();
}

#[cfg(feature = "cli")]
#[test]
fn cli_missing_executable_and_malformed_reply_are_errors() {
    use std::os::unix::fs::PermissionsExt;
    let server = Server::cold();
    let missing = backend::Cli {
        program: server.directory.path().join("missing").into_os_string(),
        socket: server.socket.clone(),
    };
    assert!(matches!(missing.sessions(), Err(backend::LookupError::Io(_))));
    let script = server.directory.path().join("fake-tmux");
    fs::write(&script, "#!/bin/sh\nprintf '%s\\n' 'human-readable but invalid session row'\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let malformed = backend::Cli {
        program: script.into_os_string(),
        socket: server.socket.clone(),
    };
    assert!(matches!(malformed.sessions(), Err(backend::LookupError::Decode(_))));
}
