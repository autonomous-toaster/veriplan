//! Grounding check module — wraps groundcontrol's RuleGrounder for veriplan integration.
//!
//! Builds a groundcontrol `Signature` from `PlanIR` tasks, then grounds each
//! requirement's SHALL statement against the signature using keyword matching
//! + positional heuristics. Produces `CheckItem`s in the same format as
//!   `src/checker/checks.rs`.

use groundcontrol::grounders::RuleGrounder;
use groundcontrol::types::{Grounder, Signature};

pub use groundcontrol::types::GroundingStatus;

use crate::input::StrictnessProfile;
use crate::ir::{CheckItem, PlanIR, Rfc2119Strength};
use crate::util::truncate;

use crate::checker::checks::make_item;

mod signature;

pub(crate) use signature::split_constraint_clauses;
use signature::with_history_tasks;
pub use signature::{GroundingOutcome, signature_from_planir};

/// Whether `s` (the start of a sentence) begins with a task-ID reference,
/// e.g. "T4.7 SHALL" or "T1.2 SHALL complete".
pub(super) fn looks_like_task_id(s: &str) -> bool {
    let s = s.trim_start();
    let mut chars = s.chars();
    if chars.next() != Some('T') {
        return false;
    }
    let mut digit1 = false;
    for c in chars.by_ref() {
        if c.is_ascii_digit() {
            digit1 = true;
        } else if c == '.' {
            break;
        } else {
            return false;
        }
    }
    digit1
}

/// Run the grounding check on a plan.
///
/// For each requirement, builds a Signature from PlanIR and calls
/// `RuleGrounder::ground()` on the requirement's SHALL statement.
/// Returns (blockers, warnings, info, outcomes) — same pattern as checks in checks.rs,
/// plus the per-requirement outcomes for downstream use (e.g., populating PatternUngrounded).
pub fn check_grounding(
    plan: &PlanIR,
    strictness: &StrictnessProfile,
) -> (
    Vec<CheckItem>,
    Vec<CheckItem>,
    Vec<CheckItem>,
    Vec<GroundingOutcome>,
) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();
    let mut outcomes = Vec::new();

    if plan.requirements.is_empty() || plan.tasks.is_empty() {
        info.push(make_item(
            "info",
            "grounding_skipped",
            "Plan".into(),
            String::new(),
            "Grounding check skipped: no requirements or no tasks".into(),
            None,
        ));
        return (blockers, warnings, info, outcomes);
    }

    let base_sig = signature_from_planir(plan);
    // Each requirement grounds against this change's tasks plus the tasks of the change that wrote
    // its body. Per requirement rather than as one union, so an id from one requirement's history
    // cannot bind a slot in another's.
    let mut history_sigs: std::collections::BTreeMap<String, Signature> =
        std::collections::BTreeMap::new();
    let grounder = RuleGrounder;

    for req in &plan.requirements {
        // Skip MAY and Informational requirements — they are informational
        // and don't need grounding.
        if req.strength == Rfc2119Strength::May
            || crate::translator::classify(&req.statement)
                == crate::ir::ConstraintCategory::Informational
        {
            info.push(make_item(
                "info",
                "grounding_may_skipped",
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                format!(
                    "MAY requirement '{}' skipped — informational, not grounded",
                    truncate(&req.statement, 80),
                ),
                None,
            ));
            // Don't push to outcomes — skipped requirements don't affect PatternUngrounded
            continue;
        }

        let sig = match &req.grounded_in {
            None => &base_sig,
            Some(history) => history_sigs
                .entry(history.change.clone())
                .or_insert_with(|| with_history_tasks(&base_sig, &history.tasks)),
        };
        let result = grounder.ground(&req.statement, sig);

        // Multi-keyword pre-check: detect if multiple predicates matched
        let mut matched_predicates: Vec<&str> = result
            .candidates
            .iter()
            .filter(|c| c.confidence > 0.5)
            .map(|c| c.predicate.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        matched_predicates.sort();

        if matched_predicates.len() > 1 {
            let keywords = matched_predicates.join(" and ");
            let clauses = split_constraint_clauses(&req.statement);
            let clause_list: Vec<String> = clauses
                .iter()
                .enumerate()
                .map(|(idx, c)| format!("  Requirement {idx}: {}", crate::util::truncate(c, 90)))
                .collect();
            let detail = format!(
                "GROUNDING AMBIGUITY: statement matches multiple temporal keywords ({}). \
                 Split into separate requirements, one temporal keyword per requirement.",
                keywords,
            );
            let fix = if clauses.len() > 1 {
                format!(
                    "Split into one requirement per temporal constraint. Suggested split:\n{}\nEach must use exactly one of: {}",
                    clause_list.join("\n"),
                    keywords,
                )
            } else {
                format!(
                    "Split the requirement into separate statements, each using only one of: {}",
                    keywords,
                )
            };

            match strictness {
                StrictnessProfile::Strict => blockers.push(make_item(
                    "blocker",
                    "grounding_ambiguous_multi_keyword",
                    format!("Requirement '{}'", req.id),
                    format!("{}:{}", req.source.file, req.source.start_line),
                    detail,
                    Some(fix),
                )),
                StrictnessProfile::Moderate => warnings.push(make_item(
                    "warning",
                    "grounding_ambiguous_multi_keyword",
                    format!("Requirement '{}'", req.id),
                    format!("{}:{}", req.source.file, req.source.start_line),
                    detail,
                    Some(fix),
                )),
                StrictnessProfile::Lax => info.push(make_item(
                    "info",
                    "grounding_ambiguous_multi_keyword",
                    format!("Requirement '{}'", req.id),
                    format!("{}:{}", req.source.file, req.source.start_line),
                    detail,
                    Some(fix),
                )),
            }

            // Still record the outcome for downstream use
            outcomes.push(GroundingOutcome {
                requirement_id: req.id.clone(),
                failed: true,
                status: GroundingStatus::Ambiguous,
            });
            continue;
        }

        let failed = matches!(
            result.status,
            GroundingStatus::Ungroundable
                | GroundingStatus::Ambiguous
                | GroundingStatus::Incomplete
                | GroundingStatus::Contradictory
        );

        outcomes.push(GroundingOutcome {
            requirement_id: req.id.clone(),
            failed,
            status: result.status.clone(),
        });

        match result.status {
            GroundingStatus::Grounded => {
                // Pass silently
            }
            GroundingStatus::Ambiguous | GroundingStatus::Incomplete => {
                let first = result.candidates.first();
                let predicate = first.map(|c| c.predicate.clone()).unwrap_or_default();
                let found = first.map(|c| c.arguments.len()).unwrap_or(0);
                let names = first.map(|c| c.names().join(", ")).unwrap_or_default();
                let exact = first.map(|c| c.exact).unwrap_or(true);
                let confidence = first.map(|c| c.confidence).unwrap_or(0.0);
                let expected = sig
                    .predicates
                    .iter()
                    .find(|p| p.name == predicate)
                    .map(|p| p.arguments.len())
                    .unwrap_or(0);
                let location = format!("{}:{}", req.source.file, req.source.start_line);
                let element = format!("Requirement '{}'", req.id);

                // A task id cited by the text that this signature does not declare is a
                // different diagnosis from an under-filled predicate: the author already
                // named a task, the change simply does not own it.
                if !result.absent_identifiers.is_empty() {
                    let missing = result.absent_identifiers.join(", ");
                    let detail = if req.grounded_in.is_none() {
                        format!(
                            "Requirement '{}' cites {} — not a task of this change, and no archived change records this requirement, so the ids belong to neither",
                            truncate(&req.statement, 80),
                            missing,
                        )
                    } else {
                        format!(
                            "Requirement '{}' cites {} — not a task of this change",
                            truncate(&req.statement, 80),
                            missing,
                        )
                    };
                    let fix =
                        "Cite a task this change defines, or add the task to tasks.md".to_string();
                    match strictness {
                        StrictnessProfile::Strict => blockers.push(make_item(
                            "blocker",
                            "grounding_absent_task",
                            element,
                            location,
                            detail,
                            Some(fix),
                        )),
                        StrictnessProfile::Moderate => warnings.push(make_item(
                            "warning",
                            "grounding_absent_task",
                            element,
                            location,
                            detail,
                            Some(fix),
                        )),
                        StrictnessProfile::Lax => info.push(make_item(
                            "info",
                            "grounding_absent_task",
                            element,
                            location,
                            detail,
                            Some(fix),
                        )),
                    }
                    continue;
                }

                // A task the author named that no slot used is a grounding weakness, not a
                // statement defect: the slots were filled from something weaker. Saying so
                // separates "you wrote it wrong" from "the binder chose badly".
                let unused = if result.unused_identifiers.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; the text names {} but no slot used it",
                        result.unused_identifiers.join(", ")
                    )
                };
                let detail = format!(
                    "Ambiguous grounding for '{}' — confidence {:.2}: {} expects {} argument(s), found {}{}{}{}",
                    truncate(&req.statement, 80),
                    confidence,
                    if predicate.is_empty() {
                        "no predicate"
                    } else {
                        predicate.as_str()
                    },
                    expected,
                    found,
                    if names.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", names)
                    },
                    if !exact && found > 0 {
                        ", at least one matched by alias"
                    } else {
                        ""
                    },
                    unused,
                );
                let fix =
                    "Add explicit task ID references (e.g., 'T2.1') to the requirement statement"
                        .to_string();
                match strictness {
                    StrictnessProfile::Strict => blockers.push(make_item(
                        "blocker",
                        "grounding_ambiguous",
                        element,
                        location,
                        detail,
                        Some(fix),
                    )),
                    StrictnessProfile::Moderate => warnings.push(make_item(
                        "warning",
                        "grounding_ambiguous",
                        element,
                        location,
                        detail,
                        Some(fix),
                    )),
                    StrictnessProfile::Lax => info.push(make_item(
                        "info",
                        "grounding_ambiguous",
                        element,
                        location,
                        detail,
                        Some(fix),
                    )),
                }
            }
            GroundingStatus::Contradictory => {
                let names = result
                    .candidates
                    .first()
                    .map(|c| c.names().join(", "))
                    .unwrap_or_default();
                let unused = if result.unused_identifiers.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; the text also names {}, which no slot used",
                        result.unused_identifiers.join(", ")
                    )
                };
                let detail = format!(
                    "Contradictory grounding for '{}' — one task fills both slots ({}){}",
                    truncate(&req.statement, 80),
                    names,
                    unused,
                );
                let fix =
                    "Use two different task ids, or drop a relation a task cannot have with itself"
                        .to_string();
                let element = format!("Requirement '{}'", req.id);
                let location = format!("{}:{}", req.source.file, req.source.start_line);
                match strictness {
                    StrictnessProfile::Strict => blockers.push(make_item(
                        "blocker",
                        "grounding_contradictory",
                        element,
                        location,
                        detail,
                        Some(fix),
                    )),
                    StrictnessProfile::Moderate => warnings.push(make_item(
                        "warning",
                        "grounding_contradictory",
                        element,
                        location,
                        detail,
                        Some(fix),
                    )),
                    StrictnessProfile::Lax => info.push(make_item(
                        "info",
                        "grounding_contradictory",
                        element,
                        location,
                        detail,
                        Some(fix),
                    )),
                }
            }
            GroundingStatus::Ungroundable => {
                // Check if any constant name from the Signature appears in the text
                let has_task_id = sig.constants.iter().any(|c| {
                    let id = c.name.trim_start_matches('T');
                    crate::util::mentions_task(&req.statement, id)
                        || c.aliases
                            .iter()
                            .any(|a| req.statement.to_lowercase().contains(a))
                });

                let (detail, fix) = if has_task_id {
                    // Task IDs present but no predicate keyword matched
                    let detail = format!(
                        "Ungroundable requirement '{}' — no matching predicate keyword found \
                         (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE)",
                        truncate(&req.statement, 80),
                    );
                    let fix = "Add a temporal keyword to the requirement statement".to_string();
                    (detail, fix)
                } else {
                    // No task IDs and no predicate keyword
                    let detail = format!(
                        "Ungroundable requirement '{}' — no matching task or predicate found",
                        truncate(&req.statement, 80),
                    );
                    let fix =
                        "Add a task ID reference (e.g., 'T5.1') or a known predicate keyword \
                                (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE)"
                            .to_string();
                    (detail, fix)
                };

                match strictness {
                    StrictnessProfile::Strict | StrictnessProfile::Moderate => {
                        blockers.push(make_item(
                            "blocker",
                            "grounding_ungroundable",
                            format!("Requirement '{}'", req.id),
                            format!("{}:{}", req.source.file, req.source.start_line),
                            detail,
                            Some(fix),
                        ))
                    }
                    StrictnessProfile::Lax => warnings.push(make_item(
                        "warning",
                        "grounding_ungroundable",
                        format!("Requirement '{}'", req.id),
                        format!("{}:{}", req.source.file, req.source.start_line),
                        detail,
                        Some(fix),
                    )),
                }
            }
        }
    }

    (blockers, warnings, info, outcomes)
}

#[cfg(test)]
mod tests;
