//! Worker-free read codecs for the owned protocol. Framing/classification are
//! shared mechanics, not conversions through a former entry orchestrator.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use tmax::operation::TypedOperation;
use tta_command::{Assignment, Command, Expression, Handoff, Word};

use super::{context, protocol::*, registry};
use crate::probe::{self as wire, entry, field, quote, strip_newline};

/// The local CLI interpreter uses exactly the same strict session decoder.
pub fn session_inventory(status: i32, stdout: &[u8], stderr: &[u8], socket: &Path) -> Result<SessionInventory> {
    Ok(match wire::session_facts(status, stdout, stderr, socket)? {
        None => SessionInventory::Absent,
        Some(sessions) => SessionInventory::Present(sessions),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reply {
    Observation(Observation),
    Registry(registry::Reply),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decoded {
    pub reply: Reply,
    pub diagnostics: wire::Diagnostics,
}

#[derive(Clone, Debug)]
pub struct Probe {
    pub request: ReadRequest,
    step: Option<registry::Step>,
    token: String,
}

impl Probe {
    pub fn new(request: ReadRequest, token: impl Into<String>) -> Result<Self> {
        ensure!(
            !matches!(request.kind, ReadKind::Registry { .. }),
            "registry reads require their bounded acquisition task"
        );
        Self::build(request, None, token.into())
    }
    pub fn registry(request: ReadRequest, step: registry::Step, token: impl Into<String>) -> Result<Self> {
        let ReadKind::Registry { path } = &request.kind else {
            anyhow::bail!("not a registry request")
        };
        if let registry::Step::Content { path: content } = &step {
            ensure!(Some(content) == path.as_ref(), "registry content path mismatch");
        }
        Self::build(request, Some(step), token.into())
    }
    fn build(request: ReadRequest, step: Option<registry::Step>, token: String) -> Result<Self> {
        wire::validate_token(&token)?;
        Ok(Self { request, step, token })
    }
    pub fn script(&self) -> Result<String> {
        wire::frame(&self.token, &self.operation()?)
    }
    pub fn shell(&self) -> Result<String> {
        let context::Origin::Ssh { setup, .. } = &self.request.scope.origin else {
            anyhow::bail!("SSH shell requested for a local read")
        };
        setup.shell_command(&self.script()?)
    }

    fn operation(&self) -> Result<String> {
        if let Some(step) = &self.step {
            return match step {
                registry::Step::Entry { parent, .. } => wire::listing(parent),
                registry::Step::Content { path } => Ok(format!("head -c {} -- {}", registry::TEXT_LIMIT + 1, quote(path.as_os_str())?)),
            };
        }
        Ok(match &self.request.kind {
            ReadKind::TargetContext => {
                let mut script = "pwd -P\nprintf '\\000'\nid -u\nprintf '\\000'\n".to_owned();
                for variable in context::VARIABLES {
                    script.push_str(&format!("printf '%s\\000' \"${{{variable}-}}\"\n"));
                }
                script
            }
            ReadKind::Sessions { endpoint, program, .. } => {
                ensure!(endpoint.target == self.request.target, "session endpoint target mismatch");
                let mut args = vec![Word::new("-u")?, Word::new("-S")?, Word::new(endpoint.socket.as_os_str())?];
                args.extend(
                    tmax::query::Sessions::new()
                        .invocation()?
                        .argv()?
                        .into_iter()
                        .map(Word::new)
                        .collect::<std::result::Result<Vec<_>, _>>()?,
                );
                Expression::from(
                    Command::builder()
                        .program(Word::new(program.clone())?)
                        .args(args)
                        .environment(vec![Assignment::new("LC_ALL", Word::new("C")?)?])
                        .build(),
                )
                .shell(Handoff::Run)?
            }
            ReadKind::Projects { root, .. } => format!(
                "cd -P {}\npwd -P\nprintf '\\000'\n{}",
                quote(root.as_os_str())?,
                wire::project_listing(Path::new("."))?
            ),
            ReadKind::Destination { path, .. } => wire::listing(path.parent().ok_or_else(|| anyhow::anyhow!("destination has no parent"))?)?,
            ReadKind::GeneratorDefaults { cwd, needs } => {
                let mut script = format!(
                    "cd -P {}\nprintf '%s\\000%s\\000' \"{}\" \"{}\"\n",
                    quote(cwd.as_os_str())?,
                    if needs.license { "${LICENSE-}" } else { "" },
                    if needs.author { "${AUTHOR-}" } else { "" }
                );
                if needs.author {
                    script.push_str("if [ -z \"${AUTHOR-}\" ]; then git config --get user.name; fi\n");
                }
                script
            }
            ReadKind::ProjectIdentity { route } => format!(
                "cd -P {}\npwd -P\nprintf '\\000'",
                quote(route.requested_root.join(&route.name).into_os_string())?
            ),
            ReadKind::CreationRootIdentity { root, .. } => format!("cd -P {}\npwd -P\nprintf '\\000'", quote(root.as_os_str())?),
            ReadKind::EndpointIdentity { .. } => anyhow::bail!("endpoint comparison is a local-only acquisition"),
            ReadKind::Registry { .. } => unreachable!("registry task step required at construction"),
        })
    }

    pub fn decode(&self, output: wire::Output<'_>) -> Result<Decoded> {
        let frame = wire::envelope(&self.token, output)?;
        let status = frame.diagnostics.command_status;
        let payload = frame.payload;
        let stderr = &frame.diagnostics.command_stderr;
        let reply = if let Some(step) = &self.step {
            ensure!(status == 0, "registry probe failed ({status}): {}", String::from_utf8_lossy(stderr));
            Reply::Registry(match step {
                registry::Step::Entry { name, .. } => registry::Reply::Entry(entry(payload, name.as_encoded_bytes(), true)?),
                registry::Step::Content { .. } => {
                    ensure!(
                        payload.len() <= registry::TEXT_LIMIT && !payload.contains(&0),
                        "registry exceeds text limit or contains NUL"
                    );
                    registry::Reply::Content(std::str::from_utf8(payload)?.into())
                }
            })
        } else {
            Reply::Observation(self.decode_fact(status, payload, stderr)?)
        };
        Ok(Decoded {
            reply,
            diagnostics: frame.diagnostics,
        })
    }

    fn decode_fact(&self, status: i32, payload: &[u8], stderr: &[u8]) -> Result<Observation> {
        if let ReadKind::Sessions { endpoint, .. } = &self.request.kind {
            return Ok(Observation::Sessions(match wire::session_facts(status, payload, stderr, &endpoint.socket)? {
                None => SessionInventory::Absent,
                Some(sessions) => SessionInventory::Present(sessions),
            }));
        }
        if let ReadKind::GeneratorDefaults { .. } = &self.request.kind {
            let (license, rest) = field(payload)?;
            let (author, git) = field(rest)?;
            let unset = status == 1 && author.is_empty() && git.is_empty() && stderr.is_empty();
            ensure!(
                status == 0 || unset,
                "Git defaults probe failed ({status}): {}",
                String::from_utf8_lossy(stderr)
            );
            let value = |v: &[u8]| -> Result<Option<String>> {
                ensure!(!v.contains(&0), "NUL in generator default");
                Ok(Some(std::str::from_utf8(v)?.to_owned()).filter(|v| !v.is_empty()))
            };
            return Ok(Observation::Defaults(GeneratorDefaults {
                author: if author.is_empty() {
                    value(std::str::from_utf8(git)?.trim().as_bytes())?
                } else {
                    value(author)?
                },
                license: value(license)?,
            }));
        }
        ensure!(status == 0, "probe command failed ({status}): {}", String::from_utf8_lossy(stderr));
        Ok(match &self.request.kind {
            ReadKind::TargetContext => {
                let (cwd, rest) = field(payload)?;
                let cwd = PathBuf::from(OsString::from_vec(strip_newline(cwd)?.to_vec()));
                ensure!(cwd.is_absolute(), "target cwd is not absolute");
                let (uid, mut rest) = field(rest)?;
                let uid = std::str::from_utf8(strip_newline(uid)?)?.parse()?;
                let mut environment = BTreeMap::new();
                for variable in context::VARIABLES {
                    let (value, next) = field(rest)?;
                    environment.insert((*variable).into(), OsString::from_vec(value.to_vec()));
                    rest = next;
                }
                ensure!(rest.is_empty(), "extra context fields");
                Observation::Context(context::Facts { cwd, uid, environment })
            }
            ReadKind::Projects {
                root: requested_root, filter, ..
            } => {
                let (root, listing) = field(payload)?;
                let root = PathBuf::from(std::str::from_utf8(strip_newline(root)?)?);
                ensure!(root.is_absolute(), "nonabsolute physical root");
                let mut entries = Vec::new();
                for found in wire::entries(listing, true)? {
                    let Ok(name) = std::str::from_utf8(found.name) else { continue };
                    if !name.contains(filter) {
                        continue;
                    }
                    found.check()?;
                    if found.kind.checked()?.directory() {
                        let directory = PathBuf::from(std::str::from_utf8(strip_newline(found.physical)?)?);
                        ensure!(directory.is_absolute(), "nonabsolute physical project directory");
                        entries.push(crate::ProjectEntry { name: name.into(), directory });
                    }
                }
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                Observation::Projects(crate::ProjectInventory {
                    requested_root: requested_root.clone(),
                    root,
                    entries,
                })
            }
            ReadKind::Destination { path, .. } => {
                let name = path.file_name().ok_or_else(|| anyhow::anyhow!("destination has no basename"))?;
                Observation::Destination(if entry(payload, name.as_encoded_bytes(), false)?.is_some() {
                    DestinationEvidence::Present
                } else {
                    DestinationEvidence::Absent
                })
            }
            ReadKind::ProjectIdentity { .. } | ReadKind::CreationRootIdentity { .. } => {
                let (path, rest) = field(payload)?;
                ensure!(rest.is_empty(), "extra physical-route fields");
                let path = PathBuf::from(std::str::from_utf8(strip_newline(path)?)?);
                ensure!(path.is_absolute(), "nonabsolute physical route");
                Observation::ProjectIdentity(path)
            }
            ReadKind::Registry { .. } | ReadKind::EndpointIdentity { .. } | ReadKind::Sessions { .. } | ReadKind::GeneratorDefaults { .. } => {
                unreachable!("handled or rejected above")
            }
        })
    }
}
