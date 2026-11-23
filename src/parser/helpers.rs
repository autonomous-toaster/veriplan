//! Parser helper functions.

use std::path::Path;

/// Extract the first paragraph from a requirement body for classification.
///
/// Returns only the text before the first blank line or `####` heading.
/// Body paragraphs after the first are excluded from classification
/// but remain in the requirement's source text for documentation.
pub fn extract_shall_statement(body: &str, _file: &str) -> String {
    body.lines()
        .take_while(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("####")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// A file path relative to the change directory, used as a finding pointer.
///
/// A basename is not an address: every spec delta in a change is named `spec.md`, so
/// `spec.md:24` cannot be resolved. The basename remains the fallback for input that has no
/// change directory (single file and stdin).
pub fn display_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .ok()
        .and_then(|p| p.to_str())
        .map(|p| p.to_string())
        .unwrap_or_else(|| file_name_str(path))
}

/// Split a step line into its kind and its text, with or without a leading list marker.
fn step_parts(text: &str) -> Option<(crate::ir::StepKind, &str)> {
    let text = text.trim();
    let text = text.strip_prefix('-').unwrap_or(text).trim();
    for (marker, kind) in [
        ("**WHEN**", crate::ir::StepKind::When),
        ("**when**", crate::ir::StepKind::When),
        ("**THEN**", crate::ir::StepKind::Then),
        ("**then**", crate::ir::StepKind::Then),
        ("**GIVEN**", crate::ir::StepKind::Given),
        ("**given**", crate::ir::StepKind::Given),
        ("**AND**", crate::ir::StepKind::And),
        ("**and**", crate::ir::StepKind::And),
    ] {
        if let Some(rest) = text.strip_prefix(marker) {
            return Some((kind, rest.trim()));
        }
    }
    None
}

/// Parse a single scenario step.
pub fn parse_step(text: &str, file: &str, line: usize) -> Option<crate::ir::ScenarioStep> {
    let (kind, step_text) = step_parts(text)?;
    Some(crate::ir::ScenarioStep {
        kind,
        text: step_text.to_string(),
        source: crate::ir::SourceLocation {
            file: file.to_string(),
            start_byte: 0,
            end_byte: 0,
            start_line: line,
            end_line: line,
        },
    })
}

/// Parse one `#### Scenario:` section node, with file-absolute step spans.
///
/// The span comes from the step's own node. The requirement body is a concatenation of
/// sibling blocks, so an index into the joined text does not address a line in the file;
/// arithmetic over the join drifted by one line for each preceding scenario.
pub fn parse_scenario_section(
    node: &tree_sitter::Node,
    source: &str,
    bytes: &[u8],
    file: &str,
) -> Option<crate::ir::Scenario> {
    let heading = find_child(node, "atx_heading")?;
    let heading_text = node_text(&heading, bytes).ok()?.trim();
    let name = heading_text
        .strip_prefix("#### Scenario:")
        .or_else(|| heading_text.strip_prefix("#### scenario:"))
        .unwrap_or("")
        .trim()
        .to_string();

    let mut steps = Vec::new();
    collect_steps(node, source, bytes, file, &mut steps);

    if steps.is_empty() {
        return None;
    }

    Some(crate::ir::Scenario {
        name,
        steps,
        source: crate::ir::SourceLocation {
            file: file.to_string(),
            start_byte: node.byte_range().start,
            end_byte: node.byte_range().end,
            start_line: find_line_for_byte(source, node.byte_range().start),
            end_line: find_line_for_byte(source, node.byte_range().end),
        },
    })
}

/// Collect the step list items of a scenario section, each with its file span.
fn collect_steps(
    node: &tree_sitter::Node,
    source: &str,
    bytes: &[u8],
    file: &str,
    steps: &mut Vec<crate::ir::ScenarioStep>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "atx_heading" {
            continue;
        }
        if child.kind() == "list_item" {
            if let Some(paragraph) = find_child(&child, "paragraph")
                && let Ok(text) = node_text(&paragraph, bytes)
                && let Some((kind, step_text)) = step_parts(text)
            {
                // The span covers the step text, not the list marker: a prose finding
                // addresses the sentence a reader has to rewrite.
                let offset = text.find(step_text).unwrap_or(0);
                let start = paragraph.byte_range().start + offset;
                steps.push(crate::ir::ScenarioStep {
                    kind,
                    text: step_text.to_string(),
                    source: crate::ir::SourceLocation {
                        file: file.to_string(),
                        start_byte: start,
                        end_byte: start + step_text.len(),
                        start_line: find_line_for_byte(source, start),
                        end_line: find_line_for_byte(source, start + step_text.len()),
                    },
                });
            }
            continue;
        }
        collect_steps(&child, source, bytes, file, steps);
    }
}

/// Detect RFC 2119 keyword strength in a statement.
pub fn detect_rfc2119(statement: &str) -> crate::ir::Rfc2119Strength {
    let upper = statement.to_uppercase();
    if upper.contains("MUST NOT") || upper.contains("SHALL NOT") {
        crate::ir::Rfc2119Strength::MustNot
    } else if upper.contains("MUST") || upper.contains("SHALL") {
        crate::ir::Rfc2119Strength::Must
    } else if upper.contains("SHOULD") {
        crate::ir::Rfc2119Strength::Should
    } else if upper.contains("MAY") {
        crate::ir::Rfc2119Strength::May
    } else {
        crate::ir::Rfc2119Strength::None
    }
}

/// Extract task ID from text.
/// Handles bare N.M from checklist items: "1.3 Add deps" → ("1.3", "1.3 Add deps")
/// Handles T-prefixed from SHALL statements: "T1.3 SHALL..." → ("1.3", "1.3")
pub fn extract_task_id(text: &str) -> (String, String) {
    let bytes = text.as_bytes();

    // Try N.M pattern first (bare, no T prefix — checklist items)
    // e.g. "1.3 Add dependencies" → id="1.3"
    if let Some(space_pos) = text.find(' ') {
        let candidate = &text[..space_pos];
        if let Some(dot_pos) = candidate.find('.') {
            let left = &candidate[..dot_pos];
            let right = &candidate[dot_pos + 1..];
            if !left.is_empty()
                && !right.is_empty()
                && left.chars().all(|c| c.is_ascii_digit())
                && right.chars().all(|c| c.is_ascii_digit())
            {
                return (candidate.to_string(), text.to_string());
            }
        }
    }

    // Try T-prefixed next: e.g. "T1.3" → id="1.3" (from SHALL refs)
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'T' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            i += 1;
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i]) {
                return (s.to_string(), text[start..i].to_string());
            }
        }
        i += 1;
    }

    (String::new(), String::new())
}

/// Get file name as string.
pub fn file_name_str(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Get text from a tree-sitter node.
pub fn node_text<'a>(
    node: &tree_sitter::Node,
    source: &'a [u8],
) -> Result<&'a str, std::str::Utf8Error> {
    std::str::from_utf8(&source[node.byte_range()])
}

/// Find child node by kind.
pub fn find_child<'a>(node: &tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|&child| child.kind() == kind)
}

/// Find the 1-based line number of a byte offset.
///
/// Newlines are counted, not `lines()`: `lines()` counts the partial line a mid-line
/// offset sits in, so an offset inside a line reported the following line. A paragraph node
/// starts after its list marker, which is exactly that case.
pub fn find_line_for_byte(source: &str, byte: usize) -> usize {
    let end = byte.min(source.len());
    source[..end].matches('\n').count() + 1
}

/// Explore a tree-sitter node (debug helper).
pub fn explore_node<'a>(node: &tree_sitter::Node<'a>, source: &'a [u8], indent: usize) {
    let text = node_text(node, source).unwrap_or("<invalid utf8>");
    println!("{}{}: {:?}", "  ".repeat(indent), node.kind(), text);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        explore_node(&child, source, indent + 1);
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; the crate denies these three lints for code that ships.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::ir::StepKind;

    #[test]
    fn test_parse_step_when() {
        let step = parse_step("- **WHEN** T3.2 runs", "spec.md", 10);
        assert!(step.is_some());
        let step = step.unwrap();
        assert_eq!(step.kind, StepKind::When);
        assert!(step.text.contains("T3.2 runs"));
    }

    #[test]
    fn test_parse_step_then() {
        let step = parse_step("- **THEN** T3.2 SHALL complete", "spec.md", 11);
        assert!(step.is_some());
        let step = step.unwrap();
        assert_eq!(step.kind, StepKind::Then);
    }

    #[test]
    fn test_parse_step_given() {
        let step = parse_step("- **GIVEN** a PlanIR with tasks", "spec.md", 9);
        assert!(step.is_some());
        let step = step.unwrap();
        assert_eq!(step.kind, StepKind::Given);
    }

    #[test]
    fn test_parse_step_and() {
        let step = parse_step("- **AND** T3.2 SHALL also run", "spec.md", 12);
        assert!(step.is_some());
        let step = step.unwrap();
        assert_eq!(step.kind, StepKind::And);
    }

    #[test]
    fn test_parse_step_invalid() {
        let step = parse_step("not a step", "spec.md", 13);
        assert!(step.is_none());
    }

    #[test]
    fn test_extract_shall_statement() {
        let result =
            extract_shall_statement("T1.1 SHALL complete BEFORE T1.2 SHALL run.", "spec.md");
        assert!(result.contains("SHALL"));
        assert!(result.contains("BEFORE"));
    }

    #[test]
    fn test_extract_shall_statement_no_shall() {
        let result = extract_shall_statement("Just some text without keywords.", "spec.md");
        assert_eq!(result, "Just some text without keywords.");
    }

    #[test]
    fn test_detect_rfc2119_must() {
        let strength = detect_rfc2119("The system MUST handle errors");
        assert_eq!(strength, crate::ir::Rfc2119Strength::Must);
    }

    #[test]
    fn test_detect_rfc2119_should() {
        let strength = detect_rfc2119("The system SHOULD retry");
        assert_eq!(strength, crate::ir::Rfc2119Strength::Should);
    }

    #[test]
    fn test_detect_rfc2119_may() {
        let strength = detect_rfc2119("The system MAY cache results");
        assert_eq!(strength, crate::ir::Rfc2119Strength::May);
    }

    #[test]
    fn test_detect_rfc2119_none() {
        let strength = detect_rfc2119("The system does something");
        assert_eq!(strength, crate::ir::Rfc2119Strength::None);
    }

    /// A step arrives from a list item paragraph, without the list marker. The step kind
    /// detection therefore runs on the paragraph text as well as on a whole line.
    #[test]
    fn test_step_parts_without_marker() {
        let (kind, text) = step_parts("**THEN** T1.2 SHALL follow").unwrap();
        assert_eq!(kind, StepKind::Then);
        assert_eq!(text, "T1.2 SHALL follow");
        assert!(step_parts("plain prose").is_none());
    }

    #[test]
    fn test_extract_shall_statement_simple() {
        let result = extract_shall_statement("T1.1 SHALL complete BEFORE T1.2.", "spec.md");
        assert!(result.contains("SHALL"));
        assert!(result.contains("BEFORE"));
    }

    #[test]
    fn test_extract_shall_statement_no_shall_2() {
        let result = extract_shall_statement("No keyword here", "spec.md");
        assert_eq!(result, "No keyword here");
    }

    #[test]
    fn test_extract_shall_statement_multi_paragraph() {
        let body = "T1.1 SHALL complete BEFORE T1.2 runs.\n\nThe system SHALL accept DATABASE_HOST as an alternative.";
        let result = extract_shall_statement(body, "spec.md");
        assert_eq!(result, "T1.1 SHALL complete BEFORE T1.2 runs.");
        assert!(!result.contains("DATABASE_HOST"));
    }

    #[test]
    fn test_extract_shall_statement_stops_at_scenario() {
        let body = "T1.1 SHALL complete BEFORE T1.2 runs.\n#### Scenario: Test\n- **WHEN** x\n- **THEN** y";
        let result = extract_shall_statement(body, "spec.md");
        assert_eq!(result, "T1.1 SHALL complete BEFORE T1.2 runs.");
        assert!(!result.contains("Scenario"));
    }

    #[test]
    fn test_extract_shall_statement_preserves_newlines_in_paragraph() {
        let body = "T1.1 SHALL complete BEFORE T1.2 runs.\nT1.2 SHALL ALWAYS resolve component vars.\n\nBody paragraph here.";
        let result = extract_shall_statement(body, "spec.md");
        assert!(result.contains("T1.1"));
        assert!(result.contains("T1.2"));
        assert!(!result.contains("Body paragraph"));
    }
}
