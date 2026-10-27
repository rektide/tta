//! Local registry and generator-default observations. Pure parsing stays in core.
use crate::backend::SessionLookup;
use anyhow::{Context, Result};
use bon::builder;
use std::path::{Path, PathBuf};
pub use tta_launcher::scaffold::{Choice, Config, Defaults, Entry, Error, Resolved};

pub fn config_path() -> Option<PathBuf> {
    fn value(name: &str) -> Option<std::ffi::OsString> {
        std::env::var_os(name).filter(|value| !value.is_empty())
    }
    if let Some(path) = value("TTA_CONFIG") {
        return Some(path.into());
    }
    let home = value("XDG_CONFIG_HOME")
        .filter(|value| Path::new(value).is_absolute())
        .map(PathBuf::from)
        .or_else(|| value("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(home.join("tta/config.toml"))
}

pub fn load_config() -> Result<Config> {
    load_config_from(config_path().as_deref())
}

pub fn load_config_from(path: Option<&Path>) -> Result<Config> {
    let text = match path {
        None => None,
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).with_context(|| format!("cannot read scaffold configuration {}", path.display())),
        },
    };
    Config::parse(path, text.as_deref())
}

pub fn generator_defaults() -> Result<Defaults> {
    observe_defaults(&std::env::current_dir()?, true, true)
}

pub fn observe_defaults(directory: &Path, author: bool, license: bool) -> Result<Defaults> {
    let value = |name| -> Result<Option<String>> {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(|value| value.into_string().map_err(|_| anyhow::anyhow!("{name} must be UTF-8")))
            .transpose()
    };
    let author = if !author {
        None
    } else {
        match value("AUTHOR")? {
            Some(author) => Some(author),
            None => {
                let mut command = std::process::Command::new("git");
                command.current_dir(directory).args(["config", "--get", "user.name"]);
                let output = crate::capture::run(&mut command, crate::capture::Limits::default()).context("cannot observe Git author default")?;
                output.require_complete()?;
                let status = output.status.expect("complete capture has status");
                if status.success() {
                    Some(String::from_utf8(output.stdout)?.trim().to_owned()).filter(|name| !name.is_empty())
                } else if status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty() {
                    None
                } else {
                    anyhow::bail!("cannot observe Git author default ({}): {}", status, String::from_utf8_lossy(&output.stderr));
                }
            }
        }
    };
    Ok(Defaults {
        author,
        license: if license { value("LICENSE")? } else { None },
    })
}

/// Compatibility convenience for local callers; the driver API centralizes the
/// full launch. All selection policy is the core's supplied-facts resolver.
#[builder]
pub fn resolve(
    lookup: &dyn SessionLookup,
    argument: &str,
    root: Option<&Path>,
    session_name: Option<&str>,
    choice: &str,
    config: Option<&Config>,
) -> std::result::Result<Resolved, Error> {
    let run = || -> Result<Resolved> {
        let loaded;
        let config = match config {
            Some(config) => config,
            None => {
                loaded = load_config()?;
                &loaded
            }
        };
        // Choice errors precede lookup, even for an existing session.
        match Choice::parse(choice)? {
            Choice::Off => {}
            Choice::Default => {
                config.default_entry()?;
            }
            Choice::Named(name) => {
                config.entry(&name)?;
            }
        }
        if argument.is_empty() {
            return Err(tta_launcher::ResolveError::Empty.into());
        }
        let sessions = lookup.sessions()?;
        let mut projects = None;
        let mut destination = None;
        // Defaults are needed only for a clean create, never for existing work.
        let mut defaults = Defaults::default();
        let mut defaults_observed = false;
        loop {
            match tta_launcher::scaffold::resolve()
                .sessions(&sessions)
                .argument(argument)
                .maybe_root(root)
                .maybe_session_name(session_name)
                .choice(choice)
                .config(config)
                .maybe_projects(projects.as_ref())
                .maybe_destination(destination)
                .defaults(&defaults)
                .call()?
            {
                tta_launcher::scaffold::Progress::Resolved(resolved) => {
                    let (author, license) = resolved.generator.as_ref().map_or((false, false), Entry::defaults_required);
                    if !defaults_observed && (author || license) {
                        defaults = observe_defaults(&std::env::current_dir()?, author, license)?;
                        defaults_observed = true;
                    } else {
                        return Ok(resolved);
                    }
                }
                tta_launcher::scaffold::Progress::Observe(tta_launcher::Resolution::NeedProjects(root)) => {
                    projects = Some(crate::resolve::project_inventory(&root, argument)?)
                }
                tta_launcher::scaffold::Progress::Observe(tta_launcher::Resolution::NeedDestination(path)) => {
                    destination = Some(crate::resolve::destination_state(&path)?);
                }
                _ => unreachable!("core only requests missing facts"),
            }
        }
    };
    run().map_err(Error::from)
}
