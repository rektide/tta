//! Named scaffold registry loaded from a TOML configuration file.
//!
//! The file maps user-chosen generator names to compiled-in adapters and may
//! pick a default generator for bare scaffold mode. An absent file means an
//! empty registry: scaffold mode has nothing to run and misses keep today's
//! error. Adapters are crates selected by cargo features, so a configuration
//! naming an adapter this build does not include fails before lookup.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use tta_command::Expression;

#[derive(Debug)]
pub struct Config {
    path: Option<PathBuf>,
    default: Option<String>,
    entries: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub adapter: String,
    scaffold: fn(&str, &Path) -> Result<Expression>,
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
}

fn compiled_adapters() -> &'static [&'static str] {
    &[
        #[cfg(feature = "scaffold-rekon")]
        "rekon",
    ]
}

impl Entry {
    pub fn scaffold(&self, project: &str, root: &Path) -> Result<Expression> {
        (self.scaffold)(project, root)
    }
}

impl Config {
    /// Load from TTA_CONFIG, or the XDG/HOME default location. A missing file
    /// is an empty registry; anything else that fails is an error.
    pub fn load() -> Result<Self> {
        Self::load_from(path_from_environment().as_deref())
    }

    fn load_from(path: Option<&Path>) -> Result<Self> {
        let empty = |path: Option<PathBuf>| Self {
            path,
            default: None,
            entries: BTreeMap::new(),
        };
        let Some(path) = path else {
            return Ok(empty(None));
        };
        let file: File = match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|error| anyhow!("invalid scaffold configuration in {}: {error}", path.display()))?,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(empty(Some(path.to_owned())));
            }
            Err(source) => {
                return Err(anyhow!(source)).context(format!("cannot read scaffold configuration {}", path.display()));
            }
        };
        let mut entries = BTreeMap::new();
        for (name, generator) in file.scaffold.generators {
            if name.is_empty() || generator.adapter.is_empty() {
                bail!("scaffold configuration {}: generator names and adapters cannot be empty", path.display());
            }
            let adapter = generator.adapter;
            let scaffold: fn(&str, &Path) -> Result<Expression> = match adapter.as_str() {
                #[cfg(feature = "scaffold-rekon")]
                "rekon" => |project, _root| tta_scaffold_rekon::scaffold(project, _root).map_err(anyhow::Error::from),
                other => bail!(
                    "scaffold configuration {}: generator {name:?} uses adapter {other:?}, which this tta build does not include; compiled adapters: {compiled:?}",
                    path.display(),
                    compiled = compiled_adapters()
                ),
            };
            entries.insert(name.clone(), Entry { name, adapter, scaffold });
        }
        if let Some(default) = &file.scaffold.default
            && !entries.contains_key(default)
        {
            bail!(
                "scaffold configuration {}: default generator {default:?} is not configured; known generators: {:?}",
                path.display(),
                entries.keys().collect::<Vec<_>>()
            );
        }
        Ok(Self {
            path: Some(path.to_owned()),
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

fn path_from_environment() -> Option<PathBuf> {
    fn nonempty(name: &str) -> Option<OsString> {
        env::var_os(name).filter(|value| !value.is_empty())
    }
    if let Some(explicit) = nonempty("TTA_CONFIG") {
        return Some(PathBuf::from(explicit));
    }
    // A relative XDG_CONFIG_HOME is invalid per the spec; use the HOME default.
    let config_home = nonempty("XDG_CONFIG_HOME")
        .filter(|value| Path::new(value).is_absolute())
        .map(PathBuf::from)
        .or_else(|| nonempty("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config_home.join("tta").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> Config {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, text).unwrap();
        Config::load_from(Some(&path)).unwrap()
    }

    fn missing() -> Config {
        let temp = tempfile::tempdir().unwrap();
        Config::load_from(Some(&temp.path().join("absent.toml"))).unwrap()
    }

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
        let expression = default.scaffold("fresh", Path::new("/root")).unwrap();
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
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        for (text, expected) in [
            ("[scaffold.generators.x]\nadapter = \"cargo\"\n", "which this tta build does not include"),
            ("[scaffold]\nunknown = true\n", "unknown field"),
            ("[scaffold.generators.x]\nadapter = \"whatever\"\nextra = 1\n", "unknown field"),
        ] {
            std::fs::write(&path, text).unwrap();
            let error = Config::load_from(Some(&path)).unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[cfg(feature = "scaffold-rekon")]
    #[test]
    fn a_default_must_name_a_configured_generator() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "[scaffold]\ndefault = \"absent\"\n[scaffold.generators.x]\nadapter = \"rekon\"\n").unwrap();
        let error = Config::load_from(Some(&path)).unwrap_err().to_string();
        assert!(error.contains("is not configured"), "{error}");
    }
}
