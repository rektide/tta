//! Internal host integration for the owned protocol. Interpreters acquire one
//! requested fact; the one read driver never chooses work or entry policy.
pub mod filesystem;
pub mod local;
pub mod ssh;

use anyhow::{Result, ensure};
use tta_launcher::entry::{
    EntryAttempt,
    context::{Origin, Scope},
    protocol::*,
};

pub trait Observer {
    fn observe(&mut self, request: &ReadRequest) -> Result<Observation>;
}

/// Keep anyhow's native source/downcast and backtrace ownership. Converting it
/// directly into Box<dyn Error> uses an opaque wrapper that hides the root type.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct HostFailure(#[source] pub anyhow::Error);

/// A task only exposes read completion. In particular this driver has no
/// delivery/default/generator/view API to accidentally call.
pub trait ReadTask {
    fn pending_read(&self) -> Option<&ReadRequest>;
    fn complete_read(&mut self, completion: ReadCompletion) -> std::result::Result<(), ProtocolError>;
}

impl ReadTask for EntryAttempt {
    fn pending_read(&self) -> Option<&ReadRequest> {
        match self.waiting() {
            Waiting::Read(request) => Some(request),
            _ => None,
        }
    }
    fn complete_read(&mut self, completion: ReadCompletion) -> std::result::Result<(), ProtocolError> {
        self.advance(Event::Observed(completion))
    }
}

impl ReadTask for tta_launcher::entry::catalog::CatalogQuery {
    fn pending_read(&self) -> Option<&ReadRequest> {
        match self.waiting() {
            tta_launcher::entry::catalog::Waiting::Read(request) => Some(request),
            _ => None,
        }
    }
    fn complete_read(&mut self, completion: ReadCompletion) -> std::result::Result<(), ProtocolError> {
        self.advance(tta_launcher::entry::catalog::Event::Observed(completion))
    }
}

pub fn drive_reads(task: &mut dyn ReadTask, observer: &mut dyn Observer) -> std::result::Result<(), ProtocolError> {
    while let Some(request) = task.pending_read().cloned() {
        let result = observer.observe(&request).map_err(|source| ObservationFailure {
            source: Box::new(HostFailure(source)),
        });
        task.complete_read(ReadCompletion { request, result })?;
    }
    Ok(())
}

/// Pin the host/setup before I/O, then freeze the observed physical cwd once.
/// A second context or a request from another attempt/setup cannot reuse it.
pub(super) struct Guard {
    scope: Scope,
    target: Option<TargetBindingId>,
    bound: bool,
    endpoint: Option<(EndpointKey, std::ffi::OsString)>,
}

impl Guard {
    pub(super) fn new(scope: Scope) -> Result<Self> {
        ensure!(scope.cwd.is_none(), "acquisition guard must start unfrozen");
        Ok(Self {
            scope,
            target: None,
            bound: false,
            endpoint: None,
        })
    }
    pub(super) fn supplied(target: &BoundTarget) -> Self {
        Self {
            scope: target.scope().clone(),
            target: Some(target.endpoint().target),
            bound: true,
            endpoint: Some((target.endpoint(), target.program().to_owned())),
        }
    }
    pub(super) fn check(&mut self, request: &ReadRequest) -> Result<()> {
        ensure!(*request.scope == self.scope, "observation target setup/cwd mismatch");
        ensure!(self.target.is_none_or(|target| target == request.target), "observation target binding mismatch");
        ensure!(request.id.attempt == request.target.attempt(), "observation attempt/target mismatch");
        if self.bound {
            ensure!(!matches!(request.kind, ReadKind::TargetContext), "context already frozen");
        } else {
            ensure!(matches!(request.kind, ReadKind::TargetContext), "context must be observed before other reads");
        }
        if let ReadKind::Sessions { endpoint, program, .. } = &request.kind {
            ensure!(endpoint.target == request.target, "session endpoint target mismatch");
            ensure!(
                self.endpoint.as_ref().is_none_or(|bound| bound == &(endpoint.clone(), program.clone())),
                "session endpoint/program differs from frozen binding"
            );
            self.endpoint = Some((endpoint.clone(), program.clone()));
        }
        self.target = Some(request.target);
        Ok(())
    }
    pub(super) fn accept(&mut self, observation: &Observation) -> Result<()> {
        if let Observation::Context(facts) = observation {
            ensure!(facts.cwd.is_absolute(), "observed cwd must be absolute");
            self.scope = self.scope.frozen(facts.cwd.clone());
            self.bound = true;
        }
        Ok(())
    }
    pub(super) fn check_target(&self, target: &BoundTarget) -> Result<()> {
        ensure!(
            self.bound && target.scope() == &self.scope && self.target == Some(target.endpoint().target),
            "handoff target differs from frozen observation target"
        );
        ensure!(
            self.endpoint
                .as_ref()
                .is_some_and(|(endpoint, program)| endpoint == &target.endpoint() && program == target.program()),
            "handoff endpoint/program differs from frozen observations"
        );
        Ok(())
    }
    pub(super) fn is_local(&self) -> bool {
        matches!(self.scope.origin, Origin::Local { .. })
    }
}
