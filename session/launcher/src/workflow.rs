//! Small observation-driven launch progression, not an executor. All target
//! inputs are explicit; callers can drive it without installing any runner.
use crate::{ClientMode, DestinationState, LaunchPlan, ProjectInventory, Selection, Session, WindowName, scaffold, socket};
use bon::Builder;
use std::ffi::OsString;
use std::path::PathBuf;

mod machine;
#[cfg(test)]
mod tests;

/// Target-local invocation inputs. Relative paths must be frozen by the caller
/// against the target cwd, never canonicalized on an unrelated local host.
#[derive(Clone, Debug, Builder)]
pub struct Input {
    pub argument: String,
    pub cwd: PathBuf,
    pub socket: socket::Resolved,
    pub client_mode: ClientMode,
    pub program: OsString,
    pub registry: scaffold::Config,
    pub scaffold: String,
    pub root: Option<PathBuf>,
    pub session_name: Option<String>,
    pub config: Option<PathBuf>,
    /// Omitted: independent outside entry; retain ordinary inside switching.
    pub grouped: Option<bool>,
    #[builder(default = "_".into())]
    pub satellite_prefix: String,
    #[builder(default)]
    pub startup: Vec<OsString>,
    pub directory: Option<PathBuf>,
    #[builder(default)]
    pub window_name: WindowName,
}

/// Server absence is classified evidence, distinct from a live empty server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionInventory {
    Absent,
    Present(Vec<Session>),
}

impl SessionInventory {
    pub fn sessions(&self) -> &[Session] {
        match self {
            Self::Absent => &[],
            Self::Present(sessions) => sessions,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationRequest {
    Sessions { socket: PathBuf, program: OsString },
    Projects { root: PathBuf, argument: String },
    Destination { path: PathBuf },
    GeneratorDefaults { directory: PathBuf, author: bool, license: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationReply {
    Sessions(SessionInventory),
    Projects(ProjectInventory),
    Destination(DestinationState),
    GeneratorDefaults(scaffold::Defaults),
}

#[derive(Clone, Debug)]
pub struct PreparedLaunch {
    pub selection: Selection,
    pub generator: Option<scaffold::Entry>,
    pub plan: LaunchPlan,
}

#[derive(Clone, Debug)]
pub enum Progress {
    Observe(ObservationRequest),
    Ready(Box<PreparedLaunch>),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Scaffold(#[from] scaffold::Error),
    #[error(transparent)]
    Plan(#[from] crate::PlanError),
    #[error(transparent)]
    Resolve(#[from] crate::ResolveError),
    #[error("observation reply does not answer the pending request {0:?}")]
    UnexpectedReply(ObservationRequest),
    #[error("launch is already ready; no observation reply is pending")]
    AlreadyReady,
    #[error("target invocation cwd must be absolute: {0:?}")]
    RelativeCwd(PathBuf),
    #[error("selected session {0} disappeared or changed name/group during preparation; run tta again")]
    SessionChanged(crate::SessionId),
}

#[derive(Clone, Debug)]
pub struct Workflow {
    input: Input,
    sessions: Option<SessionInventory>,
    projects: Option<ProjectInventory>,
    destination: Option<DestinationState>,
    defaults: Option<scaffold::Defaults>,
    satellite_sessions: Option<SessionInventory>,
}

impl Workflow {
    /// Registry/choice validation deliberately precedes the first session read.
    pub fn new(input: Input) -> Result<Self, Error> {
        match scaffold::Choice::parse(&input.scaffold)? {
            scaffold::Choice::Off => {}
            scaffold::Choice::Default => {
                input.registry.default_entry().map_err(scaffold::Error::from)?;
            }
            scaffold::Choice::Named(name) => {
                input.registry.entry(&name).map_err(scaffold::Error::from)?;
            }
        }
        if input.argument.is_empty() {
            return Err(crate::ResolveError::Empty.into());
        }
        crate::validate_satellite_prefix(&input.satellite_prefix)?;
        if !input.cwd.is_absolute() {
            return Err(Error::RelativeCwd(input.cwd.clone()));
        }
        Ok(Self {
            input,
            sessions: None,
            projects: None,
            destination: None,
            defaults: None,
            satellite_sessions: None,
        })
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn next(&self) -> Result<Progress, Error> {
        use machine::{Control, Event, Stage};

        let input = &self.input;
        let sessions_request = || ObservationRequest::Sessions {
            socket: input.socket.path.clone(),
            program: input.program.clone(),
        };
        let grouped = input.grouped.unwrap_or(input.client_mode == ClientMode::Attach);
        let mut control = Control::new();
        // Replay observations only when the graph asks for them, keeping native
        // data out of the actor. Nothing is observed or executed in this loop.
        let mut projects = None;
        let mut destination = None;
        let mut defaults = scaffold::Defaults::default();
        let mut defaults_supplied = false;
        let mut resolved = None;
        let mut pending = None;
        let mut satellite = None;
        let mut prepared = None;
        loop {
            match control.stage() {
                Stage::Sessions => {
                    if self.sessions.is_none() {
                        return Ok(Progress::Observe(sessions_request()));
                    }
                    control.send(Event::Observed);
                }
                Stage::Resolve => {
                    let sessions = self.sessions.as_ref().expect("sessions stage supplied inventory");
                    let progress = scaffold::resolve()
                        .sessions(sessions.sessions())
                        .argument(&input.argument)
                        .satellite_prefix(&input.satellite_prefix)
                        .maybe_root(input.root.as_deref())
                        .maybe_session_name(input.session_name.as_deref())
                        .choice(&input.scaffold)
                        .config(&input.registry)
                        .maybe_projects(projects)
                        .maybe_destination(destination)
                        .defaults(&defaults)
                        .call()?;
                    match progress {
                        scaffold::Progress::Observe(crate::Resolution::NeedProjects(root)) => {
                            pending = Some(ObservationRequest::Projects {
                                root,
                                argument: input.argument.clone(),
                            });
                            control.send(Event::ProjectsNeeded);
                        }
                        scaffold::Progress::Observe(crate::Resolution::NeedDestination(path)) => {
                            pending = Some(ObservationRequest::Destination { path });
                            control.send(Event::DestinationNeeded);
                        }
                        scaffold::Progress::Resolved(selection) => {
                            let needs_defaults = selection.generator.as_ref().is_some_and(|generator| {
                                let (author, license) = generator.defaults_required();
                                !defaults_supplied && (author || license)
                            });
                            let needs_satellite = grouped && matches!(selection.selection, Selection::Existing(_));
                            resolved = Some(selection);
                            control.send(Event::Resolved {
                                needs_defaults,
                                needs_satellite,
                            });
                        }
                        scaffold::Progress::Observe(crate::Resolution::Selected(_)) => unreachable!("scaffold handles selections"),
                    }
                }
                Stage::Projects => {
                    let Some(facts) = &self.projects else {
                        return Ok(Progress::Observe(pending.expect("resolver requested projects")));
                    };
                    projects = Some(facts);
                    control.send(Event::Observed);
                }
                Stage::Destination => {
                    let Some(facts) = self.destination else {
                        return Ok(Progress::Observe(pending.expect("resolver requested destination")));
                    };
                    destination = Some(facts);
                    control.send(Event::Observed);
                }
                Stage::Defaults => {
                    let Some(facts) = &self.defaults else {
                        let generator = resolved
                            .as_ref()
                            .and_then(|selection| selection.generator.as_ref())
                            .expect("defaults guard requires generator");
                        let (author, license) = generator.defaults_required();
                        return Ok(Progress::Observe(ObservationRequest::GeneratorDefaults {
                            directory: input.cwd.clone(),
                            author,
                            license,
                        }));
                    };
                    defaults = facts.clone();
                    defaults_supplied = true;
                    control.send(Event::Observed);
                }
                Stage::FreshSessions => {
                    let Selection::Existing(session) = &resolved.as_ref().expect("resolved before satellite guard").selection else {
                        unreachable!("satellite guard requires existing selection")
                    };
                    let Some(sessions) = &self.satellite_sessions else {
                        return Ok(Progress::Observe(sessions_request()));
                    };
                    if !sessions.sessions().iter().any(|fresh| fresh == session) {
                        return Err(Error::SessionChanged(session.id));
                    }
                    let name = crate::satellite_name(session, sessions.sessions(), &input.satellite_prefix);
                    tracing::info!(event = "satellite.named", base = %session.name, name = %name);
                    satellite = Some(name);
                    control.send(Event::Observed);
                }
                Stage::Prepare => {
                    let resolved = resolved.take().expect("resolved before preparation");
                    let plan = crate::prepare_launch()
                        .selection(&resolved.selection)
                        .socket(&input.socket)
                        .client_mode(input.client_mode)
                        .program(&input.program)
                        .maybe_config(input.config.as_deref())
                        .maybe_prefix(resolved.prefix.as_ref())
                        .grouped(grouped)
                        .maybe_satellite_name(satellite.as_deref())
                        .startup(&input.startup)
                        .maybe_directory(input.directory.as_deref())
                        .window_name(input.window_name.clone())
                        .call()?;
                    prepared = Some(Box::new(PreparedLaunch {
                        selection: resolved.selection,
                        generator: resolved.generator,
                        plan,
                    }));
                    control.send(Event::Prepared);
                }
                Stage::Ready => return Ok(Progress::Ready(prepared.expect("prepared before readiness"))),
            }
        }
    }

    /// Reject mismatched replies without advancing. Errors remain errors at the
    /// observer boundary; no unknown or failed observation can become absence.
    pub fn supply(&mut self, reply: ObservationReply) -> Result<(), Error> {
        let Progress::Observe(request) = self.next()? else {
            return Err(Error::AlreadyReady);
        };
        match (&request, reply) {
            (ObservationRequest::Sessions { .. }, ObservationReply::Sessions(sessions)) => {
                if self.sessions.is_none() {
                    self.sessions = Some(sessions);
                } else {
                    self.satellite_sessions = Some(sessions);
                }
            }
            (ObservationRequest::Projects { .. }, ObservationReply::Projects(projects)) => self.projects = Some(projects),
            (ObservationRequest::Destination { .. }, ObservationReply::Destination(state)) => self.destination = Some(state),
            (ObservationRequest::GeneratorDefaults { .. }, ObservationReply::GeneratorDefaults(defaults)) => self.defaults = Some(defaults),
            _ => return Err(Error::UnexpectedReply(request)),
        }
        Ok(())
    }
}
