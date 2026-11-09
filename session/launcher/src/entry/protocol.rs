use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::{action, machine, select};

/// Process-local correlation, not durable identity or crash-resume authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct AttemptId(u64);

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
pub(crate) struct RequestId {
    pub attempt: AttemptId,
    pub sequence: u64,
}

/// Scoped to a frozen supplied target in this attempt. A host name or equal
/// socket spelling alone is never an identity proof, nor is this an incarnation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TargetBindingId(AttemptId);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EndpointKey {
    pub target: TargetBindingId,
    pub socket: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClientEntry {
    AttachTerminal,
    /// Supplied same-server policy, NOT an observed invoking client ID. Guarded
    /// origin switching is a separate, uncommissioned capability.
    SwitchWithinServer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewPreference {
    Auto,
    Independent,
    Shared,
}

#[derive(Clone, Debug)]
pub(crate) struct EntryIntent {
    pub selector: String,
    pub view: ViewPreference,
    pub satellite_prefix: String,
    /// Only OpenView can retain this. Existing-work decisions have no startup
    /// argv/window-policy fields; those belong to the next stage's new work.
    pub view_directory: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct SuppliedContext {
    pub host: String,
    pub cwd: PathBuf,
    pub socket: crate::socket::Resolved,
    pub program: OsString,
    pub config: Option<PathBuf>,
    pub client: ClientEntry,
}

pub(crate) struct Invocation {
    pub context: SuppliedContext,
    pub intent: EntryIntent,
    pub registry: crate::scaffold::Config,
    pub generator: crate::scaffold::Choice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundTarget {
    pub(super) binding: TargetBindingId,
    pub(super) host: String,
    pub(super) cwd: PathBuf,
    pub(super) socket: crate::socket::Resolved,
    pub(super) program: tta_command::Word,
    pub(super) config: Option<tta_command::Word>,
    pub(super) client: ClientEntry,
}

impl BoundTarget {
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
            binding: TargetBindingId(attempt),
            host: context.host,
            cwd: context.cwd,
            socket: context.socket,
            program: tta_command::Word::new(context.program)?,
            config: context.config.map(|p| tta_command::Word::new(p.into_os_string())).transpose()?,
            client: context.client,
        })
    }

    pub(crate) fn endpoint(&self) -> EndpointKey {
        EndpointKey {
            target: self.binding,
            socket: self.socket.path.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InputError {
    #[error("provide a nonempty session/project selector")]
    EmptySelector,
    #[error("invalid supplied target: {0}")]
    Target(&'static str),
    #[error("supplied target path must be absolute: {0:?}")]
    Path(PathBuf),
    #[error(transparent)]
    Name(#[from] crate::ResolveError),
    #[error(transparent)]
    Command(#[from] tta_command::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionReadPurpose {
    Selection,
    BindAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReadKind {
    Sessions {
        endpoint: EndpointKey,
        program: OsString,
        purpose: SessionReadPurpose,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReadRequest {
    pub id: RequestId,
    pub target: TargetBindingId,
    pub kind: ReadKind,
}

impl ReadRequest {
    pub(super) fn sessions_purpose(&self) -> SessionReadPurpose {
        match self.kind {
            ReadKind::Sessions { purpose, .. } => purpose,
        }
    }

    pub(super) fn endpoint(&self) -> &EndpointKey {
        match &self.kind {
            ReadKind::Sessions { endpoint, .. } => endpoint,
        }
    }
}

/// Only complete successful inventories supply presence/absence. A partial read
/// must be an ObservationFailure; it has no conversion to this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionInventory {
    Absent,
    Present(Vec<crate::Session>),
}

impl SessionInventory {
    pub(crate) fn sessions(&self) -> &[crate::Session] {
        match self {
            Self::Absent => &[],
            Self::Present(sessions) => sessions,
        }
    }
}

pub(crate) enum Observation {
    Sessions(SessionInventory),
}

/// Native observer errors (including capture/protocol/UTF-8 failures) retain
/// their source type, bytes and nested chain; the actor only sees `stop`.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub(crate) struct ObservationFailure {
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

pub(crate) struct ReadCompletion {
    /// Exact echo checks purpose/endpoint/program as well as ID/target. Future
    /// path/slot requests must extend this envelope, not rely on enum type alone.
    pub request: ReadRequest,
    pub result: Result<Observation, ObservationFailure>,
}

pub(crate) enum Delivery {
    Export,
}
pub(crate) enum Event {
    Observed(ReadCompletion),
    Deliver(Delivery),
    Cancel,
}

pub(crate) enum Waiting<'a> {
    Read(&'a ReadRequest),
    Planned(&'a action::EntryPlan),
    Finished(&'a AttemptOutcome),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProtocolError {
    #[error("no read is pending (reply is stale, duplicate, or out of order)")]
    NoPendingRead,
    #[error("reply does not answer the exact pending request: expected {expected:?}; received {received:?}")]
    ReadMismatch { expected: Box<ReadRequest>, received: Box<ReadRequest> },
    #[error("delivery is only allowed at planned")]
    NotPlanned,
    #[error("attempt is already finished")]
    Finished,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EntryError {
    #[error("generator registry/choice validation failed: {0}")]
    Registry(#[source] anyhow::Error),
    #[error("invalid complete session inventory: {0}")]
    Inventory(String),
    #[error("invalid name in complete session inventory: {0}")]
    InventoryName(#[source] crate::ResolveError),
    #[error("ambiguous session selector {selector:?}: {candidates:?}; use a literal session name")]
    Ambiguous { selector: String, candidates: Vec<crate::Session> },
    #[error("project discovery is not yet implemented in the staged internal entry attempt")]
    DiscoveryNotImplemented,
    #[error("read {request:?} failed: {source}")]
    Observation {
        request: Box<ReadRequest>,
        #[source]
        source: ObservationFailure,
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
pub(crate) enum AttemptOutcome {
    Exported(Box<action::ExportedAction>),
    Stopped { boundary: String, cause: Arc<EntryError> },
    Cancelled { boundary: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AttemptRecord {
    Directive(machine::Directive),
    Control(machine::Event),
    ReadIssued(ReadRequest),
    ReadAccepted(RequestId),
}

#[derive(Clone, Debug)]
pub(crate) struct AttemptSnapshot {
    pub id: AttemptId,
    pub target: BoundTarget,
    pub leaf: String,
    pub pending: Option<ReadRequest>,
    pub plan: Option<action::EntryPlan>,
    pub bound: Option<action::BoundAction>,
    pub outcome: Option<AttemptOutcome>,
    pub journal: Vec<AttemptRecord>,
}
