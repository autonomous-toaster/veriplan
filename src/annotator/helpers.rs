//! Annotator helper functions.

use crate::ir::PlanIR;

/// Extract task IDs from LTL formula.
pub fn task_ids_from_ltl(ltl: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let bytes = ltl.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b't' && i + 1 < n && bytes[i + 1].is_ascii_digit() {
            i += 1;
            let start = i;
            while i < n && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && let Some(underscore) = s.find('_')
            {
                let major = &s[..underscore];
                let minor = &s[underscore + 1..];
                ids.push(format!("{}.{}", major, minor));
            }
        } else {
            i += 1;
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

/// Parse conditional LTL to extract trigger and consequent task IDs.
pub fn parse_conditional_ltl(ltl: &str) -> Option<(String, String)> {
    let trigger = extract_ltl_var(ltl, b"failed_t");
    let consequent = extract_ltl_var(ltl, b"active_t");
    match (trigger, consequent) {
        (Some(t), Some(c)) => Some((t, c)),
        _ => None,
    }
}

/// Extract a task ID from an LTL variable like `failed_t1_1` or `active_t2_1`.
fn extract_ltl_var(ltl: &str, prefix: &[u8]) -> Option<String> {
    let bytes = ltl.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(prefix) {
            i += prefix.len();
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && let Some(underscore) = s.find('_')
            {
                return Some(format!("{}.{}", &s[..underscore], &s[underscore + 1..]));
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Build phase context string from LTL.
pub fn build_phase_context(ltl: &str, plan: &PlanIR) -> Option<String> {
    let task_ids = task_ids_from_ltl(ltl);
    if task_ids.is_empty() {
        return None;
    }

    let mut phases = Vec::new();
    for task_id in &task_ids {
        for phase in &plan.phases {
            if phase.task_ids.iter().any(|id| id == task_id) {
                phases.push(phase.name.clone());
                break;
            }
        }
    }

    if phases.is_empty() {
        None
    } else {
        Some(phases.join(", "))
    }
}

/// Generate category breakdown for violations.
pub fn category_breakdown(violations: &[super::AnnotatedViolation]) -> String {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for v in violations {
        let cat = v.category.clone();
        *counts.entry(cat).or_insert(0) += 1;
    }

    let mut items: Vec<_> = counts.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));

    items
        .iter()
        .map(|(cat, count)| format!("  - {}: {}", cat, count))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotator::AnnotatedViolation;
    use crate::checker::Violation;
    use crate::ir::*;

    #[test]
    fn test_task_ids_from_ltl() {
        let ids = task_ids_from_ltl("[] ( active_t1_1 -> done_t1_2 )");
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"1.1".to_string()));
        assert!(ids.contains(&"1.2".to_string()));
    }

    #[test]
    fn test_task_ids_from_ltl_empty() {
        let ids = task_ids_from_ltl("true");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_build_phase_context() {
        let plan = PlanIR {
            tasks: vec![
                Task {
                    id: "1.1".into(),
                    description: "Setup".into(),
                    phase: "Phase 1".into(),
                    checked: false,
                    source: SourceLocation {
                        file: String::new(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: 0,
                        end_line: 0,
                    },
                },
                Task {
                    id: "1.2".into(),
                    description: "Build".into(),
                    phase: "Phase 1".into(),
                    checked: false,
                    source: SourceLocation {
                        file: String::new(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: 0,
                        end_line: 0,
                    },
                },
            ],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![Phase {
                name: "Phase 1".into(),
                task_ids: vec!["1.1".into(), "1.2".into()],
                mode: PhaseMode::Sequential,
            }],
            source_map: SourceMap::default(),
        };
        let ctx = build_phase_context("[] ( active_t1_1 -> done_t1_2 )", &plan);
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("Phase 1"));
    }

    #[test]
    fn test_build_phase_context_no_match() {
        let plan = PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: SourceMap::default(),
        };
        let ctx = build_phase_context("true", &plan);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_category_breakdown() {
        let violations = vec![AnnotatedViolation {
            violation: Violation {
                constraint_id: "R1".into(),
                requirement_statement: "test".into(),
                ltl: "".into(),
                category: "SequentialOrder".into(),
                state: "".into(),
                task_source: None,
                req_source: None,
                suggested_fix: None,
                plan: "test".into(),
            },
            category: "SequentialOrder".into(),
            phase_context: None,
            trigger_task: None,
            consequent_task: None,
            task_source: None,
            req_source: None,
        }];
        let breakdown = category_breakdown(&violations);
        assert!(breakdown.contains("SequentialOrder"));
    }
}

#[test]
fn test_extract_ltl_var_failed() {
    let result = extract_ltl_var("[] ( failed_t1_1 -> <> active_t2_1 )", b"failed_t");
    assert_eq!(result, Some("1.1".to_string()));
}

#[test]
fn test_extract_ltl_var_active() {
    let result = extract_ltl_var("[] ( failed_t1_1 -> <> active_t2_1 )", b"active_t");
    assert_eq!(result, Some("2.1".to_string()));
}

#[test]
fn test_extract_ltl_var_no_match() {
    let result = extract_ltl_var("true", b"failed_t");
    assert_eq!(result, None);
}
