//! tta's own project generator: a parameterized Rust reimplementation of the
//! [`rekon` script](file:///home/rektide/.local/bin/rekon) workflow — improved
//! rather than reproduced. It honors the configured project root instead of a
//! hardcoded `~/src`, creates the initial branch directly (`git init -b`
//! instead of init plus checkout), templates remotes and starter files, and
//! authors `package.json` itself instead of npm-init-plus-jq surgery. The
//! script's quietly-broken `pnpm i -q` (rejected by pnpm; the script has no
//! `set -e`) becomes `pnpm install --silent`.
//!
//! Heavy lifting stays an external tool invocation inside the generated
//! expression (git, jj, pnpm), so plans remain inspectable, printable, and
//! short-circuiting; only decisions and file contents are computed here.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use serde::Deserialize;
use toml::Table;
use tta_command::{And, ChangeDir, Command, Expression, Word, WriteFile};

/// Registry name of this adapter.
pub const ADAPTER: &str = "rekon-native";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid rekon-native options: {0}")]
    Options(String),
    #[error(
        "branch name {0:?} is not usable as a git branch and config key; avoid whitespace, control characters, and . : ? * [ \\ ~ ^ @ {{ }} and a leading '-'"
    )]
    Branch(String),
    #[error("remote name {0:?} is not a usable git remote name")]
    Remote(String),
    #[error("template {0:?} uses an unsupported placeholder; only {{name}} is substituted")]
    Template(String),
    #[error("cannot author package.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Command(#[from] tta_command::Error),
}

/// Adapter options, deserialized from the registry entry's `options` table.
/// Defaults reproduce the current live workflow with its rough edges removed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Options {
    /// Initial branch, created by `git init -b` (git 2.28+).
    #[serde(default = "default_branch")]
    branch: String,
    /// Git remotes by name, each a `{name}`-templated URL, merged over the
    /// defaults: `origin` (GitHub), `tngl` (tangled.org through the `tngl`
    /// SSH host alias), and `voodoo` (the archive). A value of `""` omits
    /// that remote; further keys add remotes. Only `origin` receives git
    /// tracking configuration; every configured remote gets
    /// `jj bookmark track` for the initial branch, which works before the
    /// remote repository exists and performs no network access.
    #[serde(default)]
    remotes: Option<BTreeMap<String, String>>,
    /// Empty files to create; each supports `{name}` templating.
    /// An empty list creates none.
    #[serde(default = "default_starter_files")]
    starter_files: Vec<String>,
    /// package.json `author`: parameter > supplied defaults > omitted.
    author: Option<String>,
    /// package.json `license`: parameter > supplied defaults > `"MIT"`.
    /// Empty means unset.
    license: Option<String>,
    /// Author a package.json with `module` `{name}.js` and type `module`.
    #[serde(default = "default_true")]
    package_json: bool,
    /// Run `pnpm install --silent` after authoring package.json.
    #[serde(default = "default_true")]
    install: bool,
}

fn default_branch() -> String {
    "main".to_owned()
}

/// Configured remotes merge over these defaults rather than replacing them.
fn default_remotes() -> BTreeMap<String, String> {
    [
        ("origin", "git@github.com:rektide/{name}"),
        ("tngl", "tngl:jauntywunderkind.bsky.social/{name}"),
        ("voodoo", "https://archive.voodoowarez.com/{name}"),
    ]
    .into_iter()
    .map(|(name, url)| (name.to_owned(), url.to_owned()))
    .collect()
}

fn default_starter_files() -> Vec<String> {
    vec!["README.md".to_owned(), "{name}.ts".to_owned()]
}

fn default_true() -> bool {
    true
}

/// Build the generator expression for `project` under `root`.
/// Defaults are observations supplied by the caller, never acquired here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Defaults {
    pub author: Option<String>,
    pub license: Option<String>,
}

pub fn scaffold(project: &str, root: &Path, options: &Table, defaults: &Defaults) -> Result<Expression, Error> {
    let options = Options::deserialize(toml::Value::Table(options.clone())).map_err(|error| Error::Options(error.to_string()))?;
    validate_branch(&options.branch)?;
    let directory = root.join(project);
    let mut steps: Vec<Expression> = Vec::new();

    // mkdir -p, quoted, under the configured root — the script's unquoted
    // `mkdir ~/src/$1` breaks on spaces and ignores configuration.
    steps.push(command("mkdir", vec!["-p".into(), directory.as_os_str().to_owned()])?);
    steps.push(ChangeDir::builder().path(Word::new(directory.as_os_str())?).build().into());
    // -b creates the branch directly; the script's init + `checkout -b main`
    // fails whenever git already defaults to that branch.
    steps.push(command("git", vec!["init".into(), "-b".into(), OsString::from(&options.branch)])?);

    let mut remotes = Vec::new();
    let configured = options.remotes.unwrap_or_default();
    for (name, template) in default_remotes().into_iter().chain(configured) {
        if name.is_empty() || name.chars().any(|character| character.is_control() || character.is_whitespace()) {
            return Err(Error::Remote(name));
        }
        remotes.retain(|(existing, _)| existing != &name);
        if !template.is_empty() {
            validate_template(&template)?;
            remotes.push((name, template));
        }
    }
    for (name, template) in &remotes {
        steps.push(command(
            "git",
            vec!["remote".into(), "add".into(), OsString::from(name), expand(template, project)],
        )?);
    }
    if remotes.iter().any(|(name, _)| name == "origin") {
        steps.push(command(
            "git",
            vec!["config".into(), format!("branch.{}.remote", options.branch).into(), "origin".into()],
        )?);
        steps.push(command(
            "git",
            vec![
                "config".into(),
                format!("branch.{}.merge", options.branch).into(),
                format!("refs/heads/{}", options.branch).into(),
            ],
        )?);
    }

    let mut touched = Vec::new();
    for file in &options.starter_files {
        validate_template(file)?;
        touched.push(expand(file, project));
    }
    if !touched.is_empty() {
        steps.push(command("touch", touched)?);
    }

    // jj colocates onto the existing git repository. The bookmark is created
    // at the working copy before the init commit, so the commit describes the
    // change the bookmark follows.
    steps.push(command("jj", vec!["git".into(), "init".into()])?);
    steps.push(command("jj", vec!["bookmark".into(), "create".into(), OsString::from(&options.branch)])?);

    if options.package_json {
        let author = resolve_author(options.author.as_deref(), defaults.author.as_deref(), None);
        let license = resolve_license(options.license.as_deref(), defaults.license.as_deref());
        steps.push(
            WriteFile::builder()
                .path(Word::new("package.json")?)
                .contents(package_json(project, author.as_deref(), &license)?)
                .build()
                .into(),
        );
        if options.install {
            steps.push(command("pnpm", vec!["install".into(), "--silent".into()])?);
        }
    }

    steps.push(command("jj", vec!["commit".into(), "-m".into(), "init".into()])?);
    // Tracking is local-only and valid before the remote repository exists;
    // live repos show tracked-but-not-yet-created remotes the same way.
    for (name, _) in &remotes {
        steps.push(command(
            "jj",
            vec![
                "bookmark".into(),
                "track".into(),
                OsString::from(format!("{branch}@{name}", branch = options.branch)),
            ],
        )?);
    }
    Ok(And::builder().steps(steps).build().into())
}

/// Parameter > environment > git > omitted; empty values mean unset.
fn resolve_author(parameter: Option<&str>, environment: Option<&str>, git: Option<&str>) -> Option<String> {
    parameter
        .filter(|value| !value.is_empty())
        .or_else(|| environment.filter(|value| !value.is_empty()))
        .or_else(|| git.filter(|value| !value.is_empty()))
        .map(str::to_owned)
}

fn resolve_license(parameter: Option<&str>, environment: Option<&str>) -> String {
    parameter.or(environment).filter(|value| !value.is_empty()).unwrap_or("MIT").to_owned()
}

fn package_json(name: &str, author: Option<&str>, license: &str) -> Result<Word, Error> {
    let mut package = serde_json::Map::new();
    package.insert("name".into(), serde_json::json!(name));
    package.insert("version".into(), serde_json::json!("1.0.0"));
    package.insert("description".into(), serde_json::json!(""));
    package.insert("license".into(), serde_json::json!(license));
    package.insert("type".into(), serde_json::json!("module"));
    package.insert("module".into(), serde_json::json!(format!("{name}.js")));
    package.insert("main".into(), serde_json::json!("index.js"));
    package.insert("scripts".into(), serde_json::json!({ "test": "echo \"Error: no test specified\" && exit 1" }));
    package.insert("keywords".into(), serde_json::json!([]));
    if let Some(author) = author {
        package.insert("author".into(), serde_json::json!(author));
    }
    // Keep generated bytes stable even when another dependency enables
    // serde_json/preserve_order (the entry statechart runtime does).
    package.sort_keys();
    let contents = format!("{}\n", serde_json::to_string_pretty(&package)?);
    Ok(Word::new(contents)?)
}

fn command(program: &str, args: Vec<OsString>) -> Result<Expression, Error> {
    Ok(Command::builder()
        .program(Word::new(program)?)
        .args(args.into_iter().map(Word::new).collect::<Result<Vec<_>, _>>()?)
        .build()
        .into())
}

/// Dots would nest the `branch.<name>` git config keys; the rest are git's
/// ref-format constraints or shell-adjacent metacharacters.
fn validate_branch(branch: &str) -> Result<(), Error> {
    if branch.is_empty()
        || branch.starts_with(['-', '/'])
        || branch.ends_with('/')
        || branch.contains(['.', ':', '?', '*', '[', '\\', '~', '^', '@', '{', '}'])
        || branch.chars().any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(Error::Branch(branch.to_owned()));
    }
    Ok(())
}

fn validate_template(template: &str) -> Result<(), Error> {
    if template.replace("{name}", "").contains(['{', '}']) {
        return Err(Error::Template(template.to_owned()));
    }
    Ok(())
}

fn expand(template: &str, name: &str) -> OsString {
    OsString::from(template.replace("{name}", name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tta_command::Handoff;

    fn options(text: &str) -> Table {
        text.parse::<toml::Table>().unwrap()
    }

    fn rendered(options: &Table) -> String {
        scaffold("fresh", Path::new("/root"), options, &Defaults::default())
            .unwrap()
            .shell(Handoff::Run)
            .unwrap()
    }

    #[test]
    fn package_keys_keep_the_original_sorted_emission_with_any_serde_json_features() {
        let package = package_json("fresh", Some("Author"), "MIT").unwrap();
        assert_eq!(
            package.as_os_str(),
            "{\n  \"author\": \"Author\",\n  \"description\": \"\",\n  \"keywords\": [],\n  \"license\": \"MIT\",\n  \"main\": \"index.js\",\n  \"module\": \"fresh.js\",\n  \"name\": \"fresh\",\n  \"scripts\": {\n    \"test\": \"echo \\\"Error: no test specified\\\" && exit 1\"\n  },\n  \"type\": \"module\",\n  \"version\": \"1.0.0\"\n}\n"
        );
    }

    #[test]
    fn defaults_reproduce_the_live_workflow_with_its_bugs_removed() {
        let script = rendered(&options(""));
        assert!(script.starts_with("'mkdir' '-p' '/root/fresh' && cd -P '/root/fresh'"), "{script}");
        assert!(script.contains("'git' 'init' '-b' 'main'"), "{script}");
        assert!(script.contains("'git' 'remote' 'add' 'origin' 'git@github.com:rektide/fresh'"), "{script}");
        assert!(
            script.contains("'git' 'remote' 'add' 'tngl' 'tngl:jauntywunderkind.bsky.social/fresh'"),
            "{script}"
        );
        assert!(
            script.contains("'git' 'remote' 'add' 'voodoo' 'https://archive.voodoowarez.com/fresh'"),
            "{script}"
        );
        assert!(script.contains("'git' 'config' 'branch.main.remote' 'origin'"), "{script}");
        assert!(script.contains("'git' 'config' 'branch.main.merge' 'refs/heads/main'"), "{script}");
        assert!(script.contains("'touch' 'README.md' 'fresh.ts'"), "{script}");
        // The bookmark is created before the init commit so the commit lands
        // on it, and every remote is tracked after the commit.
        let bookmark_at = script.find("'jj' 'bookmark' 'create' 'main'").unwrap();
        let commit_at = script.rfind("'jj' 'commit' '-m' 'init'").unwrap();
        let jj_at = script.find("'jj' 'git' 'init'").unwrap();
        assert!(jj_at < bookmark_at && bookmark_at < commit_at, "{script}");
        for remote in ["origin", "tngl", "voodoo"] {
            let track_at = script.find(&format!("'jj' 'bookmark' 'track' 'main@{remote}'")).unwrap();
            assert!(commit_at < track_at, "{script}");
        }
        assert!(script.contains("'pnpm' 'install' '--silent'"), "{script}");
    }

    #[test]
    fn remotes_options_add_remove_and_retarget() {
        let script = rendered(&options(
            r#"
branch = "trunk"

[remotes]
origin = "https://git.example.org/work/{name}.git"
tngl = "tngl:other.handle/{name}"
voodoo = ""
dl = "github.com:rektide/dl-{name}"
"#,
        ));
        assert!(script.contains("'git' 'init' '-b' 'trunk'"), "{script}");
        assert!(
            script.contains("'git' 'remote' 'add' 'origin' 'https://git.example.org/work/fresh.git'"),
            "{script}"
        );
        assert!(script.contains("'git' 'remote' 'add' 'tngl' 'tngl:other.handle/fresh'"), "{script}");
        assert!(script.contains("'git' 'remote' 'add' 'dl' 'github.com:rektide/dl-fresh'"), "{script}");
        assert!(script.contains("'git' 'config' 'branch.trunk.merge' 'refs/heads/trunk'"), "{script}");
        assert!(!script.contains("voodoo"), "{script}");
        for remote in ["origin", "tngl", "dl"] {
            assert!(script.contains(&format!("'jj' 'bookmark' 'track' 'trunk@{remote}'")), "{script}");
        }
        assert!(!script.contains("trunk@voodoo"), "{script}");
    }

    #[test]
    fn invalid_options_branches_remotes_and_templates_are_rejected() {
        for table in [
            "unknown-key = 1",
            "branch = \"ma in\"",
            "branch = \"release.1\"",
            "branch = \"-x\"",
            "[remotes]\norigin = \"https://x/{project}\"",
            "[remotes]\n\"bad name\" = \"https://x/{name}\"",
            "starter-files = [\"{name}.{ext}\"]",
        ] {
            let error = scaffold("fresh", Path::new("/root"), &options(table), &Defaults::default())
                .unwrap_err()
                .to_string();
            assert!(!error.is_empty());
        }
        // Disabling the origin remote also skips its config keys.
        let script = rendered(&options("[remotes]\norigin = \"\"\n"));
        assert!(!script.contains("'remote' 'add' 'origin'"), "{script}");
        assert!(!script.contains("branch.main"), "{script}");
        assert!(!script.contains("main@origin"), "{script}");
        assert!(script.contains("'jj' 'bookmark' 'track' 'main@tngl'"), "{script}");
    }

    #[test]
    fn author_and_license_resolution_layers_parameter_environment_and_defaults() {
        assert_eq!(resolve_author(Some("P"), Some("E"), Some("G")).as_deref(), Some("P"));
        assert_eq!(resolve_author(Some(""), Some("E"), Some("G")).as_deref(), Some("E"));
        assert_eq!(resolve_author(None, Some(""), Some("G")).as_deref(), Some("G"));
        assert_eq!(resolve_author(None, None, Some("")), None);
        assert_eq!(resolve_author(None, None, None), None);
        assert_eq!(resolve_license(Some("Apache-2.0"), Some("MIT")), "Apache-2.0");
        assert_eq!(resolve_license(None, Some("BSD-2-Clause")), "BSD-2-Clause");
        assert_eq!(resolve_license(Some(""), None), "MIT");
    }

    #[test]
    fn package_json_is_valid_json_with_the_script_fields() {
        let contents = package_json("fresh", Some("rektide 'quote'"), "MIT").unwrap();
        let text = contents.as_os_str().to_str().unwrap();
        assert!(text.ends_with("}\n"));
        let package: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(package["name"], "fresh");
        assert_eq!(package["module"], "fresh.js");
        assert_eq!(package["type"], "module");
        assert_eq!(package["author"], "rektide 'quote'");
        assert_eq!(package["license"], "MIT");
        let omitted = package_json("fresh", None, "MIT").unwrap();
        let package: serde_json::Value = serde_json::from_str(omitted.as_os_str().to_str().unwrap()).unwrap();
        assert!(package.get("author").is_none());
    }
}
