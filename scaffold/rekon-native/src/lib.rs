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
    #[error("template {0:?} uses an unsupported placeholder; only {{name}} is substituted")]
    Template(String),
    #[error("cannot author package.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Command(#[from] tta_command::Error),
}

/// Adapter options, deserialized from the registry entry's `options` table.
/// Defaults reproduce the script's behavior with its rough edges removed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Options {
    /// Initial branch, created by `git init -b` (git 2.28+).
    #[serde(default = "default_branch")]
    branch: String,
    /// `origin` remote template; `{name}` substitutes the project name.
    /// An empty string omits the remote and its tracking configuration.
    #[serde(default = "default_origin")]
    origin_remote: String,
    /// Secondary remote template added as `voodoo`, like the script.
    /// An empty string omits it.
    #[serde(default = "default_voodoo")]
    voodoo_remote: String,
    /// Empty files to create; each supports `{name}` templating.
    /// An empty list creates none.
    #[serde(default = "default_starter_files")]
    starter_files: Vec<String>,
    /// package.json `author`: parameter > `AUTHOR` environment >
    /// `git config user.name` > omitted. Empty means unset.
    author: Option<String>,
    /// package.json `license`: parameter > `LICENSE` environment > `"MIT"`.
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

fn default_origin() -> String {
    "git@github.com:rektide/{name}".to_owned()
}

fn default_voodoo() -> String {
    "https://archive.voodoowarez.com/{name}".to_owned()
}

fn default_starter_files() -> Vec<String> {
    vec!["README.md".to_owned(), "{name}.ts".to_owned()]
}

fn default_true() -> bool {
    true
}

/// Build the generator expression for `project` under `root`.
pub fn scaffold(project: &str, root: &Path, options: &Table) -> Result<Expression, Error> {
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

    if !options.origin_remote.is_empty() {
        validate_template(&options.origin_remote)?;
        steps.push(command(
            "git",
            vec!["remote".into(), "add".into(), "origin".into(), expand(&options.origin_remote, project)],
        )?);
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
    if !options.voodoo_remote.is_empty() {
        validate_template(&options.voodoo_remote)?;
        steps.push(command(
            "git",
            vec!["remote".into(), "add".into(), "voodoo".into(), expand(&options.voodoo_remote, project)],
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

    // jj colocates onto the existing git repository.
    steps.push(command("jj", vec!["git".into(), "init".into()])?);

    if options.package_json {
        let author = resolve_author(
            options.author.as_deref(),
            std::env::var_os("AUTHOR").as_ref().and_then(|value| value.to_str()),
            git_user_name().as_deref(),
        );
        let license = resolve_license(
            options.license.as_deref(),
            std::env::var_os("LICENSE").as_ref().and_then(|value| value.to_str()),
        );
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

/// A failed or missing git identity simply omits the author field.
fn git_user_name() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
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
        scaffold("fresh", Path::new("/root"), options).unwrap().shell(Handoff::Run).unwrap()
    }

    #[test]
    fn defaults_reproduce_the_script_with_its_bugs_removed() {
        let script = rendered(&options(""));
        assert!(script.starts_with("'mkdir' '-p' '/root/fresh' && cd -P '/root/fresh'"), "{script}");
        assert!(script.contains("'git' 'init' '-b' 'main'"), "{script}");
        assert!(script.contains("'git' 'remote' 'add' 'origin' 'git@github.com:rektide/fresh'"), "{script}");
        assert!(
            script.contains("'git' 'remote' 'add' 'voodoo' 'https://archive.voodoowarez.com/fresh'"),
            "{script}"
        );
        assert!(script.contains("'git' 'config' 'branch.main.remote' 'origin'"), "{script}");
        assert!(script.contains("'git' 'config' 'branch.main.merge' 'refs/heads/main'"), "{script}");
        assert!(script.contains("'touch' 'README.md' 'fresh.ts'"), "{script}");
        assert!(script.contains("'jj' 'git' 'init'"), "{script}");
        assert!(script.contains("'pnpm' 'install' '--silent'"), "{script}");
        assert!(script.ends_with("'jj' 'commit' '-m' 'init'"), "{script}");
        // package.json is authored before installing, after jj initializes.
        let package_at = script.find("'printf' '%s'").unwrap();
        let install_at = script.find("'pnpm' 'install'").unwrap();
        let jj_at = script.find("'jj' 'git' 'init'").unwrap();
        assert!(jj_at < package_at && package_at < install_at, "{script}");
    }

    #[test]
    fn options_parameterize_branch_remotes_files_and_package_steps() {
        let script = rendered(&options(
            r#"
branch = "trunk"
origin-remote = "https://git.example.org/work/{name}.git"
voodoo-remote = ""
starter-files = ["{name}.rs"]
package-json = false
"#,
        ));
        assert!(script.contains("'git' 'init' '-b' 'trunk'"), "{script}");
        assert!(
            script.contains("'git' 'remote' 'add' 'origin' 'https://git.example.org/work/fresh.git'"),
            "{script}"
        );
        assert!(script.contains("'git' 'config' 'branch.trunk.merge' 'refs/heads/trunk'"), "{script}");
        assert!(!script.contains("voodoo"), "{script}");
        assert!(script.contains("'touch' 'fresh.rs'"), "{script}");
        assert!(!script.contains("printf"), "{script}");
        assert!(!script.contains("pnpm"), "{script}");
        assert!(script.ends_with("'jj' 'commit' '-m' 'init'"), "{script}");
    }

    #[test]
    fn invalid_options_branches_and_templates_are_rejected() {
        for table in [
            "unknown-key = 1",
            "branch = \"ma in\"",
            "branch = \"release.1\"",
            "branch = \"-x\"",
            "origin-remote = \"https://x/{project}\"",
            "starter-files = [\"{name}.{ext}\"]",
        ] {
            let error = scaffold("fresh", Path::new("/root"), &options(table)).unwrap_err().to_string();
            assert!(!error.is_empty());
        }
        // Disabling the origin remote also skips its config keys.
        let script = rendered(&options("origin-remote = \"\""));
        assert!(!script.contains("'remote' 'add' 'origin'"), "{script}");
        assert!(!script.contains("branch.main"), "{script}");
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
