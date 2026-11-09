//! Local filesystem/environment/session observation, not an entry program.
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, ensure};
#[cfg(feature = "cli")]
use tmax::operation::TypedOperation;
use tta_launcher::entry::{context, protocol::*, registry};

use super::{Guard, Observer};

pub trait Sessions {
    fn inventory(&mut self, program: &OsStr, socket: &Path) -> Result<SessionInventory>;
}

/// Failed local command reads retain bounded bytes/status/EOF knowledge as
/// native data, and keep the original decoding error in their source chain.
#[derive(Debug, thiserror::Error)]
#[error("local {operation} observation failed: {source}")]
pub struct CaptureFailure {
    pub operation: &'static str,
    #[source]
    pub source: anyhow::Error,
    pub output: Box<crate::capture::Output>,
}

#[derive(Debug, thiserror::Error)]
#[error("{variable} must be UTF-8")]
pub struct EnvironmentUtf8 {
    pub variable: &'static str,
    pub value: std::ffi::OsString,
}

fn captured<T>(operation: &'static str, output: crate::capture::Output, decode: impl FnOnce(&crate::capture::Output) -> Result<T>) -> Result<T> {
    let result = output.require_complete().and_then(|()| decode(&output));
    result.map_err(|source| {
        CaptureFailure {
            operation,
            source,
            output: Box::new(output),
        }
        .into()
    })
}

pub enum SessionBackend {
    #[cfg(feature = "cli")]
    Cli,
    #[cfg(feature = "native")]
    Native,
}

impl Sessions for SessionBackend {
    fn inventory(&mut self, program: &OsStr, socket: &Path) -> Result<SessionInventory> {
        // Keep native endpoint errors native; no CLI fallback after any failure.
        let _ = (program, socket);
        match *self {
            #[cfg(feature = "native")]
            Self::Native => {
                let _ = program;
                match tmax::endpoint::Tmux::new().socket_path(socket).query(tmax::query::Sessions::new()) {
                    Ok(sessions) => Ok(SessionInventory::Present(sessions.into_iter().map(Into::into).collect())),
                    Err(error) if crate::backend::absent_server(&error) => Ok(if matches!(error, tmax::error::OperationError::Rejected(_)) {
                        SessionInventory::Present(Vec::new())
                    } else {
                        SessionInventory::Absent
                    }),
                    Err(error) => Err(error.into()),
                }
            }
            #[cfg(feature = "cli")]
            Self::Cli => {
                let mut command = std::process::Command::new(program);
                command
                    .arg("-u")
                    .arg("-S")
                    .arg(socket)
                    .args(tmax::query::Sessions::new().invocation()?.argv()?)
                    .env("LC_ALL", "C");
                let output = crate::capture::run(&mut command, crate::capture::Limits::default())?;
                captured("tmux sessions", output, |output| {
                    tta_launcher::entry::probe::session_inventory(
                        output
                            .status
                            .and_then(|s| s.code())
                            .ok_or_else(|| anyhow::anyhow!("tmux session read terminated without exit status"))?,
                        &output.stdout,
                        &output.stderr,
                        socket,
                    )
                })
            }
        }
    }
}

pub struct Local<'a> {
    guard: Guard,
    sessions: &'a mut dyn Sessions,
}

impl<'a> Local<'a> {
    pub fn acquire(scope: context::Scope, sessions: &'a mut dyn Sessions) -> Result<Self> {
        let guard = Guard::new(scope)?;
        ensure!(guard.is_local(), "local interpreter requires local context");
        Ok(Self { guard, sessions })
    }
    pub fn supplied(target: &BoundTarget, sessions: &'a mut dyn Sessions) -> Result<Self> {
        let guard = Guard::supplied(target);
        ensure!(guard.is_local(), "local interpreter requires local context");
        Ok(Self { guard, sessions })
    }
}

impl Observer for Local<'_> {
    fn observe(&mut self, request: &ReadRequest) -> Result<Observation> {
        self.guard.check(request)?;
        let observation = match &request.kind {
            ReadKind::TargetContext => Observation::Context(context::Facts {
                cwd: std::env::current_dir()?,
                uid: rustix::process::geteuid().as_raw(),
                environment: context::VARIABLES
                    .iter()
                    .filter_map(|name| std::env::var_os(name).map(|v| ((*name).into(), v)))
                    .collect(),
            }),
            ReadKind::Registry { path } => {
                let mut task = registry::Task::new(path.clone())?;
                while let Some(step) = task.step().cloned() {
                    task.accept(&step, registry_step(&step)?)?;
                }
                Observation::Registry(task.result().expect("finished acquisition").clone())
            }
            ReadKind::EndpointIdentity { current, selected } => {
                // Preserve supplied endpoint identity policy: failed canonicalize
                // retains its lexical path; it never certifies equal distinct paths.
                let identity = |path: &std::path::PathBuf| path.canonicalize().unwrap_or_else(|_| path.clone());
                Observation::EndpointIdentity {
                    current: identity(current),
                    selected: identity(selected),
                }
            }
            ReadKind::Sessions { endpoint, program, .. } => Observation::Sessions(self.sessions.inventory(program, &endpoint.socket)?),
            ReadKind::Projects { root, filter, .. } => {
                let mut inventory = crate::resolve::project_inventory(root, filter)?;
                inventory.entries.sort_by(|a, b| a.name.cmp(&b.name));
                Observation::Projects(inventory)
            }
            ReadKind::Destination { path, .. } => Observation::Destination(match crate::resolve::destination_state(path)? {
                tta_launcher::DestinationState::Absent => DestinationEvidence::Absent,
                tta_launcher::DestinationState::Present => DestinationEvidence::Present,
            }),
            ReadKind::GeneratorDefaults { cwd, needs } => Observation::Defaults(defaults(cwd, *needs)?),
            ReadKind::ProjectIdentity { route } => {
                let entry = route.requested_root.join(&route.name);
                let path = entry.canonicalize().with_context(|| format!("canonicalize chosen project route {entry:?}"))?;
                ensure!(path.metadata()?.is_dir(), "chosen project route is not a directory: {entry:?}");
                Observation::ProjectIdentity(path)
            }
        };
        self.guard.accept(&observation)?;
        Ok(observation)
    }
}

fn defaults(cwd: &Path, needs: DefaultNeeds) -> Result<GeneratorDefaults> {
    let value = |variable: &'static str| -> Result<Option<String>> {
        std::env::var_os(variable)
            .filter(|v| !v.is_empty())
            .map(|value| value.into_string().map_err(|value| EnvironmentUtf8 { variable, value }.into()))
            .transpose()
    };
    let author = if !needs.author {
        None
    } else if let Some(author) = value("AUTHOR")? {
        Some(author)
    } else {
        let mut command = std::process::Command::new("git");
        command.current_dir(cwd).args(["config", "--get", "user.name"]);
        let output = crate::capture::run(&mut command, crate::capture::Limits::default()).context("observe Git author default")?;
        captured("Git author", output, |output| {
            let status = output.status.expect("complete capture has status");
            if status.success() {
                Ok(Some(std::str::from_utf8(&output.stdout)?.trim().to_owned()).filter(|v| !v.is_empty()))
            } else if status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty() {
                Ok(None)
            } else {
                anyhow::bail!("Git author observation failed ({status})");
            }
        })?
    };
    Ok(GeneratorDefaults {
        author,
        license: if needs.license { value("LICENSE")? } else { None },
    })
}

pub(super) fn registry_step(step: &registry::Step) -> Result<registry::Reply> {
    Ok(match step {
        registry::Step::Content { path } => {
            let mut bytes = Vec::new();
            std::fs::File::open(path)?.take((registry::TEXT_LIMIT + 1) as u64).read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= registry::TEXT_LIMIT && !bytes.contains(&0),
                "registry exceeds text limit or contains NUL"
            );
            registry::Reply::Content(String::from_utf8(bytes)?)
        }
        registry::Step::Entry { parent, name } => {
            let mut found = None;
            for entry in std::fs::read_dir(parent)? {
                let entry = entry?;
                if entry.file_name() != *name {
                    continue;
                }
                let kind = entry.file_type()?;
                let kind = if kind.is_dir() {
                    b'd'
                } else if kind.is_symlink() {
                    b'l'
                } else {
                    b'f'
                };
                let referent = if kind != b'l' {
                    kind
                } else {
                    match std::fs::metadata(entry.path()) {
                        Ok(metadata) => {
                            if metadata.is_dir() {
                                b'd'
                            } else {
                                b'f'
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => b'N',
                        Err(error) => return Err(error.into()),
                    }
                };
                found = Some(tta_launcher::target::Entry { kind, referent });
            }
            registry::Reply::Entry(found)
        }
    })
}
