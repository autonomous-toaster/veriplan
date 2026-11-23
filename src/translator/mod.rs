//! Rule translator: maps RFC 2119 + temporal categories to LTL formulas.
//!
//! Implements the 6 VeriPlan temporal constraint categories (Table 1)
//! and maps them to LTL formulas for SPIN/Promela model checking.

use crate::ir::{
    ConstraintCategory::{self, *},
    PhaseMode, PlanIR, Rfc2119Strength,
    ltl::LtlFormula,
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
    /// Why this constraint carries no formula. Empty when the constraint is modelled, or
    /// when the requirement failed convertibility for a reason reported elsewhere.
    pub unmodelled_reason: Option<String>,
    /// The archived change this constraint's body was written against, when its arguments belong to
    /// history rather than to this change. Such a constraint carries no formula: it states an order
    /// between pieces of work that are already complete.
    pub historical: Option<String>,
    /// Cited task ids that belong to the requirement's history rather than to this change.
    pub foreign_tasks: Vec<String>,
}

impl TranslatedConstraint {
    /// A constraint is modelled when it carries a formula the engine can check.
    pub fn modelled(&self) -> bool {
        self.ltl.is_some()
    }

    /// A constraint is historical when its body is grounded on completed work.
    pub fn historical(&self) -> bool {
        self.historical.is_some()
    }
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
        // A body written against history states an order between completed work. It carries no
        // formula, is reported with its owning change, and is counted separately - never satisfied.
        let cited = crate::util::task_ids_in(&req.statement);
        let live: Vec<&String> = cited
            .iter()
            .filter(|id| plan.tasks.iter().any(|t| &t.id == *id))
            .collect();
        let foreign: Vec<String> = cited
            .iter()
            .filter(|id| !plan.tasks.iter().any(|t| &t.id == *id))
            .cloned()
            .collect();
        let historical = match (&req.grounded_in, foreign.is_empty()) {
            (Some(history), false) if live.is_empty() => Some(history.change.clone()),
            // Mixed: the live half cannot be modelled without the historical atom, so the whole
            // constraint is historical and the foreign ids are named. See design D5.
            (Some(history), false) => Some(history.change.clone()),
            _ => None,
        };

        // A category the model cannot encode yields no formula AND a reason. Emitting an
        // always-true formula here would make the constraint indistinguishable from a proof.
        let refs = extract_task_refs(&req.statement, plan);
        let (ltl, unmodelled_reason) =
            if category == ConcurrentEvents && tasks_in_same_concurrent_phase(plan, &refs) {
                (
                None,
                Some(
                    "the tasks share a concurrent phase; the model does not encode simultaneity"
                        .to_string(),
                ),
            )
            } else if category == FixedTime || category == Global {
                (
                    None,
                    Some("this category has no encoding in the task-order model".to_string()),
                )
            } else if category == NonFormalizable
                || category == PatternUngrounded
                || category == Informational
            {
                (None, None)
            } else if refs.len() >= 2 && refs[0] == refs[1] {
                (
                    None,
                    Some(format!(
                        "the two argument slots name the same task (T{})",
                        refs[0]
                    )),
                )
            } else {
                let ltl = generate_ltl(&category, &req.statement, plan);
                let reason = if ltl.is_none() {
                    Some("the statement did not yield a formula".to_string())
                } else {
                    None
                };
                (ltl, reason)
            };

        constraints.push(TranslatedConstraint {
            requirement_id: req.id.clone(),
            statement: req.statement.clone(),
            strength: req.strength.clone(),
            category,
            ltl,
            is_hard: req.strength.is_hard(),
            unmodelled_reason,
            historical,
            foreign_tasks: foreign,
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
    // Sequential order is tested BEFORE fixed time: a requirement body always cites a task
    // id, and an id used to satisfy the fixed-time clause through its digits, which made
    // every ordering statement classify as FixedTime and lose its encoding.
    if is_sequential(&lower) {
        return SequentialOrder;
    }
    if is_fixed_time(&lower) {
        return FixedTime;
    }
    if is_global(&lower) {
        return Global;
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
///
/// A task id is made of digits, so 'contains a digit' was true of every requirement body and
/// made this predicate meaningless. A time reference is a clock literal or a number with a
/// duration unit beside it.
fn is_time_ref(text: &str) -> bool {
    let bytes = text.as_bytes();
    // Clock literal: digit, ':', two digits.
    for i in 1..bytes.len().saturating_sub(2) {
        if bytes[i] == b':'
            && bytes[i - 1].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
        {
            return true;
        }
    }
    // Duration literal: a digit followed (within two bytes) by a unit word.
    const UNITS: [&str; 9] = [
        "millisecond",
        "second",
        "sec",
        "minute",
        "min",
        "hour",
        "day",
        "week",
        "ms",
    ];
    let mut last_digit_end: Option<usize> = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_ascii_digit() {
            last_digit_end = Some(idx + ch.len_utf8());
        } else if ch.is_alphabetic()
            && let Some(end) = last_digit_end
            && idx.saturating_sub(end) <= 2
        {
            let rest = &text[idx..];
            if UNITS.iter().any(|u| rest.starts_with(u)) {
                return true;
            }
        }
    }
    false
}

mod ltl;
#[cfg(any(test, kani))]
pub(crate) use ltl::normalize_id;
pub use ltl::{extract_task_refs, extract_task_refs_bare, find_sequential_pair, generate_ltl};

#[cfg(test)]
mod tests;
