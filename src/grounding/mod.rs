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

/// Build a groundcontrol Signature from a PlanIR.
///
/// Maps each PlanIR Task to a ConstantDef (name = "T{id}", aliases from description),
/// includes all 6 predicate definitions with correct argument slots, and type definitions.
pub fn signature_from_planir(plan: &PlanIR) -> Signature {
    let types = vec![
        groundcontrol::types::TypeDef {
            name: "task_id".into(),
        },
        groundcontrol::types::TypeDef {
            name: "phase_name".into(),
        },
    ];

    let predicates = vec![
        groundcontrol::types::PredicateDef {
            name: "BEFORE".into(),
            arguments: vec![
                groundcontrol::types::ArgSlot {
                    name: "earlier".into(),
                    type_name: "task_id".into(),
                },
                groundcontrol::types::ArgSlot {
                    name: "later".into(),
                    type_name: "task_id".into(),
                },
            ],
        },
        groundcontrol::types::PredicateDef {
            name: "AFTER".into(),
            arguments: vec![
                groundcontrol::types::ArgSlot {
                    name: "earlier".into(),
                    type_name: "task_id".into(),
                },
                groundcontrol::types::ArgSlot {
                    name: "later".into(),
                    type_name: "task_id".into(),
                },
            ],
        },
        groundcontrol::types::PredicateDef {
            name: "CONCURRENTLY".into(),
            arguments: vec![
                groundcontrol::types::ArgSlot {
                    name: "a".into(),
                    type_name: "task_id".into(),
                },
                groundcontrol::types::ArgSlot {
                    name: "b".into(),
                    type_name: "task_id".into(),
                },
            ],
        },
        groundcontrol::types::PredicateDef {
            name: "IF_THEN".into(),
            arguments: vec![
                groundcontrol::types::ArgSlot {
                    name: "trigger".into(),
                    type_name: "task_id".into(),
                },
                groundcontrol::types::ArgSlot {
                    name: "consequent".into(),
                    type_name: "task_id".into(),
                },
            ],
        },
        groundcontrol::types::PredicateDef {
            name: "ALWAYS".into(),
            arguments: vec![groundcontrol::types::ArgSlot {
                name: "target".into(),
                type_name: "task_id".into(),
            }],
        },
        groundcontrol::types::PredicateDef {
            name: "AT_MOST_ONE".into(),
            arguments: vec![
                groundcontrol::types::ArgSlot {
                    name: "a".into(),
                    type_name: "task_id".into(),
                },
                groundcontrol::types::ArgSlot {
                    name: "b".into(),
                    type_name: "task_id".into(),
                },
            ],
        },
    ];

    let mut constants: Vec<groundcontrol::types::ConstantDef> = plan
        .tasks
        .iter()
        .map(|task| {
            let aliases = build_aliases(&task.id, &task.description);
            groundcontrol::types::ConstantDef {
                name: format!("T{}", task.id),
                type_name: "task_id".into(),
                aliases,
            }
        })
        .collect();
    constants.sort_by(|a, b| a.name.cmp(&b.name));

    Signature {
        types,
        predicates,
        constants,
    }
}

/// Build alias list from task description (mirrors groundcontrol's approach).
fn build_aliases(id: &str, desc: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let lower = desc.to_lowercase();
    aliases.push(lower.clone());

    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() > 3 {
        aliases.push(words[..3].join(" "));
    }
    if words.len() > 5 {
        aliases.push(words[..5].join(" "));
    }
    for w in &words {
        if w.len() > 4 && !aliases.contains(&w.to_string()) {
            aliases.push(w.to_string());
        }
    }
    aliases.push(id.to_string());
    aliases
}

/// Result of grounding a single requirement.
#[derive(Debug, Clone)]
pub struct GroundingOutcome {
    /// The requirement ID that was grounded.
    pub requirement_id: String,
    /// Whether grounding failed (ungroundable or ambiguous in strict mode).
    pub failed: bool,
    /// The grounding status.
    pub status: GroundingStatus,
}

/// Split a requirement statement into its individual temporal-constraint
/// clauses. A clause is a sentence that contains a task-ID reference; we split
/// on sentence boundaries (`.` + space) where the following text begins a new
/// task-ID reference (e.g. "T4.7 SHALL inspect AFTER T4.4 ... T4.6 SHALL ...").
///
/// Returns the clauses in order, falling back to the whole statement if it
/// cannot be split (e.g. a single sentence).
fn split_constraint_clauses(statement: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0usize;
    let bytes = statement.as_bytes();
    // Heuristic: a new clause begins where a task-ID reference (T<N>.<N>) is
    // preceded by a sentence boundary (. or newline).
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        // Look for ". " boundary.
        if bytes[i] == b'.' && i + 2 < bytes.len() && bytes[i + 1] == b' ' {
            let rest = &statement[i + 2..];
            // A task-ID reference begins the next clause.
            if looks_like_task_id(rest) {
                let clause = statement[start..=i].trim();
                if !clause.is_empty() {
                    clauses.push(clause);
                }
                start = i + 2;
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    if start == 0 {
        // No split found; single clause.
        return vec![statement.trim()];
    }
    let last = statement[start..].trim();
    if !last.is_empty() {
        clauses.push(last);
    }
    if clauses.is_empty() {
        vec![statement.trim()]
    } else {
        clauses
    }
}

/// Whether `s` (the start of a sentence) begins with a task-ID reference,
/// e.g. "T4.7 SHALL" or "T1.2 SHALL complete".
fn looks_like_task_id(s: &str) -> bool {
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
        info.push(CheckItem {
            severity: "info".into(),
            check: "grounding_skipped".into(),
            element: "Plan".into(),
            location: String::new(),
            detail: "Grounding check skipped: no requirements or no tasks".into(),
            fix: None,
        });
        return (blockers, warnings, info, outcomes);
    }

    let sig = signature_from_planir(plan);
    let grounder = RuleGrounder;

    for req in &plan.requirements {
        // Skip MAY and Informational requirements — they are informational
        // and don't need grounding.
        if req.strength == Rfc2119Strength::May
            || crate::translator::classify(&req.statement) == crate::ir::ConstraintCategory::Informational
        {
            info.push(CheckItem {
                severity: "info".into(),
                check: "grounding_may_skipped".into(),
                element: format!("Requirement '{}'", req.id),
                location: format!("{}:{}", req.source.file, req.source.start_line),
                detail: format!(
                    "MAY requirement '{}' skipped — informational, not grounded",
                    truncate(&req.statement, 80),
                ),
                fix: None,
            });
            // Don't push to outcomes — skipped requirements don't affect PatternUngrounded
            continue;
        }

        let result = grounder.ground(&req.statement, &sig);

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
                StrictnessProfile::Strict => blockers.push(CheckItem {
                    severity: "blocker".into(),
                    check: "grounding_ambiguous_multi_keyword".into(),
                    element: format!("Requirement '{}'", req.id),
                    location: format!("{}:{}", req.source.file, req.source.start_line),
                    detail,
                    fix: Some(fix),
                }),
                StrictnessProfile::Moderate => warnings.push(CheckItem {
                    severity: "warning".into(),
                    check: "grounding_ambiguous_multi_keyword".into(),
                    element: format!("Requirement '{}'", req.id),
                    location: format!("{}:{}", req.source.file, req.source.start_line),
                    detail,
                    fix: Some(fix),
                }),
                StrictnessProfile::Lax => info.push(CheckItem {
                    severity: "info".into(),
                    check: "grounding_ambiguous_multi_keyword".into(),
                    element: format!("Requirement '{}'", req.id),
                    location: format!("{}:{}", req.source.file, req.source.start_line),
                    detail,
                    fix: Some(fix),
                }),
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
            GroundingStatus::Ungroundable | GroundingStatus::Ambiguous
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
            GroundingStatus::Ambiguous => {
                let best_confidence = result
                    .candidates
                    .first()
                    .map(|c| c.confidence)
                    .unwrap_or(0.0);
                let diagnostic = result
                    .candidates
                    .first()
                    .map(|c| c.diagnostic.as_str())
                    .unwrap_or("");
                let detail = format!(
                    "Ambiguous grounding for '{}' — low confidence ({:.2}){}{}",
                    truncate(&req.statement, 80),
                    best_confidence,
                    if diagnostic.is_empty() { "" } else { ": " },
                    diagnostic,
                );
                let fix =
                    "Add explicit task ID references (e.g., 'T2.1') to the requirement statement"
                        .to_string();

                match strictness {
                    StrictnessProfile::Strict => blockers.push(CheckItem {
                        severity: "blocker".into(),
                        check: "grounding_ambiguous".into(),
                        element: format!("Requirement '{}'", req.id),
                        location: format!("{}:{}", req.source.file, req.source.start_line),
                        detail,
                        fix: Some(fix),
                    }),
                    StrictnessProfile::Moderate => warnings.push(CheckItem {
                        severity: "warning".into(),
                        check: "grounding_ambiguous".into(),
                        element: format!("Requirement '{}'", req.id),
                        location: format!("{}:{}", req.source.file, req.source.start_line),
                        detail,
                        fix: Some(fix),
                    }),
                    StrictnessProfile::Lax => info.push(CheckItem {
                        severity: "info".into(),
                        check: "grounding_ambiguous".into(),
                        element: format!("Requirement '{}'", req.id),
                        location: format!("{}:{}", req.source.file, req.source.start_line),
                        detail,
                        fix: Some(fix),
                    }),
                }
            }
            GroundingStatus::Ungroundable => {
                // Check if any constant name from the Signature appears in the text
                let has_task_id = sig.constants.iter().any(|c| {
                    req.statement.contains(&c.name)
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
                        blockers.push(CheckItem {
                            severity: "blocker".into(),
                            check: "grounding_ungroundable".into(),
                            element: format!("Requirement '{}'", req.id),
                            location: format!("{}:{}", req.source.file, req.source.start_line),
                            detail,
                            fix: Some(fix),
                        })
                    }
                    StrictnessProfile::Lax => warnings.push(CheckItem {
                        severity: "warning".into(),
                        check: "grounding_ungroundable".into(),
                        element: format!("Requirement '{}'", req.id),
                        location: format!("{}:{}", req.source.file, req.source.start_line),
                        detail,
                        fix: Some(fix),
                    }),
                }
            }
        }
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "grounding_summary".into(),
        element: "Plan".into(),
        location: String::new(),
        detail: format!(
            "Grounding check complete: {} requirements checked",
            plan.requirements.len()
        ),
        fix: None,
    });

    (blockers, warnings, info, outcomes)
}

#[cfg(test)]
mod tests;
