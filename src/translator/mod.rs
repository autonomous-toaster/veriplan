//! Rule translator: maps RFC 2119 + temporal categories to LTL formulas.
//!
//! Implements the 6 VeriPlan temporal constraint categories (Table 1)
//! and maps them to LTL formulas for SPIN/Promela model checking.

use crate::ir::{
    ConstraintCategory::{self, *},
    PhaseMode, PlanIR, Rfc2119Strength,
    ltl::{LtlCondition, LtlFormula},
};

/// Result of translating a requirement to LTL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranslatedConstraint {
    pub requirement_id: String,
    pub statement: String,
    pub strength: Rfc2119Strength,
    pub category: ConstraintCategory,
    /// LTL formula AST (None if NonFormalizable)
    pub ltl: Option<LtlFormula>,
    /// Whether this is a hard constraint (MUST/MUST NOT)
    pub is_hard: bool,
}

impl TranslatedConstraint {
    /// Serialize the LTL formula to string, or return empty string if None.
    pub fn ltl_string(&self) -> String {
        self.ltl
            .as_ref()
            .map(crate::ir::ltl::ltl_to_string)
            .unwrap_or_default()
    }
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
            Some(LtlFormula::Always(LtlCondition::Atom("true".into()))) // structurally guaranteed — no LTL
        } else if category != NonFormalizable
            && category != PatternUngrounded
            && category != Informational
        {
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

    // Temporal categories take PRIORITY. A requirement that is a verifiable
    // temporal constraint is always classified as that category, even if the
    // body also happens to mention "human review only".
    if is_exclusive(&lower) {
        return Exclusive;
    }
    if is_conditional(&lower) {
        return Conditional;
    }
    if is_concurrent(&lower) {
        return ConcurrentEvents;
    }
    if is_fixed_time(&lower) {
        return FixedTime;
    }
    if is_global(&lower) {
        return Global;
    }
    if is_sequential(&lower) {
        return SequentialOrder;
    }
    // Only if the requirement is NOT a temporal constraint do we honor the
    // human-review-only marker — otherwise it would accidentally exempt
    // verifiable requirements.
    if is_informational(&lower) {
        return Informational;
    }
    NonFormalizable
}

/// Whether the statement is explicitly marked as informational /
/// human-review-only (not a temporal state-machine constraint).
fn is_informational(lower: &str) -> bool {
    // Only explicit authorial intent markers, NOT the bare word "informational"
    // (which legitimately appears in requirements that discuss the concept).
    lower.contains("human review only") || lower.contains("not formalizable by design")
}

fn is_exclusive(lower: &str) -> bool {
    lower.contains("at most one")
        || lower.contains("mutually exclusive")
        || (lower.contains("not") && lower.contains("concurrently"))
        || lower.contains("not together")
        || lower.contains("only one")
}

fn is_conditional(lower: &str) -> bool {
    let has_if = lower.starts_with("if ") || lower.contains(" if ");
    let has_when_then = lower.contains("when") && lower.contains("then");
    let has_unless = lower.contains("unless");
    let has_fail_then = lower.contains("fail") && lower.contains("then");
    has_if || has_when_then || has_unless || has_fail_then
}

fn is_concurrent(lower: &str) -> bool {
    lower.contains("concurrently")
        || lower.contains("in parallel")
        || lower.contains("simultaneously")
        || lower.contains("at the same time")
}

fn is_fixed_time(lower: &str) -> bool {
    lower.contains("within")
        || lower.contains("between") && lower.contains("and")
        || (lower.contains("before") && is_time_ref(lower))
        || (lower.contains("after") && is_time_ref(lower))
        || lower.contains("window")
}

fn is_global(lower: &str) -> bool {
    lower.contains("always") || lower.contains("throughout") || lower.contains("at all times")
}

fn is_sequential(lower: &str) -> bool {
    lower.contains(" before ")
        || lower.contains(" after ")
        || lower.contains("complete before")
        || lower.contains("only after")
        || lower.contains("must finish")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_sequential_pair_before() {
        let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
        // The function looks for "1.1 before" as a contiguous substring
        let result = find_sequential_pair("1.1 before 1.2", &task_ids);
        assert_eq!(result, Some(("1.1".to_string(), "1.2".to_string())));
    }

    #[test]
    fn test_find_sequential_pair_after() {
        let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
        // AFTER returns (other, id) — the thing after "after" is the earlier task
        let result = find_sequential_pair("1.2 after 1.1", &task_ids);
        assert_eq!(result, Some(("1.2".to_string(), "1.1".to_string())));
    }

    #[test]
    fn test_find_sequential_pair_no_match() {
        let task_ids = vec!["1.1".to_string()];
        let result = find_sequential_pair("The system SHALL be robust", &task_ids);
        assert_eq!(result, None);
    }

    #[test]
    fn test_normalize_id() {
        assert_eq!(normalize_id("1.3"), "t1_3");
        assert_eq!(normalize_id("10.7"), "t10_7");
    }
}

#[test]
fn test_generate_ltl_sequential() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::SequentialOrder,
        "T1.1 SHALL complete BEFORE T1.2",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("active_t1_2"));
    assert!(ltl_str.contains("done_t1_1"));
}

#[test]
fn test_generate_ltl_exclusive() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::Exclusive,
        "At most one of T1.1, T1.2 SHALL be active",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("active_t1_1"));
    assert!(ltl_str.contains("active_t1_2"));
}

#[test]
fn test_generate_ltl_conditional() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::Conditional,
        "IF T1.1 fails THEN T2.1 SHALL run",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("failed_t1_1"));
    assert!(ltl_str.contains("active_t2_1"));
}

#[test]
fn test_generate_ltl_concurrent() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::ConcurrentEvents,
        "T3.1 and T3.2 SHALL run concurrently",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("<->"));
}

#[test]
fn test_generate_ltl_non_formalizable() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::NonFormalizable,
        "The system SHALL be robust",
        &plan,
    );
    assert!(ltl.is_none());
}

#[test]
fn test_extract_task_refs() {
    let plan = make_test_plan();
    let refs = extract_task_refs("T1.1 SHALL complete BEFORE T1.2", &plan);
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&"1.1".to_string()));
    assert!(refs.contains(&"1.2".to_string()));
}

#[test]
fn test_extract_task_refs_bare() {
    let task_ids = vec!["1.1".to_string(), "1.2".to_string(), "2.1".to_string()];
    let refs = extract_task_refs_bare("T1.1 SHALL complete BEFORE T1.2", &task_ids);
    assert_eq!(refs.len(), 2);
}

#[allow(dead_code)]
fn make_test_plan() -> crate::ir::PlanIR {
    use crate::ir::*;
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
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 2,
                    end_line: 2,
                },
            },
            Task {
                id: "2.1".into(),
                description: "Deploy".into(),
                phase: "Phase 2".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 3,
                    end_line: 3,
                },
            },
            Task {
                id: "3.1".into(),
                description: "Monitor".into(),
                phase: "Phase 3".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 4,
                    end_line: 4,
                },
            },
            Task {
                id: "3.2".into(),
                description: "Alert".into(),
                phase: "Phase 3".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 5,
                    end_line: 5,
                },
            },
        ],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    }
}
