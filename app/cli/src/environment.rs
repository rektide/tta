//! Local CLI-value collection only. Whole-layer environment precedence,
//! conflict detection and default selection belong to the shared entry
//! context policy; the CLI never pre-expands an environment into arguments.
use std::ffi::OsString;

use anyhow::{Result, anyhow, bail};
use clap::ValueEnum;

use crate::{Args, Backend};

fn value(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

pub(crate) fn configure(mut args: Args) -> Result<Args> {
    // A transport target is a distinct environment: acquisition limits and
    // tracing stay local; launch defaults come from target facts in policy.
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
    if args.backend.is_none()
        && let Some(value) = value("TTA_BACKEND")
    {
        let text = value.to_str().ok_or_else(|| anyhow!("TTA_BACKEND must be UTF-8"))?;
        args.backend = Some(Backend::from_str(text, false).map_err(|error| anyhow!("TTA_BACKEND: {error}"))?);
    }
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
