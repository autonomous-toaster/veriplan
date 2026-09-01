//! LTL generation and task-reference extraction — extracted from translator
//! mod.rs to keep files under the 550-line file-length gate.

use crate::ir::{
    ConstraintCategory::{self, *},
    PlanIR,
    ltl::{LtlCondition, LtlFormula},
};

/// Generate an LTL formula for a classified constraint.
pub fn generate_ltl(
    category: &ConstraintCategory,
    statement: &str,
    plan: &PlanIR,
) -> Option<LtlFormula> {
    let task_ids = extract_task_refs(statement, plan);

    match category {
        SequentialOrder => {
            // Extract which task is before which
            if let Some((before_id, after_id)) = find_sequential_pair(statement, &task_ids) {
                Some(LtlFormula::Always(LtlCondition::Implies(
                    Box::new(LtlCondition::Atom(format!(
                        "active_{}",
                        normalize_id(&after_id)
                    ))),
                    Box::new(LtlCondition::Atom(format!(
                        "done_{}",
                        normalize_id(&before_id)
                    ))),
                )))
            } else if task_ids.len() >= 2 {
                // General case: if A and B are referenced, A before B
                let a = normalize_id(&task_ids[0]);
                let b = normalize_id(&task_ids[1]);
                Some(LtlFormula::Always(LtlCondition::Implies(
                    Box::new(LtlCondition::Atom(format!("active_{}", b))),
                    Box::new(LtlCondition::Atom(format!("done_{}", a))),
                )))
            } else {
                None
            }
        }
        Exclusive => {
            // Generate pairwise exclusions for all referenced task pairs
            if task_ids.len() < 2 {
                return None;
            }
            let pairs: Vec<LtlCondition> = (0..task_ids.len())
                .flat_map(|i| (i + 1..task_ids.len()).map(move |j| (i, j)))
                .map(|(i, j)| {
                    let a = normalize_id(&task_ids[i]);
                    let b = normalize_id(&task_ids[j]);
                    LtlCondition::Not(Box::new(LtlCondition::And(vec![
                        LtlCondition::Atom(format!("active_{}", a)),
                        LtlCondition::Atom(format!("active_{}", b)),
                    ])))
                })
                .collect();
            Some(LtlFormula::Always(LtlCondition::And(pairs)))
        }
        Conditional => {
            // Find the trigger task and the consequent task
            if task_ids.len() >= 2 {
                let trigger = normalize_id(&task_ids[0]);
                let consequent = normalize_id(&task_ids[1]);
                Some(LtlFormula::Always(LtlCondition::Implies(
                    Box::new(LtlCondition::Atom(format!("failed_{}", trigger))),
                    Box::new(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
                        format!("active_{}", consequent),
                    )))),
                )))
            } else {
                None
            }
        }
        ConcurrentEvents => {
            // Generate bidirectional equivalence
            if task_ids.len() >= 2 {
                let a = normalize_id(&task_ids[0]);
                let b = normalize_id(&task_ids[1]);
                Some(LtlFormula::Always(LtlCondition::Iff(
                    Box::new(LtlCondition::Atom(format!("active_{}", a))),
                    Box::new(LtlCondition::Atom(format!("active_{}", b))),
                )))
            } else {
                None
            }
        }
        FixedTime | Global => {
            // Global invariants and fixed-time constraints without reliable durations
            // Just note the constraint exists — evaluated as always-true placeholder
            Some(LtlFormula::Always(LtlCondition::Atom("true".into())))
        }
        NonFormalizable | Informational => None,
        PatternUngrounded => None,
    }
}

/// Extract task ID references from a statement using a PlanIR.
pub fn extract_task_refs(statement: &str, plan: &PlanIR) -> Vec<String> {
    // Find all referenced task IDs and sort by their position in the statement
    let mut refs_with_pos: Vec<(usize, String)> = Vec::new();
    for task in &plan.tasks {
        let id_pattern = format!("T{}", task.id);
        let alt_pattern = format!("t{}", task.id);
        if let Some(pos) = statement.find(&id_pattern) {
            refs_with_pos.push((pos, task.id.clone()));
        } else if let Some(pos) = statement.find(&alt_pattern) {
            refs_with_pos.push((pos, task.id.clone()));
        }
    }
    refs_with_pos.sort_by_key(|(pos, _)| *pos);
    refs_with_pos.into_iter().map(|(_, id)| id).collect()
}

/// Extract task ID references from a statement given a list of known IDs.
pub fn extract_task_refs_bare(statement: &str, task_ids: &[String]) -> Vec<String> {
    let mut refs = Vec::new();
    for id in task_ids {
        let id_pattern = format!("T{}", id);
        let alt_pattern = format!("t{}", id);
        if statement.contains(&id_pattern) || statement.contains(&alt_pattern) {
            refs.push(id.clone());
        }
    }
    refs
}

/// Find which task is before which in a sequential constraint.
pub fn find_sequential_pair(statement: &str, task_ids: &[String]) -> Option<(String, String)> {
    let lower = statement.to_lowercase();

    for id in task_ids {
        let before_pattern = format!("{} before", id);
        let after_pattern = format!("after {}", id);
        let complete_before = format!("{} complete", id);

        if (lower.contains(&before_pattern) || lower.contains(&complete_before))
            && let Some(other) = find_matching_task(id, task_ids, &lower, statement)
        {
            return Some((id.clone(), other));
        }
        if lower.contains(&after_pattern)
            && let Some(other) = find_matching_task(id, task_ids, &lower, statement)
        {
            return Some((other, id.clone()));
        }
    }
    None
}

/// Find a task ID that appears in the statement, different from the given ID.
fn find_matching_task(
    id: &str,
    task_ids: &[String],
    lower: &str,
    statement: &str,
) -> Option<String> {
    for other in task_ids {
        if other != id && (lower.contains(other) || statement.contains(&format!("T{}", other))) {
            return Some(other.clone());
        }
    }
    None
}

/// Normalize a task ID (1.3 → t_1_3) for use in LTL variable names.
pub(crate) fn normalize_id(id: &str) -> String {
    format!("t{}", id.replace('.', "_"))
}
