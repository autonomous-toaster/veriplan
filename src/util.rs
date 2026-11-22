/// Truncate a string to at most `max` bytes, ending on a UTF-8 char boundary.
///
/// If the string is already short enough, returns it unchanged.
/// Otherwise, finds the last complete char whose start byte is < `max - 1`
/// (to leave room for the ellipsis `…`) and appends the ellipsis.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let idx = s
        .char_indices()
        .take_while(|(i, _)| *i < max.saturating_sub(1))
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}…", &s[..idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_ascii() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_ascii() {
        let t = truncate("hello world", 5);
        assert_eq!(t, "hell…");
    }

    #[test]
    fn test_truncate_multi_byte_at_boundary() {
        // → is 3 bytes (U+2192). A 15-byte limit with room for ellipsis
        // should not panic and produce a valid string.
        let s = "café au lait → chaud";
        let t = truncate(s, 15);
        // Should end in … and have length ≤ 15 + 3 (one extra char margin)
        assert!(
            t.ends_with('…'),
            "truncated string should end with ellipsis: {t}"
        );
        assert!(
            t.len() <= s.len(),
            "truncated string should not be longer than original"
        );
    }

    #[test]
    fn test_truncate_exact_fit() {
        assert_eq!(truncate("1234567890", 10), "1234567890");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_truncate_zero_max() {
        let t = truncate("hello", 0);
        assert_eq!(t, "…");
    }
}

/// Every whole-identifier task id in a statement, in order of appearance.
pub fn task_ids_in(statement: &str) -> Vec<String> {
    let bytes = statement.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if (bytes[i] == b'T' || bytes[i] == b't')
            && bytes[i + 1].is_ascii_digit()
            && !(i > 0 && bytes[i - 1].is_ascii_alphanumeric())
        {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j < bytes.len()
                && bytes[j] == b'.'
                && j + 1 < bytes.len()
                && bytes[j + 1].is_ascii_digit()
            {
                let mut k = j + 1;
                while k < bytes.len() && bytes[k].is_ascii_digit() {
                    k += 1;
                }
                if k == bytes.len() || (!bytes[k].is_ascii_digit() && bytes[k] != b'.') {
                    let id = statement[i + 1..k].to_string();
                    if !out.contains(&id) {
                        out.push(id);
                    }
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Whether a statement mentions a task id as a whole identifier.
///
/// A plain substring test makes T1.1 match T1.10, so a body citing only the tenth
/// task reports the first, and the LTL atoms can be built from the wrong task.
pub fn mentions_task(statement: &str, task_id: &str) -> bool {
    for pattern in [format!("T{task_id}"), format!("t{task_id}")] {
        let mut from = 0usize;
        while let Some(at) = statement[from..].find(&pattern) {
            let start = from + at;
            let end = start + pattern.len();
            let before_ok = statement[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '.');
            let after_ok = statement[end..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_digit() && c != '.');
            if before_ok && after_ok {
                return true;
            }
            from = end;
        }
    }
    false
}

/// Position of the first whole-identifier mention of a task id, if any.
pub fn find_task(statement: &str, task_id: &str) -> Option<usize> {
    for pattern in [format!("T{task_id}"), format!("t{task_id}")] {
        let mut from = 0usize;
        while let Some(at) = statement[from..].find(&pattern) {
            let start = from + at;
            let end = start + pattern.len();
            let before_ok = statement[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '.');
            let after_ok = statement[end..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_digit() && c != '.');
            if before_ok && after_ok {
                return Some(start);
            }
            from = end;
        }
    }
    None
}

#[cfg(test)]
mod boundary_tests {
    #[test]
    fn task_ids_match_on_identifier_boundaries() {
        // A plain substring test made T1.1 match inside T1.10, so a body citing only the
        // tenth task reported the first, and the LTL atoms could be built from the wrong task.
        assert!(super::mentions_task("T1.10 SHALL run", "1.10"));
        assert!(!super::mentions_task("T1.10 SHALL run", "1.1"));
        assert!(!super::mentions_task("T1.1 SHALL run", "1.10"));
        assert!(super::mentions_task("T1.1 SHALL run", "1.1"));
        // A sentence-final id, and one followed by punctuation, still match.
        assert!(super::mentions_task("T2.3 SHALL run.", "2.3"));
        assert!(super::mentions_task("(T2.3)", "2.3"));
        // A decimal in prose is not a task id.
        assert!(!super::mentions_task("worth 1.10 of the budget", "1.10"));
    }
}
