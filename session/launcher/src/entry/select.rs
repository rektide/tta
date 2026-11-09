//! Guarded session selection. Small mechanics are reused, never the old resolver
//! or its scheduler. This candidate model is usable by the later read-only catalog.
use super::protocol::{EndpointKey, EntryError, EntryIntent, ReadRequest, RequestId, SessionInventory};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ViewFingerprint {
    pub id: crate::SessionId,
    pub name: String,
    pub group: Option<String>,
}

impl From<&crate::Session> for ViewFingerprint {
    fn from(session: &crate::Session) -> Self {
        Self {
            id: session.id,
            name: session.name.clone(),
            group: session.group.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceKey {
    Group(String),
    Singleton(crate::SessionId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MatchReason {
    Literal,
    Alias { name: String },
    Substring { name: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ViewRef {
    pub endpoint: EndpointKey,
    pub observation: RequestId,
    pub fingerprint: ViewFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceRef {
    pub endpoint: EndpointKey,
    pub observation: RequestId,
    pub key: WorkspaceKey,
    pub anchor: ViewFingerprint,
    pub matched_by: MatchReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceMatch {
    pub work: WorkspaceRef,
    pub view: ViewRef,
    pub request: ReadRequest,
}

pub(super) struct WorkspaceCatalog {
    pub request: ReadRequest,
    pub inventory: SessionInventory,
}

impl WorkspaceCatalog {
    pub(super) fn from_read(request: ReadRequest, inventory: SessionInventory) -> Result<Self, EntryError> {
        let mut ids = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        for session in inventory.sessions() {
            crate::resolve::validate_name(&session.name).map_err(EntryError::InventoryName)?;
            if !ids.insert(session.id) || !names.insert(&session.name) {
                return Err(EntryError::Inventory("duplicate session ID or literal name".into()));
            }
            if session.group.as_ref().is_some_and(|g| g.is_empty() || g.chars().any(char::is_control)) {
                return Err(EntryError::Inventory("empty or unprintable actual group".into()));
            }
        }
        Ok(Self { request, inventory })
    }
}

pub(super) fn resolve_session(catalog: &WorkspaceCatalog, intent: &EntryIntent) -> Result<Option<WorkspaceMatch>, EntryError> {
    // Small pure ranker: exact raw name first; otherwise raw + one-step aliases,
    // shortest Unicode length, deterministic anchor, then actual-group collapse.
    // Its ambiguity guard finishes before this function emits Some OR None.
    let selected =
        crate::workspace::select(catalog.inventory.sessions(), &intent.selector, &intent.satellite_prefix).map_err(|candidates| EntryError::Ambiguous {
            selector: intent.selector.clone(),
            candidates,
        })?;
    Ok(selected.map(|session| {
        let alias = crate::satellite_alias(&session.name, &intent.satellite_prefix);
        let matched_by = if session.name == intent.selector {
            MatchReason::Literal
        } else if alias == intent.selector {
            MatchReason::Alias { name: alias.into() }
        } else {
            let name = [&*session.name, alias]
                .into_iter()
                .filter(|name| name.contains(&intent.selector))
                .min_by_key(|name| name.chars().count())
                .expect("ranked match");
            MatchReason::Substring { name: name.into() }
        };
        let fingerprint = ViewFingerprint::from(&session);
        let endpoint = catalog.request.endpoint().clone();
        WorkspaceMatch {
            work: WorkspaceRef {
                endpoint: endpoint.clone(),
                observation: catalog.request.id,
                key: session.group.clone().map_or(WorkspaceKey::Singleton(session.id), WorkspaceKey::Group),
                anchor: fingerprint.clone(),
                matched_by,
            },
            view: ViewRef {
                endpoint,
                observation: catalog.request.id,
                fingerprint,
            },
            request: catalog.request.clone(),
        }
    }))
}
