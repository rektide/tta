//! Drive pure progression with replaceable observation, without conflating
//! shell emission with an execution outcome.
use crate::backend::SessionLookup;
use anyhow::Result;
use bon::builder;
use std::ffi::OsStr;
use std::path::Path;
use tta_launcher::workflow::{Input, ObservationReply, ObservationRequest, PreparedLaunch, Progress, Workflow};
use tta_launcher::{ClientMode, WindowName, scaffold, socket};

/// Standard-error-compatible boundary for embedded callers using eyre or other
/// reporting libraries. The complete contextual source chain is retained.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(
    #[from]
    #[source]
    anyhow::Error,
);

pub trait Observer {
    fn observe(&mut self, request: &ObservationRequest) -> Result<ObservationReply>;
}

pub fn drive(mut workflow: Workflow, observer: &mut dyn Observer) -> Result<PreparedLaunch> {
    loop {
        match workflow.next()? {
            Progress::Ready(prepared) => return Ok(*prepared),
            Progress::Observe(request) => workflow.supply(observer.observe(&request)?)?,
        }
    }
}

pub struct LocalObserver<'a> {
    pub lookup: &'a dyn SessionLookup,
}

impl Observer for LocalObserver<'_> {
    fn observe(&mut self, request: &ObservationRequest) -> Result<ObservationReply> {
        Ok(match request {
            ObservationRequest::Sessions { .. } => ObservationReply::Sessions(self.lookup.inventory()?),
            ObservationRequest::Projects { root, argument } => ObservationReply::Projects(crate::resolve::project_inventory(root, argument)?),
            ObservationRequest::Destination { path } => ObservationReply::Destination(crate::resolve::destination_state(path)?),
            ObservationRequest::GeneratorDefaults { directory, author, license } => {
                ObservationReply::GeneratorDefaults(crate::scaffold::observe_defaults(directory, *author, *license)?)
            }
        })
    }
}

/// Local convenience composition shared by the CLI and embedded consumers.
/// Registry loading is observation-only; calling this never mutates the target.
#[builder]
pub fn prepare(
    lookup: &dyn SessionLookup,
    argument: &str,
    socket: &socket::Resolved,
    client_mode: ClientMode,
    program: &OsStr,
    root: Option<&Path>,
    session_name: Option<&str>,
    choice: &str,
    config: Option<&scaffold::Config>,
    tmux_config: Option<&Path>,
    grouped: Option<bool>,
    #[builder(default = "_")] satellite_prefix: &str,
    #[builder(default = &[])] startup: &[std::ffi::OsString],
    directory: Option<&Path>,
    #[builder(default)] window_name: WindowName,
) -> std::result::Result<PreparedLaunch, Error> {
    let run = || -> Result<PreparedLaunch> {
        let loaded;
        let registry = match config {
            Some(config) => config,
            None => {
                loaded = crate::scaffold::load_config()?;
                &loaded
            }
        };
        let input = Input::builder()
            .argument(argument.to_owned())
            .cwd(std::env::current_dir()?)
            .socket(socket.clone())
            .client_mode(client_mode)
            .program(program.to_owned())
            .registry(registry.clone())
            .scaffold(choice.to_owned())
            .maybe_root(root.map(Path::to_owned))
            .maybe_session_name(session_name.map(str::to_owned))
            .maybe_config(tmux_config.map(Path::to_owned))
            .maybe_grouped(grouped)
            .satellite_prefix(satellite_prefix.to_owned())
            .startup(startup.to_vec())
            .maybe_directory(directory.map(Path::to_owned))
            .window_name(window_name)
            .build();
        drive(Workflow::new(input)?, &mut LocalObserver { lookup })
    };
    run().map_err(Error::from)
}
