//! Acquisition of local invocation context and physical endpoint identities.
pub use tta_launcher::socket::*;

pub fn context() -> std::io::Result<Context> {
    Ok(Context {
        cwd: std::env::current_dir()?,
        uid: rustix::process::geteuid().as_raw(),
        tmux: std::env::var_os("TMUX"),
        tmpdir: std::env::var_os("TMUX_TMPDIR").filter(|value| !value.is_empty()).map(Into::into),
    })
}

pub fn client_mode(context: &Context, selected: &std::path::Path) -> Result<tta_launcher::ClientMode, Error> {
    let Some(current) = context.current_server() else {
        return Ok(tta_launcher::ClientMode::Attach);
    };
    let identity = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.to_owned());
    context.client_mode_with_identity(selected, &identity(&current), &identity(selected))
}
