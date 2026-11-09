use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::{action, machine, select};

/// Process-local correlation, not durable identity or crash-resume authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AttemptId(u64);

impl AttemptId {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(
            NEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .expect("attempt identity exhausted"),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestId {
    pub attempt: AttemptId,
    pub sequence: u64,
}

/// Attempt-scoped effect correlation, sharing the attempt's monotonic
/// sequence with reads. An ID authorizes one dispatch in its issuing
/// attempt; it is not a crash-recovery or exactly-once promise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectId {
    pub attempt: AttemptId,
    pub sequence: u64,
}

/// Scoped to a frozen supplied target in this attempt. A host name or equal
/// socket spelling alone is never an identity proof, nor is this an incarnation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetBindingId(pub(super) AttemptId);

impl TargetBindingId {
    pub fn attempt(self) -> AttemptId {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointKey {
    pub target: TargetBindingId,
    pub socket: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientEntry {
    AttachTerminal,
    /// Supplied same-server policy, NOT an observed invoking client ID. Guarded
    /// origin switching is a separate, uncommissioned capability.
    SwitchWithinServer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewPreference {
    Auto,
    Independent,
    Shared,
}

/// Initial window naming for a session this attempt creates. Only new-work
/// decisions can carry it; existing-work entry has no window policy.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub enum InitialWindow {
    /// Name the initial window after the new session (the delivered default).
    #[default]
    AfterSession,
    /// Name the initial window exactly this.
    Named(String),
    /// Leave the window unnamed so `automatic-rename` keeps applying.
    Unnamed,
}

#[derive(Clone, Debug)]
pub struct EntryIntent {
    pub selector: String,
    pub view: ViewPreference,
    pub satellite_prefix: String,
    /// Only OpenView can retain this. Existing-work decisions have no startup
    /// argv/window-policy fields; those belong to new-work operations.
    pub view_directory: Option<PathBuf>,
    /// Configured target-local project roots in configured order, absolute.
    /// An empty list is legal until project fallback actually needs roots.
    pub roots: Vec<PathBuf>,
    /// Explicit session name for new work; `None` uses the project basename.
    pub session_name: Option<String>,
    /// Startup argv for new-work sessions. Raw bytes; crossing into a tmux
    /// operand is exclusively the lowerer's encoding decision.
    pub startup: Vec<OsString>,
    /// Explicit pane start-directory override for new work; `None` uses the
    /// selected project route's entry path (or the creation destination).
    pub directory: Option<PathBuf>,
    pub window: InitialWindow,
}

#[derive(Clone, Debug)]
pub struct SuppliedContext {
    pub host: String,
    pub cwd: PathBuf,
    pub socket: crate::socket::Resolved,
    pub program: OsString,
    pub config: Option<PathBuf>,
    pub client: ClientEntry,
}

pub struct Invocation {
    pub context: SuppliedContext,
    pub intent: EntryIntent,
    pub registry: crate::scaffold::Config,
    pub generator: crate::scaffold::Choice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundTarget {
    pub(super) scope: super::context::Scope,
    pub(super) binding: TargetBindingId,
    pub(super) host: String,
    pub(super) cwd: PathBuf,
    pub(super) socket: crate::socket::Resolved,
    pub(super) program: tta_command::Word,
    pub(super) config: Option<tta_command::Word>,
    pub(super) client: ClientEntry,
}

impl BoundTarget {
    pub fn scope(&self) -> &super::context::Scope {
        &self.scope
    }
    pub fn program(&self) -> &std::ffi::OsStr {
        self.program.as_os_str()
    }
    pub fn cwd(&self) -> &std::path::Path {
        &self.cwd
    }
    pub fn client(&self) -> ClientEntry {
        self.client
    }
    pub(super) fn from_supplied(attempt: AttemptId, context: SuppliedContext) -> Result<Self, InputError> {
        if context.host.is_empty() || context.host.chars().any(char::is_control) || context.program.is_empty() {
            return Err(InputError::Target("host and tmux program must be nonempty; host must be printable"));
        }
        for path in [&context.cwd, &context.socket.path].into_iter().chain(context.config.iter()) {
            if !path.is_absolute() {
                return Err(InputError::Path(path.clone()));
            }
            tta_command::Word::new(path.as_os_str())?;
        }
        if let Some(label) = &context.socket.label {
            if !label.directory.is_absolute() {
                return Err(InputError::Path(label.directory.clone()));
            }
            let bytes = label.name.as_encoded_bytes();
            if bytes.is_empty() || label.name == "." || label.name == ".." || bytes.contains(&b'/') {
                return Err(InputError::Target("invalid tmux socket label"));
            }
            tta_command::Word::new(label.name.clone())?;
            tta_command::Word::new(label.directory.as_os_str())?;
        }
        Ok(Self {
            scope: super::context::Scope {
                origin: super::context::Origin::Local { host: context.host.clone() },
                cwd: Some(context.cwd.clone()),
            },
            binding: TargetBindingId(attempt),
            host: context.host,
            cwd: context.cwd,
            socket: context.socket,
            program: tta_command::Word::new(context.program)?,
            config: context.config.map(|p| tta_command::Word::new(p.into_os_string())).transpose()?,
            client: context.client,
        })
    }

    pub fn endpoint(&self) -> EndpointKey {
        EndpointKey {
            target: self.binding,
            socket: self.socket.path.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("invalid acquisition context: {0}")]
    Context(#[source] anyhow::Error),
    #[error("provide a nonempty session/project selector")]
    EmptySelector,
    #[error("invalid supplied target: {0}")]
    Target(&'static str),
    #[error("supplied target path must be absolute: {0:?}")]
    Path(PathBuf),
    #[error("project roots must be absolute target-local paths: {0:?}")]
    Root(PathBuf),
    #[error("invalid initial window name {0:?}: pass a printable name or window-name=false")]
    Window(String),
    #[error(transparent)]
    Action(#[from] action::Error),
    #[error(transparent)]
    Name(#[from] crate::ResolveError),
    #[error(transparent)]
    Command(#[from] tta_command::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionReadPurpose {
    Catalog,
    Selection,
    BindAction,
}

/// Destination checks happen at preparation (authorizing a creation) and again
/// at binding (rechecking the already-chosen action); they never share proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationPurpose {
    Preparation,
    Binding,
}

/// One-based position in the configured root list, matching `@N` selectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootSlot(pub usize);

/// Which observed target defaults a validated generator still needs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefaultNeeds {
    pub author: bool,
    pub license: bool,
}

impl DefaultNeeds {
    pub(super) fn any(self) -> bool {
        self.author || self.license
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadKind {
    TargetContext,
    /// One semantic registry acquisition. Any bounded component walk belongs
    /// to the observer, never to the entry chart's selection course.
    Registry {
        path: Option<PathBuf>,
    },
    EndpointIdentity {
        current: PathBuf,
        selected: PathBuf,
    },
    Sessions {
        endpoint: EndpointKey,
        program: OsString,
        purpose: SessionReadPurpose,
    },
    /// One required root's basename-filtered inventory. `root` is the exact
    /// configured path the observation must answer.
    Projects {
        slot: RootSlot,
        root: PathBuf,
        filter: String,
    },
    Destination {
        path: PathBuf,
        purpose: DestinationPurpose,
    },
    GeneratorDefaults {
        cwd: PathBuf,
        needs: DefaultNeeds,
    },
    /// Canonicalize the chosen route's entry path; the reply is the physical
    /// directory it resolves to now.
    ProjectIdentity {
        route: Box<super::action::ProjectRoute>,
    },
    /// Fresh canonical identity of a chosen creation root: canonicalize the
    /// configured spelling and compare against the physical root observed at
    /// preparation. A destination lstat alone never proves the root unchanged.
    CreationRootIdentity {
        root: PathBuf,
        expected: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadRequest {
    pub id: RequestId,
    pub target: TargetBindingId,
    pub scope: Box<super::context::Scope>,
    pub kind: ReadKind,
}

impl ReadRequest {
    pub(super) fn sessions_purpose(&self) -> SessionReadPurpose {
        match self.kind {
            ReadKind::Sessions { purpose, .. } => purpose,
            _ => panic!("sessions purpose of a sessions request"),
        }
    }

    pub(super) fn destination_purpose(&self) -> DestinationPurpose {
        match self.kind {
            ReadKind::Destination { purpose, .. } => purpose,
            _ => panic!("destination purpose of a destination request"),
        }
    }

    pub(super) fn endpoint(&self) -> &EndpointKey {
        match &self.kind {
            ReadKind::Sessions { endpoint, .. } => endpoint,
            _ => panic!("endpoint of a sessions request"),
        }
    }
}

/// Only complete successful inventories supply presence/absence. A partial read
/// must be an ObservationFailure; it has no conversion to this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionInventory {
    Absent,
    Present(Vec<crate::Session>),
}

impl SessionInventory {
    pub fn sessions(&self) -> &[crate::Session] {
        match self {
            Self::Absent => &[],
            Self::Present(sessions) => sessions,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Observation {
    Context(super::context::Facts),
    Registry(Option<String>),
    EndpointIdentity {
        current: PathBuf,
        selected: PathBuf,
    },
    Sessions(SessionInventory),
    /// Complete successful inventory of one requested root. Partial reads
    /// must be `Err`; they never convert to an (empty) inventory.
    Projects(crate::ProjectInventory),
    /// Exact lstat-shaped evidence: only `Absent` (NotFound) authorizes
    /// creation; `Present` includes dangling symlinks.
    Destination(DestinationEvidence),
    Defaults(GeneratorDefaults),
    ProjectIdentity(PathBuf),
}

/// Classified destination evidence, distinct from a failed inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationEvidence {
    Absent,
    Present,
}

/// Observed target defaults. `None` fields are explicitly-absent completed
/// facts, not missing values the generator must acquire itself.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GeneratorDefaults {
    pub author: Option<String>,
    pub license: Option<String>,
}

/// Native observer errors (including capture/protocol/UTF-8 failures) retain
/// their source type, bytes and nested chain; the actor only sees `stop`.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct ObservationFailure {
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

pub struct ReadCompletion {
    /// Exact echo checks purpose/endpoint/program as well as ID/target. Future
    /// path/slot requests must extend this envelope, not rely on enum type alone.
    pub request: ReadRequest,
    pub result: Result<Observation, ObservationFailure>,
}

pub enum Delivery {
    Export,
    /// Execute the bound action. `native` states whether the driving host can
    /// perform native detached creation; eligibility itself is still decided
    /// purely from the fresh binding inventory and bound target.
    Execute {
        native: bool,
    },
}
pub enum Event {
    Observed(ReadCompletion),
    Deliver(Delivery),
    /// The driver records dispatch intent BEFORE invoking the operation.
    /// Another start for the same ID is rejected; waiting does not reissue.
    EffectStarted(EffectId),
    EffectFinished(EffectCompletion),
    /// Transfer one outstanding, undispatched terminal request and end this
    /// attempt's ownership honestly. The consumer observes the later outcome.
    DelegateTerminal(EffectId),
    Cancel,
}

pub enum Waiting<'a> {
    Read(&'a ReadRequest),
    Planned(&'a action::EntryPlan),
    Effect(&'a EffectRequest),
    InFlight(EffectId),
    Finished(&'a AttemptOutcome),
}

/// One executable effect as bound. Native payloads (generator programs, tmux
/// invocations, expressions) never cross serde; they stay in the attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectKind {
    /// Foreground generator program; success must produce the directory the
    /// following check verifies.
    Generate { program: tta_command::Expression },
    /// Post-generation observation in the execution phase, not another
    /// selection opportunity: the produced directory must exist.
    CheckCreatedDirectory { path: PathBuf },
    /// One native detached tmux session creation. Never retried and never
    /// followed by a CLI fallback once dispatched.
    CreateNative {
        invocation: tmax::operation::Invocation,
        directory: PathBuf,
    },
    /// The terminal action: a complete CLI block (aggregate for shell paths)
    /// or the native tail (directory entry + exact client move).
    Terminal { expression: tta_command::Expression },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectRequest {
    pub id: EffectId,
    pub target: TargetBindingId,
    pub kind: EffectKind,
}

impl EffectRequest {
    pub fn is_terminal(&self) -> bool {
        matches!(self.kind, EffectKind::Terminal { .. })
    }
}

/// What an acknowledged effect established, kept native and typed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectReceipt {
    /// Foreground generator completed successfully.
    Generated,
    /// The expected directory exists and is a directory.
    Directory,
    /// The server acknowledged the native creation.
    Created,
    /// A dispatched terminal block returned under `Handoff::Run`; `code` is
    /// the aggregate exit status, never proof of each internal substep.
    Terminal { code: Option<i32> },
}

/// Honest mutation knowledge for a stopped effect. `NotStarted` means the
/// effect never dispatched; it is not a claim that an earlier failed
/// generator left no residue. `Acknowledged` completion is recorded in the
/// receipt list, not here.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum MutationKnowledge {
    #[default]
    NotStarted,
    MayHaveOccurred,
}

/// Dispatch knowledge retained across cancellation or failure: finished
/// effects with their receipts in dispatch order, and dispatched-but-
/// unfinished IDs whose outcome is unknown.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EffectKnowledge {
    pub finished: Vec<(EffectId, EffectReceipt)>,
    pub in_flight: Vec<EffectId>,
}

/// A failed effect keeps its native source chain and what may have happened.
#[derive(Debug)]
pub struct EffectFailure {
    pub source: Box<dyn std::error::Error + Send + Sync>,
    pub mutation: MutationKnowledge,
}

impl std::fmt::Display for EffectFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mutation may have occurred: {:?}; {}", self.mutation, self.source)
    }
}

impl std::error::Error for EffectFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

pub struct EffectCompletion {
    pub id: EffectId,
    pub result: Result<EffectReceipt, EffectFailure>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("no read is pending (reply is stale, duplicate, or out of order)")]
    NoPendingRead,
    #[error("reply does not answer the exact pending request: expected {expected:?}; received {received:?}")]
    ReadMismatch { expected: Box<ReadRequest>, received: Box<ReadRequest> },
    #[error("reply observation does not answer the pending request kind: expected {expected:?}; received {received:?}")]
    ObservationShape { expected: Box<ReadRequest>, received: Box<Observation> },
    #[error("delivery is only allowed at planned")]
    NotPlanned,
    #[error("attempt is already finished")]
    Finished,
    #[error("no effect is pending (reply is stale, duplicate, or out of order)")]
    NoPendingEffect,
    #[error("reply does not answer the exact pending effect: expected {expected:?}; received {received:?}")]
    EffectMismatch { expected: EffectId, received: EffectId },
    #[error("effect {0:?} already started; waiting cannot reissue or redispatch it")]
    AlreadyStarted(EffectId),
    #[error("reply receipt does not answer the in-flight effect kind: effect {effect:?}; received {received:?}")]
    ReceiptShape { effect: EffectId, received: EffectReceipt },
    #[error("only an outstanding undispatched terminal effect can be delegated")]
    NotDelegatable,
}

#[derive(Debug, thiserror::Error)]
pub enum EntryError {
    #[error("target context policy failed: {0}")]
    Context(#[source] anyhow::Error),
    #[error("generator registry/choice validation failed: {0}")]
    Registry(#[source] anyhow::Error),
    #[error("invalid complete session inventory: {0}")]
    Inventory(String),
    #[error("invalid name in complete session inventory: {0}")]
    InventoryName(#[source] crate::ResolveError),
    #[error("ambiguous session selector {selector:?}: {candidates:?}; use a literal session name")]
    Ambiguous { selector: String, candidates: Vec<crate::Session> },
    #[error("project discovery guard failed: {0}")]
    Discovery(#[source] crate::ResolveError),
    #[error(
        "scaffold mode is on but no generators are configured: add [scaffold.generators] to {registry} or disable with --scaffold=false (no session or project matches {selector:?})"
    )]
    ScaffoldUnavailable { registry: String, selector: String },
    #[error("generator validation failed: {0}")]
    Generator(#[source] anyhow::Error),
    #[error("cannot create {path:?}: something already exists there")]
    DestinationExists { path: PathBuf },
    #[error("chosen project route {route:?} changed before binding: expected physical directory {expected:?}, observed {observed:?}; start a new attempt")]
    RouteChanged {
        route: Box<super::action::ProjectRoute>,
        expected: PathBuf,
        observed: PathBuf,
    },
    #[error("chosen creation destination {path:?} is no longer absent; start a new attempt")]
    DestinationChanged { path: PathBuf },
    #[error("session name {name:?} already belongs to session {id}; choose another session name")]
    NameCollision { name: String, id: crate::SessionId },
    #[error("read {request:?} failed: {source}")]
    Observation {
        request: Box<ReadRequest>,
        #[source]
        source: ObservationFailure,
    },
    #[error("chosen creation root {root:?} changed before binding: expected physical root {expected:?}, observed {observed:?}; start a new attempt")]
    CreationRootChanged { root: PathBuf, expected: PathBuf, observed: PathBuf },
    #[error("effect {effect:?} failed: {failure}")]
    Effect {
        effect: Box<EffectRequest>,
        #[source]
        failure: EffectFailure,
    },
    #[error("chosen view changed before binding: expected {expected:?}, observed {observed:?}; start a new attempt")]
    AnchorChanged {
        expected: select::ViewFingerprint,
        observed: Option<select::ViewFingerprint>,
    },
    #[error(transparent)]
    Action(#[from] action::Error),
}

#[derive(Clone, Debug)]
pub enum AttemptOutcome {
    Exported(Box<action::ExportedAction>),
    /// Every dispatched effect finished; the receipts record what was
    /// established, including an aggregate terminal status.
    Completed {
        effects: EffectKnowledge,
    },
    /// Ownership of the outstanding terminal effect transferred to the
    /// caller; later outcomes are the consumer's to observe and report.
    Delegated {
        terminal: EffectId,
        effects: EffectKnowledge,
    },
    Stopped {
        boundary: String,
        effects: EffectKnowledge,
        cause: Arc<EntryError>,
    },
    Cancelled {
        boundary: String,
        /// Prior receipts and any dispatched-but-unfinished uncertainty.
        effects: EffectKnowledge,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttemptRecord {
    Directive(machine::Directive),
    Control(machine::Event),
    ReadIssued(ReadRequest),
    ReadAccepted(RequestId),
    EffectIssued(EffectRequest),
    EffectStarted(EffectId),
    EffectFinished(EffectId),
    EffectDelegated(EffectId),
}

#[derive(Clone, Debug)]
pub struct AttemptSnapshot {
    pub id: AttemptId,
    pub target: Option<BoundTarget>,
    pub leaf: String,
    pub pending: Option<ReadRequest>,
    /// The outstanding requested or in-flight effect, when one exists.
    pub pending_effect: Option<EffectRequest>,
    pub plan: Option<action::EntryPlan>,
    pub bound: Option<action::BoundAction>,
    pub outcome: Option<AttemptOutcome>,
    pub journal: Vec<AttemptRecord>,
}
