//! Small observation-driven launch progression, not an executor. All target
//! inputs are explicit; callers can drive it without installing any runner.
use crate::{ClientMode, DestinationState, LaunchPlan, ProjectInventory, Selection, Session, WindowName, scaffold, socket};
use bon::Builder;
use std::ffi::OsString;
use std::path::PathBuf;

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
    #[builder(default)]
    pub grouped: bool,
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
        let input = &self.input;
        let sessions_request = || ObservationRequest::Sessions {
            socket: input.socket.path.clone(),
            program: input.program.clone(),
        };
        let Some(sessions) = &self.sessions else {
            return Ok(Progress::Observe(sessions_request()));
        };
        let defaults = self.defaults.clone().unwrap_or_default();
        let resolved = match scaffold::resolve()
            .sessions(sessions.sessions())
            .argument(&input.argument)
            .maybe_root(input.root.as_deref())
            .maybe_session_name(input.session_name.as_deref())
            .choice(&input.scaffold)
            .config(&input.registry)
            .maybe_projects(self.projects.as_ref())
            .maybe_destination(self.destination)
            .defaults(&defaults)
            .call()?
        {
            scaffold::Progress::Observe(crate::Resolution::NeedProjects(root)) => {
                return Ok(Progress::Observe(ObservationRequest::Projects {
                    root,
                    argument: input.argument.clone(),
                }));
            }
            scaffold::Progress::Observe(crate::Resolution::NeedDestination(path)) => return Ok(Progress::Observe(ObservationRequest::Destination { path })),
            scaffold::Progress::Resolved(resolved) => resolved,
            scaffold::Progress::Observe(crate::Resolution::Selected(_)) => unreachable!("scaffold handles selections"),
        };
        if let Some(generator) = &resolved.generator {
            let (author, license) = generator.defaults_required();
            if self.defaults.is_none() && (author || license) {
                return Ok(Progress::Observe(ObservationRequest::GeneratorDefaults {
                    directory: input.cwd.clone(),
                    author,
                    license,
                }));
            }
        }
        let satellite = match (&resolved.selection, input.grouped) {
            (Selection::Existing(session), true) => {
                let Some(sessions) = &self.satellite_sessions else {
                    return Ok(Progress::Observe(sessions_request()));
                };
                let name = crate::satellite_name(&session.name, sessions.sessions());
                tracing::info!(event = "satellite.named", base = %session.name, name = %name);
                Some(name)
            }
            _ => None,
        };
        let plan = crate::prepare_launch()
            .selection(&resolved.selection)
            .socket(&input.socket)
            .client_mode(input.client_mode)
            .program(&input.program)
            .maybe_config(input.config.as_deref())
            .maybe_prefix(resolved.prefix.as_ref())
            .grouped(input.grouped)
            .maybe_satellite_name(satellite.as_deref())
            .startup(&input.startup)
            .maybe_directory(input.directory.as_deref())
            .window_name(input.window_name.clone())
            .call()?;
        Ok(Progress::Ready(Box::new(PreparedLaunch {
            selection: resolved.selection,
            generator: resolved.generator,
            plan,
        })))
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
