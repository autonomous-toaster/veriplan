//! Signature construction for the grounding check.
//!
//! Split out of mod.rs to stay under the file-length gate. This half answers which ids a
//! requirement body may refer to; the half left behind answers what those references resolve to.

pub use groundcontrol::types::{GroundingStatus, Signature};

use super::looks_like_task_id;

use crate::ir::PlanIR;

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

/// The base signature plus the task ids of the change that wrote a requirement's body.
///
/// Archived tasks contribute no aliases: their descriptions are not read, and inventing words from
/// ids would be the incidental-alias problem again. A published body cites ids, which is enough.
pub(super) fn with_history_tasks(base: &Signature, tasks: &[String]) -> Signature {
    let mut sig = base.clone();
    for id in tasks {
        let name = format!("T{id}");
        if sig.constants.iter().any(|c| c.name == name) {
            // A live task of the same id wins: it is work this change will do.
            continue;
        }
        sig.constants.push(groundcontrol::types::ConstantDef {
            name,
            type_name: "task_id".into(),
            aliases: vec![id.clone()],
        });
    }
    sig.constants.sort_by(|a, b| a.name.cmp(&b.name));
    sig
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
    // A single word from a task description is NOT an alias for the task: one shared
    // adjective would otherwise outrank the task ids the requirement actually names,
    // and the grounder would bind the wrong constant to a slot. Only the full
    // description and its leading phrases are aliases.
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
pub(crate) fn split_constraint_clauses(statement: &str) -> Vec<&str> {
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
