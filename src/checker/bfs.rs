use std::collections::HashMap;

use crate::checker::{VerificationResult, Violation};
use crate::ir::*;
use crate::translator;

fn run_bfs_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();
    let mut violations = Vec::new();

    for state_bits in 0u64..(1u64 << plan.tasks.len().min(20)) {
        let state = build_state(state_bits, plan);
        for c in &formalizable {
            check_and_record_violation(c, &state, plan, &mut violations, plan_name);
        }
    }

    let valid = violations.is_empty();
    let violations_count = violations.len();
    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: Some(valid),
        violations,
        total_constraints: formalizable.len(),
        satisfied_constraints: if valid {
            formalizable.len()
        } else {
            formalizable.len().saturating_sub(violations_count)
        },
        constraints_summary: vec![],
        skip_reason: None,
    }
}

fn build_state(state_bits: u64, plan: &PlanIR) -> HashMap<String, u8> {
    let mut state: HashMap<String, u8> = HashMap::new();
    for (j, task) in plan.tasks.iter().enumerate() {
        let val = if (state_bits >> j) & 1 == 1 { 1 } else { 0 };
        state.insert(task.id.clone(), val);
    }
    state
}

fn check_and_record_violation(
    c: &translator::TranslatedConstraint,
    state: &HashMap<String, u8>,
    plan: &PlanIR,
    violations: &mut Vec<Violation>,
    plan_name: &str,
) {
    if let Some(ltl) = &c.ltl
        && !evaluate_ltl(ltl, state, plan)
        && !violations.iter().any(|v: &Violation| v.constraint_id == c.requirement_id)
    {
        let state_str: Vec<String> = state
            .iter()
            .filter(|(_, v)| **v == 1)
            .map(|(k, _)| k.clone())
            .collect();
        violations.push(Violation {
            constraint_id: c.requirement_id.clone(),
            requirement_statement: c.statement.clone(),
            ltl: ltl.clone(),
            category: format!("{:?}", c.category),
            state: state_str.join(", "),
            task_source: None,
            req_source: None,
            suggested_fix: None,
            plan: plan_name.to_string(),
        });
    }
}

/// Simple LTL evaluator for BFS explorer.
fn evaluate_ltl(ltl: &str, state: &HashMap<String, u8>, plan: &PlanIR) -> bool {
    // Very basic evaluation: parse simple G ( condition ) patterns
    if let Some(inner) = ltl.strip_prefix("G ( ").and_then(|s| s.strip_suffix(" )")) {
        let inner = inner.trim();
        return evaluate_ltl_condition(inner, state, plan);
    }
    // For anything we can't evaluate, assume pass (conservative)
    true
}

fn evaluate_ltl_condition(cond: &str, state: &HashMap<String, u8>, _plan: &PlanIR) -> bool {
    // Split on && for compound conditions
    if cond.contains("&&") {
        return cond
            .split("&&")
            .all(|part| evaluate_ltl_condition(part.trim(), state, _plan));
    }

    let cond = cond.trim();

    // Negation: !(expr)
    if let Some(inner) = cond.strip_prefix("!(").and_then(|s| s.strip_suffix(')')) {
        return !evaluate_ltl_condition(inner, state, _plan);
    }

    // active_X <-> active_Y (bidirectional) — check before -> to avoid matching <->
    if let Some((left, right)) = cond.split_once("<->") {
        let left_val = evaluate_ltl_atom(left.trim(), state);
        let right_val = evaluate_ltl_atom(right.trim(), state);
        return left_val == right_val;
    }

    // active_X -> done_Y (implication)
    if let Some((ante, conseq)) = cond.split_once("->") {
        let ante = ante.trim();
        let conseq = conseq.trim();
        let ante_val = evaluate_ltl_atom(ante, state);
        let conseq_val = evaluate_ltl_atom(conseq, state);
        return !ante_val || conseq_val; // implication: a -> b ≡ !a || b
    }

    // F active_X (eventually)
    if let Some(arg) = cond.strip_prefix("F ") {
        return evaluate_ltl_atom(arg.trim(), state);
    }

    // Atomic
    evaluate_ltl_atom(cond, state)
}

fn evaluate_ltl_atom(atom: &str, state: &HashMap<String, u8>) -> bool {
    let atom = atom.trim();

    // Negation
    if let Some(var) = atom.strip_prefix('!') {
        return state.get(var).copied().unwrap_or(0) == 0;
    }

    // Check if this is a variable name
    let val = state.get(atom).copied().unwrap_or(0);
    val == 1
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

pub(crate) fn find_predecessors(plan: &PlanIR, task_id: &str) -> Vec<String> {
    // Find which phase this task belongs to
    for (idx, phase) in plan.phases.iter().enumerate() {
        if !phase.task_ids.iter().any(|id| id == task_id) {
            continue;
        }
        // Concurrent phase: no intra-phase ordering.
        // All tasks in this phase wait for the previous phase to complete.
        if phase.mode == crate::ir::PhaseMode::Concurrent {
            if idx > 0 {
                let prev_phase = &plan.phases[idx - 1];
                if let Some(last_id) = prev_phase.task_ids.last() {
                    return vec![last_id.clone()];
                }
            }
            return Vec::new();
        }
        // Sequential phase: current behavior — previous task in same phase
        if let Some(pos) = phase.task_ids.iter().position(|id| id == task_id) {
            if pos > 0 {
                return vec![phase.task_ids[pos - 1].clone()];
            }
            // First task in phase: wait for last task of previous phase
            if idx > 0 {
                let prev_phase = &plan.phases[idx - 1];
                if let Some(last_id) = prev_phase.task_ids.last() {
                    return vec![last_id.clone()];
                }
            }
        }
        return Vec::new();
    }
    Vec::new()
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

pub(crate) fn timeout_command(
    cmd: &std::path::Path,
    args: &[&str],
    timeout_secs: u64,
) -> std::io::Result<std::process::Output> {
    // Use a thread-based timeout approach
    use std::sync::mpsc;
    use std::time::Duration;

    let cmd_path = cmd.to_path_buf();
    let args_owned: Vec<String> = args.iter().map(|a| a.to_string()).collect();

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = std::process::Command::new(&cmd_path)
            .args(&args_owned)
            .output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("command timed out after {}s", timeout_secs),
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "command thread disconnected",
        )),
    }
}

/// Extract task IDs like "T4.2" from a statement or "t4_2" from LTL.
/// Extract task IDs like "T4.2" or "t4_2" (from LTL) from text.
pub(crate) fn extract_task_ids(text: &str) -> Vec<String> {
    // Try T-prefixed format first: T4.2, T6.1, etc.
    let mut ids = extract_prefixed_ids(text, b'T', |s| {
        if s.contains('.') && s.chars().all(|c| c.is_ascii_digit() || c == '.') {
            Some(s.to_string())
        } else {
            None
        }
    });

    // If none found, try lowercase format: t4_2, t6_1 (from LTL active_t4_2)
    if ids.is_empty() {
        ids = extract_prefixed_ids(text, b't', |s| {
            if let Some(underscore) = s.find('_') {
                let major = &s[..underscore];
                let minor = &s[underscore + 1..];
                Some(format!("{}.{}", major, minor))
            } else {
                None
            }
        });
    }

    ids.sort();
    ids.dedup();
    ids
}

/// Extract IDs prefixed with a specific byte (e.g., 'T' or 't').
fn extract_prefixed_ids<F>(text: &str, prefix: u8, mut transform: F) -> Vec<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut ids = Vec::new();
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == prefix && i + 1 < n && bytes[i + 1].is_ascii_digit() {
            i += 1;
            let start = i;
            while i < n && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && let Some(transformed) = transform(s) {
                    ids.push(transformed);
                }
        } else {
            i += 1;
        }
    }
    ids
}

/// Generate human-readable guidance for a constraint violation.
pub(crate) fn suggest_fix(
    category: &crate::ir::ConstraintCategory,
    ltl: &str,
    _req_id: &str,
) -> Option<String> {
    let task_ids = extract_task_ids(ltl);
    let task_list = if task_ids.is_empty() {
        String::new()
    } else if task_ids.len() <= 2 {
        format!(
            " tasks {} and {}",
            task_ids.first().unwrap(),
            task_ids.get(1).unwrap_or(&task_ids[0])
        )
    } else {
        format!(" tasks {}", task_ids.join(", "))
    };

    match category {
        crate::ir::ConstraintCategory::ConcurrentEvents => {
            if task_list.is_empty() {
                Some(
                    "The model runs tasks sequentially by phase — two tasks cannot be active simultaneously.\n  Either remove the CONCURRENTLY keyword from this requirement, or restructure the plan\n  so these tasks can overlap in execution."
                        .into(),
                )
            } else {
                Some(format!(
            "The model runs tasks sequentially by phase — two tasks cannot be active simultaneously.\n  The requirement references{} but they execute one after another within the same phase.\n  Either remove the CONCURRENTLY keyword from this requirement, or restructure the plan\n  to put these tasks in a concurrent phase.",
                    task_list
                ))
            }
        }
        crate::ir::ConstraintCategory::Conditional => {
            Some(
                "The trigger task fails non-deterministically but the consequent task never activates.\n  IF...THEN is designed for **failure-recovery** patterns (e.g. 'IF T1.1 fails THEN T2.1 SHALL run').\n  For **branching/decision logic** (e.g. 'IF X THEN A, IF not X THEN B'), use Sequential ordering instead:\n  \"T1.5 SHALL complete BEFORE T1.4 SHALL run\".\n  Otherwise mark this constraint as aspirational by removing IF...THEN."
                    .into(),
            )
        }
        crate::ir::ConstraintCategory::Exclusive => {
            if task_list.is_empty() {
                Some(
                    "Two tasks can be active simultaneously in the model — they are not mutually exclusive.\n  Either add a phase ordering between them, or mark this constraint as aspirational\n  by removing AT MOST ONE / NOT CONCURRENTLY."
                        .into(),
                )
            } else {
                Some(format!(
            "Tasks{} can both be active at the same time in the model — they are not mutually exclusive.\n  Either add a phase ordering between them (different phases execute sequentially),\n  or mark this constraint as aspirational by removing AT MOST ONE / NOT CONCURRENTLY.",
                    task_list
                ))
            }
        }
        crate::ir::ConstraintCategory::SequentialOrder => {
            Some(
                "The before-task does not always complete before the after-task starts in the model.\n  Either ensure the before-task is in an earlier phase, or mark this constraint\n  as aspirational by removing BEFORE / AFTER."
                    .into(),
            )
        }
        _ => None,
    }
}

pub(crate) fn simple_result(
    plan_name: &str,
    conv_report: ConvertibilityReport,
    _constraints: Vec<translator::TranslatedConstraint>,
) -> VerificationResult {
    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: None,
        violations: vec![],
        total_constraints: 0,
        satisfied_constraints: 0,
        constraints_summary: vec![],
        skip_reason: Some("Model check error".into()),
    }
}


#[cfg(test)]
#[path = "bfs_tests.rs"]
mod tests;
