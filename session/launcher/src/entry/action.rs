//! Semantic decisions and exact bound operations. Lowering does not default
//! grouping, select work or allocate names, and has no native-plan sidecar.
use std::path::{Path, PathBuf};

use tmax::operation::Invocation;
use tta_command::{Assignment, Command, Expression, Word};

use super::{
    protocol::{BoundTarget, ClientEntry, ReadRequest},
    select::{MatchReason, ViewFingerprint, ViewRef, WorkspaceKey, WorkspaceRef},
};

/// Constructor-checked native path. Tmux's `-c` format boundary currently needs
/// UTF-8, but encoding is still exclusively the lowerer's job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ViewDirectory(PathBuf);

impl ViewDirectory {
    pub(super) fn new(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() || path.to_str().is_none() {
            return Err(Error::Directory(path.to_owned()));
        }
        Word::new(path.as_os_str())?;
        Ok(Self(path.to_owned()))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ViewNaming {
    pub(super) prefix: String,
    pub(super) directory: Option<ViewDirectory>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EntryDecision {
    EnterView { view: ViewRef, via: ClientEntry },
    OpenView { work: WorkspaceRef, naming: ViewNaming, via: ClientEntry },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EntryPlan {
    pub(super) target: BoundTarget,
    pub(super) decision: EntryDecision,
    pub(super) basis: DecisionBasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DecisionBasis {
    pub(super) selection: ReadRequest,
    pub(super) workspace: WorkspaceKey,
    pub(super) matched_by: MatchReason,
}

impl EntryPlan {
    pub(crate) fn decision(&self) -> &EntryDecision {
        &self.decision
    }
    pub(crate) fn target(&self) -> &BoundTarget {
        &self.target
    }
    pub(crate) fn selection_request(&self) -> &ReadRequest {
        &self.basis.selection
    }
    pub(crate) fn basis(&self) -> &DecisionBasis {
        &self.basis
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BoundOperation {
    EnterView {
        view: ViewFingerprint,
        via: ClientEntry,
    },
    /// Always one CLI tmux queue, not three independently dispatched effects.
    OpenView {
        anchor: ViewFingerprint,
        name: String,
        directory: Option<ViewDirectory>,
        via: ClientEntry,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundAction {
    pub(super) target: BoundTarget,
    pub(super) operation: BoundOperation,
    pub(super) witness: ReadRequest,
}

impl BoundAction {
    pub(crate) fn operation(&self) -> &BoundOperation {
        &self.operation
    }
    pub(crate) fn binding_request(&self) -> &ReadRequest {
        &self.witness
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExportedAction {
    pub(super) action: BoundAction,
    pub(super) expression: Expression,
    pub(super) shell: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    Command(#[from] tta_command::Error),
    #[error(transparent)]
    Tmux(#[from] tmax::operation::BuildError),
    #[error(transparent)]
    Name(#[from] crate::ResolveError),
    #[error("tmux start-directory must be an absolute UTF-8 path: {0:?}")]
    Directory(PathBuf),
}

fn literal_format(value: &str) -> String {
    value.replace('#', "#{a:35}")
}

pub(super) fn lower(action: &BoundAction) -> Result<Expression, Error> {
    let client = |via| match via {
        ClientEntry::AttachTerminal => "attach-session",
        ClientEntry::SwitchWithinServer => "switch-client",
    };
    let invocations = match &action.operation {
        BoundOperation::EnterView { view, via } => vec![Invocation::new(client(*via))?.args(["-t", &view.id.to_string()])],
        BoundOperation::OpenView { anchor, name, directory, via } => {
            let mut create = Invocation::new("new-session")?.args(["-d", "-s", &literal_format(name), "-t", &anchor.id.to_string()]);
            if let Some(directory) = directory {
                create = create.args(["-c", &literal_format(directory.path().to_str().expect("validated view directory"))]);
            }
            vec![
                create,
                Invocation::new(client(*via))?.args(["-t", &format!("={name}")]),
                Invocation::new("set-option")?.args(["-t", &format!("={name}:"), "destroy-unattached", "keep-last"]),
            ]
        }
    };
    let target = &action.target;
    let (mut args, environment) = match &target.socket.label {
        Some(label) => (
            vec![Word::new("-L")?, Word::new(label.name.clone())?],
            vec![Assignment::new("TMUX_TMPDIR", Word::new(label.directory.as_os_str())?)?],
        ),
        None => (vec![Word::new("-S")?, Word::new(target.socket.path.as_os_str())?], Vec::new()),
    };
    if let Some(config) = &target.config {
        args.extend([Word::new("-f")?, config.clone()]);
    }
    for (index, invocation) in invocations.into_iter().enumerate() {
        if index != 0 {
            args.push(Word::new(";")?);
        }
        args.extend(invocation.argv()?.into_iter().map(Word::new).collect::<Result<Vec<_>, _>>()?);
    }
    Ok(Command::builder()
        .program(target.program.clone())
        .args(args)
        .environment(environment)
        .build()
        .into())
}
