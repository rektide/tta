//! Version-1, nonce-bound binary probe envelopes. Construction and decoding are
//! pure: external callers can run these scripts with no tta runner or worker.
//! Profile: POSIX sh, id, pwd, cat, od, head; GNU find (`-printf`, `%Y`) for
//! filesystem facts and GNU stat for symlink referents; Git only for an actually
//! needed, unset author default. GNU find's `%Y=N` alone is NOT proof of ENOENT.
use std::path::Path;

use anyhow::{Result, bail, ensure};
use tmax::operation::{ReplyDecoder, TypedOperation};
use tta_command::Word;

use crate::{Session, target};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Diagnostics {
    pub startup_stdout: Vec<u8>,
    pub trailing_stdout: Vec<u8>,
    pub transport_stderr: Vec<u8>,
    pub command_stderr: Vec<u8>,
    pub command_status: i32,
}

/// The carrier supplies real status and EOF completeness. `None` is signal or
/// unknown termination, not a successful probe. Capture omissions mean false.
pub struct Output<'a> {
    pub stdout: &'a [u8],
    pub stderr: &'a [u8],
    pub status: Option<i32>,
    pub complete: bool,
}

// Shared framing mechanics. Neither codec owns token generation or runs I/O.
pub(crate) fn validate_token(token: &str) -> Result<()> {
    ensure!(
        (8..=128).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "invalid probe token"
    );
    Ok(())
}

pub(crate) fn frame(token: &str, operation: &str) -> Result<String> {
    validate_token(token)?;
    Ok(format!(
        "printf '\\000TTA-PROBE/1/{token}/BEGIN\\000'\ntta_meta=$( {{ (\nset -e\n{operation}\n) 2>&1 1>&3; tta_rc=$?; printf '\\000%s' \"$tta_rc\"; }} | od -An -v -tx1 | head -c 786432 ) 3>&1\ntta_capture=$?\nprintf '\\000TTA-PROBE/1/{token}/END\\000%s\\000' \"$tta_meta\"\nexit \"$tta_capture\""
    ))
}

pub(crate) struct Envelope<'a> {
    pub(crate) payload: &'a [u8],
    pub(crate) diagnostics: Diagnostics,
}

pub(crate) fn envelope<'a>(token: &str, output: Output<'a>) -> Result<Envelope<'a>> {
    ensure!(output.complete, "incomplete probe capture (EOF/deadline/output cap)");
    ensure!(
        output.status == Some(0),
        "probe carrier failed with {:?}: {}",
        output.status,
        String::from_utf8_lossy(output.stderr)
    );
    let header = format!("\0TTA-PROBE/1/{token}/BEGIN\0").into_bytes();
    let footer = format!("\0TTA-PROBE/1/{token}/END\0").into_bytes();
    let begin = unique(output.stdout, &header)?;
    let end = unique(output.stdout, &footer)?;
    ensure!(end >= begin + header.len(), "probe envelope has inverted boundaries");
    let payload = &output.stdout[begin + header.len()..end];
    let tail = &output.stdout[end + footer.len()..];
    let meta_end = tail.iter().position(|b| *b == 0).ok_or_else(|| anyhow::anyhow!("truncated probe metadata"))?;
    let (stderr, status) = metadata(&tail[..meta_end])?;
    Ok(Envelope {
        payload,
        diagnostics: Diagnostics {
            startup_stdout: output.stdout[..begin].to_vec(),
            trailing_stdout: tail[meta_end + 1..].to_vec(),
            transport_stderr: output.stderr.to_vec(),
            command_stderr: stderr,
            command_status: status,
        },
    })
}

pub(crate) fn quote(value: impl Into<std::ffi::OsString>) -> Result<String> {
    Ok(Word::new(value)?.shell()?)
}

pub(crate) fn listing(path: &Path) -> Result<String> {
    // The sentinel checks actual -printf support even for an empty directory.
    // This is not `ls`: basenames and both lstat/stat kinds are NUL-delimited.
    // A single-operand stat gets its OWN stderr/status frame. GNU find's %Y=N
    // conflates ENOENT with ENOTDIR. Only the former may be skipped. -exec's
    // predicate status is not used as evidence; all outcomes are serialized.
    let stat = "tta_meta=$( { LC_ALL=C stat -L --printf='%f' -- \"$1\" 2>&1 1>&3; tta_rc=$?; printf '\\000%s' \"$tta_rc\"; } | od -An -v -tx1 | head -c 786432 ) 3>&1\nprintf '\\000%s\\000' \"$tta_meta\"";
    Ok(format!(
        "LC_ALL=C find -H {} -maxdepth 1 -printf '%f\\0%y\\0%Y\\0' \\( -type l -exec /bin/sh -c {} tta-stat {{}} \\; -o -printf '\\0\\0' \\)",
        quote(path.as_os_str())?,
        quote(stat)?
    ))
}

pub(crate) fn project_listing(path: &Path) -> Result<String> {
    // A physical path is a separate NUL field, not an unquoted shell operand.
    // The decoder requires it for every directory in the type/stat inventory:
    // failed/racing cd or pwd can never silently erase a project candidate.
    let physical = "if [ -d \"$1\" ]; then (cd -P \"$1\" && pwd -P); fi\nprintf '\\000'";
    Ok(format!("{} -exec /bin/sh -c {} tta-physical {{}} \\;", listing(path)?, quote(physical)?))
}

pub(crate) fn strip_newline(bytes: &[u8]) -> Result<&[u8]> {
    bytes.strip_suffix(b"\n").ok_or_else(|| anyhow::anyhow!("missing pwd/id newline"))
}

pub(crate) fn field(bytes: &[u8]) -> Result<(&[u8], &[u8])> {
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| anyhow::anyhow!("truncated NUL-delimited probe field"))?;
    Ok((&bytes[..end], &bytes[end + 1..]))
}

pub(crate) struct Found<'a> {
    pub(crate) name: &'a [u8],
    pub(crate) kind: target::Entry,
    failure: Option<String>,
    pub(crate) physical: &'a [u8],
}

impl Found<'_> {
    pub(crate) fn check(&self) -> Result<()> {
        if let Some(failure) = &self.failure {
            bail!("cannot inspect symlink {:?}: {failure}", String::from_utf8_lossy(self.name));
        }
        Ok(())
    }
}

pub(crate) fn entries(mut bytes: &[u8], physical: bool) -> Result<Vec<Found<'_>>> {
    let mut result = Vec::new();
    while !bytes.is_empty() {
        let (name, rest) = field(bytes)?;
        let (kind, rest) = field(rest)?;
        let (referent, rest) = field(rest)?;
        let (mode, rest) = field(rest)?;
        let (meta, rest) = field(rest)?;
        let (physical, rest) = if physical { field(rest)? } else { (&[][..], rest) };
        ensure!(
            kind.len() == 1 && referent.len() == 1 && b"bcdpflsD".contains(&kind[0]) && b"bcdpflsDNL?".contains(&referent[0]),
            "invalid find type fields"
        );
        ensure!(!name.is_empty() && (result.is_empty() || !name.contains(&b'/')), "invalid directory basename");
        let mut found = Found {
            name,
            kind: target::Entry {
                kind: kind[0],
                referent: referent[0],
            },
            failure: None,
            physical,
        };
        if kind == b"l" {
            let (stderr, status) = metadata(meta)?;
            if status == 0 {
                let mode = u32::from_str_radix(std::str::from_utf8(mode)?, 16)?;
                ensure!(mode & 0o170000 != 0, "invalid GNU stat mode");
                found.kind.referent = if mode & 0o170000 == 0o040000 { b'd' } else { b'f' };
            } else if status == 1 && mode.is_empty() && stat_not_found(&stderr) {
                found.kind.referent = b'N';
            } else {
                found.kind.referent = b'?';
                found.failure = Some(format!("GNU stat failed ({status}): {}", String::from_utf8_lossy(&stderr)));
            }
        } else {
            ensure!(mode.is_empty() && meta.is_empty(), "unexpected symlink metadata for a non-symlink");
        }
        result.push(found);
        bytes = rest;
    }
    // -maxdepth 1 prints the parent first, including for empty directories.
    ensure!(!result.is_empty() && result[0].kind.directory(), "missing/invalid find parent sentinel");
    result.remove(0);
    let mut seen = std::collections::BTreeSet::new();
    for found in &result {
        ensure!(seen.insert(found.name), "duplicate basename in directory reply");
    }
    Ok(result)
}

pub(crate) fn entry(payload: &[u8], name: &[u8], follow: bool) -> Result<Option<target::Entry>> {
    let Some(found) = entries(payload, false)?.into_iter().find(|found| found.name == name) else {
        return Ok(None);
    };
    if follow {
        found.check()?;
    }
    Ok(Some(found.kind))
}

fn metadata(bytes: &[u8]) -> Result<(Vec<u8>, i32)> {
    ensure!(bytes.len() < 786432, "probe stderr metadata reached its cap");
    let meta = unhex(bytes)?;
    let split = meta
        .iter()
        .rposition(|b| *b == 0)
        .ok_or_else(|| anyhow::anyhow!("missing command status (possibly capped stderr)"))?;
    let status = std::str::from_utf8(&meta[split + 1..])?.parse::<i32>()?;
    ensure!((0..=255).contains(&status), "invalid probe command status");
    Ok((meta[..split].to_vec(), status))
}

fn stat_not_found(stderr: &[u8]) -> bool {
    // This is one fixed GNU stat invocation with exactly one literal operand,
    // LC_ALL=C, no stdout, and status 1. GNU's diagnostic quoting escapes path
    // newlines, so exactly one diagnostic line ends in the actual errno text.
    // Unknown versions/messages and ENOTDIR/EACCES/ELOOP fail closed. Unlike a
    // shell test or %Y=N this is affirmative errno evidence, not a false boolean.
    (stderr.starts_with(b"stat: cannot statx ") || stderr.starts_with(b"stat: cannot stat "))
        && stderr.ends_with(b": No such file or directory\n")
        && stderr.iter().filter(|byte| **byte == b'\n').count() == 1
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
    hex.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16).ok_or_else(|| anyhow::anyhow!("invalid hex probe metadata"))?;
            let lo = (pair[1] as char).to_digit(16).ok_or_else(|| anyhow::anyhow!("invalid hex probe metadata"))?;
            Ok(((hi << 4) | lo) as u8)
        })
        .collect()
}

pub(crate) fn session_facts(status: i32, stdout: &[u8], stderr: &[u8], socket: &Path) -> Result<Option<Vec<Session>>> {
    if status != 0 {
        let path = socket.to_str().ok_or_else(|| anyhow::anyhow!("socket diagnostic requires UTF-8"))?;
        if status == 1 && stdout.is_empty() {
            if stderr == b"no sessions\n" {
                return Ok(Some(Vec::new()));
            }
            for expected in [
                format!("no server running on {path}\n"),
                format!("error connecting to {path} (No such file or directory)\n"),
                format!("error connecting to {path} (Connection refused)\n"),
            ] {
                if stderr == expected.as_bytes() {
                    return Ok(None);
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
    Ok(Some(tmax::query::Sessions::collect(rows)?.into_iter().map(Session::from).collect()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn single_operand_stat_requires_classified_errno_not_failed_boolean_or_find_n() {
        assert!(super::stat_not_found(b"stat: cannot statx './missing': No such file or directory\n"));
        for diagnostic in [
            &b"stat: cannot statx './file/child': Not a directory\n"[..],
            b"stat: cannot statx './blocked/child': Permission denied\n",
            b"stat: cannot statx './loop': Too many levels of symbolic links\n",
            b"unknown tool error: No such file or directory\n",
            b"stat: cannot statx './missing': No such file or directory\nextra diagnostic\n",
        ] {
            assert!(!super::stat_not_found(diagnostic));
        }
    }
}
