//! Owned entry protocol, staged internally until the coherent frontend cutover.
pub mod action;
mod bind;
pub mod catalog;
pub mod context;
mod discover;
pub mod machine;
mod prepare;
pub mod probe;
pub mod protocol;
pub mod registry;
pub mod select;

use std::sync::Arc;

use machine::{Control, Directive, Event as ControlEvent};
use protocol::*;

/// One owner and one retained actor for an attempt. Native facts, artifacts and
/// errors never enter the runtime's serde context or event payloads.
pub struct EntryAttempt {
    control: Control,
    data: AttemptData,
    pending: Option<ReadRequest>,
    effects: EffectState,
    journal: Vec<AttemptRecord>,
    sequence: u64,
}

/// Effect progression state: one requested or in-flight effect, plus finished
/// receipts in dispatch order. `requested` cannot be reissued and `in_flight`
/// cannot redispatch; waiting is inert.
#[derive(Default)]
struct EffectState {
    requested: Option<EffectRequest>,
    in_flight: Option<EffectRequest>,
    finished: Vec<(EffectId, EffectReceipt)>,
}

impl EffectState {
    fn knowledge(&self) -> EffectKnowledge {
        EffectKnowledge {
            finished: self.finished.clone(),
            in_flight: self.in_flight.as_ref().map(|effect| vec![effect.id]).unwrap_or_default(),
        }
    }
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
    binding_evidence: bind::CollectedEvidence,
    execution: Option<bind::ExecutionChoice>,
    bound: Option<action::BoundAction>,
    outcome: Option<AttemptOutcome>,
}

impl EntryAttempt {
    pub fn start(invocation: Invocation) -> Result<Self, InputError> {
        let scope = context::Scope {
            origin: context::Origin::Local {
                host: invocation.context.host.clone(),
            },
            cwd: Some(invocation.context.cwd.clone()),
        };
        Self::start_at(invocation, scope)
    }

    /// Supplied facts use the same frozen scope type as acquired contexts.
    pub fn start_at(invocation: Invocation, scope: context::Scope) -> Result<Self, InputError> {
        let id = AttemptId::new();
        let mut target = BoundTarget::from_supplied(id, invocation.context)?;
        let host = match &scope.origin {
            context::Origin::Local { host } | context::Origin::Ssh { host, .. } => host,
        };
        if host != &target.host || scope.cwd.as_ref() != Some(&target.cwd) || scope.frozen(target.cwd.clone()) != scope {
            return Err(InputError::Target("supplied scope does not match frozen host/cwd/setup"));
        }
        if matches!(scope.origin, context::Origin::Ssh { .. }) && target.client != ClientEntry::AttachTerminal {
            return Err(InputError::Target("SSH context cannot authorize an inherited-client switch"));
        }
        if let context::Origin::Ssh { setup, .. } = &scope.origin {
            setup.shell_command(":").map_err(InputError::Context)?;
        }
        target.scope = scope;
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
                binding_evidence: bind::CollectedEvidence::default(),
                execution: None,
                bound: None,
                outcome: None,
            },
            pending: None,
            effects: EffectState::default(),
            journal: Vec::new(),
            sequence: 0,
        };
        attempt.pump();
        Ok(attempt)
    }

    pub fn acquire(input: context::Acquisition) -> Result<Self, InputError> {
        context::validate_initial_scope(&input.scope).map_err(InputError::Context)?;
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
                binding_evidence: bind::CollectedEvidence::default(),
                execution: None,
                bound: None,
                outcome: None,
            },
            pending: None,
            effects: EffectState::default(),
            journal: Vec::new(),
            sequence: 0,
        };
        attempt.pump();
        Ok(attempt)
    }

    /// Inspection is inert: no actor creation, emission drain, materialization,
    /// trace or native handler call happens here.
    pub fn waiting(&self) -> Waiting<'_> {
        if let Some(request) = &self.pending {
            return Waiting::Read(request);
        }
        if let Some(effect) = &self.effects.requested {
            return Waiting::Effect(effect);
        }
        if let Some(effect) = &self.effects.in_flight {
            return Waiting::InFlight(effect.id);
        }
        if let Some(outcome) = &self.data.outcome {
            return Waiting::Finished(outcome);
        }
        let leaf = self.control.leaf();
        assert!(
            leaf == "planned" || leaf == "exported" || leaf == "completed" || leaf == "delegated" || leaf == "stopped",
            "chart must suspend at an external boundary, not {leaf}"
        );
        Waiting::Planned(self.data.plan.as_ref().expect("planned artifact"))
    }

    pub fn snapshot(&self) -> AttemptSnapshot {
        AttemptSnapshot {
            id: self.data.id,
            target: self.data.target.clone(),
            leaf: self.control.leaf().into(),
            pending: self.pending.clone(),
            pending_effect: self.effects.requested.clone().or_else(|| self.effects.in_flight.clone()),
            plan: self.data.plan.clone(),
            bound: self.data.bound.clone(),
            outcome: self.data.outcome.clone(),
            journal: self.journal.clone(),
        }
    }

    /// Protocol errors reject the entire event before modifying any data, actor
    /// state or journal. Accepted acquisition/domain errors instead stop natively.
    pub fn advance(&mut self, event: Event) -> Result<(), ProtocolError> {
        match event {
            Event::Observed(completion) => {
                check_completion(self.pending.as_ref(), &completion)?;
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
                                    let evidence = self.data.plan.as_ref().expect("plan").decision().binding_evidence();
                                    self.send(ControlEvent::BindingObserved { evidence });
                                }
                                SessionReadPurpose::Catalog => unreachable!("entry never requests a catalog inventory"),
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
                            self.data.binding_evidence.push(request, bind::Evidence::Destination(evidence));
                            self.send(ControlEvent::Observed);
                        }
                    },
                    Ok(Observation::Defaults(defaults)) => {
                        self.data.creation.as_mut().expect("a defaults reply answers a validated generator").defaults = Some((request, defaults));
                        self.send(ControlEvent::Observed);
                    }
                    Ok(Observation::ProjectIdentity(directory)) => match &request.kind {
                        ReadKind::ProjectIdentity { .. } => {
                            self.data.binding_evidence.push(request, bind::Evidence::Route(directory));
                            self.send(ControlEvent::Observed);
                        }
                        ReadKind::CreationRootIdentity { root, expected } => {
                            // The root witness is verified HERE, before the
                            // binding destination lstat is even requested.
                            if directory == *expected {
                                self.data.binding_evidence.push(request, bind::Evidence::CreationRoot(directory));
                                self.send(ControlEvent::ObservedEvidence { destination_needed: true });
                            } else {
                                self.stop(EntryError::CreationRootChanged {
                                    root: root.clone(),
                                    expected: expected.clone(),
                                    observed: directory,
                                });
                            }
                        }
                        _ => unreachable!("project identity answers an identity request"),
                    },
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
            Event::Deliver(Delivery::Execute { native }) => {
                if self.control.leaf() != "planned" {
                    return Err(ProtocolError::NotPlanned);
                }
                self.data.execution = Some(bind::ExecutionChoice { native });
                self.send(ControlEvent::Execute);
            }
            Event::EffectStarted(id) => {
                let Some(effect) = self.effects.requested.as_ref() else {
                    // An already-started effect cannot be redispatched.
                    if self.effects.in_flight.as_ref().is_some_and(|started| started.id == id) {
                        return Err(ProtocolError::AlreadyStarted(id));
                    }
                    return Err(ProtocolError::NoPendingEffect);
                };
                if effect.id != id {
                    return Err(ProtocolError::EffectMismatch {
                        expected: effect.id,
                        received: id,
                    });
                }
                let effect = self.effects.requested.take().expect("checked");
                self.journal.push(AttemptRecord::EffectStarted(id));
                self.effects.in_flight = Some(effect);
                self.send(ControlEvent::EffectStarted);
            }
            Event::EffectFinished(completion) => {
                let Some(effect) = self.effects.in_flight.as_ref() else {
                    return Err(ProtocolError::NoPendingEffect);
                };
                if effect.id != completion.id {
                    return Err(ProtocolError::EffectMismatch {
                        expected: effect.id,
                        received: completion.id,
                    });
                }
                // Shape gate BEFORE consuming the in-flight effect.
                if let Ok(receipt) = &completion.result
                    && !receipt_answers_effect(&effect.kind, receipt)
                {
                    return Err(ProtocolError::ReceiptShape {
                        effect: effect.id,
                        received: *receipt,
                    });
                }
                let effect = self.effects.in_flight.take().expect("checked");
                self.journal.push(AttemptRecord::EffectFinished(completion.id));
                match completion.result {
                    Ok(receipt) => {
                        self.effects.finished.push((effect.id, receipt));
                        self.send(ControlEvent::EffectFinished);
                    }
                    Err(failure) => self.stop(EntryError::Effect {
                        effect: Box::new(effect),
                        failure,
                    }),
                }
            }
            Event::DelegateTerminal(id) => {
                let Some(effect) = self.effects.requested.as_ref() else {
                    return Err(ProtocolError::NotDelegatable);
                };
                if !effect.is_terminal() {
                    return Err(ProtocolError::NotDelegatable);
                }
                if effect.id != id {
                    return Err(ProtocolError::EffectMismatch {
                        expected: effect.id,
                        received: id,
                    });
                }
                let effect = self.effects.requested.take().expect("checked");
                let effects = self.effects.knowledge();
                self.journal.push(AttemptRecord::EffectDelegated(id));
                self.data.outcome = Some(AttemptOutcome::Delegated { terminal: effect.id, effects });
                self.send(ControlEvent::Delegated);
            }
            Event::Cancel => {
                if self.data.outcome.is_some() {
                    return Err(ProtocolError::Finished);
                }
                let boundary = self.control.leaf().into();
                self.pending = None;
                // A requested-but-undispatched effect is simply not dispatched;
                // an in-flight one keeps its uncertainty in the knowledge snapshot.
                let effects = self.effects.knowledge();
                self.effects.requested = None;
                self.effects.in_flight = None;
                self.data.outcome = Some(AttemptOutcome::Cancelled { boundary, effects });
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
        let boundary = self.control.leaf().into();
        let effects = self.effects.knowledge();
        self.pending = None;
        self.effects.requested = None;
        self.data.outcome = Some(AttemptOutcome::Stopped {
            boundary,
            effects,
            cause: Arc::new(error),
        });
        self.send(ControlEvent::Stop);
    }

    fn pump(&mut self) {
        loop {
            let directives = self.control.take_directives();
            if directives.is_empty() {
                // `completed` has no entry directive; materialize its outcome
                // from the recorded receipts.
                if self.data.outcome.is_none()
                    && self.pending.is_none()
                    && self.effects.requested.is_none()
                    && self.effects.in_flight.is_none()
                    && self.control.leaf() == "completed"
                {
                    let effects = self.effects.knowledge();
                    self.data.outcome = Some(AttemptOutcome::Completed { effects });
                }
                // Also checks the actor/artifact suspension invariant.
                let _ = self.waiting();
                return;
            }
            for directive in directives {
                assert!(
                    self.pending.is_none() && self.effects.requested.is_none() && self.effects.in_flight.is_none() && self.data.outcome.is_none(),
                    "directive after an external boundary"
                );
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
                let policy = context::apply(
                    data.acquisition.as_ref().expect("acquisition"),
                    data.facts.as_ref().expect("context facts"),
                    context::Use::Entry,
                )
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
                let evidence = {
                    let plan = data.plan.as_ref().expect("entry plan");
                    match plan.decision() {
                        action::EntryDecision::StartProject { project, .. } => ReadKind::ProjectIdentity {
                            route: Box::new(project.route.clone()),
                        },
                        action::EntryDecision::ScaffoldAndStart { destination, .. } => {
                            // The fresh creation-root canonical identity precedes
                            // the destination lstat; the ingress verifies it
                            // before the chart requests the destination read.
                            ReadKind::CreationRootIdentity {
                                root: destination.requested_root.clone(),
                                expected: destination.root.clone(),
                            }
                        }
                        _ => unreachable!("the chart reads binding evidence only for new work"),
                    }
                };
                self.issue(evidence);
                None
            }
            ReadBindingDestination => {
                let path = match data.plan.as_ref().expect("entry plan").decision() {
                    action::EntryDecision::ScaffoldAndStart { destination, .. } => destination.path.clone(),
                    _ => unreachable!("only scaffold bindings recheck destination absence"),
                };
                self.issue_destination(path, DestinationPurpose::Binding);
                None
            }
            CheckBinding => {
                bind::check(
                    data.plan.as_ref().expect("entry plan"),
                    data.binding.as_ref().expect("binding inventory"),
                    &data.binding_evidence,
                )?;
                Some(ControlEvent::Checked)
            }
            BindOperation => {
                data.bound = Some(bind::operation(
                    data.plan.as_ref().expect("entry plan"),
                    data.binding.as_ref().expect("checked binding inventory"),
                    &data.binding_evidence,
                    data.execution,
                )?);
                Some(ControlEvent::Bound {
                    exported: data.execution.is_none(),
                })
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
            DispatchExecution => {
                let bound = data.bound.as_ref().expect("bound action");
                Some(match bound.shape() {
                    action::ExecutionShape::TerminalBlock => ControlEvent::EffectTerminal,
                    action::ExecutionShape::NativeCreateThenEnter => match bound.operation() {
                        action::BoundOperation::ScaffoldAndStart { .. } => ControlEvent::EffectGenerate,
                        action::BoundOperation::StartProject { .. } => ControlEvent::EffectDirectory,
                        _ => unreachable!("native shape is chosen only for new work"),
                    },
                })
            }
            RequestGenerate => {
                let program = match &data.bound.as_ref().expect("bound action").operation {
                    action::BoundOperation::ScaffoldAndStart { generator, .. } => generator.clone(),
                    _ => unreachable!("generation precedes native creation only for scaffolds"),
                };
                self.issue_effect(EffectKind::Generate { program });
                None
            }
            RequestDirectory => {
                let path = Self::bound_session(data).directory.path().to_owned();
                self.issue_effect(EffectKind::CheckCreatedDirectory { path });
                None
            }
            RequestNative => {
                let session = Self::bound_session(data);
                let (invocation, directory) = action::native_creation(&session)?;
                self.issue_effect(EffectKind::CreateNative { invocation, directory });
                None
            }
            RequestTerminal => {
                let (target, shape) = {
                    let bound = data.bound.as_ref().expect("bound action");
                    (bound.bound_target().clone(), bound.shape())
                };
                let expression = match shape {
                    action::ExecutionShape::TerminalBlock => action::lower(data.bound.as_ref().expect("bound action"))?,
                    // The native tail enters the pane directory and moves this
                    // client onto the acknowledged creation. It is not a CLI
                    // fallback and carries no generation/creation steps.
                    action::ExecutionShape::NativeCreateThenEnter => action::native_tail(&target, &Self::bound_session(data))?,
                };
                self.issue_effect(EffectKind::Terminal { expression });
                None
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
            scope: Box::new(self.data.scope.clone()),
            kind,
        };
        self.journal.push(AttemptRecord::ReadIssued(request.clone()));
        self.pending = Some(request);
    }

    /// One outstanding effect per attempt; a requested effect is issued once
    /// and waiting never reissues it.
    fn issue_effect(&mut self, kind: EffectKind) {
        assert!(
            self.pending.is_none() && self.effects.requested.is_none() && self.effects.in_flight.is_none(),
            "one outstanding effect per attempt"
        );
        self.sequence = self.sequence.checked_add(1).expect("effect sequence exhausted");
        let effect = EffectRequest {
            id: EffectId {
                attempt: self.data.id,
                sequence: self.sequence,
            },
            target: TargetBindingId(self.data.id),
            kind,
        };
        self.journal.push(AttemptRecord::EffectIssued(effect.clone()));
        self.effects.requested = Some(effect);
    }

    /// The bound new-work session of a native creation path.
    fn bound_session(data: &AttemptData) -> action::BoundSession {
        match &data.bound.as_ref().expect("bound action").operation {
            action::BoundOperation::StartProject { session, .. } | action::BoundOperation::ScaffoldAndStart { session, .. } => session.clone(),
            _ => unreachable!("native effects are issued only for new work"),
        }
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
            | (ReadKind::CreationRootIdentity { .. }, Observation::ProjectIdentity(_))
    )
}

/// Exhaustive effect-kind ↔ receipt-kind compatibility, checked before the
/// in-flight effect is consumed.
fn receipt_answers_effect(kind: &EffectKind, receipt: &EffectReceipt) -> bool {
    matches!(
        (kind, receipt),
        (EffectKind::Generate { .. }, EffectReceipt::Generated)
            | (EffectKind::CheckCreatedDirectory { .. }, EffectReceipt::Directory)
            | (EffectKind::CreateNative { .. }, EffectReceipt::Created)
            | (EffectKind::Terminal { .. }, EffectReceipt::Terminal { .. })
    )
}

fn check_completion(expected: Option<&ReadRequest>, completion: &ReadCompletion) -> Result<(), ProtocolError> {
    let expected = expected.ok_or(ProtocolError::NoPendingRead)?;
    if completion.request != *expected {
        return Err(ProtocolError::ReadMismatch {
            expected: Box::new(expected.clone()),
            received: Box::new(completion.request.clone()),
        });
    }
    // Validate shape BEFORE consuming pending or modifying actor/native data.
    if let Ok(observation) = &completion.result
        && !observation_answers_request(&expected.kind, observation)
    {
        return Err(ProtocolError::ObservationShape {
            expected: Box::new(expected.clone()),
            received: Box::new(observation.clone()),
        });
    }
    Ok(())
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
