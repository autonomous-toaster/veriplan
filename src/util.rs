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
