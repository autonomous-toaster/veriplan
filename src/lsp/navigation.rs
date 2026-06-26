//! Navigation — go-to-definition and hover for task references.

use std::path::Path;

use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, Location, MarkupContent, MarkupKind, Position,
    Range, Url,
};

use crate::ir::PlanIR;

/// Try to find a task reference at the given cursor position in a spec file.
/// Returns the URI + range for go-to-definition.
pub fn goto_definition(
    plan: &PlanIR,
    uri: &Url,
    pos: &Position,
    line_text: &str,
) -> Option<GotoDefinitionResponse> {
    let task_id = find_task_ref_at_position(line_text, pos.character as usize)?;

    let task = plan.tasks.iter().find(|t| t.id == task_id)?;

    // Build URI to tasks.md
    let _tasks_path = Path::new(uri.path()).parent()?;
    // Walk up to change dir
    let change_dir = find_change_dir(uri.path())?;
    let tasks_md = change_dir.join("tasks.md");
    let tasks_uri = Url::from_file_path(&tasks_md).ok()?;

    let loc = Location {
        uri: tasks_uri,
        range: Range {
            start: Position {
                line: (task.source.start_line.saturating_sub(1)) as u32,
                character: 0,
            },
            end: Position {
                line: (task.source.end_line.saturating_sub(1)) as u32,
                character: 999,
            },
        },
    };

    Some(GotoDefinitionResponse::Scalar(loc))
}

/// Build hover content for a task reference at cursor position.
pub fn hover(plan: &PlanIR, pos: &Position, line_text: &str) -> Option<Hover> {
    let task_id = find_task_ref_at_position(line_text, pos.character as usize)?;
    let task = plan.tasks.iter().find(|t| t.id == task_id)?;

    let content = format!(
        "**T{}** — {}\n\n*Phase:* {}",
        task.id, task.description, task.phase
    );

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: None,
    })
}

/// Extract a task ID like "3.2" from a position within "T3.2" or "t3_2".
fn find_task_ref_at_position(line: &str, col: usize) -> Option<String> {
    // Walk backward from cursor to find the 'T' or 't' prefix
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    // Find the start of the current token (walk backward to find T/t or start of word)
    let mut start = col.min(chars.len() - 1);
    while start > 0 {
        let c = chars[start - 1];
        if c == 'T' || c == 't' {
            start -= 1;
            break;
        }
        if !c.is_alphanumeric() && c != '_' && c != '.' {
            break;
        }
        start -= 1;
    }

    // Find the end of the token
    let mut end = start;
    while end < chars.len()
        && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '.')
    {
        end += 1;
    }

    if start >= end {
        return None;
    }

    let token: String = chars[start..end].iter().collect();

    // Check if it starts with T or t and has digits
    if let Some(rest) = token.strip_prefix('T').or_else(|| token.strip_prefix('t')) {
        // Could be T3.2 or t3_2
        if rest.contains('.') {
            // Standard format T3.2
            return Some(rest.to_string());
        }
        if let Some(pos) = rest.find('_') {
            // LTL format t3_2 → 3.2
            let major = &rest[..pos];
            let minor = &rest[pos + 1..];
            if major.chars().all(|c| c.is_ascii_digit())
                && minor.chars().all(|c| c.is_ascii_digit())
            {
                return Some(format!("{}.{}", major, minor));
            }
        }
    }

    None
}

/// Walk up from a file path to find the openspec/changes/<name>/ directory.
fn find_change_dir(file_path: &str) -> Option<std::path::PathBuf> {
    let mut path = Path::new(file_path).parent()?;
    loop {
        // Check: parent of current dir is "changes" and grandparent is "openspec"
        if let Some(parent) = path.parent()
            && parent.file_name()?.to_string_lossy() == "changes"
            && let Some(grandparent) = parent.parent()
            && grandparent.file_name()?.to_string_lossy() == "openspec"
        {
            // path is the change directory
            return Some(path.to_path_buf());
        }
        path = path.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_task_ref_t3_2() {
        assert_eq!(
            find_task_ref_at_position("T3.2 SHALL complete", 2),
            Some("3.2".to_string())
        );
    }

    #[test]
    fn test_find_task_ref_t8_1() {
        assert_eq!(
            find_task_ref_at_position("T8.1 SHALL run", 3),
            Some("8.1".to_string())
        );
    }

    #[test]
    fn test_find_task_ref_no_match() {
        assert_eq!(find_task_ref_at_position("hello world", 3), None);
    }

    #[test]
    fn test_find_task_ref_ltl_format() {
        assert_eq!(
            find_task_ref_at_position("failed_t3_2 == 1", 9),
            Some("3.2".to_string())
        );
    }
}
