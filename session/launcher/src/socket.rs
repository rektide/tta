//! Freeze socket selection before a plan changes cwd, so native lookup, CLI
//! lookup and the final client all address exactly the same endpoint.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use bon::Builder;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Selection {
    #[default]
    Default,
    Name(OsString),
    Path(PathBuf),
}

#[derive(Clone, Debug, Builder)]
pub struct Context {
    pub cwd: PathBuf,
    pub uid: u32,
    pub tmux: Option<OsString>,
    pub tmpdir: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("socket name must be nonempty, not '.' or '..', and contain neither '/' nor NUL")]
    InvalidName,
    #[error("socket path must be nonempty and contain no NUL")]
    InvalidPath,
    #[error(
        "cannot switch a tmux client between servers ({current:?} -> {selected:?}); run outside tmux, or unset TMUX and TMUX_PANE explicitly to attach a separate client"
    )]
    DifferentServer { current: PathBuf, selected: PathBuf },
}

impl Context {
    #[cfg(unix)]
    pub fn from_environment() -> std::io::Result<Self> {
        Ok(Self {
            cwd: std::env::current_dir()?,
            uid: rustix::process::geteuid().as_raw(),
            tmux: std::env::var_os("TMUX"),
            tmpdir: std::env::var_os("TMUX_TMPDIR").filter(|value| !value.is_empty()).map(PathBuf::from),
        })
    }

    pub fn resolve(&self, selection: &Selection) -> Result<PathBuf, Error> {
        let name = match selection {
            Selection::Path(path) => {
                if path.as_os_str().is_empty() || path.as_os_str().as_encoded_bytes().contains(&0) {
                    return Err(Error::InvalidPath);
                }
                return Ok(self.absolute(path));
            }
            Selection::Default => {
                if let Some(path) = self.current_server() {
                    return Ok(path);
                }
                std::ffi::OsStr::new("default")
            }
            Selection::Name(name) => {
                if name.is_empty() || name == "." || name == ".." || name.as_encoded_bytes().contains(&b'/') || name.as_encoded_bytes().contains(&0) {
                    return Err(Error::InvalidName);
                }
                name
            }
        };
        let tmpdir = self.tmpdir.as_deref().unwrap_or_else(|| Path::new("/tmp"));
        Ok(self.absolute(tmpdir).join(format!("tmux-{}", self.uid)).join(name))
    }

    pub fn client_mode(&self, selected: &Path) -> Result<crate::ClientMode, Error> {
        let Some(current) = self.current_server() else {
            return Ok(crate::ClientMode::Attach);
        };
        let identity = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_owned());
        if identity(&current) != identity(selected) {
            return Err(Error::DifferentServer {
                current,
                selected: selected.to_owned(),
            });
        }
        Ok(crate::ClientMode::Switch)
    }

    pub fn current_server(&self) -> Option<PathBuf> {
        let tmux = self.tmux.as_ref()?;
        #[cfg(unix)]
        {
            use std::os::unix::ffi::{OsStrExt, OsStringExt};
            let prefix = tmux.as_bytes().split(|byte| *byte == b',').next()?;
            if prefix.is_empty() {
                return None;
            }
            Some(self.absolute(Path::new(&OsString::from_vec(prefix.to_vec()))))
        }
        #[cfg(not(unix))]
        {
            let prefix = tmux.to_str()?.split(',').next()?;
            if prefix.is_empty() {
                return None;
            }
            Some(self.absolute(Path::new(prefix)))
        }
    }

    fn absolute(&self, path: &Path) -> PathBuf {
        self.cwd.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context::builder()
            .cwd("/home/me".into())
            .uid(1000)
            .tmux("/tmp/old,123,4".into())
            .tmpdir("/private/tmp".into())
            .build()
    }

    #[test]
    fn selected_socket_is_frozen_and_tmux_precedence_is_explicit() {
        let context = context();
        assert_eq!(context.resolve(&Selection::Default).unwrap(), Path::new("/tmp/old"));
        assert_eq!(
            context.resolve(&Selection::Name("work".into())).unwrap(),
            Path::new("/private/tmp/tmux-1000/work")
        );
        assert_eq!(
            context.resolve(&Selection::Path("relative/socket".into())).unwrap(),
            Path::new("/home/me/relative/socket")
        );
        assert_eq!(context.client_mode(Path::new("/tmp/old")).unwrap(), crate::ClientMode::Switch);
        assert!(matches!(context.client_mode(Path::new("/tmp/new")), Err(Error::DifferentServer { .. })));
    }

    #[test]
    fn empty_inherited_tmux_uses_default_label_and_outside_mode() {
        let mut context = context();
        context.tmux = Some(",123,4".into());
        assert_eq!(context.resolve(&Selection::Default).unwrap(), Path::new("/private/tmp/tmux-1000/default"));
        assert_eq!(context.client_mode(Path::new("/tmp/new")).unwrap(), crate::ClientMode::Attach);
        assert!(context.resolve(&Selection::Name("../default".into())).is_err());
    }
}
