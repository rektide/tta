//! Pure ordered-root policy. Canonical identities are supplied observations,
//! never filesystem guesses made by the core.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::ResolveError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEntry {
    pub name: String,
    /// Observed canonical physical directory, including symlink resolution.
    pub directory: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectInventory {
    pub requested_root: PathBuf,
    /// Observed physical root; not necessarily the requested spelling.
    pub root: PathBuf,
    pub entries: Vec<ProjectEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSource {
    /// One-based slot in the configured list, including repeated roots.
    pub root_index: usize,
    pub requested_root: PathBuf,
    pub root: PathBuf,
    pub name: String,
    pub directory: PathBuf,
}

impl ProjectSource {
    pub fn selector(&self) -> String {
        format!("@{}/{}", self.root_index, self.name)
    }

    pub fn entry_path(&self) -> PathBuf {
        self.root.join(&self.name)
    }
}

/// One active layer replaces the entire list. Only PROJECTS_DIR is HOME-relative.
/// Empty environment values are unset; explicit empty lists/paths are errors.
pub fn configured_roots(
    explicit: Option<&[PathBuf]>,
    roots: Option<&str>,
    root: Option<&str>,
    projects_dir: Option<&str>,
    home: Option<&Path>,
    cwd: &Path,
) -> Result<Vec<PathBuf>, ResolveError> {
    fn nonempty(value: Option<&str>) -> Option<&str> {
        value.filter(|value| !value.is_empty())
    }
    let paths = if let Some(paths) = explicit {
        paths.to_vec()
    } else if let Some(json) = nonempty(roots) {
        serde_json::from_str::<Vec<PathBuf>>(json).map_err(|error| ResolveError::RootConfiguration(error.to_string()))?
    } else if let Some(root) = nonempty(root) {
        vec![root.into()]
    } else {
        let project = PathBuf::from(nonempty(projects_dir).unwrap_or("src"));
        return Ok(if project.is_absolute() {
            vec![project]
        } else {
            home.map(|home| vec![cwd.join(home).join(project)]).unwrap_or_default()
        });
    };
    if paths.is_empty() || paths.iter().any(|path| path.as_os_str().is_empty()) {
        return Err(ResolveError::RootConfiguration("provide a nonempty list of nonempty paths".into()));
    }
    Ok(paths.into_iter().map(|path| cwd.join(path)).collect())
}

pub(crate) fn component(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\0'])
}

pub(crate) struct Scope<'a> {
    pub fragment: &'a str,
    pub indices: Vec<usize>,
    pub qualified: bool,
}

/// Basename filter for an observation after the resolver has validated scope.
/// This does not itself validate a selector's root slot or authorize fallback.
pub fn observation_fragment(argument: &str) -> &str {
    if let Some((slot, fragment)) = argument.strip_prefix('@').and_then(|s| s.split_once('/'))
        && !slot.is_empty()
        && slot.bytes().all(|byte| byte.is_ascii_digit())
    {
        fragment
    } else {
        argument
    }
}

pub(crate) fn scope<'a>(argument: &'a str, roots: &[PathBuf]) -> Result<Scope<'a>, ResolveError> {
    if let Some((slot, fragment)) = argument.strip_prefix('@').and_then(|s| s.split_once('/'))
        && !slot.is_empty()
        && slot.bytes().all(|byte| byte.is_ascii_digit())
    {
        let index = slot.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).filter(|index| *index < roots.len());
        let Some(index) = index.filter(|_| component(fragment)) else {
            return Err(ResolveError::Selector {
                argument: argument.into(),
                roots: roots.len(),
            });
        };
        return Ok(Scope {
            fragment,
            indices: vec![index],
            qualified: true,
        });
    }
    if roots.is_empty() {
        return Err(ResolveError::MissingRoot);
    }
    Ok(Scope {
        fragment: argument,
        indices: (0..roots.len()).collect(),
        qualified: false,
    })
}

/// Validate each observation's request binding and physical facts. The order is
/// the request order, not filesystem enumeration order.
pub(crate) fn sources(roots: &[PathBuf], indices: &[usize], inventories: &[ProjectInventory]) -> Result<Vec<ProjectSource>, ResolveError> {
    if inventories.len() > indices.len() {
        return Err(ResolveError::InvalidInventory);
    }
    let mut result = Vec::new();
    for (&index, inventory) in indices.iter().zip(inventories) {
        let mut names = BTreeSet::new();
        if inventory.requested_root != roots[index]
            || !inventory.requested_root.is_absolute()
            || !inventory.root.is_absolute()
            || inventory
                .entries
                .iter()
                .any(|entry| !component(&entry.name) || !entry.directory.is_absolute() || !names.insert(&entry.name))
        {
            return Err(ResolveError::InvalidInventory);
        }
        let mut entries: Vec<_> = inventory.entries.iter().collect();
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        result.extend(entries.into_iter().map(|entry| ProjectSource {
            root_index: index + 1,
            requested_root: inventory.requested_root.clone(),
            root: inventory.root.clone(),
            name: entry.name.clone(),
            directory: entry.directory.clone(),
        }));
    }
    Ok(result)
}

pub(crate) fn select<'a>(sources: &'a [ProjectSource], argument: &str, fragment: &str) -> Result<Option<&'a ProjectSource>, ResolveError> {
    let mut candidates = match crate::matching::select(sources.iter().collect(), fragment, |source| &source.name) {
        Ok(None) => return Ok(None),
        Ok(Some(source)) => vec![source],
        Err(candidates) => candidates,
    };
    candidates.sort_by(|left, right| left.root_index.cmp(&right.root_index).then(left.name.cmp(&right.name)));
    let mut physical = BTreeSet::new();
    let winners: Vec<_> = candidates.iter().filter(|source| physical.insert(&source.directory)).copied().collect();
    if winners.len() > 1 {
        return Err(ResolveError::Projects {
            argument: argument.into(),
            candidates: candidates.iter().map(|source| source.selector()).collect(),
        });
    }
    Ok(winners.first().copied())
}

/// Completion never picks a launch winner. Qualified alternatives are always
/// exposed; a bare basename is offered only for one physical exact-name winner.
pub fn completion_candidates(roots: &[PathBuf], inventories: &[ProjectInventory]) -> Result<Vec<String>, ResolveError> {
    if roots.len() != inventories.len() {
        return Err(ResolveError::InvalidInventory);
    }
    let sources = sources(roots, &(0..roots.len()).collect::<Vec<_>>(), inventories)?;
    let mut identities: BTreeMap<&str, BTreeSet<&Path>> = BTreeMap::new();
    for source in &sources {
        identities.entry(&source.name).or_default().insert(&source.directory);
    }
    let mut candidates = BTreeSet::new();
    for source in &sources {
        candidates.insert(source.selector());
        if identities[source.name.as_str()].len() == 1 {
            candidates.insert(source.name.clone());
        }
    }
    Ok(candidates.into_iter().collect())
}
