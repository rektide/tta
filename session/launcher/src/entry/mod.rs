//! Owned entry protocol, staged internally until the coherent frontend cutover.
mod action;
mod bind;
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
    target: BoundTarget,
    intent: EntryIntent,
    registry: crate::scaffold::Config,
    generator_choice: crate::scaffold::Choice,
    generator: Option<crate::scaffold::Entry>,
    selection: Option<select::WorkspaceCatalog>,
    workspace: Option<select::WorkspaceMatch>,
    plan: Option<action::EntryPlan>,
    binding: Option<select::WorkspaceCatalog>,
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
        crate::validate_satellite_prefix(&invocation.intent.satellite_prefix)?;
        let mut attempt = Self {
            control: Control::start(),
            data: AttemptData {
                id,
                target,
                intent: invocation.intent,
                registry: invocation.registry,
                generator_choice: invocation.generator,
                generator: None,
                selection: None,
                workspace: None,
                plan: None,
                binding: None,
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
                // Exhaustive shape check belongs here as new observation kinds
                // land. Sessions are the only supported fact type in this stage.
                let request = self.pending.take().expect("checked pending request");
                self.journal.push(AttemptRecord::ReadAccepted(request.id));
                match completion.result {
                    Ok(Observation::Sessions(inventory)) => {
                        let purpose = request.sessions_purpose();
                        match select::WorkspaceCatalog::from_read(request, inventory) {
                            Ok(catalog) => {
                                match purpose {
                                    SessionReadPurpose::Selection => self.data.selection = Some(catalog),
                                    SessionReadPurpose::BindAction => self.data.binding = Some(catalog),
                                }
                                self.send(ControlEvent::Observed);
                            }
                            Err(error) => self.stop(error),
                        }
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
            BeginProjects => return Err(EntryError::DiscoveryNotImplemented),
            SettleView => Some(ControlEvent::ViewPolicy {
                independent: prepare::independent(data.intent.view, data.target.client),
            }),
            PlanOpenView => {
                data.plan = Some(prepare::open_view(
                    &data.target,
                    &data.intent,
                    data.workspace.as_ref().expect("workspace match"),
                )?);
                Some(ControlEvent::Prepared)
            }
            PlanEnterView => {
                data.plan = Some(prepare::enter_view(&data.target, data.workspace.as_ref().expect("workspace match")));
                Some(ControlEvent::Prepared)
            }
            ReadBindingSessions => {
                self.issue_sessions(SessionReadPurpose::BindAction);
                None
            }
            CheckBinding => {
                bind::check(data.plan.as_ref().expect("entry plan"), data.binding.as_ref().expect("binding inventory"))?;
                Some(ControlEvent::Checked)
            }
            BindOperation => {
                data.bound = Some(bind::operation(
                    data.plan.as_ref().expect("entry plan"),
                    data.binding.as_ref().expect("checked binding inventory"),
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

    fn issue_sessions(&mut self, purpose: SessionReadPurpose) {
        assert!(self.pending.is_none(), "one outstanding request per attempt");
        self.sequence = self.sequence.checked_add(1).expect("request sequence exhausted");
        let request = ReadRequest {
            id: RequestId {
                attempt: self.data.id,
                sequence: self.sequence,
            },
            target: self.data.target.binding,
            kind: ReadKind::Sessions {
                endpoint: self.data.target.endpoint(),
                program: self.data.target.program.as_os_str().to_owned(),
                purpose,
            },
        };
        self.journal.push(AttemptRecord::ReadIssued(request.clone()));
        self.pending = Some(request);
    }
}

#[cfg(test)]
mod tests;
