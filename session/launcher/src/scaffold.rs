//! Shared scaffold registry and session-first resolution for CLI and embedded callers.
//!
//! The file maps user-chosen generator names to compiled-in adapters and may
//! pick a default generator for bare scaffold mode. An absent file means an
//! empty registry: scaffold mode has nothing to run and misses keep today's
//! error. Adapters are crates selected by cargo features, so a configuration
//! naming an adapter this build does not include fails before lookup.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use bon::builder;
use serde::Deserialize;
use toml::Table;
use tta_command::Expression;

use crate::{DestinationState, ProjectInventory, Resolution, Selection, Session};

/// Observed target defaults, already layered AUTHOR > Git identity and LICENSE.
/// Absent values are explicit; the generator never reads the local host.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Defaults {
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug)]
pub enum Progress {
    Observe(Resolution),
    Resolved(Resolved),
}

/// Standard-error-compatible boundary for embedded callers (including eyre).
/// The source retains the configuration or resolution error and its context.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(
    #[from]
    #[source]
    anyhow::Error,
);

/// The caller chooses its default posture and layers CLI/environment values.
/// Registry location and generator selection are shared by all callers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Choice {
    Off,
    Default,
    Named(String),
}

impl Choice {
    pub fn parse(value: &str) -> std::result::Result<Self, Error> {
        Ok(match value {
            "" => return Err(anyhow!("empty scaffold value; use a generator name, 'default', or 'false'").into()),
            "default" | "true" | "1" => Self::Default,
            "false" | "0" | "off" | "none" => Self::Off,
            name => Self::Named(name.to_owned()),
        })
    }
}

#[derive(Debug)]
pub struct Resolved {
    pub selection: Selection,
    /// Pass this to launch().maybe_prefix(...); only Create selections have one.
    pub prefix: Option<Expression>,
    /// Present only when a Create selection uses this generator.
    pub generator: Option<Entry>,
}

/// Apply a validated registry and choice to supplied facts. Missing facts yield
/// observation requests; no environment, filesystem or process is consulted.
#[builder]
pub fn resolve(
    sessions: &[Session],
    argument: &str,
    #[builder(default = "_")] satellite_prefix: &str,
    #[builder(default = &[])] roots: &[PathBuf],
    session_name: Option<&str>,
    choice: &str,
    config: &Config,
    #[builder(default = &[])] projects: &[ProjectInventory],
    destination: Option<DestinationState>,
    defaults: &Defaults,
) -> std::result::Result<Progress, Error> {
    resolve_with_config(
        config,
        sessions,
        argument,
        satellite_prefix,
        roots,
        session_name,
        choice,
        projects,
        destination,
        defaults,
    )
    .map_err(Error::from)
}

#[allow(clippy::too_many_arguments)]
fn resolve_with_config(
    config: &Config,
    sessions: &[Session],
    argument: &str,
    satellite_prefix: &str,
    roots: &[PathBuf],
    session_name: Option<&str>,
    choice: &str,
    projects: &[ProjectInventory],
    destination: Option<DestinationState>,
    defaults: &Defaults,
) -> Result<Progress> {
    let choice = Choice::parse(choice)?;
    let generator = match &choice {
        Choice::Off => None,
        Choice::Named(name) => Some(config.entry(name)?),
        Choice::Default => config.default_entry()?,
    };
    let selection = match crate::resolve()
        .sessions(sessions)
        .argument(argument)
        .satellite_prefix(satellite_prefix)
        .roots(roots)
        .maybe_session_name(session_name)
        .create(generator.is_some())
        .projects(projects)
        .maybe_destination(destination)
        .call()
    {
        Ok(Resolution::Selected(selection)) => selection,
        Ok(request) => return Ok(Progress::Observe(request)),
        Err(error @ crate::ResolveError::Missing { .. }) if matches!(choice, Choice::Default) && config.is_empty() => {
            return Err(anyhow::Error::new(error).context(format!(
                "scaffold mode is on but no generators are configured; add [scaffold.generators] to {} or disable with --scaffold=false",
                config.path_description()
            )));
        }
        Err(error) => return Err(error.into()),
    };
    let (prefix, generator) = if let Selection::Create { directory, .. } = &selection {
        let entry = generator.expect("create selections require a chosen generator");
        // The external rekon script takes only the project name and lands it
        // in its own hardcoded location, ignoring the selected root. With one
        // configured root that divergence is the documented single-root
        // limitation; with several configured roots no script destination can
        // agree with the destination tta selected, so refuse before generation
        // instead of mutating elsewhere. A qualified @N selector does not help:
        // the script never sees it either.
        if entry.adapter == "rekon" && roots.len() > 1 {
            bail!(
                "generator {:?} uses the external rekon adapter, whose script ignores the selected project root; refusing to create {:?} across configured roots {roots:?}: use a rekon-native generator (which honors the root), configure a single root, or change the rekon script's destination contract",
                entry.name,
                directory
            );
        }
        let project = directory
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .with_context(|| format!("created project directory {directory:?} must have a UTF-8 name"))?;
        let root = directory.parent().context("created project directory has no parent")?;
        (Some(entry.scaffold(project, root, defaults)?), Some(entry))
    } else {
        (None, None)
    };
    Ok(Progress::Resolved(Resolved { selection, prefix, generator }))
}

#[derive(Clone, Debug)]
pub struct Config {
    path: Option<PathBuf>,
    default: Option<String>,
    entries: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub adapter: String,
    /// Per-entry adapter options, validated by the adapter itself.
    pub options: Table,
    scaffold: Adapter,
    validate: AdapterValidate,
    destination: DestinationContract,
}

/// Where a generator's adapter puts the new project, declared by the adapter
/// implementation and carried on the validated generator. Creation policy
/// refuses destinations the contract cannot hold authoritative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationContract {
    /// The adapter creates exactly the selected root/name destination.
    SelectedRoot,
    /// The adapter's own program decides where the project lands; tta's
    /// selected root does not govern it (declared by the external rekon
    /// adapter, whose script uses a hardcoded location).
    AdapterDecided,
}

/// A validated generator specification: adapter options already checked, with
/// the observed-default requirements and destination contract it declares.
/// Materialization happens once, later, with actual observed facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Validated {
    pub needs_defaults: (bool, bool),
    pub destination: DestinationContract,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    scaffold: FileScaffold,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileScaffold {
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    generators: BTreeMap<String, FileGenerator>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileGenerator {
    adapter: String,
    /// Adapter-defined options; see the adapter's documentation.
    #[serde(default)]
    options: Table,
}

fn compiled_adapters() -> &'static [&'static str] {
    &[
        #[cfg(feature = "scaffold-rekon")]
        "rekon",
        #[cfg(feature = "scaffold-rekon-native")]
        "rekon-native",
    ]
}

type Adapter = fn(&str, &Path, &Table, &Defaults) -> Result<Expression>;
type AdapterValidate = fn(&Table) -> Result<(bool, bool)>;

// Keep feature-dependent dispatch out of the insertion loop so a build with no
// adapters still typechecks without unreachable-code/unused-mut warnings.
fn adapter(name: &str) -> Option<(Adapter, AdapterValidate, DestinationContract)> {
    match name {
        #[cfg(feature = "scaffold-rekon")]
        "rekon" => Some((
            |project, root, options, _defaults| {
                tta_scaffold_rekon::validate(options).map_err(anyhow::Error::from)?;
                tta_scaffold_rekon::scaffold(project, root).map_err(anyhow::Error::from)
            },
            |options| {
                let needs = tta_scaffold_rekon::validate(options)?;
                Ok((needs.author, needs.license))
            },
            if tta_scaffold_rekon::SELECTS_OWN_DESTINATION {
                DestinationContract::AdapterDecided
            } else {
                DestinationContract::SelectedRoot
            },
        )),
        #[cfg(feature = "scaffold-rekon-native")]
        "rekon-native" => Some((
            |project, root, options, defaults| {
                tta_scaffold_rekon_native::scaffold(
                    project,
                    root,
                    options,
                    &tta_scaffold_rekon_native::Defaults {
                        author: defaults.author.clone(),
                        license: defaults.license.clone(),
                    },
                )
                .map_err(anyhow::Error::from)
            },
            |options| {
                let needs = tta_scaffold_rekon_native::validate(options)?;
                Ok((needs.author, needs.license))
            },
            if tta_scaffold_rekon_native::SELECTS_OWN_DESTINATION {
                DestinationContract::AdapterDecided
            } else {
                DestinationContract::SelectedRoot
            },
        )),
        _ => None,
    }
}

impl Entry {
    /// Which target defaults are still needed after validating adapter options.
    /// External rekon owns its defaults; it needs no launcher observation.
    pub fn defaults_required(&self) -> (bool, bool) {
        if self.adapter != "rekon-native" || self.options.get("package-json").and_then(toml::Value::as_bool) == Some(false) {
            return (false, false);
        }
        let author = !self.options.get("author").and_then(toml::Value::as_str).is_some_and(|value| !value.is_empty());
        let license = !self.options.contains_key("license");
        (author, license)
    }

    /// Validate this generator's adapter options and report its declared
    /// default requirements and destination contract, without materializing
    /// its program. The single later materialization receives actual facts.
    pub fn validated(&self) -> Result<Validated> {
        let needs_defaults = (self.validate)(&self.options)?;
        Ok(Validated {
            needs_defaults,
            destination: self.destination,
        })
    }

    pub fn scaffold(&self, project: &str, root: &Path, defaults: &Defaults) -> Result<Expression> {
        (self.scaffold)(project, root, &self.options, defaults)
    }

    /// The destination contract this generator's adapter declared.
    pub fn destination_contract(&self) -> DestinationContract {
        self.destination
    }
}

impl Config {
    /// Parse supplied target text. `None` means an observed missing file or no
    /// registry location, not a suppressed read error. `path` is provenance only.
    pub fn parse(path: Option<&Path>, text: Option<&str>) -> Result<Self> {
        let empty = |path: Option<PathBuf>| Self {
            path,
            default: None,
            entries: BTreeMap::new(),
        };
        let Some(text) = text else {
            return Ok(empty(path.map(Path::to_owned)));
        };
        let description = path.map(|path| path.display().to_string()).unwrap_or_else(|| "supplied registry".into());
        let file: File = toml::from_str(text).map_err(|error| anyhow!("invalid scaffold configuration in {description}: {error}"))?;
        let mut entries = BTreeMap::new();
        for (name, generator) in file.scaffold.generators {
            if name.is_empty() || generator.adapter.is_empty() {
                bail!("scaffold configuration {description}: generator names and adapters cannot be empty");
            }
            let adapter_name = generator.adapter;
            let (scaffold, validate, destination) = adapter(&adapter_name).ok_or_else(|| anyhow!(
                    "scaffold configuration {}: generator {name:?} uses adapter {other:?}, which this tta build does not include; compiled adapters: {compiled:?}",
                    description,
                    other = adapter_name,
                    compiled = compiled_adapters()
                ))?;
            entries.insert(
                name.clone(),
                Entry {
                    name,
                    adapter: adapter_name,
                    options: generator.options,
                    scaffold,
                    validate,
                    destination,
                },
            );
        }
        if let Some(default) = &file.scaffold.default
            && !entries.contains_key(default)
        {
            bail!(
                "scaffold configuration {}: default generator {default:?} is not configured; known generators: {:?}",
                description,
                entries.keys().collect::<Vec<_>>()
            );
        }
        Ok(Self {
            path: path.map(Path::to_owned),
            default: file.scaffold.default,
            entries,
        })
    }

    pub fn names(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, name: &str) -> Result<Entry> {
        self.entries
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown scaffold generator {name:?}; configured: {:?}", self.names()))
    }

    /// Bare scaffold mode's generator: the configured default, the sole entry,
    /// nothing when the registry is empty, or an error when the choice would
    /// be a guess.
    pub fn default_entry(&self) -> Result<Option<Entry>> {
        if let Some(default) = &self.default {
            return Ok(Some(self.entries.get(default).expect("validated at load").clone()));
        }
        let mut entries = self.entries.iter();
        match (entries.next(), entries.next()) {
            (None, _) => Ok(None),
            (Some((_, sole)), None) => Ok(Some(sole.clone())),
            (Some(_), Some(_)) => Err(anyhow!(
                "multiple scaffold generators {:?} are configured without a [scaffold] default; pass --scaffold=NAME or set one as the default",
                self.names()
            )),
        }
    }

    pub fn path_description(&self) -> String {
        match &self.path {
            Some(path) => path.display().to_string(),
            None => "the default location (set TTA_CONFIG, XDG_CONFIG_HOME, or HOME)".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(feature = "scaffold-rekon", feature = "scaffold-rekon-native"))]
    fn load(text: &str) -> Config {
        Config::parse(Some(Path::new("/target/config.toml")), Some(text)).unwrap()
    }

    fn missing() -> Config {
        Config::parse(Some(Path::new("/target/absent.toml")), None).unwrap()
    }

    #[cfg(feature = "scaffold-rekon")]
    const MINIMAL: &str = "\
[scaffold]
default = \"rk\"

[scaffold.generators.rk]
adapter = \"rekon\"
";

    #[test]
    fn a_missing_file_is_an_empty_registry() {
        let config = missing();
        assert!(config.is_empty());
        assert_eq!(config.names(), Vec::<String>::new());
        assert!(config.default_entry().unwrap().is_none());
        assert!(config.path_description().contains("absent.toml"));
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn a_named_default_resolves_and_scaffolds_through_its_adapter() {
        let config = load(MINIMAL);
        assert_eq!(config.names(), ["rk"]);
        let entry = config.entry("rk").unwrap();
        assert_eq!(entry.adapter, "rekon");
        let default = config.default_entry().unwrap().unwrap();
        assert_eq!(default.name, "rk");
        let expression = default.scaffold("fresh", Path::new("/root"), &Defaults::default()).unwrap();
        assert!(expression.shell(tta_command::Handoff::Run).unwrap().starts_with("'rekon' 'fresh'"));
        assert!(config.entry("other").is_err());
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn a_sole_generator_is_the_default_without_being_named() {
        let config = load("[scaffold.generators.solo]\nadapter = \"rekon\"\n");
        let sole = config.default_entry().unwrap().unwrap();
        assert_eq!(sole.name, "solo");
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn multiple_generators_need_an_explicit_default() {
        let config = load("[scaffold.generators.alpha]\nadapter = \"rekon\"\n[scaffold.generators.beta]\nadapter = \"rekon\"\n");
        let error = config.default_entry().unwrap_err().to_string();
        assert!(error.contains("alpha") && error.contains("beta"), "{error}");
    }

    #[test]
    fn unknown_adapters_and_structural_mistakes_are_rejected() {
        for (text, expected) in [
            ("[scaffold.generators.x]\nadapter = \"cargo\"\n", "which this tta build does not include"),
            ("[scaffold]\nunknown = true\n", "unknown field"),
            ("[scaffold.generators.x]\nadapter = \"whatever\"\nextra = 1\n", "unknown field"),
        ] {
            let error = Config::parse(None, Some(text)).unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn a_default_must_name_a_configured_generator() {
        let error = Config::parse(None, Some("[scaffold]\ndefault = \"absent\"\n[scaffold.generators.x]\nadapter = \"rekon\"\n"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("is not configured"), "{error}");
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn external_rekon_fails_closed_for_multi_root_creation() {
        use crate::{DestinationState, ProjectInventory, Selection};
        let config = load(MINIMAL);
        let inventory = |root: &str| ProjectInventory {
            requested_root: root.into(),
            root: root.into(),
            entries: Vec::new(),
        };
        let resolve = |roots: &[std::path::PathBuf], argument: &str| {
            crate::scaffold::resolve()
                .sessions(&[])
                .argument(argument)
                .roots(roots)
                .choice("default")
                .config(&config)
                .projects(&roots.iter().map(|root| inventory(root.to_str().unwrap())).collect::<Vec<_>>())
                .maybe_destination(Some(DestinationState::Absent))
                .defaults(&Defaults::default())
                .call()
        };
        let single = resolve(&["/only".into()], "fresh").unwrap();
        let Progress::Resolved(single) = single else {
            panic!("all facts were supplied; expected a resolution");
        };
        assert!(
            matches!(&single.selection, Selection::Create { directory, .. } if directory == std::path::Path::new("/only/fresh")),
            "single-root external rekon keeps its documented divergence"
        );
        // A qualified selector with distinct roots, and an unqualified miss
        // whose roots alias one physical destination: both would otherwise be
        // creatable, so the adapter boundary is what must refuse.
        for (roots, supplied, argument) in [
            (vec!["/one".into(), "/two".into()], vec![("/two", "/two")], "@2/fresh"),
            (
                vec!["/alias".into(), "/physical".into()],
                vec![("/alias", "/physical"), ("/physical", "/physical")],
                "fresh",
            ),
        ] {
            let projects = supplied
                .iter()
                .map(|&(requested, root): &(&str, &str)| ProjectInventory {
                    requested_root: requested.into(),
                    root: root.into(),
                    entries: Vec::new(),
                })
                .collect::<Vec<_>>();
            let error = crate::scaffold::resolve()
                .sessions(&[])
                .argument(argument)
                .roots(&roots)
                .choice("default")
                .config(&config)
                .projects(&projects)
                .maybe_destination(Some(DestinationState::Absent))
                .defaults(&Defaults::default())
                .call()
                .unwrap_err()
                .to_string();
            assert!(error.contains("external rekon adapter"), "{argument}: {error}");
            assert!(error.contains("configured roots"), "{argument}: {error}");
            assert!(error.contains("rekon-native"), "{argument}: {error}");
        }
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn adapters_reject_options_they_do_not_take() {
        // Structure parses at load; the adapter refuses when it is selected.
        let config = load("[scaffold.generators.rk]\nadapter = \"rekon\"\n\n[scaffold.generators.rk.options]\nanything = 1\n");
        let error = config
            .entry("rk")
            .unwrap()
            .scaffold("fresh", Path::new("/root"), &Defaults::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("takes no options"), "{error}");
    }

    #[cfg(feature = "scaffold-rekon-native")]
    #[test]
    fn native_options_reach_the_adapter_and_direct_its_steps() {
        let config = load(
            "[scaffold.generators.rn]\nadapter = \"rekon-native\"\n\n[scaffold.generators.rn.options]\nbranch = \"trunk\"\npackage-json = false\n\n[scaffold.generators.rn.options.remotes]\norigin = \"\"\n",
        );
        let expression = config.entry("rn").unwrap().scaffold("fresh", Path::new("/root"), &Defaults::default()).unwrap();
        let script = expression.shell(tta_command::Handoff::Run).unwrap();
        assert!(script.contains("'git' 'init' '-b' 'trunk'"), "{script}");
        assert!(!script.contains("'remote' 'add' 'origin'"), "{script}");
        assert!(!script.contains("branch.trunk"), "{script}");
        assert!(!script.contains("printf"), "{script}");
        assert!(!script.contains("pnpm"), "{script}");
    }

    #[cfg(feature = "scaffold-rekon-native")]
    #[test]
    fn native_option_mistakes_are_reported_when_the_generator_runs() {
        let config = load("[scaffold.generators.rn]\nadapter = \"rekon-native\"\n\n[scaffold.generators.rn.options]\nbrunch = \"main\"\n");
        let error = config
            .entry("rn")
            .unwrap()
            .scaffold("fresh", Path::new("/root"), &Defaults::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown field"), "{error}");
    }
}
