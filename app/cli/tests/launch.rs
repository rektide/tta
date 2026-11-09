//! Native mutation policy with independently observed, exclusively owned tmux
//! servers. The recording client delegates to real tmux; it is not a fake server.
#![cfg(unix)]

use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

mod common;

struct Fixture {
    temp: tempfile::TempDir,
    socket: PathBuf,
    client: Option<Child>,
}

impl Fixture {
    fn new() -> Self {
        let temp = common::scratch_dir("tta-launch-");
        let socket = temp.path().join("sock");
        assert!(!socket.exists());
        fs::create_dir(temp.path().join("records")).unwrap();
        fs::create_dir(temp.path().join("projects")).unwrap();
        let wrapper = temp.path().join("tmux-client");
        fs::write(&wrapper, "#!/bin/sh\nprintf '%s\\0' \"$@\" > \"$TTA_TEST_RECORDS/$$\"\nexec tmux \"$@\"\n").unwrap();
        fs::set_permissions(wrapper, fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = Self { temp, socket, client: None };
        let absent = fixture.command().arg("ls").output().unwrap();
        assert!(!absent.status.success());
        let diagnostic = String::from_utf8(absent.stderr).unwrap();
        assert!(diagnostic.contains(fixture.socket.to_str().unwrap()), "{diagnostic}");
        assert!(
            diagnostic.contains("no server running") || diagnostic.contains("No such file or directory"),
            "{diagnostic}"
        );
        fixture.run(&["new-session", "-d", "-s", "origin", "sleep", "120"]);
        fixture
    }

    fn command(&self) -> Command {
        let mut command = Command::new("tmux");
        command
            .args(["-S"])
            .arg(&self.socket)
            .args(["-f", "/dev/null"])
            .env_remove("TMUX")
            .env_remove("TMUX_PANE");
        command
    }

    fn run(&self, args: &[&str]) -> String {
        text(self.command().args(args).output().unwrap())
    }

    fn attach(&mut self) {
        let mut child = self
            .command()
            .args(["-C", "attach-session", "-t", "=origin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            let _ = io::copy(&mut output, &mut io::sink());
        });
        self.client = Some(child);
        wait(|| !self.clients().is_empty());
    }

    fn clients(&self) -> String {
        self.run(&["list-clients", "-F", "#{client_pid} #{session_name}"])
    }

    fn tta(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tta"));
        for (key, _) in std::env::vars_os() {
            if key.as_encoded_bytes().starts_with(b"TTA_") || key == "PROJECTS_DIR" {
                command.env_remove(key);
            }
        }
        command
            .env("TTA_CONFIG", self.temp.path().join("no-config"))
            .env("TTA_TEST_RECORDS", self.temp.path().join("records"))
            .env("TMUX", format!("{},0,0", self.socket.display()))
            .env("TMUX_PANE", self.run(&["display-message", "-p", "-t", "=origin:", "#{pane_id}"]))
            .args(["--root"])
            .arg(self.temp.path().join("projects"))
            .arg("--socket-path")
            .arg(&self.socket)
            .arg("--tmux")
            .arg(self.temp.path().join("tmux-client"))
            .env("TTA_TMUX_CONFIG", "/dev/null")
            .arg("--trace");
        command
    }

    fn records(&self) -> Vec<Vec<Vec<u8>>> {
        fs::read_dir(self.temp.path().join("records"))
            .unwrap()
            .map(|entry| {
                let bytes = fs::read(entry.unwrap().path()).unwrap();
                bytes.strip_suffix(&[0]).unwrap().split(|byte| *byte == 0).map(Vec::from).collect()
            })
            .collect()
    }

    fn has_recorded(&self, operation: &str) -> bool {
        self.records().iter().any(|args| args.iter().any(|arg| arg == operation.as_bytes()))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // This fixture proved the socket absent before creating its sole server.
        match self.command().arg("kill-server").status() {
            Ok(status) if !status.success() => eprintln!("owned cleanup {}: {status}", self.socket.display()),
            Err(error) => eprintln!("owned cleanup {}: {error}", self.socket.display()),
            _ => {}
        }
        if let Some(client) = &mut self.client {
            if client.try_wait().unwrap().is_none() {
                let _ = client.kill();
            }
            let _ = client.wait();
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

fn project_creation(backend: Option<&str>, native: bool, raw: bool) {
    let mut fixture = Fixture::new();
    fixture.attach();
    let client_before = fixture.clients().split_whitespace().next().unwrap().to_owned();
    let directory = fixture.temp.path().join("chosen #{session_name};");
    fs::create_dir(&directory).unwrap();
    fs::create_dir(fixture.temp.path().join("projects/project")).unwrap();
    let mut command = fixture.tta();
    if let Some(backend) = backend {
        command.args(["--backend", backend]);
    }
    let argument = if raw {
        OsString::from_vec(b"raw\xff\\;".to_vec())
    } else {
        "space ' $(false) #{session_name};".into()
    };
    let output = command
        .arg("--dir")
        .arg(&directory)
        .args([
            "--session-name",
            "project #[x];",
            "project",
            "--",
            "/bin/sh",
            "-c",
            "printf '%s\\0' \"$@\" > argv; pwd > cwd; exec sleep 120",
            "recorder",
            "",
            ";",
        ])
        .arg(&argument)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fixture.clients(), format!("{client_before} project #[x];"));
    assert!(fixture.has_recorded("switch-client"));
    assert_eq!(fixture.has_recorded("new-session"), !native);
    assert_eq!(String::from_utf8_lossy(&output.stderr).contains("launch.native"), native);
    wait(|| directory.join("argv").exists() && directory.join("cwd").exists());
    let mut expected = b"\0;\0".to_vec();
    expected.extend_from_slice(argument.as_encoded_bytes());
    expected.push(0);
    assert_eq!(fs::read(directory.join("argv")).unwrap(), expected);
    assert_eq!(fs::read_to_string(directory.join("cwd")).unwrap().trim_end(), directory.to_str().unwrap());
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn default_native_creation_preserves_argv_cwd_and_original_client() {
    project_creation(None, true, false);
}

#[cfg(feature = "cli")]
#[test]
fn cli_selection_keeps_creation_and_switch_on_the_cli() {
    project_creation(Some("cli"), false, false);
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn raw_unix_startup_argv_deliberately_keeps_the_cli_creation_path() {
    project_creation(Some("native"), false, true);
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn native_lone_executable_with_spaces_and_semicolon_is_not_shell_source() {
    let mut fixture = Fixture::new();
    fixture.attach();
    let directory = fixture.temp.path().join("projects/project");
    fs::create_dir(&directory).unwrap();
    let executable = fixture.temp.path().join("my executable;");
    fs::write(&executable, "#!/bin/sh\nprintf done > marker\nexec sleep 120\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let output = fixture.tta().args(["project", "--"]).arg(executable).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!fixture.has_recorded("new-session"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("launch.native"));
    wait(|| directory.join("marker").exists());
    assert_eq!(fs::read(directory.join("marker")).unwrap(), b"done");
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn repeated_roots_select_globally_and_launch_the_second_root_over_real_tmux() {
    let mut fixture = Fixture::new();
    fixture.attach();
    let second = fixture.temp.path().join("second root");
    fs::create_dir_all(second.join("work")).unwrap();
    fs::create_dir(fixture.temp.path().join("projects/long-work")).unwrap();
    // The shortest basename wins across both configured roots; the repeated
    // second root is not lower priority than the fixture's first root.
    let output = fixture
        .tta()
        .arg("--root")
        .arg(&second)
        .args(["work", "--", "/bin/sh", "-c", "pwd > cwd-marker; exec sleep 120"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(fixture.has_recorded("switch-client"));
    wait(|| second.join("work/cwd-marker").exists());
    assert_eq!(
        fs::read_to_string(second.join("work/cwd-marker")).unwrap().trim_end(),
        second.join("work").to_str().unwrap()
    );
    assert_eq!(fixture.run(&["list-sessions", "-F", "#{session_name}"]), "origin\nwork");
    assert!(!fixture.temp.path().join("projects/work").exists());
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn print_shell_stays_observation_only_for_a_native_eligible_launch() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.temp.path().join("projects/project")).unwrap();
    let mut command = fixture.tta();
    let printed = text(command.args(["--print-shell", "project"]).output().unwrap());
    assert!(printed.contains("'new-session' '-d'"), "{printed}");
    assert!(printed.contains("'switch-client'"), "{printed}");
    assert_eq!(fixture.run(&["list-sessions", "-F", "#{session_name}"]), "origin");
    assert!(fixture.records().is_empty());
    #[cfg(feature = "cli")]
    assert_eq!(printed, text(command.args(["--backend", "cli"]).output().unwrap()));
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn stale_inside_context_uses_cli_cold_start_and_selected_configuration() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.temp.path().join("projects/project")).unwrap();
    let config = fixture.temp.path().join("tmux.conf");
    fs::write(&config, "set-option -g @tta-cold-start selected\n").unwrap();
    let mut command = fixture.tta();
    fixture.run(&["kill-server"]);
    wait(|| std::os::unix::net::UnixStream::connect(&fixture.socket).is_err());
    let output = command
        .arg("--tmux-config")
        .arg(config)
        .args(["project", "--", "sleep", "120"])
        .output()
        .unwrap();
    // Creation starts the server as before; switching a stale inside context
    // still fails rather than inventing a new terminal attachment.
    assert!(!output.status.success());
    assert!(fixture.has_recorded("new-session"), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(fixture.has_recorded("switch-client"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("launch.native"));
    assert_eq!(fixture.run(&["show-options", "-gv", "@tta-cold-start"]), "selected");
    assert_eq!(fixture.run(&["list-sessions", "-F", "#{session_name}"]), "project");
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn grouped_launch_retains_one_cli_queue_even_with_destroy_unattached_on() {
    let mut fixture = Fixture::new();
    fixture.attach();
    let client_before = fixture.clients().split_whitespace().next().unwrap().to_owned();
    fixture.run(&["set-option", "-g", "destroy-unattached", "on"]);
    let output = fixture.tta().args(["--group", "origin"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fixture.clients(), format!("{client_before} _origin-1"));
    assert_eq!(fixture.run(&["show-options", "-v", "-t", "=_origin-1:", "destroy-unattached"]), "keep-last");
    let records = fixture.records();
    assert_eq!(records.len(), 1);
    let args = &records[0];
    for operation in ["new-session", "switch-client", "set-option"] {
        assert!(args.iter().any(|arg| arg == operation.as_bytes()), "{args:?}");
    }
    assert!(!String::from_utf8_lossy(&output.stderr).contains("launch.native"));
}

#[cfg(all(feature = "native", target_os = "linux", target_endian = "little"))]
#[test]
fn failed_native_create_never_hands_off_and_failed_handoff_leaves_creation() {
    use tta_command::{Command as Step, Expression, Handoff, Word};
    use tta_launcher::{ClientMode, Selection, prepare_launch, socket};
    use tta_runner::execution::{ExecuteError, ExecuteLaunch, LaunchBackend};

    let mut fixture = Fixture::new();
    fixture.attach();
    let before = fixture.clients();
    let directory = fixture.temp.path().join("generated");
    let selection = Selection::Create {
        sources: vec![],
        directory: directory.clone(),
        session_name: "fresh".into(),
    };
    let make_plan = |prefix: &Expression, program: &Path| {
        prepare_launch()
            .selection(&selection)
            .socket(&socket::Resolved::path(&fixture.socket))
            .client_mode(ClientMode::Switch)
            .program(program.as_os_str())
            .prefix(prefix)
            .call()
            .unwrap()
    };
    let prefix = Expression::from(
        Step::builder()
            .program(Word::new("sh").unwrap())
            .args(vec![Word::new("-c").unwrap(), Word::new("exit 23").unwrap()])
            .build(),
    );
    let plan = make_plan(&prefix, &fixture.temp.path().join("tmux-client"));
    assert_eq!(plan.execute(LaunchBackend::Native, Handoff::Run).unwrap().unwrap().code(), Some(23));
    assert!(!directory.exists());
    assert!(fixture.records().is_empty());
    assert_eq!(fixture.clients(), before);

    let prefix = Expression::from(Step::builder().program(Word::new("true").unwrap()).build());
    let plan = make_plan(&prefix, &fixture.temp.path().join("tmux-client"));
    assert!(matches!(
        plan.execute(LaunchBackend::Native, Handoff::Run),
        Err(ExecuteError::Command(tta_runner::command::Error::Directory { .. }))
    ));
    assert!(fixture.records().is_empty());

    let prefix = Expression::from(
        Step::builder()
            .program(Word::new("mkdir").unwrap())
            .args(vec![Word::new(directory.as_os_str()).unwrap()])
            .build(),
    );
    let plan = make_plan(&prefix, &fixture.temp.path().join("missing-client"));
    assert!(matches!(plan.execute(LaunchBackend::Native, Handoff::Run), Err(ExecuteError::Command(_))));
    assert_eq!(fixture.run(&["display-message", "-p", "-t", "=fresh:", "#{session_name}"]), "fresh");
    assert_eq!(fixture.clients(), before);

    let prefix = Expression::from(Step::builder().program(Word::new("true").unwrap()).build());
    let plan = make_plan(&prefix, &fixture.temp.path().join("tmux-client"));
    assert!(matches!(plan.execute(LaunchBackend::Native, Handoff::Run), Err(ExecuteError::Native(_))));
    assert!(fixture.records().is_empty());
    assert_eq!(fixture.clients(), before);
}
