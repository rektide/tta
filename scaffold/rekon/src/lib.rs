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

use toml::Table;
use tta_command::{Command, Expression, Word};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Command(#[from] tta_command::Error),
    #[error("generator uses adapter \"rekon\", which takes no options; found {0:?}")]
    Options(Table),
}

/// Registry name of this adapter.
pub const ADAPTER: &str = "rekon";

/// Destination contract declared by this adapter: the external script decides
/// where the project lands (currently a hardcoded location), so tta's selected
/// root does not govern it.
pub const SELECTS_OWN_DESTINATION: bool = true;

/// Which observed target defaults this adapter still needs after validation.
/// The external script owns its own defaults; this adapter needs none.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DefaultsNeeds {
    pub author: bool,
    pub license: bool,
}

/// Validate this generator's options without materializing anything.
pub fn validate(options: &Table) -> Result<DefaultsNeeds, Error> {
    if !options.is_empty() {
        return Err(Error::Options(options.clone()));
    }
    Ok(DefaultsNeeds::default())
}

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

    #[test]
    fn validation_rejects_options_and_needs_no_target_defaults() {
        let options: Table = "anything = 1".parse().unwrap();
        assert!(validate(&options).is_err());
        assert_eq!(validate(&Table::new()).unwrap(), DefaultsNeeds::default());
        assert!(SELECTS_OWN_DESTINATION);
    }
}
