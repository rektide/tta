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

/// Exact lookup path plus client startup policy. A label remains a label so
/// tmux creates/checks its private socket directory on a cold system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub path: PathBuf,
    pub label: Option<Label>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Label {
    pub name: OsString,
    pub directory: PathBuf,
}

impl Resolved {
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            label: None,
        }
    }
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
    pub fn resolve(&self, selection: &Selection) -> Result<Resolved, Error> {
        let name = match selection {
            Selection::Path(path) => {
                if path.as_os_str().is_empty() || path.as_os_str().as_encoded_bytes().contains(&0) {
                    return Err(Error::InvalidPath);
                }
                return Ok(Resolved::path(self.absolute(path)));
            }
            Selection::Default => {
                if let Some(path) = self.current_server() {
                    return Ok(Resolved::path(path));
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
        let directory = self.absolute(tmpdir);
        Ok(Resolved {
            path: directory.join(format!("tmux-{}", self.uid)).join(name),
            label: Some(Label {
                name: name.to_owned(),
                directory,
            }),
        })
    }

    pub fn client_mode(&self, selected: &Path) -> Result<crate::ClientMode, Error> {
        let Some(current) = self.current_server() else {
            return Ok(crate::ClientMode::Attach);
        };
        self.client_mode_with_identity(selected, &current, selected)
    }

    /// Compare supplied physical identities. Observers may canonicalize existing
    /// paths; the core must never canonicalize a target path on the local host.
    pub fn client_mode_with_identity(&self, selected: &Path, current_identity: &Path, selected_identity: &Path) -> Result<crate::ClientMode, Error> {
        let Some(current) = self.current_server() else {
            return Ok(crate::ClientMode::Attach);
        };
        if current_identity != selected_identity {
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
        assert_eq!(context.resolve(&Selection::Default).unwrap().path, Path::new("/tmp/old"));
        assert_eq!(
            context.resolve(&Selection::Name("work".into())).unwrap().path,
            Path::new("/private/tmp/tmux-1000/work")
        );
        assert_eq!(
            context.resolve(&Selection::Path("relative/socket".into())).unwrap().path,
            Path::new("/home/me/relative/socket")
        );
        assert_eq!(context.client_mode(Path::new("/tmp/old")).unwrap(), crate::ClientMode::Switch);
        assert!(matches!(context.client_mode(Path::new("/tmp/new")), Err(Error::DifferentServer { .. })));
    }

    #[test]
    fn empty_inherited_tmux_uses_default_label_and_outside_mode() {
        let mut context = context();
        context.tmux = Some(",123,4".into());
        assert_eq!(context.resolve(&Selection::Default).unwrap().path, Path::new("/private/tmp/tmux-1000/default"));
        assert_eq!(context.client_mode(Path::new("/tmp/new")).unwrap(), crate::ClientMode::Attach);
        assert!(context.resolve(&Selection::Name("../default".into())).is_err());
    }
}
