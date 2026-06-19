//! Completions — task ID and temporal keyword suggestions for spec files.

use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};

use crate::ir::PlanIR;

/// Build completion list for a given cursor context in a spec file.
/// Returns None if the file isn't in a known change.
pub fn get_completions(plan: &PlanIR, line: &str, col: usize) -> Option<CompletionList> {
    let mut items = Vec::new();

    // Check if we're after SHALL/MUST/SHOULD → temporal keywords
    let before_cursor = &line[..col.min(line.len())];
    let trimmed_before = before_cursor.trim();

    if trimmed_before.ends_with("SHALL ")
        || trimmed_before.ends_with("SHALL")
        || trimmed_before.ends_with("MUST ")
        || trimmed_before.ends_with("MUST")
        || trimmed_before.ends_with("SHOULD ")
        || trimmed_before.ends_with("SHOULD")
    {
        items.extend(temporal_keyword_completions());
    }

    // Always suggest task IDs when T or t is typed
    items.extend(task_id_completions(plan));

    if items.is_empty() {
        return None;
    }

    Some(CompletionList {
        is_incomplete: false,
        items,
    })
}

/// Build task ID completion items from a PlanIR.
pub fn task_id_completions(plan: &PlanIR) -> Vec<CompletionItem> {
    plan.tasks
        .iter()
        .map(|task| {
            let label = format!("T{} — {}", task.id, truncate(&task.description, 40));
            CompletionItem {
                label,
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("Phase: {}", task.phase)),
                insert_text: Some(task.id.clone()),
                insert_text_format: None,
                ..Default::default()
            }
        })
        .collect()
}

/// Build temporal keyword completion items.
pub fn temporal_keyword_completions() -> Vec<CompletionItem> {
    vec![
        keyword_item(
            "BEFORE",
            "Sequential — T<N> SHALL complete BEFORE T<N>",
            "BEFORE T",
        ),
        keyword_item(
            "CONCURRENTLY",
            "Concurrent — T<N> SHALL run CONCURRENTLY with T<N>",
            "CONCURRENTLY WITH T",
        ),
        keyword_item("AFTER", "Sequential — T<N> SHALL run AFTER T<N>", "AFTER T"),
        keyword_item(
            "IF...THEN",
            "Conditional — IF T<N> fails THEN T<N> SHALL run",
            "IF T",
        ),
        keyword_item(
            "ALWAYS",
            "Global invariants — SHALL ALWAYS <condition>",
            "ALWAYS",
        ),
        keyword_item(
            "AT MOST ONE",
            "Exclusive — AT MOST ONE of T<N>/T<N> SHALL be active",
            "AT MOST ONE",
        ),
    ]
}

fn keyword_item(label: &str, detail: &str, insert: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        insert_text: Some(insert.to_string()),
        ..Default::default()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
