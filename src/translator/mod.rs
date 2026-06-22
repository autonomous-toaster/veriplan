//! Rule translator: maps RFC 2119 + temporal categories to LTL formulas.
//!
//! Implements the 6 VeriPlan temporal constraint categories (Table 1)
//! and maps them to LTL formulas for SPIN/Promela model checking.

use crate::ir::{
    ConstraintCategory::{self, *},
    PhaseMode, PlanIR, Rfc2119Strength,
};

/// Result of translating a requirement to LTL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranslatedConstraint {
    pub requirement_id: String,
    pub statement: String,
    pub strength: Rfc2119Strength,
    pub category: ConstraintCategory,
    /// LTL formula (None if NonFormalizable)
    pub ltl: Option<String>,
    /// Whether this is a hard constraint (MUST/MUST NOT)
    pub is_hard: bool,
}

/// Check if all referenced task IDs are in the same concurrent phase.
fn tasks_in_same_concurrent_phase(plan: &PlanIR, task_ids: &[String]) -> bool {
    if task_ids.len() < 2 {
        return false;
    }
    plan.phases.iter().any(|p| {
        p.mode == PhaseMode::Concurrent && task_ids.iter().all(|id| p.task_ids.contains(id))
    })
}

/// Translate all formalizable requirements in a PlanIR to LTL constraints.
pub fn translate_all(plan: &PlanIR) -> Vec<TranslatedConstraint> {
    let mut constraints = Vec::new();

    for req in &plan.requirements {
        let category = classify(&req.statement);
        let ltl = if category == ConcurrentEvents
            && tasks_in_same_concurrent_phase(plan, &extract_task_refs(&req.statement, plan))
        {
            Some("true".into()) // structurally guaranteed — no LTL
        } else if category != NonFormalizable && category != PatternUngrounded {
            generate_ltl(&category, &req.statement, plan)
        } else {
            None
        };

        constraints.push(TranslatedConstraint {
            requirement_id: req.id.clone(),
            statement: req.statement.clone(),
            strength: req.strength.clone(),
            category,
            ltl,
            is_hard: req.strength.is_hard(),
        });
    }

    constraints
}

/// Classify a SHALL statement into a VeriPlan temporal category.
pub fn classify(statement: &str) -> ConstraintCategory {
    let lower = statement.to_lowercase();

    // Exclusive: "at most one", "not ... concurrently", "mutually exclusive", "not together"
    if lower.contains("at most one")
        || lower.contains("mutually exclusive")
        || (lower.contains("not") && lower.contains("concurrently"))
        || lower.contains("not together")
        || lower.contains("only one")
    {
        return Exclusive;
    }

    // Conditional: "if", "unless", "when ... then", "in case of"
    let has_if = lower.starts_with("if ") || lower.contains(" if ");
    let has_when_then = lower.contains("when") && lower.contains("then");
    let has_unless = lower.contains("unless");
    let has_fail_then = lower.contains("fail") && lower.contains("then");
    if has_if || has_when_then || has_unless || has_fail_then {
        return Conditional;
    }

    // Concurrent: "concurrently", "in parallel", "at the same time", "simultaneously"
    if lower.contains("concurrently")
        || lower.contains("in parallel")
        || lower.contains("simultaneously")
        || lower.contains("at the same time")
    {
        return ConcurrentEvents;
    }

    // Fixed time: "within", "between ... and", "before 0", "after 0", "window"
    if lower.contains("within")
        || lower.contains("between") && lower.contains("and")
        || (lower.contains("before") && is_time_ref(&lower))
        || (lower.contains("after") && is_time_ref(&lower))
        || lower.contains("window")
    {
        return FixedTime;
    }

    // Global: "always", "throughout", "throughout", "at all times", "available"
    if lower.contains("always")
        || lower.contains("throughout")
        || lower.contains("at all times")
        || lower.contains("throughout")
    {
        return Global;
    }

    // Sequential: "before", "after", "must complete", "must finish", "only after"
    if lower.contains(" before ")
        || lower.contains(" after ")
        || lower.contains("complete before")
        || lower.contains("only after")
        || lower.contains("must finish")
    {
        return SequentialOrder;
    }

    // Default: can't classify
    NonFormalizable
}

/// Check if the text references actual clock/calendar time (not task IDs).
fn is_time_ref(text: &str) -> bool {
    text.contains("min")
        || text.contains("hour")
        || text.contains("sec")
        || text.contains(":00")
        || text.contains("am")
        || text.contains("pm")
        || text.chars().any(|c| c.is_ascii_digit())
}

/// Generate an LTL formula for a classified constraint.
pub fn generate_ltl(
    category: &ConstraintCategory,
    statement: &str,
    plan: &PlanIR,
) -> Option<String> {
    let task_ids = extract_task_refs(statement, plan);

    match category {
        SequentialOrder => {
            // Extract which task is before which
            if let Some((before_id, after_id)) = find_sequential_pair(statement, &task_ids) {
                Some(format!(
                    "[] ( active_{} -> done_{} )",
                    normalize_id(&after_id),
                    normalize_id(&before_id),
                ))
            } else if task_ids.len() >= 2 {
                // General case: if A and B are referenced, A before B
                let a = normalize_id(&task_ids[0]);
                let b = normalize_id(&task_ids[1]);
                Some(format!("[] ( active_{} -> done_{} )", b, a))
            } else {
                None
            }
        }
        Exclusive => {
            // Generate pairwise exclusions for all referenced task pairs
            if task_ids.len() < 2 {
                return None;
            }
            let pairs: Vec<String> = (0..task_ids.len())
                .flat_map(|i| (i + 1..task_ids.len()).map(move |j| (i, j)))
                .map(|(i, j)| {
                    let a = normalize_id(&task_ids[i]);
                    let b = normalize_id(&task_ids[j]);
                    format!("!(active_{} && active_{})", a, b)
                })
                .collect();
            Some(format!("[] ( {} )", pairs.join(" && ")))
        }
        Conditional => {
            // Find the trigger task and the consequent task
            if task_ids.len() >= 2 {
                let trigger = normalize_id(&task_ids[0]);
                let consequent = normalize_id(&task_ids[1]);
                Some(format!(
                    "[] ( failed_{} -> <> active_{} )",
                    trigger, consequent
                ))
            } else {
                None
            }
        }
        ConcurrentEvents => {
            // Generate bidirectional equivalence
            if task_ids.len() >= 2 {
                let a = normalize_id(&task_ids[0]);
                let b = normalize_id(&task_ids[1]);
                Some(format!("[] ( active_{} <-> active_{} )", a, b))
            } else {
                None
            }
        }
        FixedTime | Global => {
            // Global invariants and fixed-time constraints without reliable durations
            // Just note the constraint exists — evaluated as always-true placeholder
            // since we lack a concrete condition.
            Some("true".into())
        }
        NonFormalizable => None,
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

        if lower.contains(&before_pattern) || lower.contains(&complete_before) {
            // This task is before some other task
            for other in task_ids {
                if other != id
                    && (lower.contains(other) || statement.contains(&format!("T{}", other)))
                {
                    return Some((id.clone(), other.clone()));
                }
            }
        }
        if lower.contains(&after_pattern) {
            for other in task_ids {
                if other != id
                    && (lower.contains(other) || statement.contains(&format!("T{}", other)))
                {
                    return Some((other.clone(), id.clone()));
                }
            }
        }
    }
    None
}

/// Normalize a task ID (1.3 → t_1_3) for use in LTL variable names.
fn normalize_id(id: &str) -> String {
    format!("t{}", id.replace('.', "_"))
}
