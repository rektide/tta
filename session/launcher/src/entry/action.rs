//! Semantic decisions and exact bound operations. Lowering does not default
//! grouping, select work or allocate names, and has no native-plan sidecar.
use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use tmax::operation::Invocation;
use tta_command::{And, Assignment, ChangeDir, Command, Expression, Word};

use super::{
    protocol::{BoundTarget, ClientEntry, InitialWindow, ReadRequest},
    select::{MatchReason, ViewFingerprint, ViewRef, WorkspaceKey, WorkspaceRef},
};

/// Constructor-checked native path. Tmux's `-c` format boundary currently needs
/// UTF-8, but encoding is still exclusively the lowerer's job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewDirectory(PathBuf);

impl ViewDirectory {
    pub(super) fn new(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() || path.to_str().is_none() {
            return Err(Error::Directory(path.to_owned()));
        }
        Word::new(path.as_os_str())?;
        Ok(Self(path.to_owned()))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Constructor-checked new-work pane directory: the explicit override or the
/// selected project route's entry path (or the creation destination).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneDirectory(PathBuf);

impl PaneDirectory {
    pub(super) fn new(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() || path.to_str().is_none() {
            return Err(Error::Directory(path.to_owned()));
        }
        Word::new(path.as_os_str())?;
        Ok(Self(path.to_owned()))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewNaming {
    pub(super) prefix: String,
    pub(super) directory: Option<ViewDirectory>,
}

/// One observed route to a physical project directory: the configured slot,
/// its requested spelling, the observed physical root, and the entry's
/// basename plus canonical directory on that route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRoute {
    /// One-based configured root slot, matching `@N` selectors.
    pub slot: usize,
    pub requested_root: PathBuf,
    /// Observed physical root (after symlink resolution).
    pub root: PathBuf,
    pub name: String,
    /// Observed canonical physical directory of this route's entry.
    pub directory: PathBuf,
}

impl ProjectRoute {
    /// The entry path through this route's physical root; tmux `-c`/cd target.
    pub fn entry_path(&self) -> PathBuf {
        self.root.join(&self.name)
    }
}

impl From<&crate::ProjectSource> for ProjectRoute {
    fn from(source: &crate::ProjectSource) -> Self {
        Self {
            slot: source.root_index,
            requested_root: source.requested_root.clone(),
            root: source.root.clone(),
            name: source.name.clone(),
            directory: source.directory.clone(),
        }
    }
}

/// A chosen existing project: physical identity, deterministic selected route
/// and every observed equivalent route to the same physical directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRef {
    pub name: String,
    /// Selected route's entry path (physical root + basename).
    pub entry: PathBuf,
    /// Observed canonical physical directory all routes share.
    pub physical: PathBuf,
    pub route: ProjectRoute,
    pub routes: Vec<ProjectRoute>,
}

/// The fixed destination of a scaffold creation: configured slot, observed
/// physical creation root, new basename and the exact path to be created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreationSite {
    /// One-based configured root slot whose physical root was chosen.
    pub slot: usize,
    pub requested_root: PathBuf,
    pub root: PathBuf,
    pub name: String,
    pub path: PathBuf,
}

/// New-work session specification, settled once at planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewSessionSpec {
    pub name: String,
    pub startup: Vec<OsString>,
    pub directory: PaneDirectory,
    pub window: InitialWindow,
}

/// The materialized generator program of a scaffold decision, built exactly
/// once with the actually observed defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorProgram {
    pub generator: String,
    pub program: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryDecision {
    EnterView {
        view: ViewRef,
        via: ClientEntry,
    },
    OpenView {
        work: WorkspaceRef,
        naming: ViewNaming,
        via: ClientEntry,
    },
    StartProject {
        project: ProjectRef,
        session: NewSessionSpec,
        via: ClientEntry,
    },
    ScaffoldAndStart {
        destination: CreationSite,
        generator: GeneratorProgram,
        session: NewSessionSpec,
        via: ClientEntry,
    },
}

impl EntryDecision {
    /// New-work decisions need a second binding read (route identity or
    /// destination absence) beyond the fresh session inventory.
    pub(crate) fn needs_binding_evidence(&self) -> bool {
        matches!(self, Self::StartProject { .. } | Self::ScaffoldAndStart { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryPlan {
    pub(super) target: BoundTarget,
    pub(super) decision: EntryDecision,
    pub(super) basis: DecisionBasis,
}

/// Typed provenance of a decision: which requests produced it and the
/// identity/match facts that survive into the plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionBasis {
    Workspace {
        selection: ReadRequest,
        workspace: WorkspaceKey,
        matched_by: MatchReason,
    },
    Project(Box<ProjectBasis>),
    Creation(Box<CreationBasis>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectBasis {
    pub selection: ReadRequest,
    pub roots: Vec<ReadRequest>,
    pub routes: Vec<ProjectRoute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreationBasis {
    pub selection: ReadRequest,
    pub roots: Vec<ReadRequest>,
    pub destination: Option<ReadRequest>,
    pub defaults: Option<ReadRequest>,
    pub generator: String,
}

impl EntryPlan {
    pub fn decision(&self) -> &EntryDecision {
        &self.decision
    }
    pub fn target(&self) -> &BoundTarget {
        &self.target
    }
    pub fn selection_request(&self) -> &ReadRequest {
        match &self.basis {
            DecisionBasis::Workspace { selection, .. } => selection,
            DecisionBasis::Project(basis) => &basis.selection,
            DecisionBasis::Creation(basis) => &basis.selection,
        }
    }
    pub fn basis(&self) -> &DecisionBasis {
        &self.basis
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundOperation {
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
    StartProject {
        session: BoundSession,
        via: ClientEntry,
    },
    ScaffoldAndStart {
        generator: Expression,
        session: BoundSession,
        via: ClientEntry,
    },
}

/// A new-work session exactly as bound: fresh-checked name, pane directory,
/// startup argv and window policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundSession {
    pub name: String,
    pub directory: PaneDirectory,
    pub startup: Vec<OsString>,
    pub window: InitialWindow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundAction {
    pub(super) target: BoundTarget,
    pub(super) operation: BoundOperation,
    /// Fresh session inventory that witnessed the binding.
    pub(super) sessions_witness: ReadRequest,
    /// Route-identity or destination-absence witness for new-work bindings.
    pub(super) evidence_witness: Option<ReadRequest>,
}

impl BoundAction {
    pub fn operation(&self) -> &BoundOperation {
        &self.operation
    }
    pub fn binding_requests(&self) -> Vec<&ReadRequest> {
        self.evidence_witness.iter().chain(std::iter::once(&self.sessions_witness)).collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportedAction {
    pub(super) action: BoundAction,
    pub(super) expression: Expression,
    pub(super) shell: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Command(#[from] tta_command::Error),
    #[error(transparent)]
    Tmux(#[from] tmax::operation::BuildError),
    #[error(transparent)]
    Name(#[from] crate::ResolveError),
    #[error("tmux start-directory must be an absolute UTF-8 path: {0:?}")]
    Directory(PathBuf),
    #[error("invalid initial window name {0:?}: pass a printable name or window-name=false")]
    Window(String),
}

fn literal_format(value: &str) -> String {
    value.replace('#', "#{a:35}")
}

/// `-s`, `-n` and `-c` are tmux formats, a separate language from shell/argv
/// quoting. Emit hashes through the character modifier so neither formats nor
/// #() jobs nor the special ##[ style rule can interpret a literal name/path.
fn directory_format(directory: &Path) -> Result<String, Error> {
    directory
        .to_str()
        .filter(|_| directory.is_absolute())
        .map(literal_format)
        .ok_or_else(|| Error::Directory(directory.to_owned()))
}

/// The `-n` value for a created session's initial window, format-escaped.
/// Custom names may contain `.`/`:`/spaces (tmux window names are freer than
/// session names) but must be printable: the value crosses a line-oriented
/// CLI and lands in the status line.
fn window_title(window: &InitialWindow, session_name: &str) -> Result<Option<String>, Error> {
    Ok(match window {
        InitialWindow::Unnamed => None,
        InitialWindow::AfterSession => Some(literal_format(session_name)),
        InitialWindow::Named(name) => {
            if name.is_empty() || name.chars().any(char::is_control) {
                return Err(Error::Window(name.clone()));
            }
            Some(literal_format(name))
        }
    })
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

/// One tmux invocation queue over the bound target. Startup argv belongs to
/// the final invocation as literal `--` operands; a lone executable stays an
/// argv value via `/bin/sh -c exec "$0"` rather than shell source.
fn tmux_queue(target: &BoundTarget, invocations: Vec<Invocation>, startup: &[OsString]) -> Result<Expression, Error> {
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
    if !startup.is_empty() {
        args.push(Word::new("--")?);
        if startup.len() == 1 {
            args.extend(["/bin/sh", "-c", "exec \"$0\""].into_iter().map(Word::new).collect::<Result<Vec<_>, _>>()?);
        }
        args.extend(startup.iter().map(startup_word).collect::<Result<Vec<_>, _>>()?);
    }
    Ok(Command::builder()
        .program(target.program.clone())
        .args(args)
        .environment(environment)
        .build()
        .into())
}

/// New-work session creation: enter the pane directory, create the session
/// (detached only when this client switches within the server), then move
/// this client onto it by exact name. Startup argv rides the creation queue.
fn session_steps(target: &BoundTarget, session: &BoundSession, via: ClientEntry) -> Result<Vec<Expression>, Error> {
    crate::resolve::validate_name(&session.name)?;
    let mut create = Invocation::new("new-session")?;
    if via == ClientEntry::SwitchWithinServer {
        create = create.arg("-d");
    }
    create = create.args(["-s", &literal_format(&session.name)]);
    if let Some(window) = window_title(&session.window, &session.name)? {
        create = create.args(["-n", &window]);
    }
    create = create.args(["-c", &directory_format(session.directory.path())?]);
    let mut steps = vec![
        ChangeDir::builder().path(Word::new(session.directory.path().as_os_str())?).build().into(),
        tmux_queue(target, vec![create], &session.startup)?,
    ];
    if via == ClientEntry::SwitchWithinServer {
        steps.push(tmux_queue(
            target,
            vec![Invocation::new(client(via))?.args(["-t", &format!("={}", session.name)])],
            &[],
        )?);
    }
    Ok(steps)
}

fn client(via: ClientEntry) -> &'static str {
    match via {
        ClientEntry::AttachTerminal => "attach-session",
        ClientEntry::SwitchWithinServer => "switch-client",
    }
}

pub(super) fn lower(action: &BoundAction) -> Result<Expression, Error> {
    let target = &action.target;
    let invocations = match &action.operation {
        BoundOperation::EnterView { view, via } => {
            vec![Invocation::new(client(*via))?.args(["-t", &view.id.to_string()])]
        }
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
        BoundOperation::StartProject { session, via } => {
            return Ok(And::builder().steps(session_steps(target, session, *via)?).build().into());
        }
        BoundOperation::ScaffoldAndStart { generator, session, via } => {
            let mut steps = Vec::with_capacity(4);
            steps.push(generator.clone());
            steps.extend(session_steps(target, session, *via)?);
            return Ok(And::builder().steps(steps).build().into());
        }
    };
    tmux_queue(target, invocations, &[])
}

/// Raw startup operands must survive as native argv values; check them once
/// at planning so no later encoding surprise exists.
pub(super) fn validate_startup(startup: &[OsString]) -> Result<(), Error> {
    for word in startup {
        Word::new(word)?;
    }
    Ok(())
}
