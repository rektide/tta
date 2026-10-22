//! Adapter for the external `rekon` scaffold script.
//!
//! `rekon <name>` is the user's own script; this adapter only expresses it as
//! a tta command value. The script itself decides where the project lands
//! (currently a hardcoded `~/src`) and everything it does afterwards, so a
//! non-default `--root` diverges until the script is ported natively. The
//! program is a bare PATH name resolved when the plan runs, and the command
//! runs from the invocation cwd; tta's following `cd` step verifies that the
//! expected directory appeared.
#![forbid(unsafe_code)]

use std::path::Path;

use tta_command::{Command, Expression, Word};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Command(#[from] tta_command::Error),
}

/// Registry name of this adapter.
pub const ADAPTER: &str = "rekon";

/// Materialize `project` as `rekon 'project'`; `root` is informational for
/// adapters that need a target location.
pub fn scaffold(project: &str, _root: &Path) -> Result<Expression, Error> {
    Ok(Command::builder().program(Word::new(ADAPTER)?).args(vec![Word::new(project)?]).build().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tta_command::Handoff;

    #[test]
    fn the_script_is_invoked_with_just_the_project_name() {
        let expression = scaffold("fresh-project", Path::new("/root")).unwrap();
        assert_eq!(expression.shell(Handoff::Run).unwrap(), "'rekon' 'fresh-project'");
    }
}
