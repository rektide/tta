//! Small, explicit CLI > environment > default layering. Endpoint selectors form
//! one family, so either explicit CLI selector suppresses both environment ones.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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
    // One active layer replaces the whole ordered root list: repeated --root >
    // TTA_ROOTS (JSON array) > TTA_ROOT > HOME-relative PROJECTS_DIR > ~/src.
    // Explicit roots suppress lower layers entirely, so an invalid TTA_ROOTS
    // value is never even parsed. TTA_ROOTS itself must be UTF-8 JSON; the
    // singular layers keep raw path bytes. Relative entries freeze against the
    // invocation cwd. SSH keeps explicit roots only: its defaults are
    // target-local and are layered from target facts.
    let roots_json = match std::env::var("TTA_ROOTS") {
        Ok(json) => Some(json),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => bail!("TTA_ROOTS must be a UTF-8 JSON array of paths"),
    };
    let explicit = (!args.root.is_empty()).then(|| args.root.clone());
    args.root = tta_launcher::projects::configured_roots(
        explicit.as_deref(),
        roots_json.as_deref(),
        value("TTA_ROOT").as_deref(),
        value("PROJECTS_DIR").as_deref(),
        value("HOME").as_deref().map(Path::new),
        &std::env::current_dir()?,
    )?;
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
    if args.satellite_prefix.is_none()
        && let Some(value) = value("TTA_SATELLITE_PREFIX")
    {
        args.satellite_prefix = Some(value.into_string().map_err(|_| anyhow!("TTA_SATELLITE_PREFIX must be UTF-8"))?);
    }
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
