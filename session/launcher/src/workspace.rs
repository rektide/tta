//! Workspace aliases are navigation aids, not evidence of group membership.
use crate::Session;

pub fn validate_satellite_prefix(prefix: &str) -> Result<(), crate::ResolveError> {
    if prefix.is_empty() || prefix.contains(['.', ':']) || prefix.chars().any(char::is_control) {
        return Err(crate::ResolveError::InvalidSatellitePrefix(prefix.into()));
    }
    Ok(())
}

/// Strip exactly one configured prefix and one numeric suffix. Non-satellites
/// keep their literal names; arbitrary internal hyphens/digits are preserved.
pub fn satellite_alias<'a>(name: &'a str, prefix: &str) -> &'a str {
    name.strip_prefix(prefix)
        .and_then(|rest| rest.rsplit_once('-'))
        .filter(|(base, suffix)| !base.is_empty() && !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
        .map_or(name, |(base, _)| base)
}

/// Completion includes literal selectors and usable workspace aliases, even if
/// the original session no longer exists. Resolution still checks group facts.
pub fn session_names(sessions: &[Session], prefix: &str) -> Vec<String> {
    let mut names: Vec<_> = sessions
        .iter()
        .flat_map(|s| [s.name.clone(), satellite_alias(&s.name, prefix).to_owned()])
        .collect();
    names.sort();
    names.dedup();
    names
}

pub(crate) fn select(sessions: &[Session], needle: &str, prefix: &str) -> Result<Option<Session>, Vec<Session>> {
    // A literal full selector always identifies that session, even if another
    // workspace has an identical alias. This also makes ambiguity actionable.
    if let Some(session) = sessions.iter().find(|s| s.name == needle) {
        return Ok(Some(session.clone()));
    }
    let mut candidates: Vec<_> = sessions
        .iter()
        .filter_map(|session| {
            [&*session.name, satellite_alias(&session.name, prefix)]
                .into_iter()
                .filter(|name| name.contains(needle))
                .map(|name| name.chars().count())
                .min()
                .map(|length| (length, session.clone()))
        })
        .collect();
    if let Some(shortest) = candidates.iter().map(|(length, _)| *length).min() {
        candidates.retain(|(length, _)| *length == shortest);
    }
    candidates.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    let mut workspaces: Vec<Session> = Vec::new();
    for (_, session) in candidates {
        if !workspaces
            .iter()
            .any(|other| session.id == other.id || (session.group.is_some() && session.group == other.group))
        {
            workspaces.push(session);
        }
    }
    match workspaces.len() {
        0 => Ok(None),
        1 => Ok(workspaces.pop()),
        _ => Err(workspaces),
    }
}

/// Choose the smallest free name globally. A real group's non-satellite member
/// supplies the base when available; orphan satellites supply their alias. Do
/// not recursively decorate a satellite selected by name or by substring.
pub fn satellite_name(selected: &Session, existing: &[Session], prefix: &str) -> String {
    let base = existing
        .iter()
        .filter(|s| s.id == selected.id || (selected.group.is_some() && s.group == selected.group))
        .filter(|s| satellite_alias(&s.name, prefix) == s.name)
        .min_by_key(|s| (s.name.chars().count(), &s.name))
        .map_or_else(|| satellite_alias(&selected.name, prefix), |s| s.name.as_str());
    let stem = format!("{prefix}{base}-");
    let mut counter = 1_u64;
    loop {
        let name = format!("{stem}{counter}");
        if !existing.iter().any(|s| s.name == name) {
            return name;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: u32, name: &str, group: Option<&str>) -> Session {
        Session {
            id: format!("${id}").parse().unwrap(),
            name: name.into(),
            group: group.map(str::to_owned),
        }
    }

    #[test]
    fn aliases_strip_one_prefix_and_one_ascii_numeric_suffix() {
        for (raw, alias) in [
            ("_project-42", "project"),
            ("_project-7-42", "project-7"),
            ("_project-0", "project"),
            ("_project-01", "project"),
            ("__project-1-2", "_project-1"),
            ("_project-x", "_project-x"),
            ("project-42", "project-42"),
            ("_project-٢", "_project-٢"),
            ("_-1", "_-1"),
            ("_project", "_project"),
        ] {
            assert_eq!(satellite_alias(raw, "_"), alias);
        }
        assert_eq!(satellite_alias("view-project-42", "view-"), "project");
        assert_eq!(satellite_alias("_project-42", "view-"), "_project-42");
        for prefix in ["", ".", ":", "\n"] {
            assert!(validate_satellite_prefix(prefix).is_err());
        }
        for prefix in ["_", "view-", "#", "雪 "] {
            assert!(validate_satellite_prefix(prefix).is_ok());
        }
    }

    #[test]
    fn literal_exact_then_alias_exact_then_shortest_with_real_group_ties() {
        let sessions = vec![
            session(1, "_project-42", Some("work")),
            session(2, "_project-7", Some("work")),
            session(3, "project-extra", None),
        ];
        assert_eq!(select(&sessions, "project", "_").unwrap().unwrap().id, sessions[0].id);
        assert_eq!(select(&sessions, "_project-7", "_").unwrap(), Some(sessions[1].clone()));
        assert_eq!(select(&sessions, "proj", "_").unwrap().unwrap().id, sessions[0].id);
        assert!(select(&sessions, "PROJECT", "_").unwrap().is_none());
        let unrelated = vec![sessions[0].clone(), session(4, "_project-8", Some("other"))];
        assert_eq!(select(&unrelated, "project", "_").unwrap_err().len(), 2);
        let ungrouped = vec![session(1, "_project-1", None), session(2, "_project-2", None)];
        assert_eq!(select(&ungrouped, "project", "_").unwrap_err().len(), 2);
        let exact = vec![sessions[0].clone(), session(9, "project", None)];
        assert_eq!(select(&exact, "project", "_").unwrap(), Some(exact[1].clone()));
        let shorter = vec![sessions[0].clone(), session(10, "proj", None)];
        assert_eq!(select(&shorter, "pro", "_").unwrap(), Some(shorter[1].clone()));
        let tied = vec![session(1, "_afoo-1", Some("a")), session(2, "zfoo", None), session(3, "longfoo", None)];
        assert_eq!(select(&tied, "foo", "_").unwrap_err(), tied[..2]);
    }

    #[test]
    fn orphan_completion_and_allocation_share_custom_aliases_without_recursion() {
        let orphan = session(42, "view-project-42", Some("project"));
        assert_eq!(session_names(std::slice::from_ref(&orphan), "view-"), ["project", "view-project-42"]);
        assert_eq!(satellite_name(&orphan, std::slice::from_ref(&orphan), "view-"), "view-project-1");
        let existing = vec![
            orphan.clone(),
            session(1, "view-project-1", Some("unrelated")),
            session(2, "view-project-3", Some("project")),
        ];
        assert_eq!(satellite_name(&orphan, &existing, "view-"), "view-project-2");
        let leading_zero = vec![orphan.clone(), session(1, "view-project-01", None)];
        assert_eq!(satellite_name(&orphan, &leading_zero, "view-"), "view-project-1");
        let renamed_base = vec![orphan.clone(), session(9, "renamed", Some("project")), session(10, "p", Some("unrelated"))];
        assert_eq!(satellite_name(&orphan, &renamed_base, "view-"), "view-renamed-1");
        assert_eq!(select(std::slice::from_ref(&orphan), "project", "view-").unwrap(), Some(orphan));
    }
}
