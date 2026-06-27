//! Parser helper functions.

use std::path::Path;

/// Extract SHALL statement from requirement body.
pub fn extract_shall_statement(body: &str, _file: &str) -> String {
    let bytes = body.as_bytes();
    let mut start = 0;
    let mut end = body.len();

    while start < body.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    body[start..end].to_string()
}

/// Extract scenarios from requirement body.
pub fn extract_scenarios(
    body: &str,
    _bytes: &[u8],
    file: &str,
) -> (Vec<crate::ir::Scenario>, Vec<crate::ir::Scenario>) {
    let mut scenarios = Vec::new();
    let standalone_scenarios = Vec::new();

    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("#### Scenario:") || line.starts_with("#### scenario:") {
            let name = line
                .strip_prefix("#### Scenario:")
                .or_else(|| line.strip_prefix("#### scenario:"))
                .unwrap_or("")
                .trim();

            let mut steps = Vec::new();
            i += 1;

            while i < lines.len() {
                let step_line = lines[i].trim();
                if step_line.starts_with('#') || step_line.is_empty() {
                    break;
                }

                if let Some(step) = parse_step(step_line, file, i + 1) {
                    steps.push(step);
                }
                i += 1;
            }

            if !steps.is_empty() {
                let steps_len = steps.len();
                scenarios.push(crate::ir::Scenario {
                    name: name.to_string(),
                    steps,
                    source: crate::ir::SourceLocation {
                        file: file.to_string(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: i - steps_len,
                        end_line: i,
                    },
                });
            }
        } else {
            i += 1;
        }
    }

    (scenarios, standalone_scenarios)
}

/// Parse a single scenario step.
pub fn parse_step(text: &str, file: &str, line: usize) -> Option<crate::ir::ScenarioStep> {
    let text = text.trim();
    if !text.starts_with('-') {
        return None;
    }

    let text = text[1..].trim();

    let (kind, step_text) = if text.starts_with("**WHEN**") || text.starts_with("**when**") {
        (
            crate::ir::StepKind::When,
            text.strip_prefix("**WHEN**")
                .or_else(|| text.strip_prefix("**when**"))
                .unwrap_or("")
                .trim(),
        )
    } else if text.starts_with("**THEN**") || text.starts_with("**then**") {
        (
            crate::ir::StepKind::Then,
            text.strip_prefix("**THEN**")
                .or_else(|| text.strip_prefix("**then**"))
                .unwrap_or("")
                .trim(),
        )
    } else if text.starts_with("**GIVEN**") || text.starts_with("**given**") {
        (
            crate::ir::StepKind::Given,
            text.strip_prefix("**GIVEN**")
                .or_else(|| text.strip_prefix("**given**"))
                .unwrap_or("")
                .trim(),
        )
    } else if text.starts_with("**AND**") || text.starts_with("**and**") {
        (
            crate::ir::StepKind::And,
            text.strip_prefix("**AND**")
                .or_else(|| text.strip_prefix("**and**"))
                .unwrap_or("")
                .trim(),
        )
    } else {
        return None;
    };

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

/// Find line number for a byte offset.
pub fn find_line_for_byte(source: &str, byte: usize) -> usize {
    source[..byte].lines().count() + 1
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
