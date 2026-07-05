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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn make_plan() -> PlanIR {
        PlanIR {
            tasks: vec![
                Task {
                    id: "1.1".into(), description: "Setup".into(), phase: "Phase 1".into(), checked: false,
                    source: SourceLocation { file: "tasks.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 },
                },
                Task {
                    id: "1.2".into(), description: "Build".into(), phase: "Phase 1".into(), checked: true,
                    source: SourceLocation { file: "tasks.md".into(), start_byte: 0, end_byte: 0, start_line: 2, end_line: 2 },
                },
            ],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![Phase { name: "Phase 1".into(), task_ids: vec!["1.1".into(), "1.2".into()], mode: PhaseMode::Sequential }],
            source_map: SourceMap::default(),
        }
    }

    #[test]
    fn test_get_completions_empty_line() {
        let plan = make_plan();
        let result = get_completions(&plan, "", 0);
        assert!(result.is_some());
        let list = result.unwrap();
        assert!(!list.items.is_empty());
    }

    #[test]
    fn test_get_completions_with_t_prefix() {
        let plan = make_plan();
        let result = get_completions(&plan, "T", 1);
        assert!(result.is_some());
    }

    #[test]
    fn test_task_id_completions() {
        let plan = make_plan();
        let items = task_id_completions(&plan);
        assert!(items.iter().any(|i| i.label.contains("T1.1")));
        assert!(items.iter().any(|i| i.label.contains("T1.2")));
    }

    #[test]
    fn test_temporal_keyword_completions() {
        let items = temporal_keyword_completions();
        assert!(items.iter().any(|i| i.label == "BEFORE"));
        assert!(items.iter().any(|i| i.label == "AFTER"));
    }

    #[test]
    fn test_keyword_item() {
        let item = keyword_item("BEFORE", "Temporal ordering", "BEFORE");
        assert_eq!(item.label, "BEFORE");
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let t = truncate("hello world", 5);
        assert_eq!(t, "hello\u{2026}");
    }
}
