//! Owned entry protocol, staged internally until the coherent frontend cutover.
mod action;
mod bind;
pub mod context;
mod discover;
mod machine;
mod prepare;
mod protocol;
mod select;

use std::sync::Arc;

use machine::{Control, Directive, Event as ControlEvent};
use protocol::*;

/// One owner and one retained actor for an attempt. Native facts, artifacts and
/// errors never enter the runtime's serde context or event payloads.
pub(crate) struct EntryAttempt {
    control: Control,
    data: AttemptData,
    pending: Option<ReadRequest>,
    journal: Vec<AttemptRecord>,
    sequence: u64,
}

struct AttemptData {
    id: AttemptId,
    target: Option<BoundTarget>,
    scope: context::Scope,
    acquisition: Option<context::Acquisition>,
    facts: Option<context::Facts>,
    policy: Option<context::Policy>,
    intent: EntryIntent,
    registry: crate::scaffold::Config,
    generator_choice: crate::scaffold::Choice,
    generator: Option<crate::scaffold::Entry>,
    selection: Option<select::WorkspaceCatalog>,
    workspace: Option<select::WorkspaceMatch>,
    discovery: Option<discover::Task>,
    project: Option<action::ProjectRef>,
    creation: Option<discover::Creation>,
    validated_generator: Option<crate::scaffold::Validated>,
    plan: Option<action::EntryPlan>,
    binding: Option<select::WorkspaceCatalog>,
    binding_evidence: Option<(ReadRequest, bind::Evidence)>,
    bound: Option<action::BoundAction>,
    outcome: Option<AttemptOutcome>,
}

impl EntryAttempt {
    pub(crate) fn start(invocation: Invocation) -> Result<Self, InputError> {
        let id = AttemptId::new();
        let target = BoundTarget::from_supplied(id, invocation.context)?;
        if invocation.intent.selector.is_empty() {
            return Err(InputError::EmptySelector);
        }
        validate_intent(&invocation.intent)?;
        let mut attempt = Self {
            control: Control::start(),
            data: AttemptData {
                id,
                scope: target.scope.clone(),
                target: Some(target),
                acquisition: None,
                facts: None,
                policy: None,
                intent: invocation.intent,
                registry: invocation.registry,
                generator_choice: invocation.generator,
                generator: None,
                selection: None,
                workspace: None,
                discovery: None,
                project: None,
                creation: None,
                validated_generator: None,
                plan: None,
                binding: None,
                binding_evidence: None,
                bound: None,
                outcome: None,
            },
            pending: None,
            journal: Vec::new(),
            sequence: 0,
        };
        attempt.pump();
        Ok(attempt)
    }

    pub(crate) fn acquire(input: context::Acquisition) -> Result<Self, InputError> {
        if input.intent.selector.is_empty() {
            return Err(InputError::EmptySelector);
        }
        let mut attempt = Self {
            control: Control::start(),
            data: AttemptData {
                id: AttemptId::new(),
                target: None,
                scope: input.scope.clone(),
                intent: input.intent.clone(),
                acquisition: Some(input),
                facts: None,
                policy: None,
                registry: crate::scaffold::Config::parse(None, None).expect("empty registry"),
                generator_choice: crate::scaffold::Choice::Off,
                generator: None,
                selection: None,
                workspace: None,
                discovery: None,
                project: None,
                creation: None,
                validated_generator: None,
                plan: None,
                binding: None,
                binding_evidence: None,
                bound: None,
                outcome: None,
            },
            pending: None,
            journal: Vec::new(),
            sequence: 0,
        };
        attempt.pump();
        Ok(attempt)
    }

    /// Inspection is inert: no actor creation, emission drain, materialization,
    /// trace or native handler call happens here.
    pub(crate) fn waiting(&self) -> Waiting<'_> {
        if let Some(request) = &self.pending {
            return Waiting::Read(request);
        }
        if let Some(outcome) = &self.data.outcome {
            return Waiting::Finished(outcome);
        }
        assert_eq!(self.control.leaf(), "planned", "chart must suspend at an external boundary");
        Waiting::Planned(self.data.plan.as_ref().expect("planned artifact"))
    }

    pub(crate) fn snapshot(&self) -> AttemptSnapshot {
        AttemptSnapshot {
            id: self.data.id,
            target: self.data.target.clone(),
            leaf: self.control.leaf().into(),
            pending: self.pending.clone(),
            plan: self.data.plan.clone(),
            bound: self.data.bound.clone(),
            outcome: self.data.outcome.clone(),
            journal: self.journal.clone(),
        }
    }

    /// Protocol errors reject the entire event before modifying any data, actor
    /// state or journal. Accepted acquisition/domain errors instead stop natively.
    pub(crate) fn advance(&mut self, event: Event) -> Result<(), ProtocolError> {
        match event {
            Event::Observed(completion) => {
                let expected = self.pending.as_ref().ok_or(ProtocolError::NoPendingRead)?;
                if completion.request != *expected {
                    return Err(ProtocolError::ReadMismatch {
                        expected: Box::new(expected.clone()),
                        received: Box::new(completion.request),
                    });
                }
                // Exhaustive request-kind/observation-kind compatibility check
                // BEFORE consuming the pending request: a wrong-shape reply
                // changes no actor, data or journal state.
                if let Ok(observation) = &completion.result
                    && !observation_answers_request(&expected.kind, observation)
                {
                    return Err(ProtocolError::ObservationShape {
                        expected: Box::new(expected.clone()),
                        received: Box::new(observation.clone()),
                    });
                }
                let request = self.pending.take().expect("checked pending request");
                self.journal.push(AttemptRecord::ReadAccepted(request.id));
                match completion.result {
                    Ok(Observation::Context(facts)) => {
                        self.data.facts = Some(facts);
                        self.send(ControlEvent::Observed);
                    }
                    Ok(Observation::Registry(text)) => {
                        let ReadKind::Registry { path } = &request.kind else { unreachable!() };
                        match crate::scaffold::Config::parse(path.as_deref(), text.as_deref()) {
                            Ok(registry) => {
                                self.data.registry = registry;
                                self.send(ControlEvent::Observed);
                            }
                            Err(error) => self.stop(EntryError::Registry(error)),
                        }
                    }
                    Ok(Observation::EndpointIdentity { current, selected }) => {
                        if current == selected {
                            self.send(ControlEvent::Observed);
                        } else {
                            let (current, selected) = self
                                .data
                                .policy
                                .as_ref()
                                .expect("context policy")
                                .endpoint_identity
                                .clone()
                                .expect("endpoint identity request");
                            self.stop(EntryError::Context(crate::socket::Error::DifferentServer { current, selected }.into()));
                        }
                    }
                    Ok(Observation::Sessions(inventory)) => {
                        let purpose = request.sessions_purpose();
                        match select::WorkspaceCatalog::from_read(request, inventory) {
                            Ok(catalog) => match purpose {
                                SessionReadPurpose::Selection => {
                                    self.data.selection = Some(catalog);
                                    self.send(ControlEvent::Observed);
                                }
                                SessionReadPurpose::BindAction => {
                                    self.data.binding = Some(catalog);
                                    let evidence_needed = self.data.plan.as_ref().is_some_and(|plan| plan.decision().needs_binding_evidence());
                                    self.send(ControlEvent::BindingObserved { evidence_needed });
                                }
                            },
                            Err(error) => self.stop(error),
                        }
                    }
                    Ok(Observation::Projects(inventory)) => {
                        let task = self.data.discovery.as_mut().expect("a projects reply answers a scoped discovery task");
                        match task.accept(&self.data.intent.roots, request, inventory) {
                            Ok(more_roots) => self.send(ControlEvent::ObservedProjects { more_roots }),
                            Err(error) => self.stop(error),
                        }
                    }
                    Ok(Observation::Destination(evidence)) => match request.destination_purpose() {
                        DestinationPurpose::Preparation => {
                            let path = self
                                .data
                                .creation
                                .as_ref()
                                .expect("a preparation destination reply answers an eligible creation")
                                .site
                                .path
                                .clone();
                            let absent = evidence == DestinationEvidence::Absent;
                            self.data.creation.as_mut().expect("checked eligible creation").destination = Some((request, evidence));
                            if absent {
                                self.send(ControlEvent::Observed);
                            } else {
                                self.stop(EntryError::DestinationExists { path });
                            }
                        }
                        DestinationPurpose::Binding => {
                            self.data.binding_evidence = Some((request, bind::Evidence::Destination(evidence)));
                            self.send(ControlEvent::Observed);
                        }
                    },
                    Ok(Observation::Defaults(defaults)) => {
                        self.data.creation.as_mut().expect("a defaults reply answers a validated generator").defaults = Some((request, defaults));
                        self.send(ControlEvent::Observed);
                    }
                    Ok(Observation::ProjectIdentity(directory)) => {
                        self.data.binding_evidence = Some((request, bind::Evidence::Route(directory)));
                        self.send(ControlEvent::Observed);
                    }
                    Err(source) => self.stop(EntryError::Observation {
                        request: Box::new(request),
                        source,
                    }),
                }
            }
            Event::Deliver(Delivery::Export) => {
                if self.control.leaf() != "planned" {
                    return Err(ProtocolError::NotPlanned);
                }
                self.send(ControlEvent::Export);
            }
            Event::Cancel => {
                if self.data.outcome.is_some() {
                    return Err(ProtocolError::Finished);
                }
                let boundary = self.control.leaf().into();
                self.pending = None;
                self.data.outcome = Some(AttemptOutcome::Cancelled { boundary });
                self.send(ControlEvent::Cancel);
            }
        }
        self.pump();
        Ok(())
    }

    fn send(&mut self, event: ControlEvent) {
        self.journal.push(AttemptRecord::Control(event));
        self.control.send(event);
    }

    fn stop(&mut self, error: EntryError) {
        self.pending = None;
        self.data.outcome = Some(AttemptOutcome::Stopped {
            boundary: self.control.leaf().into(),
            cause: Arc::new(error),
        });
        self.send(ControlEvent::Stop);
    }

    fn pump(&mut self) {
        loop {
            let directives = self.control.take_directives();
            if directives.is_empty() {
                // Also checks the actor/artifact suspension invariant.
                let _ = self.waiting();
                return;
            }
            for directive in directives {
                assert!(self.pending.is_none() && self.data.outcome.is_none(), "directive after an external boundary");
                self.journal.push(AttemptRecord::Directive(directive));
                match self.handle(directive) {
                    Ok(Some(event)) => self.send(event),
                    Ok(None) => {}
                    Err(error) => self.stop(error),
                }
            }
        }
    }

    /// Native directive vocabulary, NOT a state dispatcher or transition table.
    /// Only result events leave these handlers. They never select state IDs,
    /// observe a host, invoke a legacy resolver, or reconstruct earlier stages.
    fn handle(&mut self, directive: Directive) -> Result<Option<ControlEvent>, EntryError> {
        use Directive::*;
        let data = &mut self.data;
        Ok(match directive {
            ReadContext => {
                if data.acquisition.is_none() {
                    Some(ControlEvent::Supplied)
                } else {
                    self.issue(ReadKind::TargetContext);
                    None
                }
            }
            ApplyContextPolicy => {
                let policy = context::apply(data.acquisition.as_ref().expect("acquisition"), data.facts.as_ref().expect("context facts"))
                    .map_err(EntryError::Context)?;
                validate_intent(&policy.intent).map_err(|error| EntryError::Context(error.into()))?;
                let mut target = BoundTarget::from_supplied(data.id, policy.supplied.clone()).map_err(|error| EntryError::Context(error.into()))?;
                target.scope = policy.scope.clone();
                data.scope = policy.scope.clone();
                data.target = Some(target);
                data.intent = policy.intent.clone();
                data.generator_choice = policy.generator.clone();
                let needs_identity = policy.endpoint_identity.is_some();
                data.policy = Some(policy);
                Some(ControlEvent::ContextPolicy { needs_identity })
            }
            ReadEndpointIdentity => {
                let (current, selected) = data
                    .policy
                    .as_ref()
                    .expect("context policy")
                    .endpoint_identity
                    .clone()
                    .expect("different endpoint spellings");
                self.issue(ReadKind::EndpointIdentity { current, selected });
                None
            }
            ReadRegistry => {
                let path = data.policy.as_ref().expect("context policy").registry.clone();
                self.issue(ReadKind::Registry { path });
                None
            }
            ValidateRegistry => {
                data.generator = prepare::generator(&data.registry, &data.generator_choice)?;
                Some(ControlEvent::Validated)
            }
            ReadSelectionSessions => {
                self.issue_sessions(SessionReadPurpose::Selection);
                None
            }
            RankSessions => {
                let found = select::resolve_session(data.selection.as_ref().expect("selection inventory"), &data.intent)?;
                // The uniqueness guard has returned successfully BEFORE sending
                // either continuation. Ambiguity cannot reach project fallback.
                Some(match found {
                    Some(workspace) => {
                        data.workspace = Some(workspace);
                        ControlEvent::Unique
                    }
                    None => ControlEvent::Empty,
                })
            }
            ScopeProjects => {
                let scope = crate::projects::scope(&data.intent.selector, &data.intent.roots).map_err(EntryError::Discovery)?;
                data.discovery = Some(discover::Task::new(scope));
                Some(ControlEvent::Scoped)
            }
            ReadProjects => {
                let (slot, root) = data
                    .discovery
                    .as_ref()
                    .expect("scoped discovery task")
                    .next_root(&data.intent.roots)
                    .expect("the chart re-enters the read state only while a root remains");
                self.issue_projects(slot, root);
                None
            }
            RankProjects => {
                // All required roots are complete; missing root evidence can
                // never have supplied absence to reach this directive.
                let task = data.discovery.as_ref().expect("scoped discovery task");
                Some(match discover::rank(&data.intent.roots, &data.intent.selector, task)? {
                    Some(project) => {
                        data.project = Some(project);
                        ControlEvent::ProjectUnique
                    }
                    None => ControlEvent::ProjectEmpty,
                })
            }
            PrepareStart => {
                data.plan = Some(prepare::start_project(
                    data.target.as_ref().expect("bound context"),
                    &data.intent,
                    data.selection.as_ref().expect("selection inventory"),
                    data.discovery.as_ref().expect("scoped discovery task"),
                    data.project.as_ref().expect("ranked project"),
                )?);
                Some(ControlEvent::Prepared)
            }
            CreatePolicy => {
                data.creation = Some(discover::creation_policy(
                    &data.intent,
                    data.selection.as_ref().expect("selection inventory"),
                    data.discovery.as_ref().expect("scoped discovery task"),
                    data.generator.as_ref(),
                    &data.generator_choice,
                    &data.registry,
                )?);
                Some(ControlEvent::Eligible)
            }
            ReadDestination => {
                let path = data.creation.as_ref().expect("eligible creation").site.path.clone();
                self.issue_destination(path, DestinationPurpose::Preparation);
                None
            }
            ValidateGenerator => {
                let generator = data.generator.as_ref().expect("chosen generator");
                let validated = generator.validated().map_err(EntryError::Generator)?;
                discover::refuse_undeclared_destination(generator, validated.destination, &data.intent.roots)?;
                data.validated_generator = Some(validated.clone());
                Some(ControlEvent::ValidatedGenerator {
                    needs_defaults: discover::needs(validated.needs_defaults).any(),
                })
            }
            ReadDefaults => {
                let needs = discover::needs(data.validated_generator.as_ref().expect("validated generator").needs_defaults);
                self.issue_defaults(needs);
                None
            }
            PrepareCreation => {
                data.plan = Some(prepare::scaffold_and_start(
                    data.target.as_ref().expect("bound context"),
                    &data.intent,
                    data.selection.as_ref().expect("selection inventory"),
                    data.discovery.as_ref().expect("scoped discovery task"),
                    data.creation.as_ref().expect("eligible creation"),
                    data.generator.as_ref().expect("chosen generator"),
                    &data
                        .creation
                        .as_ref()
                        .expect("checked eligible creation")
                        .defaults
                        .as_ref()
                        .map(|(_, defaults)| defaults.clone())
                        .unwrap_or_default(),
                )?);
                Some(ControlEvent::Prepared)
            }
            SettleView => Some(ControlEvent::ViewPolicy {
                independent: prepare::independent(data.intent.view, data.target.as_ref().expect("bound context").client),
            }),
            PlanOpenView => {
                data.plan = Some(prepare::open_view(
                    data.target.as_ref().expect("bound context"),
                    &data.intent,
                    data.workspace.as_ref().expect("workspace match"),
                )?);
                Some(ControlEvent::Prepared)
            }
            PlanEnterView => {
                data.plan = Some(prepare::enter_view(
                    data.target.as_ref().expect("bound context"),
                    data.workspace.as_ref().expect("workspace match"),
                ));
                Some(ControlEvent::Prepared)
            }
            ReadBindingSessions => {
                self.issue_sessions(SessionReadPurpose::BindAction);
                None
            }
            ReadBindingEvidence => {
                enum EvidenceTarget {
                    Route(action::ProjectRoute),
                    Destination(std::path::PathBuf),
                }
                let target = match &data.plan.as_ref().expect("entry plan").decision() {
                    action::EntryDecision::StartProject { project, .. } => EvidenceTarget::Route(project.route.clone()),
                    action::EntryDecision::ScaffoldAndStart { destination, .. } => EvidenceTarget::Destination(destination.path.clone()),
                    _ => unreachable!("the chart reads binding evidence only for new work"),
                };
                match target {
                    EvidenceTarget::Route(route) => self.issue_project_identity(route),
                    EvidenceTarget::Destination(path) => self.issue_destination(path, DestinationPurpose::Binding),
                }
                None
            }
            CheckBinding => {
                bind::check(
                    data.plan.as_ref().expect("entry plan"),
                    data.binding.as_ref().expect("binding inventory"),
                    data.binding_evidence.as_ref(),
                )?;
                Some(ControlEvent::Checked)
            }
            BindOperation => {
                data.bound = Some(bind::operation(
                    data.plan.as_ref().expect("entry plan"),
                    data.binding.as_ref().expect("checked binding inventory"),
                    data.binding_evidence.as_ref(),
                )?);
                Some(ControlEvent::Bound)
            }
            ExportAction => {
                let action = data.bound.as_ref().expect("bound action");
                let expression = action::lower(action)?;
                let shell = expression.shell(tta_command::Handoff::Run).map_err(action::Error::from)?;
                data.outcome = Some(AttemptOutcome::Exported(Box::new(action::ExportedAction {
                    action: action.clone(),
                    expression,
                    shell,
                })));
                Some(ControlEvent::Exported)
            }
        })
    }

    fn issue(&mut self, kind: ReadKind) {
        assert!(self.pending.is_none(), "one outstanding request per attempt");
        self.sequence = self.sequence.checked_add(1).expect("request sequence exhausted");
        let request = ReadRequest {
            id: RequestId {
                attempt: self.data.id,
                sequence: self.sequence,
            },
            target: TargetBindingId(self.data.id),
            scope: self.data.scope.clone(),
            kind,
        };
        self.journal.push(AttemptRecord::ReadIssued(request.clone()));
        self.pending = Some(request);
    }

    fn issue_sessions(&mut self, purpose: SessionReadPurpose) {
        let kind = ReadKind::Sessions {
            endpoint: self.data.target.as_ref().expect("bound context").endpoint(),
            program: self.data.target.as_ref().expect("bound context").program.as_os_str().to_owned(),
            purpose,
        };
        self.issue(kind);
    }

    fn issue_projects(&mut self, slot: RootSlot, root: std::path::PathBuf) {
        let filter = self.data.discovery.as_ref().expect("scoped discovery task").fragment().to_owned();
        self.issue(ReadKind::Projects { slot, root, filter });
    }

    fn issue_destination(&mut self, path: std::path::PathBuf, purpose: DestinationPurpose) {
        self.issue(ReadKind::Destination { path, purpose });
    }

    fn issue_defaults(&mut self, needs: DefaultNeeds) {
        let cwd = self.data.target.as_ref().expect("bound context").cwd.clone();
        self.issue(ReadKind::GeneratorDefaults { cwd, needs });
    }

    fn issue_project_identity(&mut self, route: action::ProjectRoute) {
        self.issue(ReadKind::ProjectIdentity { route });
    }
}

/// Exhaustive request-kind ↔ observation-kind compatibility. The request
/// envelope's exact echo (path/slot/endpoint/purpose/program) is checked by
/// `ReadRequest` equality before this; kind compatibility is the second,
/// independent ingress gate.
fn observation_answers_request(kind: &ReadKind, observation: &Observation) -> bool {
    matches!(
        (kind, observation),
        (ReadKind::TargetContext, Observation::Context(_))
            | (ReadKind::Registry { .. }, Observation::Registry(_))
            | (ReadKind::EndpointIdentity { .. }, Observation::EndpointIdentity { .. })
            | (ReadKind::Sessions { .. }, Observation::Sessions(_))
            | (ReadKind::Projects { .. }, Observation::Projects(_))
            | (ReadKind::Destination { .. }, Observation::Destination(_))
            | (ReadKind::GeneratorDefaults { .. }, Observation::Defaults(_))
            | (ReadKind::ProjectIdentity { .. }, Observation::ProjectIdentity(_))
    )
}

fn validate_intent(intent: &EntryIntent) -> Result<(), InputError> {
    crate::validate_satellite_prefix(&intent.satellite_prefix)?;
    for root in &intent.roots {
        if !root.is_absolute() {
            return Err(InputError::Root(root.clone()));
        }
    }
    if let Some(name) = &intent.session_name {
        crate::resolve::validate_name(name)?;
    }
    if let InitialWindow::Named(name) = &intent.window
        && (name.is_empty() || name.chars().any(char::is_control))
    {
        return Err(InputError::Window(name.clone()));
    }
    action::validate_startup(&intent.startup)?;
    Ok(())
}

#[cfg(test)]
mod tests;
