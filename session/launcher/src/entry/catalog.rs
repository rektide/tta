//! A separate read-only protocol, not an entry attempt in dry-run mode. Its
//! event/waiting types have no delivery, plan, binding-action or mutation value.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Serialize};
use xmachines_runtime::{Actor, Ev, EventType, Machine, Registry};

use super::{
    action::ProjectRoute,
    context,
    protocol::*,
    select::{ViewRef, WorkspaceCatalog, WorkspaceKey},
};

#[derive(Clone, Debug)]
pub struct Input {
    pub scope: context::Scope,
    pub overrides: context::Overrides,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectSelector {
    Bare(String),
    Qualified { slot: RootSlot, name: String },
}

impl ProjectSelector {
    pub fn text(&self) -> String {
        match self {
            Self::Bare(name) => name.clone(),
            Self::Qualified { slot, name } => format!("@{}/{name}", slot.0),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub physical: PathBuf,
    /// All successfully observed configured routes, including aliases.
    pub routes: Vec<ProjectRoute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workspace {
    pub key: WorkspaceKey,
    pub views: Vec<ViewRef>,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Candidate {
    View { selector: String, view: ViewRef },
    WorkspaceAlias { selector: String, key: WorkspaceKey, views: Vec<ViewRef> },
    Project { selector: ProjectSelector, project: Project },
}

impl Candidate {
    pub fn text(&self) -> String {
        match self {
            Self::View { selector, .. } | Self::WorkspaceAlias { selector, .. } => selector.clone(),
            Self::Project { selector, .. } => selector.text(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Source {
    pub request: ReadRequest,
    pub result: Result<Observation, Arc<EntryError>>,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub target: Option<BoundTarget>,
    pub workspaces: Vec<Workspace>,
    pub projects: Vec<Project>,
    pub candidates: Vec<Candidate>,
    /// Names from the once-parsed successful registry source, not generator
    /// selection or adapter validation/materialization.
    pub generators: Vec<String>,
    pub sources: Vec<Source>,
    /// Binding/policy errors that are not observation failures.
    pub errors: Vec<Arc<EntryError>>,
    pub roots_complete: bool,
}

impl Report {
    pub fn complete(&self) -> bool {
        self.target.is_some() && self.roots_complete && self.errors.is_empty() && self.sources.iter().all(|source| source.result.is_ok())
    }
    /// Thin line-oriented projection: candidate identity and partial failures
    /// stay in the report; filtering controls here never changes selection.
    pub fn completion_names(&self) -> Vec<String> {
        self.candidates
            .iter()
            .map(Candidate::text)
            .filter(|name| !name.chars().any(char::is_control))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Report(Box<Report>),
    Cancelled { boundary: String },
}
pub enum Waiting<'a> {
    Read(&'a ReadRequest),
    Finished(&'a Outcome),
}
pub enum Event {
    Observed(ReadCompletion),
    Cancel,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Directive {
    ReadContext,
    ApplyContextPolicy,
    ReadEndpointIdentity,
    ReadRegistry,
    ValidateRegistry,
    ReadCatalogSessions,
    ReadCatalogProjects,
    ReportCatalog,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControlEvent {
    Observed,
    ContextPolicy { needs_identity: bool },
    Validated,
    CatalogSessionsObserved { has_roots: bool },
    CatalogRootObserved { more_roots: bool },
    Reported,
    Stop,
    Cancel,
}

impl EventType for ControlEvent {
    fn event_type(&self) -> &str {
        match self {
            Self::Observed => "observed",
            Self::ContextPolicy { .. } => "context_policy",
            Self::Validated => "validated",
            Self::CatalogSessionsObserved { .. } => "catalog_sessions_observed",
            Self::CatalogRootObserved { .. } => "catalog_root_observed",
            Self::Reported => "reported",
            Self::Stop => "stop",
            Self::Cancel => "cancel",
        }
    }
}

/// Static chart composition: the entry chart owns the common bind subchart;
/// only its exit is wired to catalog collection. No native directive can set a
/// successor. The graph renderer composes this same source before projection.
pub(super) fn definition() -> serde_json::Value {
    let entry: serde_json::Value = serde_json::from_str(super::machine::DEFINITION).expect("entry definition");
    let mut binding = entry["states"]["active"]["states"]["bind_target"].clone();
    binding["states"]["bind_registry"]["on"]["validated"]["target"] = "#catalog_sessions".into();
    let mut query: serde_json::Value = serde_json::from_str(include_str!("catalog.machine.json")).expect("catalog definition");
    query["states"]["catalog_active"]["states"]["bind_target"] = binding;
    query
}

static MACHINE: LazyLock<Arc<Machine<(), ControlEvent>>> = LazyLock::new(|| {
    let mut registry = Registry::new();
    registry.guard("needs_endpoint_identity", |args| {
        matches!(args.event, Ev::User(ControlEvent::ContextPolicy { needs_identity: true }))
    });
    registry.guard("has_roots", |args| {
        matches!(args.event, Ev::User(ControlEvent::CatalogSessionsObserved { has_roots: true }))
    });
    registry.guard("more_catalog_roots", |args| {
        matches!(args.event, Ev::User(ControlEvent::CatalogRootObserved { more_roots: true }))
    });
    Arc::new(Machine::from_json_str(&definition().to_string(), &registry).expect("catalog chart"))
});

pub struct CatalogQuery {
    control: Actor<(), ControlEvent>,
    id: AttemptId,
    sequence: u64,
    input: context::Acquisition,
    facts: Option<context::Facts>,
    policy: Option<context::Policy>,
    target: Option<BoundTarget>,
    pending: Option<ReadRequest>,
    root_cursor: usize,
    inventories: BTreeMap<usize, crate::ProjectInventory>,
    sessions: Option<WorkspaceCatalog>,
    generators: Vec<String>,
    sources: Vec<Source>,
    errors: Vec<Arc<EntryError>>,
    outcome: Option<Outcome>,
}

impl CatalogQuery {
    pub fn acquire(input: Input) -> Self {
        // Catalog has no creation/generator intent. Inactive creation policy
        // cannot poison a useful read-only discovery query.
        let mut overrides = input.overrides;
        overrides.scaffold = Some("false".into());
        let mut query = Self {
            control: Actor::start(Arc::clone(&MACHINE), 0).expect("catalog initialization"),
            id: AttemptId::new(),
            sequence: 0,
            input: context::Acquisition {
                scope: input.scope,
                overrides,
                intent: EntryIntent::selector("catalog"),
                default_scaffold: "false".into(),
            },
            facts: None,
            policy: None,
            target: None,
            pending: None,
            root_cursor: 0,
            inventories: BTreeMap::new(),
            sessions: None,
            generators: Vec::new(),
            sources: Vec::new(),
            errors: Vec::new(),
            outcome: None,
        };
        query.pump();
        query
    }
    pub fn waiting(&self) -> Waiting<'_> {
        if let Some(request) = &self.pending {
            Waiting::Read(request)
        } else {
            Waiting::Finished(self.outcome.as_ref().expect("catalog must suspend at a read or report"))
        }
    }
    pub fn leaf(&self) -> String {
        self.control.active_leaf_ids()[0].to_owned()
    }
    pub fn advance(&mut self, event: Event) -> Result<(), ProtocolError> {
        match event {
            Event::Cancel => {
                if self.outcome.is_some() {
                    return Err(ProtocolError::Finished);
                }
                self.pending = None;
                self.outcome = Some(Outcome::Cancelled { boundary: self.leaf() });
                self.send(ControlEvent::Cancel);
            }
            Event::Observed(completion) => {
                super::check_completion(self.pending.as_ref(), &completion)?;
                let request = self.pending.take().expect("validated pending read");
                let result = completion.result.map_err(|source| {
                    Arc::new(EntryError::Observation {
                        request: Box::new(request.clone()),
                        source,
                    })
                });
                let result = self.validate_fact(&request, result);
                self.sources.push(Source {
                    request: request.clone(),
                    result: result.clone(),
                });
                match (&request.kind, result) {
                    (ReadKind::TargetContext, Ok(Observation::Context(facts))) => {
                        self.facts = Some(facts);
                        self.send(ControlEvent::Observed);
                    }
                    (ReadKind::TargetContext | ReadKind::EndpointIdentity { .. }, Err(_)) => self.send(ControlEvent::Stop),
                    (ReadKind::EndpointIdentity { .. }, Ok(Observation::EndpointIdentity { current, selected })) => {
                        if current == selected {
                            self.send(ControlEvent::Observed);
                        } else {
                            self.errors
                                .push(Arc::new(EntryError::Context(anyhow::anyhow!("chosen endpoint differs from inherited server"))));
                            self.send(ControlEvent::Stop);
                        }
                    }
                    (ReadKind::Registry { .. }, _) => self.send(ControlEvent::Observed),
                    (ReadKind::Sessions { .. }, result) => {
                        if let Ok(Observation::Sessions(inventory)) = result {
                            self.sessions = Some(WorkspaceCatalog::from_read(request, inventory).expect("validated catalog"));
                        }
                        self.send(ControlEvent::CatalogSessionsObserved {
                            has_roots: !self.roots().is_empty(),
                        });
                    }
                    (ReadKind::Projects { slot, .. }, result) => {
                        if let Ok(Observation::Projects(inventory)) = result {
                            self.inventories.insert(slot.0, inventory);
                        }
                        self.root_cursor += 1;
                        self.send(ControlEvent::CatalogRootObserved {
                            more_roots: self.root_cursor < self.roots().len(),
                        });
                    }
                    _ => unreachable!("catalog only requests its closed read vocabulary"),
                }
            }
        }
        self.pump();
        Ok(())
    }

    fn validate_fact(&mut self, request: &ReadRequest, result: Result<Observation, Arc<EntryError>>) -> Result<Observation, Arc<EntryError>> {
        let observation = result?;
        match &observation {
            Observation::Sessions(inventory) => {
                WorkspaceCatalog::from_read(request.clone(), inventory.clone()).map_err(Arc::new)?;
            }
            Observation::Projects(inventory) => {
                let ReadKind::Projects { slot, .. } = &request.kind else { unreachable!() };
                crate::projects::sources(self.roots(), &[slot.0 - 1], std::slice::from_ref(inventory))
                    .map_err(|error| Arc::new(EntryError::Discovery(error)))?;
            }
            Observation::Registry(text) => {
                let ReadKind::Registry { path } = &request.kind else { unreachable!() };
                self.generators = crate::scaffold::Config::parse(path.as_deref(), text.as_deref())
                    .map_err(|error| Arc::new(EntryError::Registry(error)))?
                    .names();
            }
            _ => {}
        }
        Ok(observation)
    }

    fn roots(&self) -> &[PathBuf] {
        self.policy.as_ref().map(|p| p.intent.roots.as_slice()).unwrap_or_default()
    }
    fn send(&mut self, event: ControlEvent) {
        self.control.send(event, 0).expect("catalog control event");
    }
    fn issue(&mut self, kind: ReadKind) {
        assert!(self.pending.is_none());
        self.sequence = self.sequence.checked_add(1).expect("catalog sequence exhausted");
        let scope = self.policy.as_ref().map(|p| &p.scope).unwrap_or(&self.input.scope).clone();
        self.pending = Some(ReadRequest {
            id: RequestId {
                attempt: self.id,
                sequence: self.sequence,
            },
            target: TargetBindingId(self.id),
            scope: Box::new(scope),
            kind,
        });
    }
    fn pump(&mut self) {
        loop {
            let directives = self.control.take_emitted();
            if directives.is_empty() {
                let _ = self.waiting();
                return;
            }
            for directive in directives {
                let directive: Directive = serde_json::from_value(directive).expect("catalog emitted non-read directive");
                match directive {
                    Directive::ReadContext => match context::validate_initial_scope(&self.input.scope) {
                        Ok(()) => self.issue(ReadKind::TargetContext),
                        Err(error) => {
                            self.errors.push(Arc::new(EntryError::Context(error)));
                            self.send(ControlEvent::Stop);
                        }
                    },
                    Directive::ApplyContextPolicy => {
                        let result = context::apply(&self.input, self.facts.as_ref().expect("context facts"), context::Use::Catalog).and_then(|policy| {
                            crate::validate_satellite_prefix(&policy.intent.satellite_prefix)?;
                            let mut target = BoundTarget::from_supplied(self.id, policy.supplied.clone())?;
                            target.scope = policy.scope.clone();
                            Ok((policy, target))
                        });
                        match result {
                            Ok((policy, target)) => {
                                let needs_identity = policy.endpoint_identity.is_some();
                                self.policy = Some(policy);
                                self.target = Some(target);
                                self.send(ControlEvent::ContextPolicy { needs_identity });
                            }
                            Err(error) => {
                                self.errors.push(Arc::new(EntryError::Context(error)));
                                self.send(ControlEvent::Stop);
                            }
                        }
                    }
                    Directive::ReadEndpointIdentity => {
                        let (current, selected) = self.policy.as_ref().expect("policy").endpoint_identity.clone().expect("identity request");
                        self.issue(ReadKind::EndpointIdentity { current, selected });
                    }
                    Directive::ReadRegistry => self.issue(ReadKind::Registry {
                        path: self.policy.as_ref().expect("policy").registry.clone(),
                    }),
                    Directive::ValidateRegistry => self.send(ControlEvent::Validated),
                    Directive::ReadCatalogSessions => {
                        let target = self.target.as_ref().expect("bound target");
                        self.issue(ReadKind::Sessions {
                            endpoint: target.endpoint(),
                            program: target.program.as_os_str().to_owned(),
                            purpose: SessionReadPurpose::Catalog,
                        });
                    }
                    Directive::ReadCatalogProjects => self.issue(ReadKind::Projects {
                        slot: RootSlot(self.root_cursor + 1),
                        root: self.roots()[self.root_cursor].clone(),
                        filter: String::new(),
                    }),
                    Directive::ReportCatalog => {
                        self.outcome = Some(Outcome::Report(Box::new(self.report())));
                        self.send(ControlEvent::Reported);
                    }
                }
            }
        }
    }

    fn report(&self) -> Report {
        let mut workspaces: Vec<Workspace> = Vec::new();
        let mut candidates = Vec::new();
        if let Some(catalog) = &self.sessions {
            let prefix = &self.policy.as_ref().expect("policy").intent.satellite_prefix;
            let names = crate::session_names(catalog.inventory.sessions(), prefix);
            for session in catalog.inventory.sessions() {
                let view = ViewRef {
                    endpoint: catalog.request.endpoint().clone(),
                    observation: catalog.request.id,
                    fingerprint: session.into(),
                };
                candidates.push(Candidate::View {
                    selector: session.name.clone(),
                    view: view.clone(),
                });
                let key = session.group.clone().map_or(WorkspaceKey::Singleton(session.id), WorkspaceKey::Group);
                let index = workspaces.iter().position(|w| w.key == key).unwrap_or_else(|| {
                    workspaces.push(Workspace {
                        key: key.clone(),
                        views: Vec::new(),
                        aliases: Vec::new(),
                    });
                    workspaces.len() - 1
                });
                workspaces[index].views.push(view);
                let alias = crate::satellite_alias(&session.name, prefix).to_owned();
                if alias != session.name && names.contains(&alias) && !workspaces[index].aliases.contains(&alias) {
                    workspaces[index].aliases.push(alias);
                }
            }
            for workspace in &workspaces {
                for alias in &workspace.aliases {
                    candidates.push(Candidate::WorkspaceAlias {
                        selector: alias.clone(),
                        key: workspace.key.clone(),
                        views: workspace.views.clone(),
                    });
                }
            }
        }
        let roots_complete = self.target.is_some() && self.inventories.len() == self.roots().len();
        let mut sources = Vec::new();
        for (&slot, inventory) in &self.inventories {
            sources.extend(crate::projects::sources(self.roots(), &[slot - 1], std::slice::from_ref(inventory)).expect("validated root inventory"));
        }
        let mut projects = BTreeMap::<PathBuf, Project>::new();
        for source in &sources {
            projects
                .entry(source.directory.clone())
                .or_insert_with(|| Project {
                    physical: source.directory.clone(),
                    routes: Vec::new(),
                })
                .routes
                .push(source.into());
        }
        let selectors = if roots_complete {
            crate::projects::completion_candidates(self.roots(), &self.inventories.values().cloned().collect::<Vec<_>>())
                .expect("complete validated inventories")
        } else {
            sources.iter().map(crate::ProjectSource::selector).collect()
        };
        for selector in selectors {
            let (typed, source) = if let Some(source) = sources.iter().find(|source| source.selector() == selector) {
                (
                    ProjectSelector::Qualified {
                        slot: RootSlot(source.root_index),
                        name: source.name.clone(),
                    },
                    source,
                )
            } else {
                let source = sources.iter().find(|source| source.name == selector).expect("bare candidate source");
                (ProjectSelector::Bare(selector), source)
            };
            candidates.push(Candidate::Project {
                selector: typed,
                project: projects[&source.directory].clone(),
            });
        }
        candidates.sort_by_key(Candidate::text);
        Report {
            target: self.target.clone(),
            workspaces,
            projects: projects.into_values().collect(),
            candidates,
            generators: self.generators.clone(),
            sources: self.sources.clone(),
            errors: self.errors.clone(),
            roots_complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    fn query(roots: &[&str]) -> CatalogQuery {
        CatalogQuery::acquire(Input {
            scope: context::Scope {
                origin: context::Origin::Local { host: "no-host-I/O".into() },
                cwd: None,
            },
            overrides: context::Overrides {
                roots: Some(roots.iter().map(PathBuf::from).collect()),
                satellite_prefix: Some("view-".into()),
                ..Default::default()
            },
        })
    }
    fn pending(query: &CatalogQuery) -> ReadRequest {
        match query.waiting() {
            Waiting::Read(request) => request.clone(),
            _ => panic!("expected read"),
        }
    }
    fn supply(query: &mut CatalogQuery, result: Result<Observation, ObservationFailure>) {
        query
            .advance(Event::Observed(ReadCompletion {
                request: pending(query),
                result,
            }))
            .unwrap();
    }
    fn bind(query: &mut CatalogQuery) {
        supply(
            query,
            Ok(Observation::Context(context::Facts {
                cwd: "/never/observed/by/core".into(),
                uid: 1,
                environment: Default::default(),
            })),
        );
        assert!(matches!(pending(query).kind, ReadKind::Registry { path: None }));
        supply(query, Ok(Observation::Registry(None)));
        assert!(matches!(
            pending(query).kind,
            ReadKind::Sessions {
                purpose: SessionReadPurpose::Catalog,
                ..
            }
        ));
    }
    fn inventory(root: &str, physical: &str, entries: &[(&str, &str)]) -> Observation {
        Observation::Projects(crate::ProjectInventory {
            requested_root: root.into(),
            root: physical.into(),
            entries: entries
                .iter()
                .map(|(name, path)| crate::ProjectEntry {
                    name: (*name).into(),
                    directory: (*path).into(),
                })
                .collect(),
        })
    }
    fn report(query: &CatalogQuery) -> &Report {
        match query.waiting() {
            Waiting::Finished(Outcome::Report(report)) => report,
            _ => panic!("expected report"),
        }
    }

    #[test]
    fn complete_catalog_keeps_physical_groups_routes_qualified_selectors_and_workspace_aliases() {
        let mut query = query(&["/first", "/second"]);
        bind(&mut query);
        supply(
            &mut query,
            Ok(Observation::Sessions(SessionInventory::Present(vec![crate::Session {
                id: "$42".parse().unwrap(),
                name: "view-project-42".into(),
                group: Some("project".into()),
            }]))),
        );
        supply(
            &mut query,
            Ok(inventory(
                "/first",
                "/physical/first",
                &[("foo", "/projects/shared"), ("duplicate", "/projects/a")],
            )),
        );
        assert!(matches!(pending(&query).kind, ReadKind::Projects { slot: RootSlot(2), .. }));
        supply(
            &mut query,
            Ok(inventory(
                "/second",
                "/physical/second",
                &[("foo", "/projects/shared"), ("duplicate", "/projects/b"), ("alias", "/projects/shared")],
            )),
        );
        let report = report(&query);
        assert!(report.complete());
        assert_eq!(report.projects.len(), 3);
        assert_eq!(
            report
                .projects
                .iter()
                .find(|p| p.physical == std::path::Path::new("/projects/shared"))
                .unwrap()
                .routes
                .len(),
            3
        );
        let names = report.completion_names();
        assert!(names.contains(&"foo".into()) && names.contains(&"alias".into()) && names.contains(&"project".into()));
        assert!(!names.contains(&"duplicate".into()));
        assert!(names.contains(&"@1/duplicate".into()) && names.contains(&"@2/duplicate".into()));
        assert!(report.candidates.iter().any(|c| matches!(c, Candidate::WorkspaceAlias { key: WorkspaceKey::Group(group), views, .. } if group == "project" && views[0].fingerprint.id.to_string() == "$42")));
        assert_eq!(report.sources.len(), 5);
        for _ in 0..3 {
            assert_eq!(report.completion_names(), names);
        }
    }

    #[test]
    fn partial_catalog_never_certifies_bare_names_but_retains_successful_qualified_routes_and_typed_failure() {
        let mut query = query(&["/first", "/second", "/third"]);
        bind(&mut query);
        supply(&mut query, Ok(Observation::Sessions(SessionInventory::Absent)));
        supply(&mut query, Ok(inventory("/first", "/physical/first", &[("foo", "/projects/foo")])));
        supply(
            &mut query,
            Err(ObservationFailure {
                source: Box::new(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            }),
        );
        supply(&mut query, Ok(inventory("/third", "/physical/third", &[("bar", "/projects/bar")])));
        let report = report(&query);
        assert!(!report.complete() && !report.roots_complete);
        assert_eq!(report.completion_names(), ["@1/foo", "@3/bar"]);
        let failed = report.sources.iter().find_map(|s| s.result.as_ref().err()).unwrap();
        let EntryError::Observation { source, request } = failed.as_ref() else {
            panic!()
        };
        assert!(matches!(request.kind, ReadKind::Projects { slot: RootSlot(2), .. }));
        assert_eq!(
            source.source().unwrap().downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn registry_and_session_failures_are_visible_without_hiding_successful_project_sources() {
        let mut query = query(&["/root"]);
        supply(
            &mut query,
            Ok(Observation::Context(context::Facts {
                cwd: "/never/read".into(),
                uid: 1,
                environment: Default::default(),
            })),
        );
        supply(&mut query, Ok(Observation::Registry(Some("invalid registry".into()))));
        supply(
            &mut query,
            Err(ObservationFailure {
                source: Box::new(std::io::Error::from(std::io::ErrorKind::ConnectionRefused)),
            }),
        );
        supply(&mut query, Ok(inventory("/root", "/root", &[("foo", "/root/foo")])));
        let report = report(&query);
        assert!(!report.complete() && report.roots_complete);
        assert_eq!(report.sources.iter().filter(|s| s.result.is_err()).count(), 2);
        assert_eq!(report.completion_names(), ["@1/foo", "foo"]);
    }

    #[test]
    fn malformed_inventory_is_partial_not_empty_and_ingress_rejection_is_inert() {
        let mut query = query(&["/first", "/second"]);
        let request = pending(&query);
        let mut wrong = request.clone();
        wrong.scope.cwd = Some("/other".into());
        assert!(matches!(
            query.advance(Event::Observed(ReadCompletion {
                request: wrong,
                result: Ok(Observation::Registry(None))
            })),
            Err(ProtocolError::ReadMismatch { .. })
        ));
        assert_eq!(pending(&query), request);
        assert!(matches!(
            query.advance(Event::Observed(ReadCompletion {
                request: request.clone(),
                result: Ok(Observation::Registry(None))
            })),
            Err(ProtocolError::ObservationShape { .. })
        ));
        assert_eq!(pending(&query), request);
        bind(&mut query);
        supply(&mut query, Ok(Observation::Sessions(SessionInventory::Absent)));
        supply(&mut query, Ok(inventory("/wrong-requested-root", "/first", &[])));
        supply(&mut query, Ok(inventory("/second", "/second", &[("line\nfeed", "/second/control")])));
        let report = report(&query);
        assert!(!report.roots_complete);
        assert!(report.completion_names().is_empty());
        assert!(report.candidates.iter().any(|c| c.text() == "@2/line\nfeed"));
        assert!(matches!(
            query.advance(Event::Observed(ReadCompletion {
                request,
                result: Ok(Observation::Registry(None))
            })),
            Err(ProtocolError::NoPendingRead)
        ));
    }

    #[test]
    fn catalog_chart_shares_binding_and_has_only_a_closed_read_directive_vocabulary() {
        fn walk(state: &serde_json::Value, reads: &mut usize) {
            if let Some(entries) = state["entry"].as_array() {
                for entry in entries {
                    let _: Directive = serde_json::from_value(entry["event"].clone()).unwrap();
                    *reads += 1;
                }
            }
            if let Some(states) = state["states"].as_object() {
                for state in states.values() {
                    walk(state, reads);
                }
            }
        }
        let definition = definition();
        let mut reads = 0;
        walk(&definition, &mut reads);
        assert_eq!(reads, 8);
        let references = Machine::<(), ControlEvent>::references(&definition).unwrap();
        assert!(references.actions.is_empty() && references.actors.is_empty() && references.delays.is_empty());
        let mut query = query(&["/root"]);
        let request = pending(&query);
        query.advance(Event::Cancel).unwrap();
        assert!(matches!(query.waiting(), Waiting::Finished(Outcome::Cancelled { .. })));
        assert!(matches!(
            query.advance(Event::Observed(ReadCompletion {
                request,
                result: Ok(Observation::Registry(None))
            })),
            Err(ProtocolError::NoPendingRead)
        ));
    }

    #[test]
    fn read_only_catalog_can_observe_another_server_without_authorizing_a_client_switch() {
        let mut query = CatalogQuery::acquire(Input {
            scope: context::Scope {
                origin: context::Origin::Local { host: "local".into() },
                cwd: None,
            },
            overrides: context::Overrides {
                socket: Some(crate::socket::Selection::Path("/chosen".into())),
                ..Default::default()
            },
        });
        supply(
            &mut query,
            Ok(Observation::Context(context::Facts {
                cwd: "/supplied/no-io".into(),
                uid: 1,
                environment: [("TMUX".into(), "/current,1,1".into())].into(),
            })),
        );
        assert!(matches!(pending(&query).kind, ReadKind::Registry { .. }));
        supply(&mut query, Ok(Observation::Registry(None)));
        assert!(matches!(pending(&query).kind, ReadKind::Sessions { endpoint, .. } if endpoint.socket == std::path::Path::new("/chosen")));
        supply(&mut query, Ok(Observation::Sessions(SessionInventory::Present(Vec::new()))));
        let report = report(&query);
        assert!(report.complete());
        assert_eq!(report.target.as_ref().unwrap().client, ClientEntry::AttachTerminal);
    }
}
