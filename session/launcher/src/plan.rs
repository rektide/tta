use std::ffi::OsStr;
use std::path::Path;

use bon::builder;
use tmax::operation::Invocation;
use tta_command::{And, ChangeDir, Command, Expression, Word};

use crate::Selection;

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
    #[error("tmux project start-directory must be an absolute UTF-8 path: {0:?}")]
    Directory(std::path::PathBuf),
}

/// Assemble commands only; this performs no I/O. New outside sessions are
/// created and attached in one tmux invocation. Inside tmux, create detached,
/// then switch the invoking client only if creation succeeds.
#[builder]
pub fn launch(selection: &Selection, socket: &Path, client_mode: ClientMode, program: &OsStr) -> Result<Expression, PlanError> {
    let command = |invocation: Invocation| -> Result<Expression, PlanError> {
        let mut args = vec![Word::new("-S")?, Word::new(socket.as_os_str())?];
        args.extend(invocation.argv()?.into_iter().map(Word::new).collect::<Result<Vec<_>, _>>()?);
        Ok(Command::builder().program(Word::new(program)?).args(args).build().into())
    };
    let client = match client_mode {
        ClientMode::Attach => "attach-session",
        ClientMode::Switch => "switch-client",
    };
    match selection {
        Selection::Existing(session) => command(Invocation::new(client)?.args(["-t", &session.id.to_string()])),
        Selection::Project { directory, session_name } => {
            crate::resolve::validate_name(session_name)?;
            let path = directory
                .to_str()
                .filter(|_| directory.is_absolute())
                .ok_or_else(|| PlanError::Directory(directory.clone()))?;
            let mut create = Invocation::new("new-session")?;
            if client_mode == ClientMode::Switch {
                create = create.arg("-d");
            }
            create = create.args(["-s", &literal_format(session_name), "-c", &literal_format(path)]);
            let mut steps = vec![ChangeDir::builder().path(Word::new(directory.as_os_str())?).build().into(), command(create)?];
            if client_mode == ClientMode::Switch {
                steps.push(command(Invocation::new(client)?.args(["-t", &format!("={session_name}")]))?);
            }
            Ok(And::builder().steps(steps).build().into())
        }
    }
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
                .socket(Path::new("/tmp/sock"))
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
            .socket(Path::new("/tmp/sock"))
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
}
