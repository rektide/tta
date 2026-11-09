//! Shared, pure context policy. Select an entire active layer before decoding;
//! callers supply raw host facts, never a pre-expanded remote environment.
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};

use super::protocol::{ClientEntry, EntryIntent, InitialWindow, SuppliedContext, ViewPreference};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Origin {
    Local { host: String },
    Ssh { host: String, setup: crate::target::TargetSetup },
}

/// Setup/cwd actually used for a read. Part of the exact request echo, not a
/// hint that a host may replace after binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    pub origin: Origin,
    pub cwd: Option<PathBuf>,
}

impl Scope {
    pub fn frozen(&self, cwd: PathBuf) -> Self {
        let mut scope = self.clone();
        if let Origin::Ssh { setup, .. } = &mut scope.origin {
            setup.cwd = Some(cwd.clone());
        }
        scope.cwd = Some(cwd);
        scope
    }
}

/// Native local bytes survive acquisition. Only active text-valued policy
/// (JSON roots, prefix and generator choice) needs UTF-8.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Facts {
    pub cwd: PathBuf,
    pub uid: u32,
    pub environment: BTreeMap<String, OsString>,
}

pub const VARIABLES: &[&str] = &[
    "HOME",
    "XDG_CONFIG_HOME",
    "TTA_CONFIG",
    "TTA_ROOTS",
    "TTA_ROOT",
    "PROJECTS_DIR",
    "TTA_SOCKET_NAME",
    "TTA_SOCKET_PATH",
    "TMUX_TMPDIR",
    "TMUX",
    "TTA_TMUX",
    "TTA_TMUX_CONFIG",
    "TTA_SCAFFOLD",
    "TTA_SATELLITE_PREFIX",
    "PATH",
    "SHELL",
];

impl Facts {
    fn value(&self, name: &str) -> Option<&OsStr> {
        self.environment.get(name).map(OsString::as_os_str).filter(|v| !v.is_empty())
    }
    fn text(&self, name: &str) -> Result<Option<&str>> {
        self.value(name)
            .map(|v| v.to_str().ok_or_else(|| anyhow::anyhow!("{name} must be UTF-8")))
            .transpose()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Overrides {
    pub roots: Option<Vec<PathBuf>>,
    pub socket: Option<crate::socket::Selection>,
    pub program: Option<OsString>,
    pub config: Option<PathBuf>,
    pub registry: Option<PathBuf>,
    pub scaffold: Option<String>,
    pub satellite_prefix: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Acquisition {
    pub scope: Scope,
    pub overrides: Overrides,
    pub intent: EntryIntent,
    /// Caller default, not environment copied from the invoking host.
    pub default_scaffold: String,
}

pub(super) fn validate_initial_scope(scope: &Scope) -> Result<()> {
    ensure!(scope.cwd.is_none(), "acquisition scope must not claim an already-frozen cwd");
    let host = match &scope.origin {
        Origin::Local { host } | Origin::Ssh { host, .. } => host,
    };
    ensure!(
        !host.is_empty() && !host.chars().any(char::is_control),
        "target host must be nonempty and printable"
    );
    if let Origin::Ssh { setup, .. } = &scope.origin {
        setup.shell_command(":")?;
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct Policy {
    pub supplied: SuppliedContext,
    pub scope: Scope,
    pub intent: EntryIntent,
    pub registry: Option<PathBuf>,
    pub generator: crate::scaffold::Choice,
    pub endpoint_identity: Option<(PathBuf, PathBuf)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Use {
    Entry,
    Catalog,
}

pub(super) fn apply(input: &Acquisition, facts: &Facts, usage: Use) -> Result<Policy> {
    ensure!(facts.cwd.is_absolute(), "observed target cwd is not absolute");
    let overrides = &input.overrides;
    // Do not even decode TTA_ROOTS if an explicit list replaces it.
    let roots = if let Some(roots) = &overrides.roots {
        crate::projects::configured_roots(Some(roots), None, None, None, None, &facts.cwd)?
    } else {
        crate::projects::configured_roots(
            None,
            facts.text("TTA_ROOTS")?,
            facts.value("TTA_ROOT"),
            facts.value("PROJECTS_DIR"),
            facts.value("HOME").map(Path::new),
            &facts.cwd,
        )?
    };
    let selection = match &overrides.socket {
        Some(socket) => socket.clone(),
        None => match (facts.value("TTA_SOCKET_NAME"), facts.value("TTA_SOCKET_PATH")) {
            (Some(_), Some(_)) => anyhow::bail!("TTA_SOCKET_NAME and TTA_SOCKET_PATH conflict"),
            (Some(name), None) => crate::socket::Selection::Name(name.into()),
            (None, Some(path)) => crate::socket::Selection::Path(path.into()),
            _ => crate::socket::Selection::Default,
        },
    };
    let remote = matches!(input.scope.origin, Origin::Ssh { .. });
    let socket_context = crate::socket::Context {
        cwd: facts.cwd.clone(),
        uid: facts.uid,
        tmux: if remote { None } else { facts.value("TMUX").map(OsStr::to_owned) },
        tmpdir: facts.value("TMUX_TMPDIR").map(PathBuf::from),
    };
    let socket = socket_context.resolve(&selection)?;
    let current = socket_context.current_server();
    // Physical endpoint comparison is acquisition, not a core filesystem call.
    let endpoint_identity = current
        .as_ref()
        .filter(|current| usage == Use::Entry && **current != socket.path)
        .map(|current| (current.clone(), socket.path.clone()));
    let client = if usage == Use::Entry && current.is_some() {
        ClientEntry::SwitchWithinServer
    } else {
        ClientEntry::AttachTerminal
    };
    let mut program = overrides
        .program
        .clone()
        .or_else(|| facts.value("TTA_TMUX").map(OsStr::to_owned))
        .unwrap_or_else(|| "tmux".into());
    if program.as_encoded_bytes().contains(&b'/') {
        program = facts.cwd.join(program).into_os_string();
    }
    let config = overrides
        .config
        .clone()
        .or_else(|| facts.value("TTA_TMUX_CONFIG").map(PathBuf::from))
        .map(|p| facts.cwd.join(p));
    let registry = overrides
        .registry
        .clone()
        .or_else(|| facts.value("TTA_CONFIG").map(PathBuf::from))
        .or_else(|| {
            facts
                .value("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .or_else(|| facts.value("HOME").map(|home| Path::new(home).join(".config")))
                .map(|p| p.join("tta/config.toml"))
        })
        .map(|p| facts.cwd.join(p));
    let choice = match &overrides.scaffold {
        Some(choice) => choice.as_str(),
        None => facts.text("TTA_SCAFFOLD")?.unwrap_or(&input.default_scaffold),
    };
    let prefix = match &overrides.satellite_prefix {
        Some(prefix) => prefix.clone(),
        None => facts.text("TTA_SATELLITE_PREFIX")?.unwrap_or("_").to_owned(),
    };
    let mut intent = input.intent.clone();
    intent.roots = roots;
    intent.satellite_prefix = prefix;
    intent.directory = intent.directory.map(|p| facts.cwd.join(p));
    intent.view_directory = intent.view_directory.map(|p| facts.cwd.join(p));
    let host = match &input.scope.origin {
        Origin::Local { host } | Origin::Ssh { host, .. } => host.clone(),
    };
    Ok(Policy {
        supplied: SuppliedContext {
            host,
            cwd: facts.cwd.clone(),
            socket,
            program,
            config,
            client,
        },
        scope: input.scope.frozen(facts.cwd.clone()),
        intent,
        registry,
        generator: crate::scaffold::Choice::parse(choice)?,
        endpoint_identity,
    })
}

impl EntryIntent {
    pub fn selector(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
            view: ViewPreference::Auto,
            satellite_prefix: "_".into(),
            view_directory: None,
            roots: Vec::new(),
            session_name: None,
            startup: Vec::new(),
            directory: None,
            window: InitialWindow::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    fn facts() -> Facts {
        Facts {
            cwd: "/target/work".into(),
            uid: 4321,
            environment: [
                ("HOME".into(), "/target/home".into()),
                ("TTA_ROOT".into(), "projects".into()),
                ("TTA_TMUX".into(), "./bin/tmux".into()),
                ("TMUX".into(), "/local/leak,1,1".into()),
            ]
            .into(),
        }
    }
    pub(super) fn input() -> Acquisition {
        Acquisition {
            scope: Scope {
                origin: Origin::Ssh {
                    host: "target".into(),
                    setup: crate::target::TargetSetup::default(),
                },
                cwd: None,
            },
            overrides: Overrides::default(),
            intent: EntryIntent::selector("work"),
            default_scaffold: "false".into(),
        }
    }

    #[test]
    fn active_whole_layers_are_selected_before_text_decoding_and_local_bytes_survive() {
        let mut facts = facts();
        for name in ["TTA_ROOTS", "TTA_SCAFFOLD", "TTA_SATELLITE_PREFIX"] {
            facts.environment.insert(name.into(), OsString::from_vec(vec![0xff]));
        }
        facts.environment.insert("TTA_SOCKET_NAME".into(), "conflict".into());
        facts.environment.insert("TTA_SOCKET_PATH".into(), "conflict".into());
        let mut input = input();
        input.scope.origin = Origin::Local { host: "local".into() };
        input.overrides = Overrides {
            roots: Some(vec![PathBuf::from(OsString::from_vec(b"raw-\xff".to_vec()))]),
            socket: Some(crate::socket::Selection::Path(PathBuf::from(OsString::from_vec(b"socket-\xff".to_vec())))),
            program: Some(OsString::from_vec(b"./tmux-\xff".to_vec())),
            scaffold: Some("false".into()),
            satellite_prefix: Some("view-".into()),
            ..Default::default()
        };
        let policy = apply(&input, &facts, Use::Entry).unwrap();
        assert_eq!(policy.intent.roots, [facts.cwd.join(OsString::from_vec(b"raw-\xff".to_vec()))]);
        assert_eq!(
            policy.supplied.program,
            facts.cwd.join(OsString::from_vec(b"./tmux-\xff".to_vec())).into_os_string()
        );
        input.overrides.roots = None;
        assert!(apply(&input, &facts, Use::Entry).unwrap_err().to_string().contains("TTA_ROOTS"));
    }

    #[test]
    fn remote_policy_uses_only_supplied_target_facts_and_freezes_setup() {
        let facts = facts();
        let mut input = input();
        input.scope.origin = Origin::Ssh {
            host: "target".into(),
            setup: crate::target::TargetSetup {
                cwd: Some("relative-start".into()),
                login: true,
                ..Default::default()
            },
        };
        let policy = apply(&input, &facts, Use::Entry).unwrap();
        assert_eq!(policy.intent.roots, [PathBuf::from("/target/work/projects")]);
        assert_eq!(policy.supplied.socket.path, Path::new("/tmp/tmux-4321/default"));
        assert_eq!(policy.supplied.client, ClientEntry::AttachTerminal);
        assert!(policy.endpoint_identity.is_none());
        assert_eq!(policy.supplied.program, "/target/work/./bin/tmux");
        let Origin::Ssh { setup, .. } = policy.scope.origin else { panic!() };
        assert!(setup.login);
        assert_eq!(setup.cwd, Some(facts.cwd));
    }
}
