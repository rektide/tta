//! Chart-owned project discovery: root scope, remaining-root cursor, guarded
//! physical ranking and creation preparation. The pure `projects` primitives
//! are reused deliberately; this task never schedules I/O, returns a request
//! or chooses a successor state — the executable chart owns the course.
use std::collections::VecDeque;
use std::path::PathBuf;

use super::{
    action::{CreationSite, ProjectRef, ProjectRoute},
    protocol::{DefaultNeeds, DestinationEvidence, EntryError, EntryIntent, GeneratorDefaults, ReadRequest, RootSlot},
    select::WorkspaceCatalog,
};

/// The discovery task's owned state: parsed scope, the remaining-root cursor
/// and the accepted inventories with their requests, in request order.
pub(super) struct Task {
    fragment: String,
    indices: Vec<usize>,
    qualified: bool,
    remaining: VecDeque<usize>,
    inventories: Vec<crate::ProjectInventory>,
    requests: Vec<ReadRequest>,
}

impl Task {
    pub(super) fn new(scope: crate::projects::Scope<'_>) -> Self {
        Self {
            fragment: scope.fragment.to_owned(),
            remaining: scope.indices.iter().copied().collect(),
            indices: scope.indices,
            qualified: scope.qualified,
            inventories: Vec::new(),
            requests: Vec::new(),
        }
    }

    pub(super) fn fragment(&self) -> &str {
        &self.fragment
    }

    pub(super) fn qualified(&self) -> bool {
        self.qualified
    }

    pub(super) fn indices(&self) -> &[usize] {
        &self.indices
    }

    pub(super) fn roots_requests(&self) -> &[ReadRequest] {
        &self.requests
    }

    pub(super) fn inventories(&self) -> &[crate::ProjectInventory] {
        &self.inventories
    }

    /// Next required root the chart must observe: one-based slot plus the
    /// exact configured path the observation must answer.
    pub(super) fn next_root(&self, roots: &[PathBuf]) -> Option<(RootSlot, PathBuf)> {
        self.remaining.front().map(|&index| (RootSlot(index + 1), roots[index].clone()))
    }

    /// Accept one complete root inventory, validating the accumulated prefix
    /// through the pure source primitive (request binding, absolute physical
    /// paths, unique immediate basenames). Returns whether more required
    /// roots remain. A malformed inventory is a native stop, never absence.
    pub(super) fn accept(&mut self, roots: &[PathBuf], request: ReadRequest, inventory: crate::ProjectInventory) -> Result<bool, EntryError> {
        let mut accepted = self.inventories.clone();
        accepted.push(inventory);
        crate::projects::sources(roots, &self.indices, &accepted).map_err(EntryError::Discovery)?;
        let _ = self
            .remaining
            .pop_front()
            .expect("the chart issues a projects request only while a root remains");
        self.inventories = accepted;
        self.requests.push(request);
        Ok(!self.remaining.is_empty())
    }
}

/// Guarded physical ranking over ALL required roots. Distinct physical
/// directories tie ambiguously; equivalent routes share one identity with a
/// deterministic chosen route and every observed route retained.
pub(super) fn rank(roots: &[PathBuf], selector: &str, task: &Task) -> Result<Option<ProjectRef>, EntryError> {
    let sources = crate::projects::sources(roots, task.indices(), task.inventories()).map_err(EntryError::Discovery)?;
    match crate::projects::select(&sources, selector, task.fragment()).map_err(EntryError::Discovery)? {
        Some(chosen) => {
            let routes: Vec<ProjectRoute> = sources
                .iter()
                .filter(|route| route.directory == chosen.directory)
                .map(ProjectRoute::from)
                .collect();
            let route = ProjectRoute::from(chosen);
            Ok(Some(ProjectRef {
                name: chosen.name.clone(),
                entry: chosen.entry_path(),
                physical: chosen.directory.clone(),
                route: route.clone(),
                routes,
            }))
        }
        None => Ok(None),
    }
}

/// Creation preparation settled before destination absence is observed:
/// generator availability, safe fresh name, explicit destination choice and
/// session name/collision guards. Destination absence precedes adapter work.
pub(super) struct Creation {
    pub site: CreationSite,
    pub session_name: String,
    pub destination: Option<(ReadRequest, DestinationEvidence)>,
    pub defaults: Option<(ReadRequest, GeneratorDefaults)>,
}

pub(super) fn creation_policy(
    intent: &EntryIntent,
    selection: &WorkspaceCatalog,
    task: &Task,
    generator: Option<&crate::scaffold::Entry>,
    choice: &crate::scaffold::Choice,
    registry: &crate::scaffold::Config,
) -> Result<Creation, EntryError> {
    if generator.is_none() {
        return Err(match choice {
            crate::scaffold::Choice::Default if registry.is_empty() => EntryError::ScaffoldUnavailable {
                registry: registry.path_description(),
                selector: intent.selector.clone(),
            },
            _ => EntryError::Discovery(crate::ResolveError::Missing {
                argument: intent.selector.clone(),
                roots: task.indices().iter().map(|index| intent.roots[*index].clone()).collect(),
            }),
        });
    }
    if !crate::projects::component(task.fragment()) {
        return Err(EntryError::Discovery(crate::ResolveError::UnsafeName {
            argument: intent.selector.clone(),
        }));
    }
    // Unqualified creation is allowed only when every observed root slot
    // refers to one physical root; list order never authorizes a destination.
    let physical: std::collections::BTreeSet<&std::path::Path> = task.inventories().iter().map(|inventory| inventory.root.as_path()).collect();
    if !task.qualified() && physical.len() > 1 {
        return Err(EntryError::Discovery(crate::ResolveError::Destinations {
            argument: intent.selector.clone(),
            candidates: task.indices().iter().map(|index| format!("@{}/{}", index + 1, task.fragment())).collect(),
        }));
    }
    let inventory = task.inventories().first().expect("scoped discovery observed at least one root");
    let site = CreationSite {
        slot: task.indices()[0] + 1,
        requested_root: inventory.requested_root.clone(),
        root: inventory.root.clone(),
        name: task.fragment().to_owned(),
        path: inventory.root.join(task.fragment()),
    };
    let name = intent.session_name.as_deref().unwrap_or(task.fragment());
    validate_new_session_name(selection, name)?;
    Ok(Creation {
        site,
        session_name: name.to_owned(),
        destination: None,
        defaults: None,
    })
}

/// Validate a new-work session name against the complete selection inventory.
pub(super) fn validate_new_session_name(selection: &WorkspaceCatalog, name: &str) -> Result<(), EntryError> {
    crate::validation::validate_name(name).map_err(EntryError::Discovery)?;
    if let Some(session) = selection.inventory.sessions().iter().find(|session| session.name == name) {
        return Err(EntryError::NameCollision {
            name: name.to_owned(),
            id: session.id,
        });
    }
    Ok(())
}

/// The external adapter boundary: refuse a creation whose generator declares
/// that its own program selects the destination while tta has more than one
/// configured root. The declared contract drives the refusal, not an
/// adapter-name comparison inside generic resolution. The single-root
/// divergence remains the documented limitation it always was.
pub(super) fn refuse_undeclared_destination(
    generator: &crate::scaffold::Entry,
    contract: crate::scaffold::DestinationContract,
    configured_roots: &[PathBuf],
) -> Result<(), EntryError> {
    if contract == crate::scaffold::DestinationContract::AdapterDecided && configured_roots.len() > 1 {
        return Err(EntryError::Generator(anyhow::anyhow!(
            "generator {:?} uses the external rekon adapter, whose script ignores the selected project root; refusing to create across configured roots {configured_roots:?}: use a rekon-native generator (which honors the root), configure a single root, or change the rekon script's destination contract",
            generator.name
        )));
    }
    Ok(())
}

/// Adapter-declared needs converted to the entry protocol's request shape.
pub(super) fn needs(needs_defaults: (bool, bool)) -> DefaultNeeds {
    DefaultNeeds {
        author: needs_defaults.0,
        license: needs_defaults.1,
    }
}
