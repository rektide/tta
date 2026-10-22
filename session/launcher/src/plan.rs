use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

use bon::builder;
use tmax::operation::Invocation;
use tta_command::{And, Assignment, ChangeDir, Command, Expression, Word};

use crate::{Selection, Session};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientMode {
    Attach,
    Switch,
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error(transparent)]
    Command(#[from] tta_command::Error),
    #[error(transparent)]
    Tmux(#[from] tmax::operation::BuildError),
    #[error(transparent)]
    Name(#[from] crate::ResolveError),
    #[error("grouped launch requires a satellite session name chosen from the session inventory")]
    UnnamedSatellite,
    #[error("tmux start-directory must be an absolute UTF-8 path: {0:?}")]
    Directory(std::path::PathBuf),
    #[error("a create selection needs its generator command as the plan prefix")]
    MissingPrefix,
}

/// Assemble commands only; this performs no I/O. New outside sessions are
/// created and attached in one tmux invocation. Grouped satellites are created
/// detached under an exact chosen name in both client modes, then this client
/// moves onto the satellite: attach-session outside tmux, switch-client inside.
/// Grouping applies only to existing selections; startup argv only to project
/// and create selections. An explicit directory overrides the selected
/// project's initial directory and supplies `-c` for satellites. Library
/// callers must supply an absolute path. Create selections run their generator
/// command as the first step, before entering the directory it must produce.
#[builder]
pub fn launch(
    selection: &Selection,
    socket: &crate::socket::Resolved,
    client_mode: ClientMode,
    program: &OsStr,
    config: Option<&Path>,
    prefix: Option<&Expression>,
    #[builder(default)] grouped: bool,
    satellite_name: Option<&str>,
    #[builder(default = &[])] startup: &[OsString],
    directory: Option<&Path>,
) -> Result<Expression, PlanError> {
    let command = |invocations: Vec<Invocation>, startup: &[OsString]| -> Result<Expression, PlanError> {
        let (mut args, environment) = match &socket.label {
            Some(label) => (
                vec![Word::new("-L")?, Word::new(&label.name)?],
                vec![Assignment::new("TMUX_TMPDIR", Word::new(label.directory.as_os_str())?)?],
            ),
            None => (vec![Word::new("-S")?, Word::new(socket.path.as_os_str())?], Vec::new()),
        };
        if let Some(config) = config {
            args.extend([Word::new("-f")?, Word::new(config.as_os_str())?]);
        }
        for (index, invocation) in invocations.into_iter().enumerate() {
            if index != 0 {
                args.push(Word::new(";")?);
            }
            args.extend(invocation.argv()?.into_iter().map(Word::new).collect::<Result<Vec<_>, _>>()?);
        }
        if !startup.is_empty() {
            args.push(Word::new("--")?);
            // tmux treats a single shell-command operand as shell source. Keep
            // even a lone executable (including spaces) an argv value instead.
            if startup.len() == 1 {
                args.extend(["/bin/sh", "-c", "exec \"$0\""].into_iter().map(Word::new).collect::<Result<Vec<_>, _>>()?);
            }
            args.extend(startup.iter().map(startup_word).collect::<Result<Vec<_>, _>>()?);
        }
        Ok(Command::builder()
            .program(Word::new(program)?)
            .args(args)
            .environment(environment)
            .build()
            .into())
    };
    let client = match client_mode {
        ClientMode::Attach => "attach-session",
        ClientMode::Switch => "switch-client",
    };
    let new_project_steps =
        |directory: Option<&Path>, project_directory: &Path, session_name: &str, startup: &[OsString]| -> Result<Vec<Expression>, PlanError> {
            crate::resolve::validate_name(session_name)?;
            let directory = directory.unwrap_or(project_directory);
            let path = directory_format(directory)?;
            let mut create = Invocation::new("new-session")?;
            if client_mode == ClientMode::Switch {
                create = create.arg("-d");
            }
            create = create.args(["-s", &literal_format(session_name), "-c", &path]);
            let mut steps = vec![
                ChangeDir::builder().path(Word::new(directory.as_os_str())?).build().into(),
                command(vec![create], startup)?,
            ];
            if client_mode == ClientMode::Switch {
                steps.push(command(vec![Invocation::new(client)?.args(["-t", &format!("={session_name}")])], &[])?);
            }
            Ok(steps)
        };
    match selection {
        Selection::Existing(session) if grouped => {
            let satellite = satellite_name.ok_or(PlanError::UnnamedSatellite)?;
            crate::resolve::validate_name(satellite)?;
            let target = format!("={satellite}");
            let mut create = Invocation::new("new-session")?.args(["-d", "-s", &literal_format(satellite), "-t", &session.id.to_string()]);
            if let Some(directory) = directory {
                create = create.args(["-c", &directory_format(directory)?]);
            }
            // Both client modes create the satellite detached, then move this
            // client onto it by exact name: nothing depends on the create
            // command's unchanged current-target context.
            let invocations = vec![
                create,
                Invocation::new(client)?.args(["-t", &target]),
                // set-option parses a pane target; the colon fixes the exact
                // session component rather than treating the name as a pane.
                Invocation::new("set-option")?.args(["-t", &format!("{target}:"), "destroy-unattached", "keep-last"]),
            ];
            command(invocations, &[])
        }
        Selection::Existing(session) => command(vec![Invocation::new(client)?.args(["-t", &session.id.to_string()])], &[]),
        Selection::Project {
            directory: project_directory,
            session_name,
        } => Ok(And::builder()
            .steps(new_project_steps(directory, project_directory, session_name, startup)?)
            .build()
            .into()),
        Selection::Create {
            directory: project_directory,
            session_name,
        } => {
            let prefix = prefix.ok_or(PlanError::MissingPrefix)?;
            let mut steps = Vec::with_capacity(4);
            steps.push(prefix.to_owned());
            steps.extend(new_project_steps(directory, project_directory, session_name, startup)?);
            Ok(And::builder().steps(steps).build().into())
        }
    }
}

/// A grouped satellite's name: `_`, the base session's name, then the smallest
/// counter no live session already holds (`compfuzor` → `_compfuzor-1`,
/// `_compfuzor-2`, …). The `_` prefix keeps the base name an unambiguous
/// completion prefix while satellites are alive, and uniqueness is judged
/// against every live session name, not only other satellites.
pub fn satellite_name(base: &str, existing: &[Session]) -> String {
    let prefix = format!("_{base}-");
    let used: HashSet<u64> = existing
        .iter()
        .filter_map(|session| session.name.strip_prefix(&prefix).and_then(|suffix| suffix.parse().ok()))
        .collect();
    let mut counter = 1;
    while used.contains(&counter) {
        counter += 1;
    }
    format!("{prefix}{counter}")
}

fn directory_format(directory: &Path) -> Result<String, PlanError> {
    directory
        .to_str()
        .filter(|_| directory.is_absolute())
        .map(literal_format)
        .ok_or_else(|| PlanError::Directory(directory.to_owned()))
}

/// Invocation is UTF-8-only. Its argv boundary rule also applies to raw Unix
/// startup operands: escape only the final semicolon, without lossy conversion.
/// Startup operands are not tmux formats, so hashes remain literal.
fn startup_word(value: &OsString) -> Result<Word, tta_command::Error> {
    let mut bytes = value.as_bytes().to_vec();
    if bytes.last() == Some(&b';') {
        bytes.insert(bytes.len() - 1, b'\\');
    }
    Word::new(OsString::from_vec(bytes))
}

/// `-s` and `-c` are tmux formats, a separate language from shell/argv quoting.
/// Emit hashes through the character modifier so neither formats nor #() jobs
/// nor the special ##[ style rule can interpret a literal name/path.
fn literal_format(value: &str) -> String {
    value.replace('#', "#{a:35}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tta_command::Handoff;

    #[test]
    fn existing_sessions_target_canonical_ids_not_fuzzy_names() {
        let session = Selection::Existing(crate::Session {
            id: "$7".parse().unwrap(),
            name: "a; #[x]".into(),
        });
        for (mode, expected) in [(ClientMode::Attach, "attach-session"), (ClientMode::Switch, "switch-client")] {
            let expression = launch()
                .selection(&session)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(mode)
                .program(OsStr::new("tmux"))
                .call()
                .unwrap();
            assert_eq!(
                expression.shell(Handoff::Run).unwrap(),
                format!("'tmux' '-S' '/tmp/sock' '{expected}' '-t' '$7'")
            );
        }
    }

    #[test]
    fn inside_creation_is_detached_then_exact_switch_and_values_are_tmux_encoded() {
        let selection = Selection::Project {
            directory: "/src/#[a]' ;".into(),
            session_name: "#[a];".into(),
        };
        let expression = launch()
            .selection(&selection)
            .socket(&crate::socket::Resolved::path("/tmp/sock"))
            .client_mode(ClientMode::Switch)
            .program(OsStr::new("tmux"))
            .call()
            .unwrap();
        let Expression::And(and) = expression else { panic!("expected sequence") };
        let Expression::Command(create) = &and.steps[1] else {
            panic!("expected create")
        };
        let args: Vec<_> = create.args.iter().map(|word| word.as_os_str().to_str().unwrap()).collect();
        assert_eq!(
            args,
            ["-S", "/tmp/sock", "new-session", "-d", "-s", "#{a:35}[a]\\;", "-c", "/src/#{a:35}[a]' \\;"]
        );
        let Expression::Command(switch) = &and.steps[2] else {
            panic!("expected switch")
        };
        assert_eq!(switch.args.last().unwrap().as_os_str(), "=#[a]\\;");
    }

    #[test]
    fn grouped_satellites_are_created_detached_and_targeted_exactly_in_both_modes() {
        let session = Selection::Existing(crate::Session {
            id: "$7".parse().unwrap(),
            name: "seed".into(),
        });
        for (mode, client) in [(ClientMode::Attach, "attach-session"), (ClientMode::Switch, "switch-client")] {
            let expression = launch()
                .selection(&session)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(mode)
                .program(OsStr::new("tmux"))
                .grouped(true)
                .satellite_name("_seed-2 #[x]")
                .call()
                .unwrap();
            assert_eq!(
                expression.shell(Handoff::Run).unwrap(),
                format!(
                    "'tmux' '-S' '/tmp/sock' 'new-session' '-d' '-s' '_seed-2 #{{a:35}}[x]' '-t' '$7' ';' \
                     '{client}' '-t' '=_seed-2 #[x]' ';' 'set-option' '-t' '=_seed-2 #[x]:' \
                     'destroy-unattached' 'keep-last'"
                )
            );
        }
    }

    #[test]
    fn grouped_launch_refuses_to_plan_without_a_satellite_name() {
        let session = Selection::Existing(crate::Session {
            id: "$7".parse().unwrap(),
            name: "seed".into(),
        });
        assert!(matches!(
            launch()
                .selection(&session)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(ClientMode::Attach)
                .program(OsStr::new("tmux"))
                .grouped(true)
                .call(),
            Err(PlanError::UnnamedSatellite)
        ));
    }

    #[test]
    fn satellite_names_prefix_the_base_and_skip_live_counters() {
        let sessions = |names: &[&str]| {
            names
                .iter()
                .map(|name| Session {
                    id: "$1".parse().unwrap(),
                    name: (*name).to_owned(),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(satellite_name("compfuzor", &sessions(&["compfuzor"])), "_compfuzor-1");
        assert_eq!(satellite_name("compfuzor", &sessions(&["compfuzor", "_compfuzor-1"])), "_compfuzor-2");
        assert_eq!(satellite_name("compfuzor", &sessions(&["_compfuzor-3", "_compfuzor-1"])), "_compfuzor-2");
        // Non-numeric suffixes, other bases, and unparsable counters never consume a slot.
        assert_eq!(satellite_name("a", &sessions(&["_a-x", "_b-1", "_a-99999999999999999999999"])), "_a-1");
        // A satellite of a satellite still derives from its own base name.
        assert_eq!(satellite_name("_a-1", &sessions(&["__a-1-1"])), "__a-1-2");
    }

    #[test]
    fn create_plans_run_the_generator_before_entering_its_directory() {
        let selection = Selection::Create {
            directory: "/src/fresh".into(),
            session_name: "fresh".into(),
        };
        let prefix = Expression::from(
            Command::builder()
                .program(Word::new("rekon").unwrap())
                .args(vec![Word::new("fresh").unwrap()])
                .build(),
        );
        let expression = launch()
            .selection(&selection)
            .socket(&crate::socket::Resolved::path("/tmp/sock"))
            .client_mode(ClientMode::Attach)
            .program(OsStr::new("tmux"))
            .prefix(&prefix)
            .call()
            .unwrap();
        assert_eq!(
            expression.shell(Handoff::Run).unwrap(),
            "'rekon' 'fresh' && cd -P '/src/fresh' && 'tmux' '-S' '/tmp/sock' 'new-session' '-s' 'fresh' '-c' '/src/fresh'"
        );
        // A create selection without its generator command is refused.
        assert!(matches!(
            launch()
                .selection(&selection)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(ClientMode::Attach)
                .program(OsStr::new("tmux"))
                .call(),
            Err(PlanError::MissingPrefix)
        ));
    }
}
