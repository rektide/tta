//! Target-local shell setup and inspectable context/registry acquisition. This
//! module never reads the environment or resolves a path on the calling host.
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail, ensure};
use bon::Builder;
use tta_command::{Assignment, Handoff, Word};

use crate::{ClientMode, WindowName, scaffold, socket, workflow};

/// Applied *after* the selected shell's startup. `None` explicitly unsets a
/// variable; an empty string is a real value. These are literal data, not code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Environment {
    pub name: String,
    pub value: Option<String>,
}

/// Reused for every probe and terminal action. With no explicit shell the
/// target's SHELL (then /bin/sh) is selected, never the caller's local SHELL.
#[derive(Clone, Debug, Default, Eq, PartialEq, Builder)]
pub struct TargetSetup {
    pub shell: Option<String>,
    #[builder(default)]
    pub login: bool,
    #[builder(default)]
    pub interactive: bool,
    #[builder(default)]
    pub environment: Vec<Environment>,
    pub cwd: Option<PathBuf>,
}

fn quote(value: impl Into<OsString>) -> Result<String> {
    Ok(Word::new(value)?.shell()?)
}

fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn nu_quote(value: &str) -> String {
    use std::fmt::Write;
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            ch if ch.is_control() => write!(&mut out, "\\u{{{:x}}}", ch as u32).unwrap(),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

impl TargetSetup {
    /// A POSIX-compatible SSH command shell carries the selected shell. Fish
    /// and Nu startup are supported; the actual probe/action language is sh.
    /// A new SSH terminal is always outside tmux: startup cannot supply a local
    /// client identity, and TMUX/TMUX_PANE are cleared after explicit overrides.
    pub fn shell_command(&self, source: &str) -> Result<String> {
        Word::new(source)?;
        let mut body = String::new();
        for variable in &self.environment {
            Assignment::new(&variable.name, Word::new(variable.value.as_deref().unwrap_or(""))?)?;
            match &variable.value {
                Some(value) => body.push_str(&format!("export {}={} || exit $?\n", variable.name, quote(value.as_str())?)),
                None => body.push_str(&format!("unset {} || exit $?\n", variable.name)),
            }
        }
        body.push_str("unset TMUX TMUX_PANE || exit $?\n");
        if let Some(cwd) = &self.cwd {
            // ./ prevents option/CDPATH interpretation without expanding ~.
            let path = if cwd.is_absolute() { cwd.clone() } else { Path::new(".").join(cwd) };
            body.push_str(&format!("cd -P {} || exit $?\n", quote(path.into_os_string())?));
        }
        body.push_str(source);
        let carrier = format!("exec /bin/sh -c {}", quote(body.as_str())?);
        let fish = format!("exec /bin/sh -c {}", fish_quote(&body));
        let nu = format!("^/bin/sh -c {}", nu_quote(&body));
        let shell = match &self.shell {
            Some(shell) => {
                ensure!(!shell.is_empty(), "selected target shell is empty");
                quote(shell.as_str())?
            }
            None => "\"${SHELL:-/bin/sh}\"".into(),
        };
        let mut flags = String::new();
        if self.login {
            flags.push_str(" -l");
        }
        if self.interactive {
            flags.push_str(" -i");
        }
        // Choose the family on the target even when SHELL is not known locally.
        let bootstrap = format!(
            "tta_shell={shell}\ncase \"${{tta_shell##*/}}\" in\nfish) exec \"$tta_shell\"{flags} -c {} ;;\nnu|nushell) exec \"$tta_shell\" -c {} ;;\n*) exec \"$tta_shell\"{flags} -c {} ;;\nesac",
            quote(fish)?,
            quote(nu)?,
            quote(carrier)?
        );
        Ok(format!("/bin/sh -c {}", quote(bootstrap)?))
    }

    pub fn launch(&self, prepared: &workflow::PreparedLaunch, handoff: Handoff) -> Result<String> {
        self.shell_command(&prepared.plan.expression().shell(handoff)?)
    }
}

/// Only target-observed values participate in remote defaults. Missing HOME is
/// legal for session-only entry. Empty variables follow existing unset policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Facts {
    pub cwd: PathBuf,
    pub uid: u32,
    pub environment: BTreeMap<String, String>,
}

pub const VARIABLES: &[&str] = &[
    "HOME",
    "XDG_CONFIG_HOME",
    "TTA_CONFIG",
    "TTA_ROOT",
    "PROJECTS_DIR",
    "TTA_SOCKET_NAME",
    "TTA_SOCKET_PATH",
    "TMUX_TMPDIR",
    "TTA_TMUX",
    "TTA_TMUX_CONFIG",
    "TTA_SCAFFOLD",
    "PATH",
    "SHELL",
];

/// Explicit caller values win over target environment. No backend default is
/// inherited from the local host: SSH observations/terminal actions use tmux CLI.
#[derive(Clone, Debug, Builder)]
pub struct Options {
    pub argument: String,
    pub root: Option<PathBuf>,
    pub socket: Option<socket::Selection>,
    pub program: Option<OsString>,
    pub registry_path: Option<PathBuf>,
    pub scaffold: Option<String>,
    #[builder(default = "false".into())]
    pub default_scaffold: String,
    pub config: Option<PathBuf>,
    pub session_name: Option<String>,
    #[builder(default)]
    pub grouped: bool,
    #[builder(default)]
    pub startup: Vec<OsString>,
    pub directory: Option<PathBuf>,
    #[builder(default)]
    pub window_name: WindowName,
}

impl Facts {
    pub fn value(&self, name: &str) -> Option<&str> {
        self.environment.get(name).map(String::as_str).filter(|value| !value.is_empty())
    }

    pub fn registry_path(&self, options: &Options) -> Option<PathBuf> {
        let path = options
            .registry_path
            .clone()
            .or_else(|| self.value("TTA_CONFIG").map(PathBuf::from))
            .or_else(|| {
                self.value("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .or_else(|| self.value("HOME").map(|home| Path::new(home).join(".config")))
                    .map(|config| config.join("tta/config.toml"))
            });
        path.map(|path| self.cwd.join(path))
    }

    pub fn input(&self, options: &Options, registry: scaffold::Config) -> Result<workflow::Input> {
        ensure!(self.cwd.is_absolute(), "target cwd is not absolute: {:?}", self.cwd);
        let root = options
            .root
            .clone()
            .or_else(|| self.value("TTA_ROOT").map(PathBuf::from))
            .or_else(|| {
                let projects = PathBuf::from(self.value("PROJECTS_DIR").unwrap_or("src"));
                if projects.is_absolute() {
                    Some(projects)
                } else {
                    self.value("HOME").map(|home| Path::new(home).join(projects))
                }
            })
            .map(|path| self.cwd.join(path));
        let selection = match &options.socket {
            Some(selection) => selection.clone(),
            None => match (self.value("TTA_SOCKET_NAME"), self.value("TTA_SOCKET_PATH")) {
                (Some(_), Some(_)) => bail!("target TTA_SOCKET_NAME and TTA_SOCKET_PATH conflict"),
                (Some(name), None) => socket::Selection::Name(name.into()),
                (None, Some(path)) => socket::Selection::Path(path.into()),
                _ => socket::Selection::Default,
            },
        };
        let context = socket::Context {
            cwd: self.cwd.clone(),
            uid: self.uid,
            tmux: None,
            tmpdir: self.value("TMUX_TMPDIR").map(PathBuf::from),
        };
        let mut program = options.program.clone().unwrap_or_else(|| self.value("TTA_TMUX").unwrap_or("tmux").into());
        if program.as_encoded_bytes().contains(&b'/') {
            program = self.cwd.join(program).into_os_string();
        }
        Ok(workflow::Input::builder()
            .argument(options.argument.clone())
            .cwd(self.cwd.clone())
            .socket(context.resolve(&selection)?)
            .client_mode(ClientMode::Attach)
            .program(program)
            .registry(registry)
            .scaffold(
                options
                    .scaffold
                    .clone()
                    .or_else(|| self.value("TTA_SCAFFOLD").map(str::to_owned))
                    .unwrap_or_else(|| options.default_scaffold.clone()),
            )
            .maybe_root(root)
            .maybe_session_name(options.session_name.clone())
            .maybe_config(
                options
                    .config
                    .clone()
                    .or_else(|| self.value("TTA_TMUX_CONFIG").map(PathBuf::from))
                    .map(|p| self.cwd.join(p)),
            )
            .grouped(options.grouped)
            .startup(options.startup.clone())
            .maybe_directory(options.directory.as_ref().map(|p| self.cwd.join(p)))
            .window_name(options.window_name.clone())
            .build())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Context,
    /// A successful enumeration of this parent, not a false shell `test`.
    Entry {
        parent: PathBuf,
        name: OsString,
    },
    Registry {
        path: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    pub kind: u8,
    pub referent: u8,
}

impl Entry {
    pub fn directory(self) -> bool {
        self.kind == b'd' || (self.kind == b'l' && self.referent == b'd')
    }
    pub fn dangling(self) -> bool {
        self.kind == b'l' && self.referent == b'N'
    }
    pub fn checked(self) -> Result<Self> {
        ensure!(!matches!(self.referent, b'?' | b'L'), "cannot inspect symlink referent: {self:?}");
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reply {
    Context(Facts),
    Entry(Option<Entry>),
    Registry(String),
}

#[derive(Clone, Debug)]
pub enum Progress {
    Observe(Request),
    Ready(Box<workflow::Input>),
}

/// This prelude is pure and separately driveable. Registry validation is still
/// complete before the session workflow is constructed. Path components are
/// inspected on the target; `..` is never canonicalized on the calling host.
#[derive(Clone, Debug)]
pub struct Bootstrap {
    options: Options,
    facts: Option<Facts>,
    path: Option<PathBuf>,
    parent: PathBuf,
    components: VecDeque<OsString>,
    read: bool,
    registry: Option<scaffold::Config>,
}

impl Bootstrap {
    pub fn new(options: Options) -> Self {
        Self {
            options,
            facts: None,
            path: None,
            parent: "/".into(),
            components: VecDeque::new(),
            read: false,
            registry: None,
        }
    }

    pub fn facts(&self) -> Option<&Facts> {
        self.facts.as_ref()
    }

    pub fn next(&self) -> Result<Progress> {
        let Some(facts) = &self.facts else {
            return Ok(Progress::Observe(Request::Context));
        };
        if let Some(registry) = &self.registry {
            return Ok(Progress::Ready(Box::new(facts.input(&self.options, registry.clone())?)));
        }
        if self.read {
            return Ok(Progress::Observe(Request::Registry {
                path: self.path.clone().expect("registry path"),
            }));
        }
        Ok(Progress::Observe(Request::Entry {
            parent: self.parent.clone(),
            name: self.components.front().expect("registry component").clone(),
        }))
    }

    fn missing(&mut self) -> Result<()> {
        self.registry = Some(scaffold::Config::parse(self.path.as_deref(), None)?);
        Ok(())
    }

    fn skip_parent_components(&mut self) {
        while self.components.front().is_some_and(|part| part == "..") && self.components.len() > 1 {
            self.parent.push(self.components.pop_front().unwrap());
        }
    }

    pub fn supply(&mut self, reply: Reply) -> Result<()> {
        match (self.next()?, reply) {
            (Progress::Observe(Request::Context), Reply::Context(facts)) => {
                ensure!(facts.cwd.is_absolute(), "target cwd must be absolute");
                self.path = facts.registry_path(&self.options);
                self.facts = Some(facts);
                if let Some(path) = &self.path {
                    for part in path.components() {
                        match part {
                            Component::Normal(name) => self.components.push_back(name.to_owned()),
                            Component::ParentDir => self.components.push_back("..".into()),
                            Component::RootDir | Component::CurDir => {}
                            _ => bail!("unsupported target registry path: {path:?}"),
                        }
                    }
                    ensure!(!self.components.is_empty(), "registry path names a directory: {path:?}");
                    ensure!(
                        self.components.back().is_none_or(|part| part != ".."),
                        "registry path names a parent directory: {path:?}"
                    );
                    self.skip_parent_components();
                } else {
                    self.missing()?;
                }
            }
            (Progress::Observe(Request::Entry { .. }), Reply::Entry(entry)) => {
                let Some(entry) = entry else {
                    return self.missing();
                };
                let entry = entry.checked()?;
                if entry.dangling() {
                    return self.missing();
                }
                let component = self.components.pop_front().expect("pending component");
                if self.components.is_empty() {
                    self.read = true;
                } else {
                    ensure!(
                        entry.directory(),
                        "registry path component is not a directory: {:?}",
                        self.parent.join(component.clone())
                    );
                    self.parent.push(component);
                    self.skip_parent_components();
                }
            }
            (Progress::Observe(Request::Registry { .. }), Reply::Registry(text)) => {
                self.registry = Some(scaffold::Config::parse(self.path.as_deref(), Some(&text))?);
            }
            (pending, reply) => bail!("setup reply {reply:?} does not answer {pending:?}"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts {
        Facts {
            cwd: "/target/invocation".into(),
            uid: 4321,
            environment: [
                ("HOME", "/target/home"),
                ("TTA_ROOT", "target-projects"),
                ("TTA_TMUX", "./bin/tmux"),
                ("TTA_SOCKET_PATH", "target.sock"),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect(),
        }
    }

    #[test]
    fn target_precedence_and_new_terminal_identity_never_use_local_context() {
        let facts = facts();
        let options = Options::builder().argument("work".into()).build();
        let registry = scaffold::Config::parse(None, None).unwrap();
        let input = facts.input(&options, registry.clone()).unwrap();
        assert_eq!(input.root, Some("/target/invocation/target-projects".into()));
        assert_eq!(input.socket.path, Path::new("/target/invocation/target.sock"));
        assert_eq!(input.program, "/target/invocation/./bin/tmux");
        assert_eq!(input.client_mode, ClientMode::Attach);
        let options = Options::builder()
            .argument("work".into())
            .root("explicit".into())
            .socket(socket::Selection::Name("chosen".into()))
            .program("chosen-tmux".into())
            .scaffold("false".into())
            .build();
        let input = facts.input(&options, registry).unwrap();
        assert_eq!(input.root, Some("/target/invocation/explicit".into()));
        assert_eq!(input.socket.path, Path::new("/tmp/tmux-4321/chosen"));
        assert_eq!(input.program, "chosen-tmux");
    }

    #[test]
    fn registry_prelude_is_inspectable_and_wrong_replies_cannot_advance_it() {
        let options = Options::builder()
            .argument("work".into())
            .registry_path("/target/missing/config.toml".into())
            .build();
        let mut bootstrap = Bootstrap::new(options);
        assert!(bootstrap.supply(Reply::Registry(String::new())).is_err());
        assert!(matches!(bootstrap.next().unwrap(), Progress::Observe(Request::Context)));
        bootstrap.supply(Reply::Context(facts())).unwrap();
        assert!(matches!(bootstrap.next().unwrap(), Progress::Observe(Request::Entry { parent, name }) if parent == Path::new("/") && name == "target"));
        bootstrap.supply(Reply::Entry(Some(Entry { kind: b'd', referent: b'd' }))).unwrap();
        bootstrap.supply(Reply::Entry(None)).unwrap();
        assert!(matches!(bootstrap.next().unwrap(), Progress::Ready(_)));
        assert!(bootstrap.supply(Reply::Entry(None)).is_err());
    }

    #[test]
    fn setup_preserves_literal_values_and_rejects_executable_environment_names() {
        let setup = TargetSetup::builder()
            .shell("/target/shell".into())
            .login(true)
            .interactive(true)
            .environment(vec![Environment {
                name: "HOME".into(),
                value: Some("/target/quote' $;\n".into()),
            }])
            .build();
        let source = setup.shell_command("printf ok").unwrap();
        assert!(source.contains(" -l -i -c "));
        let bad = TargetSetup::builder()
            .environment(vec![Environment {
                name: "PATH; touch x".into(),
                value: None,
            }])
            .build();
        assert!(bad.shell_command(":").is_err());
    }
}
