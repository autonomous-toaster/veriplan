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
            // The keyword decides which task is earlier. Statement position is only the
            // fallback, for a body naming three or more tasks, where no single pair exists.
            let pair = match find_sequential_pair(statement, &task_ids) {
                Some(pair) => Some(pair),
                None if task_ids.len() >= 2 => {
                    let (first, second) = (task_ids[0].clone(), task_ids[1].clone());
                    if names_after_ordering(statement) {
                        Some((second, first))
                    } else {
                        Some((first, second))
                    }
                }
                None => None,
            }?;
            Some(ordering_formula(&pair.0, &pair.1))
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
        // No placeholder. A formula that is always true is not a proof, and a report that
        // counts it as satisfied is worse than a report that says the constraint is not
        // modelled. translate_all records the reason.
        FixedTime | Global => None,
        NonFormalizable | Informational => None,
        PatternUngrounded => None,
    }
}

/// Extract task ID references from a statement using a PlanIR.
pub fn extract_task_refs(statement: &str, plan: &PlanIR) -> Vec<String> {
    // Find all referenced task IDs and sort by their position in the statement
    let mut refs_with_pos: Vec<(usize, String)> = Vec::new();
    for task in &plan.tasks {
        // Whole-identifier match: "T1.1" must not match inside "T1.10".
        if let Some(pos) = crate::util::find_task(statement, &task.id) {
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
        // Whole-identifier match, same rule as extract_task_refs.
        if crate::util::mentions_task(statement, id) {
            refs.push(id.clone());
        }
    }
    refs
}

/// The ordering formula: the later task may start only once the earlier task is done.
fn ordering_formula(earlier: &str, later: &str) -> LtlFormula {
    LtlFormula::Always(LtlCondition::Implies(
        Box::new(LtlCondition::Atom(format!(
            "active_{}",
            normalize_id(later)
        ))),
        Box::new(LtlCondition::Atom(format!(
            "done_{}",
            normalize_id(earlier)
        ))),
    ))
}

/// The sequential keyword in a body: its byte position and whether the keyword is BEFORE.
///
/// A body carries one temporal keyword (the grammar forbids two), so the earliest of
/// " before " and " after " is the keyword that decides the direction.
fn sequential_keyword(lower: &str) -> Option<(usize, bool)> {
    match (lower.find(" before "), lower.find(" after ")) {
        (Some(b), Some(a)) if a < b => Some((a, false)),
        (Some(b), _) => Some((b, true)),
        (None, Some(a)) => Some((a, false)),
        (None, None) => None,
    }
}

/// Whether the body states the ordering with AFTER, which names the earlier task second.
fn names_after_ordering(statement: &str) -> bool {
    matches!(
        sequential_keyword(&statement.to_lowercase()),
        Some((_, false))
    )
}

/// Find which task is earlier and which is later in a sequential constraint.
///
/// The body grammar puts an RFC 2119 verb between a task id and the keyword
/// (`T1.1 SHALL complete BEFORE T2.1 SHALL start`), so the keyword is located first and
/// the ids are split around it. BEFORE names the earlier task first; AFTER names the
/// earlier task second. Statement position alone never decides the direction.
pub fn find_sequential_pair(statement: &str, task_ids: &[String]) -> Option<(String, String)> {
    let lower = statement.to_lowercase();
    let (keyword_pos, is_before) = sequential_keyword(&lower)?;

    let mut first_side: Option<String> = None;
    let mut second_side: Option<String> = None;
    for id in task_ids {
        let Some(pos) = crate::util::find_task(&lower, id) else {
            continue;
        };
        if pos < keyword_pos {
            first_side.get_or_insert_with(|| id.clone());
        } else {
            second_side.get_or_insert_with(|| id.clone());
        }
    }

    let (earlier_in_text, later_in_text) = (first_side?, second_side?);
    if is_before {
        Some((earlier_in_text, later_in_text))
    } else {
        Some((later_in_text, earlier_in_text))
    }
}

/// Normalize a task ID (1.3 → t_1_3) for use in LTL variable names.
pub(crate) fn normalize_id(id: &str) -> String {
    format!("t{}", id.replace('.', "_"))
}
