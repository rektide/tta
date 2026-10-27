//! Bounded, complete-EOF process acquisition. Each child gets its own process
//! group; deadline/cap cleanup never addresses a caller's group or a server.
use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub timeout: Duration,
    pub per_stream: usize,
    pub cleanup_grace: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            per_stream: 256 * 1024,
            cleanup_grace: Duration::from_millis(250),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stop {
    Complete,
    Deadline,
    OutputCap,
    ReadError(String),
    WaitError(String),
}

#[derive(Debug)]
pub struct Output {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_eof: bool,
    pub stderr_eof: bool,
    pub omitted_stdout: usize,
    pub omitted_stderr: usize,
    pub stop: Stop,
    pub cleanup: Vec<String>,
}

impl Output {
    /// Exit alone is insufficient: inherited open pipes are not complete data.
    pub fn complete(&self) -> bool {
        self.stop == Stop::Complete && self.status.is_some() && self.stdout_eof && self.stderr_eof
    }

    pub fn success(&self) -> bool {
        self.complete() && self.status.is_some_and(|status| status.success())
    }

    pub fn require_complete(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.complete(), "incomplete process capture: {self:?}");
        Ok(())
    }
}

struct Owned(Child, bool);

impl Owned {
    fn terminate(&mut self) -> io::Result<()> {
        let id = rustix::process::Pid::from_raw(self.0.id() as i32).expect("child PID is positive");
        match rustix::process::kill_process_group(id, rustix::process::Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // Also covers acquisition/descriptor setup errors. Never block Drop on
        // an uninterruptible child; try_wait is sufficient after normal capture.
        if self.1 {
            let _ = self.terminate();
            let _ = self.0.try_wait();
        }
    }
}

fn nonblocking(fd: &impl std::os::fd::AsFd) -> io::Result<()> {
    let flags = rustix::fs::fcntl_getfl(fd)?;
    rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK)?;
    Ok(())
}

fn drain(reader: &mut impl Read, retained: &mut Vec<u8>, eof: &mut bool, omitted: &mut usize, limit: usize) -> io::Result<()> {
    if *eof {
        return Ok(());
    }
    // Bound work per iteration as well as retained memory (a hot writer must
    // not starve the deadline, stderr, or the child status check).
    let mut buffer = [0; 8192];
    for _ in 0..16 {
        match reader.read(&mut buffer) {
            Ok(0) => {
                *eof = true;
                break;
            }
            Ok(count) => {
                let keep = count.min(limit.saturating_sub(retained.len()));
                retained.extend_from_slice(&buffer[..keep]);
                *omitted = omitted.saturating_add(count - keep);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// No reader threads, blocking joins, or post-exit read grace masquerading as
/// EOF. A pipe-holding descendant remains subject to the original deadline.
/// Stdin is null: use a separate terminal/stream API for interactive execution.
pub fn run(command: &mut Command, limits: Limits) -> io::Result<Output> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).process_group(0);
    let mut child = Owned(command.spawn()?, true);
    let mut stdout = child.0.stdout.take().expect("piped stdout");
    let mut stderr = child.0.stderr.take().expect("piped stderr");
    nonblocking(&stdout)?;
    nonblocking(&stderr)?;
    let start = Instant::now();
    let mut output = Output {
        status: None,
        stdout: Vec::new(),
        stderr: Vec::new(),
        stdout_eof: false,
        stderr_eof: false,
        omitted_stdout: 0,
        omitted_stderr: 0,
        stop: Stop::Complete,
        cleanup: Vec::new(),
    };
    let mut stopped_at = None;
    loop {
        if let Err(error) = drain(
            &mut stdout,
            &mut output.stdout,
            &mut output.stdout_eof,
            &mut output.omitted_stdout,
            limits.per_stream,
        )
        .and_then(|()| {
            drain(
                &mut stderr,
                &mut output.stderr,
                &mut output.stderr_eof,
                &mut output.omitted_stderr,
                limits.per_stream,
            )
        }) {
            output.stop = Stop::ReadError(error.to_string());
        }
        if output.status.is_none() {
            match child.0.try_wait() {
                Ok(status) => output.status = status,
                Err(error) => output.stop = Stop::WaitError(error.to_string()),
            }
        }
        if stopped_at.is_none() {
            if output.omitted_stdout > 0 || output.omitted_stderr > 0 {
                output.stop = Stop::OutputCap;
            }
            if output.stop == Stop::Complete && output.status.is_some() && output.stdout_eof && output.stderr_eof {
                break;
            }
            if output.stop == Stop::Complete && start.elapsed() >= limits.timeout {
                output.stop = Stop::Deadline;
            }
            if output.stop != Stop::Complete {
                if let Err(error) = child.terminate() {
                    output.cleanup.push(format!("terminate owned process group: {error}"));
                }
                stopped_at = Some(Instant::now());
            }
        }
        if let Some(stopped) = stopped_at
            && ((output.status.is_some() && output.stdout_eof && output.stderr_eof) || stopped.elapsed() >= limits.cleanup_grace)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    // Normal completion must not kill deliberately backgrounded, pipe-detached
    // work. On incomplete acquisition cleanup was explicitly attempted above.
    if output.complete() {
        child.1 = false;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        command
    }

    fn limits() -> Limits {
        Limits {
            timeout: Duration::from_millis(180),
            per_stream: 4096,
            ..Limits::default()
        }
    }

    #[test]
    fn retains_full_streams_and_nonzero_status_through_eof() {
        let out = run(&mut shell("printf 'out\n'; printf 'diagnostic\n' >&2; exit 7"), limits()).unwrap();
        assert!(out.complete(), "{out:?}");
        assert_eq!(out.status.unwrap().code(), Some(7));
        assert_eq!(out.stdout, b"out\n");
        assert_eq!(out.stderr, b"diagnostic\n");
    }

    #[test]
    fn child_exit_is_not_eof_and_cleanup_only_targets_owned_group() {
        let mut independent = shell("exec sleep 5").spawn().unwrap();
        let start = Instant::now();
        let out = run(&mut shell("printf before; sleep 20 & exit 9"), limits()).unwrap();
        assert_eq!(out.status.unwrap().code(), Some(9));
        assert_eq!(out.stop, Stop::Deadline);
        assert!(!out.complete());
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(independent.try_wait().unwrap().is_none());
        independent.kill().unwrap();
        independent.wait().unwrap();
    }

    #[test]
    fn caps_both_streams_and_does_not_starve_deadline() {
        let out = run(
            &mut shell("while :; do printf '0123456789abcdef'; printf 'fedcba9876543210' >&2; done"),
            limits(),
        )
        .unwrap();
        assert_eq!(out.stop, Stop::OutputCap);
        assert!(out.stdout.len() <= 4096 && out.stderr.len() <= 4096);
        assert!(out.omitted_stdout + out.omitted_stderr > 0);
        assert!(!out.complete());
    }
}
