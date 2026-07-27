//! Document symbols — hierarchical tree of phases/tasks and requirements/scenarios.

use lsp_types::{DocumentSymbol, DocumentSymbolResponse, Position, Range, SymbolKind};

use crate::ir::{PlanIR, Requirement};

/// Build document symbols for tasks.md: phases → tasks.
#[allow(deprecated)]
pub fn tasks_document_symbols(plan: &PlanIR) -> Option<DocumentSymbolResponse> {
    let mut symbols = Vec::new();

    for phase in &plan.phases {
        let mut children = Vec::new();
        for task_id in &phase.task_ids {
            if let Some(task) = plan.tasks.iter().find(|t| t.id == *task_id) {
                let status = if task.checked { "✓" } else { "☐" };
                let detail = if task.checked {
                    Some("Done".to_string())
                } else {
                    Some("Pending".to_string())
                };
                children.push(DocumentSymbol {
                    name: format!("{} T{} — {}", status, task.id, task.description),
                    detail,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    deprecated: None,
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
                    selection_range: Range {
                        start: Position {
                            line: (task.source.start_line.saturating_sub(1)) as u32,
                            character: 0,
                        },
                        end: Position {
                            line: (task.source.start_line.saturating_sub(1)) as u32,
                            character: 999,
                        },
                    },
                    children: None,
                });
            }
        }

        let mode = match phase.mode {
            crate::ir::PhaseMode::Concurrent => " [concurrent]",
            crate::ir::PhaseMode::Sequential => "",
        };
        symbols.push(DocumentSymbol {
            name: format!("{}{}", phase.name, mode),
            detail: Some(format!("{} tasks", phase.task_ids.len())),
            kind: SymbolKind::NAMESPACE,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(999, 0),
            },
            selection_range: Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            },
            children: Some(children),
        });
    }

    if symbols.is_empty() {
        return None;
    }

    Some(DocumentSymbolResponse::Nested(symbols))
}

/// Build document symbols for a spec file: requirements → scenarios.
#[allow(deprecated)]
pub fn spec_document_symbols_with_labels(
    requirements: &[Requirement],
    labels: &[String],
) -> Option<DocumentSymbolResponse> {
    let mut symbols = Vec::new();

    for (i, req) in requirements.iter().enumerate() {
        let cat_label = labels.get(i).map(|s| s.as_str()).unwrap_or("");

        let mut children = Vec::new();
        for scenario in &req.scenarios {
            children.push(DocumentSymbol {
                name: scenario.name.clone(),
                detail: None,
                kind: SymbolKind::EVENT,
                tags: None,
                deprecated: None,
                range: Range {
                    start: Position {
                        line: (scenario.source.start_line.saturating_sub(1)) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: (scenario.source.end_line.saturating_sub(1)) as u32,
                        character: 999,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: (scenario.source.start_line.saturating_sub(1)) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: (scenario.source.start_line.saturating_sub(1)) as u32,
                        character: 999,
                    },
                },
                children: None,
            });
        }

        symbols.push(DocumentSymbol {
            name: req.id.split("::").last().unwrap_or(&req.id).to_string(),
            detail: if cat_label.is_empty() {
                None
            } else {
                Some(cat_label.to_string())
            },
            kind: SymbolKind::INTERFACE,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position {
                    line: (req.source.start_line.saturating_sub(1)) as u32,
                    character: 0,
                },
                end: Position {
                    line: (req.source.end_line.saturating_sub(1)) as u32,
                    character: 999,
                },
            },
            selection_range: Range {
                start: Position {
                    line: (req.source.start_line.saturating_sub(1)) as u32,
                    character: 0,
                },
                end: Position {
                    line: (req.source.start_line.saturating_sub(1)) as u32,
                    character: 999,
                },
            },
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        });
    }

    if symbols.is_empty() {
        return None;
    }

    Some(DocumentSymbolResponse::Nested(symbols))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn make_plan() -> PlanIR {
        PlanIR {
            tasks: vec![
                Task {
                    id: "1.1".into(),
                    description: "Setup".into(),
                    phase: "Phase 1".into(),
                    checked: false,
                    source: SourceLocation {
                        file: "tasks.md".into(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: 1,
                        end_line: 1,
                    },
                },
                Task {
                    id: "1.2".into(),
                    description: "Build".into(),
                    phase: "Phase 1".into(),
                    checked: true,
                    source: SourceLocation {
                        file: "tasks.md".into(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: 2,
                        end_line: 2,
                    },
                },
            ],
            requirements: vec![Requirement {
                id: "R1".into(),
                statement: "T1.1 SHALL complete".into(),
                strength: Rfc2119Strength::Must,
                category: ConstraintCategory::SequentialOrder,
                ltl: None,
                scenarios: vec![],
                source: SourceLocation {
                    file: "specs/cap/spec.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 3,
                    end_line: 3,
                },
            }],
            scenarios: vec![],
            phases: vec![Phase {
                name: "Phase 1".into(),
                task_ids: vec!["1.1".into(), "1.2".into()],
                mode: PhaseMode::Sequential,
            }],
            source_map: SourceMap::default(),
        }
    }

    #[test]
    fn test_tasks_document_symbols_has_phase() {
        let plan = make_plan();
        let result = tasks_document_symbols(&plan);
        assert!(result.is_some());
    }

    #[test]
    fn test_tasks_document_symbols_empty() {
        let plan = PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: SourceMap::default(),
        };
        let result = tasks_document_symbols(&plan);
        assert!(result.is_none());
    }

    #[test]
    fn test_spec_document_symbols_with_labels_has_requirement() {
        let plan = make_plan();
        let reqs: Vec<Requirement> = plan.requirements.clone();
        let categories: Vec<String> = reqs.iter().map(|r| format!("{:?}", r.category)).collect();
        let result = spec_document_symbols_with_labels(&reqs, &categories);
        assert!(result.is_some());
    }

    #[test]
    fn test_spec_document_symbols_with_labels_empty() {
        let result = spec_document_symbols_with_labels(&[], &[]);
        assert!(result.is_none());
    }
}
