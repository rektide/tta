//! Small, explicit CLI > environment > default layering. Endpoint selectors form
//! one family, so either explicit CLI selector suppresses both environment ones.
use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use clap::ValueEnum;

use crate::{Args, Backend};

fn value(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

pub(crate) fn configure(mut args: Args) -> Result<Args> {
    // A transport target is a distinct environment. Tracing and acquisition
    // limits are local policy; launch defaults follow target shell setup.
    if args.ssh.is_some() {
        if args.ssh_probe_timeout.is_none()
            && let Some(value) = value("TTA_SSH_PROBE_TIMEOUT")
        {
            args.ssh_probe_timeout = Some(
                value
                    .to_str()
                    .ok_or_else(|| anyhow!("TTA_SSH_PROBE_TIMEOUT must be UTF-8"))?
                    .parse()
                    .map_err(|error| anyhow!("TTA_SSH_PROBE_TIMEOUT must be positive integer milliseconds: {error}"))?,
            );
        }
        if args.ssh_probe_retention.is_none()
            && let Some(value) = value("TTA_SSH_PROBE_RETENTION")
        {
            args.ssh_probe_retention = Some(
                value
                    .to_str()
                    .ok_or_else(|| anyhow!("TTA_SSH_PROBE_RETENTION must be UTF-8"))?
                    .parse()
                    .map_err(|error| anyhow!("TTA_SSH_PROBE_RETENTION must be positive integer bytes: {error}"))?,
            );
        }
        configure_trace(&mut args)?;
        return Ok(args);
    }
    args.root = args.root.or_else(|| value("TTA_ROOT").map(PathBuf::from)).or_else(|| {
        let projects = value("PROJECTS_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("src"));
        if projects.is_absolute() {
            Some(projects)
        } else {
            value("HOME").map(|home| PathBuf::from(home).join(projects))
        }
    });
    if args.backend.is_none()
        && let Some(value) = value("TTA_BACKEND")
    {
        let text = value.to_str().ok_or_else(|| anyhow!("TTA_BACKEND must be UTF-8"))?;
        args.backend = Some(Backend::from_str(text, false).map_err(|error| anyhow!("TTA_BACKEND: {error}"))?);
    }
    if args.socket_name.is_none() && args.socket_path.is_none() {
        args.socket_name = value("TTA_SOCKET_NAME");
        args.socket_path = value("TTA_SOCKET_PATH").map(PathBuf::from);
        if args.socket_name.is_some() && args.socket_path.is_some() {
            bail!("TTA_SOCKET_NAME and TTA_SOCKET_PATH conflict; set only one or override with --socket-name/--socket-path");
        }
    }
    if args.scaffold.is_none()
        && let Some(value) = value("TTA_SCAFFOLD")
    {
        let text = value.to_str().ok_or_else(|| anyhow!("TTA_SCAFFOLD must be UTF-8"))?;
        tta_launcher::scaffold::Choice::parse(text).map_err(|error| anyhow!("TTA_SCAFFOLD: {error}"))?;
        args.scaffold = Some(text.to_owned());
    }
    args.tmux = args.tmux.or_else(|| value("TTA_TMUX"));
    args.tmux_config = args.tmux_config.or_else(|| value("TTA_TMUX_CONFIG").map(PathBuf::from));
    configure_trace(&mut args)?;
    Ok(args)
}

fn configure_trace(args: &mut Args) -> Result<()> {
    if args.trace.is_none()
        && let Some(value) = value("TTA_TRACE")
    {
        args.trace = Some(match value.to_str() {
            Some("true" | "1") => true,
            Some("false" | "0") => false,
            _ => bail!("TTA_TRACE must be true, false, 1, or 0"),
        });
    }
    Ok(())
}
