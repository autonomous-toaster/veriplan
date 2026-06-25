//! Annotator helper functions.

use crate::ir::PlanIR;

/// Extract task IDs from LTL formula.
pub fn task_ids_from_ltl(ltl: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let bytes = ltl.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b't' && i + 1 < n && bytes[i+1].is_ascii_digit() {
            i += 1;
            let start = i;
            while i < n && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && let Some(underscore) = s.find('_') {
                    let major = &s[..underscore];
                    let minor = &s[underscore+1..];
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
    // Look for patterns like: [](failed_t1_1 -> <>active_t2_1)
    let bytes = ltl.as_bytes();
    let mut trigger = None;
    let mut consequent = None;

    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"failed_t") {
            i += 8;
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && let Some(underscore) = s.find('_') {
                    trigger = Some(format!("{}.{}", &s[..underscore], &s[underscore+1..]));
                }
        } else if bytes[i..].starts_with(b"active_t") {
            i += 8;
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && let Some(underscore) = s.find('_') {
                    consequent = Some(format!("{}.{}", &s[..underscore], &s[underscore+1..]));
                }
        } else {
            i += 1;
        }
    }

    match (trigger, consequent) {
        (Some(t), Some(c)) => Some((t, c)),
        _ => None,
    }
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
