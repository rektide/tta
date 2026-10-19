/// Case-sensitive literal matching. Exact names win; substring matches must be
/// unique. Return all ambiguous candidates, sorted independently of input order.
pub(crate) fn select<T>(mut candidates: Vec<T>, needle: &str, name: impl Fn(&T) -> &str) -> Result<Option<T>, Vec<T>> {
    candidates.retain(|candidate| name(candidate).contains(needle));
    candidates.sort_by(|left, right| name(left).cmp(name(right)));
    if candidates.iter().any(|candidate| name(candidate) == needle) {
        candidates.retain(|candidate| name(candidate) == needle);
    }
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.pop()),
        _ => Err(candidates),
    }
}

#[cfg(test)]
mod tests {
    use super::select;

    #[test]
    fn exact_then_unique_substring() {
        assert_eq!(select(vec!["foobar", "foo", "afoo"], "foo", |s| s), Ok(Some("foo")));
        assert_eq!(select(vec!["foobar", "other"], "foo", |s| s), Ok(Some("foobar")));
        assert_eq!(select(vec!["foobar"], "FOO", |s| s), Ok(None));
    }

    #[test]
    fn ambiguity_is_sorted_and_metacharacters_are_literal() {
        assert_eq!(select(vec!["zfoo", "afoo"], "foo", |s| s), Err(vec!["afoo", "zfoo"]));
        assert_eq!(select(vec!["foo", "x*foo"], "*", |s| s), Ok(Some("x*foo")));
    }
}
