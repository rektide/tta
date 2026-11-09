//! Bounded acquisition of ONE registry fact. This task knows no selectors,
//! generators, views, defaults or entry continuations.
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Component, PathBuf};

use anyhow::{Result, bail, ensure};

pub const TEXT_LIMIT: usize = 1024 * 1024;
const COMPONENT_LIMIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Step {
    Entry { parent: PathBuf, name: OsString },
    Content { path: PathBuf },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reply {
    Entry(Option<crate::target::Entry>),
    Content(String),
}

pub struct Task {
    path: Option<PathBuf>,
    parent: PathBuf,
    components: VecDeque<OsString>,
    pending: Option<Step>,
    text: Option<Option<String>>,
}

impl Task {
    pub fn new(path: Option<PathBuf>) -> Result<Self> {
        let mut task = Self {
            path,
            parent: "/".into(),
            components: VecDeque::new(),
            pending: None,
            text: None,
        };
        if let Some(path) = &task.path {
            ensure!(path.is_absolute(), "registry path must be frozen and absolute");
            for part in path.components() {
                match part {
                    Component::Normal(name) => task.components.push_back(name.into()),
                    Component::ParentDir => task.components.push_back("..".into()),
                    Component::RootDir | Component::CurDir => {}
                    _ => bail!("unsupported registry path: {path:?}"),
                }
            }
            ensure!(
                !task.components.is_empty() && task.components.back().is_none_or(|p| p != ".."),
                "registry path names a directory: {path:?}"
            );
            ensure!(task.components.len() <= COMPONENT_LIMIT, "registry component limit exceeded");
            task.next_component();
        } else {
            task.text = Some(None);
        }
        Ok(task)
    }

    fn next_component(&mut self) {
        while self.components.front().is_some_and(|p| p == "..") && self.components.len() > 1 {
            self.parent.push(self.components.pop_front().expect("front"));
        }
        self.pending = Some(Step::Entry {
            parent: self.parent.clone(),
            name: self.components.front().expect("component").clone(),
        });
    }

    pub fn step(&self) -> Option<&Step> {
        self.pending.as_ref()
    }
    pub fn result(&self) -> Option<&Option<String>> {
        self.text.as_ref()
    }

    pub fn accept(&mut self, step: &Step, reply: Reply) -> Result<()> {
        ensure!(self.pending.as_ref() == Some(step), "stale or mismatched registry acquisition step");
        match (step, reply) {
            (Step::Entry { .. }, Reply::Entry(entry)) => {
                let entry = entry.map(crate::target::Entry::checked).transpose()?;
                if entry.is_none_or(|entry| entry.dangling()) {
                    self.pending = None;
                    self.text = Some(None);
                    return Ok(());
                }
                let component = self.components.front().expect("component").clone();
                if self.components.len() == 1 {
                    self.components.pop_front();
                    self.pending = Some(Step::Content {
                        path: self.path.clone().expect("path"),
                    });
                } else {
                    ensure!(
                        entry.expect("present").directory(),
                        "registry path component is not a directory: {:?}",
                        self.parent.join(&component)
                    );
                    self.components.pop_front();
                    self.parent.push(component);
                    self.next_component();
                }
            }
            (Step::Content { .. }, Reply::Content(text)) => {
                ensure!(text.len() <= TEXT_LIMIT && !text.contains('\0'), "registry exceeds text limit or contains NUL");
                self.pending = None;
                self.text = Some(Some(text));
            }
            _ => bail!("wrong registry acquisition reply shape"),
        }
        Ok(())
    }
}
