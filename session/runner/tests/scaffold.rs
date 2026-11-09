use std::error::Error as _;
#[cfg(feature = "scaffold-rekon")]
use std::{fs, path::Path};

#[cfg(feature = "scaffold-rekon")]
use tta_launcher::Selection;
use tta_launcher::{ResolveError, Session};
#[cfg(feature = "scaffold-rekon")]
use tta_runner::command::Execute;
use tta_runner::{
    backend::{LookupError, SessionLookup},
    scaffold,
};

struct Inventory(Vec<Session>);
impl SessionLookup for Inventory {
    fn sessions(&self) -> Result<Vec<Session>, LookupError> {
        Ok(self.0.clone())
    }
}

fn cause(error: &scaffold::Error) -> &ResolveError {
    let mut source = error.source();
    while let Some(error) = source {
        if let Some(error) = error.downcast_ref::<ResolveError>() {
            return error;
        }
        source = error.source();
    }
    panic!("missing typed resolution cause: {error:?}")
}

#[test]
fn empty_registry_and_disabled_policy_preserve_resolution_causes() {
    let root = tempfile::tempdir().unwrap();
    let config = scaffold::load_config_from(None).unwrap();
    for choice in ["default", "false"] {
        let error = scaffold::resolve()
            .lookup(&Inventory(vec![]))
            .argument("missing")
            .roots(&[root.path().into()])
            .choice(choice)
            .config(&config)
            .call()
            .unwrap_err();
        assert!(matches!(cause(&error), ResolveError::Missing { .. }));
        assert_eq!(error.to_string().contains("no generators are configured"), choice == "default");
    }
    for choice in ["unknown", ""] {
        assert!(
            scaffold::resolve()
                .lookup(&Inventory(vec![]))
                .argument("missing")
                .roots(&[root.path().into()])
                .choice(choice)
                .config(&config)
                .call()
                .is_err()
        );
    }
}

#[test]
fn choice_aliases_keep_the_existing_cli_meanings() {
    for choice in ["false", "0", "off", "none"] {
        assert_eq!(scaffold::Choice::parse(choice).unwrap(), scaffold::Choice::Off);
    }
    for choice in ["default", "true", "1"] {
        assert_eq!(scaffold::Choice::parse(choice).unwrap(), scaffold::Choice::Default);
    }
    assert_eq!(scaffold::Choice::parse("rk").unwrap(), scaffold::Choice::Named("rk".into()));
}

#[cfg(feature = "scaffold-rekon")]
fn registry(directory: &Path, text: &str) -> scaffold::Config {
    let path = directory.join("config.toml");
    fs::write(&path, text).unwrap();
    scaffold::load_config_from(Some(&path)).unwrap()
}

#[cfg(feature = "scaffold-rekon")]
const REKON: &str = "[scaffold.generators.rk]\nadapter = 'rekon'\n";

#[cfg(feature = "scaffold-rekon")]
#[test]
fn shared_resolution_only_generates_a_prefix_for_a_clean_miss() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("projects");
    fs::create_dir(&root).unwrap();
    let config = registry(temp.path(), REKON);
    let existing = Inventory(vec![Session {
        id: "$1".parse().unwrap(),
        name: "existing".into(),
        group: None,
    }]);
    let result = scaffold::resolve()
        .lookup(&existing)
        .argument("existing")
        .choice("default")
        .config(&config)
        .call()
        .unwrap();
    assert!(matches!(result.selection, Selection::Existing(_)));
    assert!(result.prefix.is_none() && result.generator.is_none());
    fs::create_dir(root.join("project")).unwrap();
    let result = scaffold::resolve()
        .lookup(&existing)
        .argument("project")
        .roots(std::slice::from_ref(&root))
        .choice("rk")
        .config(&config)
        .call()
        .unwrap();
    assert!(matches!(result.selection, Selection::Project { .. }));
    assert!(result.prefix.is_none());
    for choice in ["default", "rk"] {
        let result = scaffold::resolve()
            .lookup(&existing)
            .argument("fresh")
            .roots(std::slice::from_ref(&root))
            .session_name("override")
            .choice(choice)
            .config(&config)
            .call()
            .unwrap();
        assert!(matches!(result.selection, Selection::Create { session_name, .. } if session_name == "override"));
        assert_eq!(result.generator.unwrap().name, "rk");
        assert_eq!(result.prefix.unwrap().shell(tta_command::Handoff::Run).unwrap(), "'rekon' 'fresh'");
        assert!(!root.join("fresh").exists(), "resolution must not execute the generator");
    }
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn ambiguous_matches_and_generator_choices_stop_without_creation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("projects");
    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::create_dir(root.join("aloft")).unwrap();
    let config = registry(temp.path(), REKON);
    let sessions = Inventory(vec![
        Session {
            id: "$1".parse().unwrap(),
            name: "alpha".into(),
            group: None,
        },
        Session {
            id: "$2".parse().unwrap(),
            name: "aloft".into(),
            group: None,
        },
    ]);
    for inventory in [Inventory(vec![]), sessions] {
        let error = scaffold::resolve()
            .lookup(&inventory)
            .argument("al")
            .roots(std::slice::from_ref(&root))
            .choice("default")
            .config(&config)
            .call()
            .unwrap_err();
        assert!(matches!(cause(&error), ResolveError::Sessions { .. } | ResolveError::Projects { .. }));
        assert!(!root.join("al").exists());
    }
    let config = registry(temp.path(), &format!("{REKON}[scaffold.generators.second]\nadapter = 'rekon'\n"));
    for (choice, expected) in [("default", "multiple scaffold generators"), ("absent", "unknown scaffold generator")] {
        let error = scaffold::resolve()
            .lookup(&Inventory(vec![]))
            .argument("fresh")
            .roots(std::slice::from_ref(&root))
            .choice(choice)
            .config(&config)
            .call()
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
    assert!(!root.join("fresh").exists());
}

#[cfg(feature = "scaffold-rekon")]
#[test]
fn generator_prefix_runs_before_creation_and_failure_blocks_the_client() {
    use std::os::unix::fs::PermissionsExt;
    use tta_command::{Assignment, Expression, Handoff, Word};
    use tta_launcher::{ClientMode, launch, socket};

    fn environment(expression: &mut Expression, assignments: &[Assignment]) {
        match expression {
            Expression::Command(command) => command.environment.extend_from_slice(assignments),
            Expression::And(and) => and.steps.iter_mut().for_each(|step| environment(step, assignments)),
            Expression::ChangeDir(_) | Expression::Write(_) => {}
        }
    }

    for shell in [false, true] {
        for fail in [false, true] {
            let temp = tempfile::Builder::new().prefix("tta-scaffold-").tempdir_in("/tmp/opencode").unwrap();
            let root = temp.path().join("projects");
            let bin = temp.path().join("bin");
            fs::create_dir(&root).unwrap();
            fs::create_dir(&bin).unwrap();
            let events = temp.path().join("events");
            // PATH contains only this shim, so the user's real rekon cannot run.
            fs::write(
                bin.join("rekon"),
                "#!/bin/sh\nprintf 'rekon %s\\n' \"$1\" >> \"$TTA_TEST_EVENTS\"\n[ \"$TTA_TEST_FAIL\" = 1 ] && exit 29\n/bin/mkdir \"$TTA_TEST_ROOT/$1\"\n",
            )
            .unwrap();
            fs::write(bin.join("tmux"), "#!/bin/sh\nprintf 'tmux %s\\n' \"$PWD\" >> \"$TTA_TEST_EVENTS\"\n").unwrap();
            for name in ["rekon", "tmux"] {
                fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o700)).unwrap();
            }
            let config = registry(temp.path(), REKON);
            let resolved = scaffold::resolve()
                .lookup(&Inventory(vec![]))
                .argument("fresh")
                .roots(std::slice::from_ref(&root))
                .choice("default")
                .config(&config)
                .call()
                .unwrap();
            let mut plan = launch()
                .selection(&resolved.selection)
                .socket(&socket::Resolved::path(temp.path().join("unused-sock")))
                .client_mode(ClientMode::Attach)
                .program(bin.join("tmux").as_os_str())
                .maybe_prefix(resolved.prefix.as_ref())
                .call()
                .unwrap();
            environment(
                &mut plan,
                &[
                    Assignment::new("PATH", Word::new(bin.as_os_str()).unwrap()).unwrap(),
                    Assignment::new("TTA_TEST_ROOT", Word::new(root.as_os_str()).unwrap()).unwrap(),
                    Assignment::new("TTA_TEST_EVENTS", Word::new(events.as_os_str()).unwrap()).unwrap(),
                    Assignment::new("TTA_TEST_FAIL", Word::new(if fail { "1" } else { "0" }).unwrap()).unwrap(),
                ],
            );
            assert!(!events.exists());
            let status = if shell {
                std::process::Command::new("/bin/sh")
                    .args(["-c", &plan.shell(Handoff::Run).unwrap()])
                    .status()
                    .unwrap()
            } else {
                plan.execute(Handoff::Run).unwrap().unwrap()
            };
            assert_eq!(status.code(), Some(if fail { 29 } else { 0 }));
            assert_eq!(root.join("fresh").exists(), !fail);
            let expected = if fail {
                "rekon fresh\n".to_owned()
            } else {
                format!("rekon fresh\ntmux {}\n", root.join("fresh").display())
            };
            assert_eq!(fs::read_to_string(events).unwrap(), expected);
        }
    }
}
