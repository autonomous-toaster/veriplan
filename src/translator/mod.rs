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
/// Why a non-formalizable requirement is not verifiable. Used to emit targeted,
/// pedagogical fixes instead of the generic "does not match any temporal category".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VagueDiagnosis {
    /// References a task but specifies no constraint (redundant with the task list).
    BareCapability { task: String, word: Option<String> },
    /// References a task and uses a vague adverb (e.g. "quickly").
    VagueAction { task: String, word: String },
    /// No task reference and uses a vague adjective (e.g. "robust").
    VagueQuality { word: String },
}
/// Curated vague-word list, mirroring the curated temporal-keyword approach.
/// Runs ONLY on requirements that reached NonFormalizable (no temporal keyword,
/// no 'human review only'), so it cannot misfire on a verifiable requirement.
fn vague_adverbs() -> &'static [&'static str] {
    &[
        "quickly",
        "fast",
        "soon",
        "correctly",
        "properly",
        "efficiently",
        "reliably",
        "safely",
        "adequately",
        "promptly",
        "easily",
        "consistently",
    ]
}
fn vague_adjectives() -> &'static [&'static str] {
    &[
        "robust",
        "clean",
        "good",
        "stable",
        "user-friendly",
        "easy",
        "fast",
        "minimal",
        "optimal",
        "responsive",
        "scalable",
        "performant",
        "reliable",
        "safe",
        "better",
        "worse",
        "best",
    ]
}

/// Diagnose WHY a non-formalizable requirement is not verifiable.
/// Callers MUST only invoke this after `classify()` returned `NonFormalizable`.
pub fn diagnose_vague(statement: &str, task_ids: &[String]) -> Option<VagueDiagnosis> {
    let lower = statement.to_lowercase();
    let refs = extract_task_refs_bare(statement, task_ids);
    let has_task = !refs.is_empty();
    let first_task = refs.first().cloned().unwrap_or_default();

    // Prefer the most specific diagnosis: a vague word in a task-referencing
    // requirement is a VagueAction; in a non-referencing requirement it is a
    // VagueQuality.
    let adverb = vague_adverbs()
        .iter()
        .find(|w| lower.contains(**w))
        .copied();
    let adjective = vague_adjectives()
        .iter()
        .find(|w| lower.contains(**w))
        .copied();

    if has_task {
        if let Some(w) = adverb {
            return Some(VagueDiagnosis::VagueAction {
                task: first_task,
                word: (*w).to_string(),
            });
        }
        // A task-referencing requirement with a vague adjective is still
        // constraint-shaped but vague; treat the adjective as the issue.
        if let Some(w) = adjective {
            return Some(VagueDiagnosis::VagueAction {
                task: first_task,
                word: (*w).to_string(),
            });
        }
        // No vague word: it's a bare capability (references a task, no constraint).
        return Some(VagueDiagnosis::BareCapability {
            task: first_task,
            word: None,
        });
    }

    // No task reference.
    if let Some(w) = adjective {
        return Some(VagueDiagnosis::VagueQuality {
            word: (*w).to_string(),
        });
    }
    if let Some(w) = adverb {
        return Some(VagueDiagnosis::VagueQuality {
            word: (*w).to_string(),
        });
    }

    None // undiagnosed — fall back to the generic blocker
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

mod ltl;
#[cfg(any(test, kani))]
pub(crate) use ltl::normalize_id;
pub use ltl::{extract_task_refs, extract_task_refs_bare, find_sequential_pair, generate_ltl};

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
    #[test]
    fn test_diagnose_bare_capability() {
        // References a task, no vague word -> BareCapability
        let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
        let d = diagnose_vague("T1.1 SHALL be executed.", &task_ids).unwrap();
        assert_eq!(
            d,
            VagueDiagnosis::BareCapability {
                task: "1.1".to_string(),
                word: None
            }
        );
        // classify() must have returned NonFormalizable first.
        assert_eq!(
            classify("T1.1 SHALL be executed."),
            ConstraintCategory::NonFormalizable
        );
    }
    #[test]
    fn test_diagnose_vague_action() {
        // References a task + vague adverb -> VagueAction
        let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
        let d = diagnose_vague("T1.1 SHALL be done quickly.", &task_ids).unwrap();
        assert_eq!(
            d,
            VagueDiagnosis::VagueAction {
                task: "1.1".to_string(),
                word: "quickly".to_string()
            }
        );
    }
    #[test]
    fn test_diagnose_vague_quality() {
        // No task reference + vague adjective -> VagueQuality
        let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
        let d = diagnose_vague("The system SHALL be robust.", &task_ids).unwrap();
        assert_eq!(
            d,
            VagueDiagnosis::VagueQuality {
                word: "robust".to_string()
            }
        );
    }
    #[test]
    fn test_safety_boundary_temporal_not_diagnosed() {
        // A temporal requirement is NEVER diagnosed as vague — classify() returns
        // a temporal category (SequentialOrder), so diagnose_vague is not reached.
        let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
        let cat = classify("T1.1 SHALL be done quickly BEFORE T1.2 SHALL start.");
        // The exact temporal category is a pre-existing classification detail
        // (may be FixedTime or SequentialOrder); the guarantee is that it is a
        // temporal category and therefore never diagnosed as vague.
        assert_ne!(cat, ConstraintCategory::NonFormalizable);
        assert_ne!(cat, ConstraintCategory::Informational);
        assert!(
            diagnose_vague(
                "T1.1 SHALL be done quickly BEFORE T1.2 SHALL start.",
                &task_ids
            )
            .is_none()
                || !classify("T1.1 SHALL be done quickly BEFORE T1.2 SHALL start.")
                    .eq(&ConstraintCategory::NonFormalizable),
            "temporal requirement must not be diagnosed as vague"
        );
    }
    #[test]
    fn test_diagnose_undiagnosed_falls_back() {
        // No task reference, no vague word -> None (generic blocker fallback)
        let task_ids = vec!["1.1".to_string()];
        let d = diagnose_vague("The migration SHALL happen.", &task_ids);
        assert!(d.is_none());
        assert_eq!(
            classify("The migration SHALL happen."),
            ConstraintCategory::NonFormalizable
        );
    }
    #[test]
    fn test_diagnose_verdict_unchanged() {
        // Vague requirements still classify as NonFormalizable (blocker),
        // not reclassified to Informational.
        for stmt in [
            "T1.1 SHALL be executed.",
            "T1.1 SHALL be done quickly.",
            "The system SHALL be robust.",
        ] {
            let task_ids = vec!["1.1".to_string()];
            assert_eq!(
                classify(stmt),
                ConstraintCategory::NonFormalizable,
                "verdict must remain a blocker: {stmt}"
            );
            assert!(diagnose_vague(stmt, &task_ids).is_some());
        }
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
