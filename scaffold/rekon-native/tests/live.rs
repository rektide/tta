//! Live exercise of a generated expression with real git and jj, mirroring the
//! launcher's live tmux tests. Requires git and jj on PATH.
#![cfg(unix)]

use std::path::Path;
use std::process::Command;

use tta_command::Handoff;
use tta_scaffold_rekon_native::scaffold;

fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git").current_dir(directory).args(args).output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn the_generated_sequence_builds_a_tracked_colocated_project() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    // A local bare repository stands in for origin; tangled and the archive
    // are disabled so nothing leaves the temporary directory.
    let bare = temp.path().join("origin.git");
    assert!(Command::new("git").args(["init", "--bare"]).arg(&bare).status().unwrap().success());
    let options = format!("[remotes]\norigin = \"{}\"\ntngl = \"\"\nvoodoo = \"\"\n", bare.display())
        .parse::<toml::Table>()
        .unwrap();

    let expression = scaffold("fresh", &root, &options).unwrap();
    expression.execute(Handoff::Run).unwrap();

    let project = root.join("fresh");
    assert!(project.join("README.md").is_file());
    assert!(project.join("fresh.ts").is_file());
    let package: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(project.join("package.json")).unwrap()).unwrap();
    assert_eq!(package["module"], "fresh.js");
    assert_eq!(package["type"], "module");

    assert_eq!(git(&project, &["config", "branch.main.remote"]), "origin\n");
    assert_eq!(git(&project, &["config", "branch.main.merge"]), "refs/heads/main\n");
    let remote = git(&project, &["remote", "get-url", "origin"]);
    assert_eq!(remote.trim(), bare.display().to_string());

    // The bookmark sits on the described init commit, not an empty working
    // copy, and tracks origin even though nothing was pushed anywhere.
    let bookmark = String::from_utf8(
        Command::new("jj")
            .current_dir(&project)
            .args(["bookmark", "list", "--remote", "origin", "--no-pager"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(bookmark.contains("main"), "{bookmark}");
    assert!(bookmark.contains("not created yet"), "{bookmark}");
    let described = String::from_utf8(
        Command::new("jj")
            .current_dir(&project)
            .args(["log", "-r", "main", "--no-graph", "-T", "description", "--no-pager"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(described.trim(), "init", "bookmark must point at the init commit");
    // The exported bookmark is visible to git as the branch.
    assert!(!git(&project, &["rev-parse", "--verify", "refs/heads/main"]).is_empty());
}
