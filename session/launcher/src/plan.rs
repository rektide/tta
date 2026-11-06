use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

use bon::builder;
use tmax::operation::Invocation;
use tta_command::{And, Assignment, ChangeDir, Command, Expression, Word};

use crate::Selection;
use crate::{LaunchPlan, NativeCreation};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientMode {
    Attach,
    Switch,
}

/// The initial window name for a session this plan creates. Naming a window
/// makes tmux disable `automatic-rename` for it. Grouped satellites cannot
/// take one at all: their creation targets the group (`-t`) and tmux rejects
/// `-n` combined with `-t`, so the choice only reaches plain `new-session`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum WindowName {
    /// Name the initial window after the new session (the default).
    #[default]
    Session,
    /// Name the initial window exactly this.
    Custom(String),
    /// Leave the window unnamed so `automatic-rename` keeps applying.
    Off,
}

impl WindowName {
    /// Parse the shared flag vocabulary: bare/`default`/`true`/`1` keep the
    /// session name, `false`/`off`/`none`/`0` disable, anything else names
    /// the window. Mirrors [`crate::scaffold::Choice::parse`].
    pub fn parse(value: &str) -> Result<Self, PlanError> {
        Ok(match value {
            "" => return Err(PlanError::InvalidWindowName(value.to_owned())),
            "default" | "true" | "1" => Self::Session,
            "false" | "0" | "off" | "none" => Self::Off,
            name => Self::Custom(name.to_owned()),
        })
    }
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
    #[error("window name {0:?} is empty or contains control characters; pass a printable --window-name or --window-name=false")]
    InvalidWindowName(String),
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
/// Created sessions name their initial window per `window_name` (default:
/// after the session); grouped satellites cannot take a window name.
#[builder]
pub fn launch(
    selection: &Selection,
    socket: &crate::socket::Resolved,
    client_mode: ClientMode,
    program: &OsStr,
    config: Option<&Path>,
    prefix: Option<&Expression>,
    grouped: Option<bool>,
    satellite_name: Option<&str>,
    #[builder(default = &[])] startup: &[OsString],
    directory: Option<&Path>,
    #[builder(default)] window_name: WindowName,
) -> Result<Expression, PlanError> {
    Ok(prepare_launch()
        .selection(selection)
        .socket(socket)
        .client_mode(client_mode)
        .program(program)
        .maybe_config(config)
        .maybe_prefix(prefix)
        .maybe_grouped(grouped)
        .maybe_satellite_name(satellite_name)
        .startup(startup)
        .maybe_directory(directory)
        .window_name(window_name)
        .call()?
        .into_expression())
}

/// Prepare both the portable CLI expression and eligible native execution.
/// Like [`launch`], preparation performs no I/O; shell output always uses the
/// complete CLI expression, never a partially executed native plan.
#[builder]
pub fn prepare_launch(
    selection: &Selection,
    socket: &crate::socket::Resolved,
    client_mode: ClientMode,
    program: &OsStr,
    config: Option<&Path>,
    prefix: Option<&Expression>,
    grouped: Option<bool>,
    satellite_name: Option<&str>,
    #[builder(default = &[])] startup: &[OsString],
    directory: Option<&Path>,
    #[builder(default)] window_name: WindowName,
) -> Result<LaunchPlan, PlanError> {
    let grouped = grouped.unwrap_or(client_mode == ClientMode::Attach);
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
    let mut native_creation = None;
    let new_project_steps =
        &mut |directory: Option<&Path>, project_directory: &Path, session_name: &str, startup: &[OsString]| -> Result<Vec<Expression>, PlanError> {
            crate::resolve::validate_name(session_name)?;
            let directory = directory.unwrap_or(project_directory);
            let path = directory_format(directory)?;
            let mut create = Invocation::new("new-session")?;
            if client_mode == ClientMode::Switch {
                create = create.arg("-d");
            }
            create = create.args(["-s", &literal_format(session_name)]);
            if let Some(window) = window_title(&window_name, session_name)? {
                create = create.args(["-n", &window]);
            }
            create = create.args(["-c", &path]);
            if client_mode == ClientMode::Switch
                && let Some(startup) = startup.iter().map(|word| word.to_str()).collect::<Option<Vec<_>>>()
            {
                let mut invocation = create.clone();
                if !startup.is_empty() {
                    invocation = invocation.arg("--");
                    if startup.len() == 1 {
                        invocation = invocation.args(["/bin/sh", "-c", "exec \"$0\""]);
                    }
                    invocation = invocation.args(startup);
                }
                native_creation = Some(NativeCreation {
                    invocation,
                    prefix: if matches!(selection, Selection::Create { .. }) {
                        prefix.cloned()
                    } else {
                        None
                    },
                    directory: directory.to_owned(),
                    handoff: command(vec![Invocation::new(client)?.args(["-t", &format!("={session_name}")])], &[])?,
                });
            }
            let mut steps = vec![
                ChangeDir::builder().path(Word::new(directory.as_os_str())?).build().into(),
                command(vec![create], startup)?,
            ];
            if client_mode == ClientMode::Switch {
                steps.push(command(vec![Invocation::new(client)?.args(["-t", &format!("={session_name}")])], &[])?);
            }
            Ok(steps)
        };
    let expression = match selection {
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
            let handoff = vec![
                Invocation::new(client)?.args(["-t", &target]),
                // set-option parses a pane target; the colon fixes the exact
                // session component rather than treating the name as a pane.
                Invocation::new("set-option")?.args(["-t", &format!("{target}:"), "destroy-unattached", "keep-last"]),
            ];
            let mut invocations = vec![create];
            invocations.extend(handoff);
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
    }?;
    Ok(LaunchPlan {
        expression,
        socket: socket.path.clone(),
        native_creation,
    })
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

/// `-s`, `-n` and `-c` are tmux formats, a separate language from shell/argv quoting.
/// Emit hashes through the character modifier so neither formats nor #() jobs
/// nor the special ##[ style rule can interpret a literal name/path.
fn literal_format(value: &str) -> String {
    value.replace('#', "#{a:35}")
}

/// The `-n` value for a created session's initial window, format-escaped.
/// Custom names may contain `.`/`:`/spaces (tmux window names are freer than
/// session names) but must be printable: the value crosses a line-oriented
/// CLI and lands in the status line.
fn window_title(window_name: &WindowName, session_name: &str) -> Result<Option<String>, PlanError> {
    Ok(match window_name {
        WindowName::Off => None,
        WindowName::Session => Some(literal_format(session_name)),
        WindowName::Custom(name) => {
            if name.is_empty() || name.chars().any(char::is_control) {
                return Err(PlanError::InvalidWindowName(name.to_owned()));
            }
            Some(literal_format(name))
        }
    })
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
            group: None,
        });
        for (mode, expected) in [(ClientMode::Attach, "attach-session"), (ClientMode::Switch, "switch-client")] {
            let expression = launch()
                .selection(&session)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(mode)
                .program(OsStr::new("tmux"))
                .grouped(false)
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
            [
                "-S",
                "/tmp/sock",
                "new-session",
                "-d",
                "-s",
                "#{a:35}[a]\\;",
                // The default window name is the session name, equally escaped.
                "-n",
                "#{a:35}[a]\\;",
                "-c",
                "/src/#{a:35}[a]' \\;"
            ]
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
            group: None,
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
            group: None,
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
    fn omitted_grouping_is_independent_outside_and_ordinary_switching_inside() {
        let selection = Selection::Existing(crate::Session {
            id: "$7".parse().unwrap(),
            name: "seed".into(),
            group: None,
        });
        for mode in [ClientMode::Attach, ClientMode::Switch] {
            let shell = launch()
                .selection(&selection)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(mode)
                .program(OsStr::new("tmux"))
                .satellite_name("_seed-1")
                .call()
                .unwrap()
                .shell(Handoff::Run)
                .unwrap();
            assert_eq!(shell.contains("new-session"), mode == ClientMode::Attach);
            assert_eq!(shell.contains("keep-last"), mode == ClientMode::Attach);
        }
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
            "'rekon' 'fresh' && cd -P '/src/fresh' && 'tmux' '-S' '/tmp/sock' 'new-session' '-s' 'fresh' '-n' 'fresh' '-c' '/src/fresh'"
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

    #[test]
    fn window_names_customize_disable_and_survive_format_characters() {
        let selection = Selection::Project {
            directory: "/src/project".into(),
            session_name: "project".into(),
        };
        let plan = |window_name: WindowName| {
            launch()
                .selection(&selection)
                .socket(&crate::socket::Resolved::path("/tmp/sock"))
                .client_mode(ClientMode::Attach)
                .program(OsStr::new("tmux"))
                .window_name(window_name)
                .call()
        };
        // A custom name may carry the characters session names reject and is
        // format-escaped like -s/-c values.
        assert_eq!(
            plan(WindowName::Custom("#[a] edit: main".into())).unwrap().shell(Handoff::Run).unwrap(),
            "cd -P '/src/project' && 'tmux' '-S' '/tmp/sock' 'new-session' '-s' 'project' '-n' '#{a:35}[a] edit: main' '-c' '/src/project'"
        );
        // Off restores today's unnamed window: no -n at all.
        assert_eq!(
            plan(WindowName::Off).unwrap().shell(Handoff::Run).unwrap(),
            "cd -P '/src/project' && 'tmux' '-S' '/tmp/sock' 'new-session' '-s' 'project' '-c' '/src/project'"
        );
        // Parse maps the flag vocabulary; unprintable customs are refused.
        assert_eq!(WindowName::parse("default").unwrap(), WindowName::Session);
        assert_eq!(WindowName::parse("false").unwrap(), WindowName::Off);
        assert_eq!(WindowName::parse("none").unwrap(), WindowName::Off);
        assert_eq!(WindowName::parse("editor").unwrap(), WindowName::Custom("editor".into()));
        assert!(matches!(WindowName::parse(""), Err(PlanError::InvalidWindowName(_))));
        assert!(matches!(plan(WindowName::Custom("\u{7}".into())), Err(PlanError::InvalidWindowName(_))));
    }

    #[test]
    fn grouped_satellites_never_take_a_window_name() {
        let session = Selection::Existing(crate::Session {
            id: "$7".parse().unwrap(),
            name: "seed".into(),
            group: None,
        });
        let expression = launch()
            .selection(&session)
            .socket(&crate::socket::Resolved::path("/tmp/sock"))
            .client_mode(ClientMode::Attach)
            .program(OsStr::new("tmux"))
            .grouped(true)
            .satellite_name("_seed-1")
            // Even a custom name stays out: tmux rejects -n with -t and the
            // satellite shares the group's existing windows.
            .window_name(WindowName::Custom("editor".into()))
            .call()
            .unwrap();
        assert_eq!(
            expression.shell(Handoff::Run).unwrap(),
            "'tmux' '-S' '/tmp/sock' 'new-session' '-d' '-s' '_seed-1' '-t' '$7' ';' \
             'attach-session' '-t' '=_seed-1' ';' 'set-option' '-t' '=_seed-1:' \
             'destroy-unattached' 'keep-last'"
        );
    }
}
