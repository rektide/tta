//! Version-1, nonce-bound binary probe envelopes. Construction and decoding are
//! pure: external callers can run these scripts with no tta runner or worker.
//! Profile: POSIX sh, id, pwd, cat, od, head; GNU find (`-printf`, `%Y`) for
//! filesystem facts; Git only for an actually needed, unset author default.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail, ensure};
use tmax::operation::{ReplyDecoder, TypedOperation};
use tta_command::{Command, Expression, Handoff, Word};

use crate::{DestinationState, ProjectInventory, Session, target, workflow};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Setup(target::Request),
    Workflow(workflow::ObservationRequest),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reply {
    Setup(target::Reply),
    Workflow(workflow::ObservationReply),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Diagnostics {
    pub startup_stdout: Vec<u8>,
    pub trailing_stdout: Vec<u8>,
    pub transport_stderr: Vec<u8>,
    pub command_stderr: Vec<u8>,
    pub command_status: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decoded {
    pub reply: Reply,
    pub diagnostics: Diagnostics,
}

/// The carrier supplies real status and EOF completeness. `None` is signal or
/// unknown termination, not a successful probe. Capture omissions mean false.
pub struct Output<'a> {
    pub stdout: &'a [u8],
    pub stderr: &'a [u8],
    pub status: Option<i32>,
    pub complete: bool,
}

#[derive(Clone, Debug)]
pub struct Probe {
    pub request: Request,
    token: String,
}

fn quote(value: impl Into<std::ffi::OsString>) -> Result<String> {
    Ok(Word::new(value)?.shell()?)
}

impl Probe {
    /// Use a fresh unpredictable token for each execution. Framing separates
    /// ordinary startup noise; it is not authentication against a hostile host.
    pub fn new(request: Request, token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        ensure!(
            (8..=128).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "invalid probe token"
        );
        Ok(Self { request, token })
    }

    fn header(&self) -> Vec<u8> {
        format!("\0TTA-PROBE/1/{}/BEGIN\0", self.token).into_bytes()
    }
    fn footer(&self) -> Vec<u8> {
        format!("\0TTA-PROBE/1/{}/END\0", self.token).into_bytes()
    }

    /// Command stdout streams literally, including NULs. Its stderr and status
    /// travel together as bounded hex metadata, so startup stderr cannot turn
    /// an exact endpoint-absence diagnostic into a false classification.
    pub fn script(&self) -> Result<String> {
        let operation = self.operation()?;
        // head bounds remote metadata retention too. Truncation loses the
        // mandatory final NUL/status pair and is rejected, never accepted empty.
        Ok(format!(
            "printf '\\000TTA-PROBE/1/{token}/BEGIN\\000'\ntta_meta=$( {{ (\nset -e\n{operation}\n) 2>&1 1>&3; tta_rc=$?; printf '\\000%s' \"$tta_rc\"; }} | od -An -v -tx1 | head -c 786432 ) 3>&1\ntta_capture=$?\nprintf '\\000TTA-PROBE/1/{token}/END\\000%s\\000' \"$tta_meta\"\nexit \"$tta_capture\"",
            token = self.token
        ))
    }

    pub fn shell(&self, setup: &target::TargetSetup) -> Result<String> {
        setup.shell_command(&self.script()?)
    }

    fn operation(&self) -> Result<String> {
        Ok(match &self.request {
            Request::Setup(target::Request::Context) => {
                let mut script = "pwd -P\nprintf '\\000'\nid -u\nprintf '\\000'\n".to_owned();
                for variable in target::VARIABLES {
                    script.push_str(&format!("printf '%s\\000' \"${{{variable}-}}\"\n"));
                }
                script
            }
            Request::Setup(target::Request::Entry { parent, .. }) => listing(parent)?,
            Request::Setup(target::Request::Registry { path }) => format!("cat -- {}", quote(path.as_os_str())?),
            Request::Workflow(workflow::ObservationRequest::Sessions { socket, program }) => {
                let query = tmax::query::Sessions::new();
                let mut args = vec![Word::new("-u")?, Word::new("-S")?, Word::new(socket.as_os_str())?];
                for arg in query.invocation()?.argv()? {
                    args.push(Word::new(arg)?);
                }
                let command = Command::builder()
                    .program(Word::new(program.clone())?)
                    .args(args)
                    .environment(vec![tta_command::Assignment::new("LC_ALL", Word::new("C")?)?])
                    .build();
                Expression::from(command).shell(Handoff::Run)?
            }
            Request::Workflow(workflow::ObservationRequest::Projects { root, .. }) => {
                format!("cd -P {}\npwd -P\nprintf '\\000'\n{}", quote(root.as_os_str())?, listing(Path::new("."))?)
            }
            Request::Workflow(workflow::ObservationRequest::Destination { path }) => {
                listing(path.parent().ok_or_else(|| anyhow::anyhow!("destination has no parent"))?)?
            }
            Request::Workflow(workflow::ObservationRequest::GeneratorDefaults { directory, author, license }) => {
                let license = if *license { "${LICENSE-}" } else { "" };
                let author_value = if *author { "${AUTHOR-}" } else { "" };
                let mut script = format!(
                    "cd -P {}\nprintf '%s\\000%s\\000' \"{license}\" \"{author_value}\"\n",
                    quote(directory.as_os_str())?
                );
                if *author {
                    script.push_str("if [ -z \"${AUTHOR-}\" ]; then git config --get user.name; fi\n");
                }
                script
            }
        })
    }

    pub fn decode(&self, output: Output<'_>) -> Result<Decoded> {
        ensure!(output.complete, "incomplete probe capture (EOF/deadline/output cap)");
        ensure!(
            output.status == Some(0),
            "probe carrier failed with {:?}: {}",
            output.status,
            String::from_utf8_lossy(output.stderr)
        );
        let header = self.header();
        let footer = self.footer();
        let begin = unique(output.stdout, &header)?;
        let end = unique(output.stdout, &footer)?;
        ensure!(end >= begin + header.len(), "probe envelope has inverted boundaries");
        let payload = &output.stdout[begin + header.len()..end];
        let tail = &output.stdout[end + footer.len()..];
        let meta_end = tail.iter().position(|b| *b == 0).ok_or_else(|| anyhow::anyhow!("truncated probe metadata"))?;
        ensure!(meta_end < 786432, "probe stderr metadata reached its cap");
        let meta = unhex(&tail[..meta_end])?;
        let split = meta
            .iter()
            .rposition(|b| *b == 0)
            .ok_or_else(|| anyhow::anyhow!("missing command status (possibly capped stderr)"))?;
        let stderr = &meta[..split];
        let status = std::str::from_utf8(&meta[split + 1..])?.parse::<i32>()?;
        ensure!((0..=255).contains(&status), "invalid probe command status");
        let reply = self.decode_payload(status, payload, stderr)?;
        Ok(Decoded {
            reply,
            diagnostics: Diagnostics {
                startup_stdout: output.stdout[..begin].to_vec(),
                trailing_stdout: tail[meta_end + 1..].to_vec(),
                transport_stderr: output.stderr.to_vec(),
                command_stderr: stderr.to_vec(),
                command_status: status,
            },
        })
    }

    fn decode_payload(&self, status: i32, payload: &[u8], stderr: &[u8]) -> Result<Reply> {
        if let Request::Workflow(workflow::ObservationRequest::Sessions { socket, .. }) = &self.request {
            return Ok(Reply::Workflow(workflow::ObservationReply::Sessions(decode_sessions(
                status, payload, stderr, socket,
            )?)));
        }
        if let Request::Workflow(workflow::ObservationRequest::GeneratorDefaults { .. }) = &self.request {
            let (license, rest) = field(payload)?;
            let (author, git) = field(rest)?;
            let unset_git = status == 1 && author.is_empty() && git.is_empty() && stderr.is_empty();
            ensure!(
                status == 0 || unset_git,
                "Git defaults probe failed ({status}): {}",
                String::from_utf8_lossy(stderr)
            );
            let value = |bytes: &[u8]| -> Result<Option<String>> {
                ensure!(!bytes.contains(&0), "NUL in generator default");
                Ok(Some(std::str::from_utf8(bytes)?.to_owned()).filter(|v| !v.is_empty()))
            };
            let author = if !author.is_empty() {
                value(author)?
            } else {
                value(std::str::from_utf8(git)?.trim().as_bytes())?
            };
            return Ok(Reply::Workflow(workflow::ObservationReply::GeneratorDefaults(scaffold_defaults(
                author,
                value(license)?,
            ))));
        }
        ensure!(status == 0, "probe command failed ({status}): {}", String::from_utf8_lossy(stderr));
        Ok(match &self.request {
            Request::Setup(target::Request::Context) => {
                let (cwd, rest) = field(payload)?;
                let cwd = PathBuf::from(std::str::from_utf8(strip_newline(cwd)?)?);
                ensure!(cwd.is_absolute(), "target pwd is not absolute");
                let (uid, mut rest) = field(rest)?;
                let uid = std::str::from_utf8(strip_newline(uid)?)?.parse::<u32>()?;
                let mut environment = BTreeMap::new();
                for variable in target::VARIABLES {
                    let (value, next) = field(rest)?;
                    environment.insert((*variable).into(), std::str::from_utf8(value)?.into());
                    rest = next;
                }
                ensure!(rest.is_empty(), "extra context fields");
                Reply::Setup(target::Reply::Context(target::Facts { cwd, uid, environment }))
            }
            Request::Setup(target::Request::Entry { name, .. }) => Reply::Setup(target::Reply::Entry(entry(payload, name.as_encoded_bytes())?)),
            Request::Setup(target::Request::Registry { .. }) => {
                ensure!(!payload.contains(&0), "NUL in registry text");
                Reply::Setup(target::Reply::Registry(std::str::from_utf8(payload)?.into()))
            }
            Request::Workflow(workflow::ObservationRequest::Projects { argument, .. }) => {
                let (root, listing) = field(payload)?;
                let root = PathBuf::from(std::str::from_utf8(strip_newline(root)?)?);
                ensure!(root.is_absolute(), "nonabsolute canonical root");
                let mut names = Vec::new();
                for (name, kind) in entries(listing)? {
                    let Ok(name) = std::str::from_utf8(name) else {
                        continue;
                    };
                    if name.contains(argument) && kind.checked()?.directory() {
                        names.push(name.into());
                    }
                }
                names.sort();
                Reply::Workflow(workflow::ObservationReply::Projects(ProjectInventory { root, names }))
            }
            Request::Workflow(workflow::ObservationRequest::Destination { path }) => {
                let name = path.file_name().ok_or_else(|| anyhow::anyhow!("destination has no basename"))?;
                let state = if entry(payload, name.as_encoded_bytes())?.is_some() {
                    DestinationState::Present
                } else {
                    DestinationState::Absent
                };
                Reply::Workflow(workflow::ObservationReply::Destination(state))
            }
            _ => unreachable!("sessions/defaults handled above"),
        })
    }
}

fn scaffold_defaults(author: Option<String>, license: Option<String>) -> crate::scaffold::Defaults {
    crate::scaffold::Defaults { author, license }
}

fn listing(path: &Path) -> Result<String> {
    // The sentinel checks actual -printf support even for an empty directory.
    // This is not `ls`: basenames and both lstat/stat kinds are NUL-delimited.
    Ok(format!("LC_ALL=C find -H {} -maxdepth 1 -printf '%f\\0%y\\0%Y\\0'", quote(path.as_os_str())?))
}

fn strip_newline(bytes: &[u8]) -> Result<&[u8]> {
    bytes.strip_suffix(b"\n").ok_or_else(|| anyhow::anyhow!("missing pwd/id newline"))
}

fn field(bytes: &[u8]) -> Result<(&[u8], &[u8])> {
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| anyhow::anyhow!("truncated NUL-delimited probe field"))?;
    Ok((&bytes[..end], &bytes[end + 1..]))
}

fn entries(mut bytes: &[u8]) -> Result<Vec<(&[u8], target::Entry)>> {
    let mut result = Vec::new();
    while !bytes.is_empty() {
        let (name, rest) = field(bytes)?;
        let (kind, rest) = field(rest)?;
        let (referent, rest) = field(rest)?;
        ensure!(
            kind.len() == 1 && referent.len() == 1 && b"bcdpflsD".contains(&kind[0]) && b"bcdpflsDNL?".contains(&referent[0]),
            "invalid find type fields"
        );
        ensure!(!name.is_empty() && (result.is_empty() || !name.contains(&b'/')), "invalid directory basename");
        result.push((
            name,
            target::Entry {
                kind: kind[0],
                referent: referent[0],
            },
        ));
        bytes = rest;
    }
    // -maxdepth 1 prints the parent first, including for empty directories.
    ensure!(!result.is_empty() && result[0].1.directory(), "missing/invalid find parent sentinel");
    result.remove(0);
    let mut seen = std::collections::BTreeSet::new();
    for (name, _) in &result {
        ensure!(seen.insert(*name), "duplicate basename in directory reply");
    }
    Ok(result)
}

fn entry(payload: &[u8], name: &[u8]) -> Result<Option<target::Entry>> {
    Ok(entries(payload)?.into_iter().find(|(candidate, _)| *candidate == name).map(|(_, kind)| kind))
}

fn unique(bytes: &[u8], needle: &[u8]) -> Result<usize> {
    let mut matches = bytes
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(index, _)| index);
    let first = matches.next().ok_or_else(|| anyhow::anyhow!("missing/truncated probe envelope"))?;
    ensure!(matches.next().is_none(), "duplicate probe envelope boundary");
    Ok(first)
}

fn unhex(bytes: &[u8]) -> Result<Vec<u8>> {
    let hex: Vec<_> = bytes.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
    ensure!(hex.len().is_multiple_of(2), "truncated hex probe metadata");
    hex.as_chunks::<2>().0.iter()
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16).ok_or_else(|| anyhow::anyhow!("invalid hex probe metadata"))?;
            let lo = (pair[1] as char).to_digit(16).ok_or_else(|| anyhow::anyhow!("invalid hex probe metadata"))?;
            Ok(((hi << 4) | lo) as u8)
        })
        .collect()
}

/// Shared by the local CLI adapter and framed remote probes. Only exact
/// C-locale diagnostics for the selected endpoint classify server absence.
pub fn decode_sessions(status: i32, stdout: &[u8], stderr: &[u8], socket: &Path) -> Result<workflow::SessionInventory> {
    use workflow::SessionInventory;
    if status != 0 {
        let path = socket.to_str().ok_or_else(|| anyhow::anyhow!("socket diagnostic requires UTF-8"))?;
        if status == 1 && stdout.is_empty() {
            if stderr == b"no sessions\n" {
                return Ok(SessionInventory::Present(Vec::new()));
            }
            for expected in [
                format!("no server running on {path}\n"),
                format!("error connecting to {path} (No such file or directory)\n"),
                format!("error connecting to {path} (Connection refused)\n"),
            ] {
                if stderr == expected.as_bytes() {
                    return Ok(SessionInventory::Absent);
                }
            }
        }
        bail!("tmux observation failed ({status}): {}", String::from_utf8_lossy(stderr));
    }
    let text = std::str::from_utf8(stdout)?;
    let query = tmax::query::Sessions::new();
    let mut decoder = query.decoder();
    let mut rows = Vec::new();
    if !text.is_empty() {
        // A truncated final row is not a complete response, even with exit 0.
        let text = text.strip_suffix('\n').ok_or_else(|| anyhow::anyhow!("unterminated tmux session row"))?;
        for line in text.split('\n') {
            if let Some(row) = decoder.push(line)? {
                rows.push(row);
            }
        }
    }
    decoder.finish()?;
    Ok(SessionInventory::Present(
        tmax::query::Sessions::collect(rows)?.into_iter().map(Session::from).collect(),
    ))
}
