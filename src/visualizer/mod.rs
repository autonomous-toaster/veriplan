//! Visualizer: generates state-machine diagrams from PlanIR + constraints.
//!
//! Three formats:
//! - mermaid: flowchart with phase subgraphs, constraint edges, optional pass/fail colors
//! - dot: Graphviz digraph with clusters
//! - markdown: simple table

use std::fmt::Write;

use crate::ir::{PhaseMode, PlanIR};
use crate::translator::TranslatedConstraint;

/// Generate a Mermaid flowchart diagram.
pub fn format_mermaid(plan: &PlanIR, constraints: &[TranslatedConstraint]) -> String {
    let mut s = String::new();
    s.push_str("flowchart TB\n");
    s.push_str("%% Legend:\n");
    s.push_str("%%   ✅ Node = task completed (checked in tasks.md)\n");
    s.push_str("%%   Plain node = task pending\n");
    s.push_str("%%   Dashed arrow = spec constraint between tasks\n");
    s.push_str("%%   Subgraph = phase group\n");
    s.push('\n');

    // ── Phase subgraphs ──
    for (pi, phase) in plan.phases.iter().enumerate() {
        let mode = match phase.mode {
            PhaseMode::Concurrent => " [concurrent]",
            PhaseMode::Sequential => "",
        };
        let cleaned_name = clean_label(&phase.name);
        let label = format!("Phase {}: {}{}", pi + 1, cleaned_name, mode);

        // Tasks in this phase
        let tasks_in_phase: Vec<&crate::ir::Task> = plan
            .tasks
            .iter()
            .filter(|t| phase.task_ids.contains(&t.id))
            .collect();

        let sub_id = format!("Phase{}", pi);
        writeln!(s, "    subgraph {}[\"{}\"]", sub_id, label).ok();

        for task in &tasks_in_phase {
            let nid = node_id(&task.id);
            let desc = clean_label(&truncate(&task.description, 40));
            if task.checked {
                writeln!(s, "        {}[\"✅ T{}: {}\"]", nid, task.id, desc).ok();
            } else {
                writeln!(s, "        {}[\"T{}: {}\"]", nid, task.id, desc).ok();
            }
        }
        writeln!(s, "    end").ok();
    }

    // ── Structural edges: unlabeled thin arrows between phases ──
    for w in plan.phases.windows(2) {
        let prev_last = plan.tasks.iter().rfind(|t| w[0].task_ids.contains(&t.id));
        let next_first = plan.tasks.iter().find(|t| w[1].task_ids.contains(&t.id));
        if let (Some(prev), Some(next)) = (prev_last, next_first) {
            writeln!(s, "    {} --> {}", node_id(&prev.id), node_id(&next.id)).ok();
        }
    }

    // ── Constraint edges ──
    let mut colored_edges: Vec<(usize, &str)> = Vec::new();
    let mut edge_idx = 0usize;
    for c in constraints {
        if c.ltl.is_some() {
            let task_ids = crate::translator::extract_task_refs(&c.statement, plan);
            if task_ids.len() >= 2 {
                let label = display_label(c);
                let edge_style = if c.category == crate::ir::ConstraintCategory::SequentialOrder {
                    "==>"
                } else {
                    "-.->"
                };

                for pair in task_ids.windows(2) {
                    let a = node_id(&pair[0]);
                    let b = node_id(&pair[1]);
                    let (color, status_mark) = ("", "");

                    writeln!(
                        s,
                        "    {} {}|\"{}{}\"| {}",
                        a, edge_style, label, status_mark, b
                    )
                    .ok();
                    if !color.is_empty() {
                        colored_edges.push((edge_idx, color));
                    }
                    edge_idx += 1;
                }
            }
        }
    }

    // ── linkStyle for colored edges (must use indices, not "default") ──
    for (idx, color) in &colored_edges {
        writeln!(s, "    linkStyle {} stroke:{},stroke-width:2px", idx, color).ok();
    }

    s
}

/// Generate a Graphviz DOT digraph.
pub fn format_dot(plan: &PlanIR, constraints: &[TranslatedConstraint]) -> String {
    let mut s = String::new();
    s.push_str("digraph plan {\n");
    s.push_str("    rankdir=TB;\n");
    s.push_str("    node [shape=box, style=rounded];\n");
    s.push_str("    edge [fontsize=10];\n\n");

    // ── Phase clusters ──
    for (pi, phase) in plan.phases.iter().enumerate() {
        let mode = match phase.mode {
            PhaseMode::Concurrent => " [concurrent]",
            PhaseMode::Sequential => "",
        };
        let cleaned_name = clean_label(&phase.name);
        writeln!(
            s,
            "    subgraph cluster_phase{} {{ label=\"Phase {}: {}{}\"; color=blue; }}",
            pi,
            pi + 1,
            cleaned_name,
            mode
        )
        .ok();

        let tasks_in_phase: Vec<&crate::ir::Task> = plan
            .tasks
            .iter()
            .filter(|t| phase.task_ids.contains(&t.id))
            .collect();

        for task in &tasks_in_phase {
            let nid = node_id_dot(&task.id);
            let desc = escape_dot(&clean_label(&truncate(&task.description, 50)));
            if task.checked {
                writeln!(
                    s,
                    "    {} [label=\"T{}: {}\", style=filled, fillcolor=\"#e1f5e1\"];",
                    nid, task.id, desc
                )
                .ok();
            } else {
                writeln!(
                    s,
                    "    {} [label=\"T{}: {}\", fillcolor=white];",
                    nid, task.id, desc
                )
                .ok();
            }
        }
    }

    // ── Phase transition edges (unlabeled, gray) ──
    s.push('\n');
    for w in plan.phases.windows(2) {
        let prev_last = plan.tasks.iter().rfind(|t| w[0].task_ids.contains(&t.id));
        let next_first = plan.tasks.iter().find(|t| w[1].task_ids.contains(&t.id));
        if let (Some(prev), Some(next)) = (prev_last, next_first) {
            writeln!(
                s,
                "    {} -> {} [color=gray, style=dotted];",
                node_id_dot(&prev.id),
                node_id_dot(&next.id)
            )
            .ok();
        }
    }

    // ── Constraint edges ──
    s.push('\n');
    for c in constraints {
        if c.ltl.is_some() {
            let task_ids = crate::translator::extract_task_refs(&c.statement, plan);
            if task_ids.len() >= 2 {
                let label = display_label(c);
                let edge_color = "gray";

                let style = if c.category == crate::ir::ConstraintCategory::SequentialOrder {
                    "bold"
                } else {
                    "dashed"
                };

                for pair in task_ids.windows(2) {
                    writeln!(
                        s,
                        "    {} -> {} [label=\"{}\", color={}, style={}];",
                        node_id_dot(&pair[0]),
                        node_id_dot(&pair[1]),
                        label,
                        edge_color,
                        style
                    )
                    .ok();
                }
            }
        }
    }

    s.push_str("}\n");
    s
}

/// Generate a plain markdown table.
pub fn format_markdown(plan: &PlanIR, constraints: &[TranslatedConstraint]) -> String {
    let mut s = String::new();
    s.push_str("| Phase | Task | Status | Constraints |\n");
    s.push_str("|-------|------|--------|-------------|\n");

    for (pi, phase) in plan.phases.iter().enumerate() {
        let tasks_in_phase: Vec<&crate::ir::Task> = plan
            .tasks
            .iter()
            .filter(|t| phase.task_ids.contains(&t.id))
            .collect();

        for (ti, task) in tasks_in_phase.iter().enumerate() {
            let status = if task.checked {
                "✅ done"
            } else {
                "⬜ pending"
            };

            // Collect constraints referencing this task
            let mut con_refs: Vec<String> = Vec::new();
            for c in constraints {
                if c.ltl.is_some() {
                    let refs = crate::translator::extract_task_refs(&c.statement, plan);
                    if refs.contains(&task.id) {
                        con_refs.push(markdown_label(c, plan));
                    }
                }
            }
            let constraints_str = con_refs.join(", ");

            // First row of phase gets the phase name
            let phase_name = if ti == 0 {
                format!("Phase {}: {}", pi + 1, phase.name)
            } else {
                String::new()
            };

            let task_link = source_markdown_link(task);

            writeln!(
                s,
                "| {} | [T{}]({}): {} | {} | {} |",
                phase_name,
                task.id,
                task_link,
                task.description.replace('|', "\\|"),
                status,
                constraints_str
            )
            .ok();
        }
    }

    // ── Task index appendix with source links ──
    s.push_str("\n## Task Index\n");
    s.push_str("| ID | Phase | Description | Source |\n");
    s.push_str("|----|-------|-------------|--------|\n");
    for (pi, phase) in plan.phases.iter().enumerate() {
        let tasks_in_phase: Vec<&crate::ir::Task> = plan
            .tasks
            .iter()
            .filter(|t| phase.task_ids.contains(&t.id))
            .collect();
        for task in &tasks_in_phase {
            let link = source_markdown_link(task);
            let pname = format!("Phase {}: {}", pi + 1, phase.name);
            writeln!(
                s,
                "| T{} | {} | {} | [{}]({}) |",
                task.id,
                pname,
                task.description.replace('|', "\\|"),
                link,
                link
            )
            .ok();
        }
    }

    s
}

// ── Helpers ──

/// Generate a markdown link to the task's source file: `rel/path#L<N>`
fn source_markdown_link(task: &crate::ir::Task) -> String {
    let file = &task.source.file;
    let line = task.source.start_line;
    let rel_path = if let Some(pos) = file.rfind("openspec/") {
        &file[pos..]
    } else {
        file.as_str()
    };
    format!("{}#L{}", rel_path, line)
}

/// Display label for a constraint edge in diagrams (Mermaid/DOT).
/// Overrides "fixed-time" to "sequential" when the statement clearly
/// describes task ordering (BEFORE/AFTER + task references).
fn display_label(c: &TranslatedConstraint) -> String {
    use crate::ir::ConstraintCategory::*;
    if c.category == FixedTime {
        let lower = c.statement.to_lowercase();
        let has_ordering = lower.contains(" before ")
            || lower.contains(" after ")
            || lower.contains("complete before")
            || lower.contains("only after")
            || lower.contains("must finish");
        if has_ordering {
            return "sequential".to_string();
        }
    }
    category_label(&c.category).to_string()
}

/// Label for the markdown table — shows the actual task relationship
/// instead of a repetitive category keyword.
fn markdown_label(c: &TranslatedConstraint, plan: &PlanIR) -> String {
    let refs = crate::translator::extract_task_refs(&c.statement, plan);
    if refs.len() >= 2 {
        let mut s = String::new();
        for (i, r) in refs.iter().enumerate() {
            if i > 0 {
                s.push_str(" → ");
            }
            s.push_str(&format!("T{}", r));
        }
        s
    } else {
        // Fallback: first few words of the statement
        let first = c
            .statement
            .split(['.', '\n'])
            .next()
            .unwrap_or(&c.statement);
        truncate(first.trim(), 30)
    }
}

fn node_id(id: &str) -> String {
    format!("T{}", id.replace('.', "_"))
}

fn node_id_dot(id: &str) -> String {
    format!("t_{}", id.replace('.', "_"))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Find the char boundary at or before max
    let idx = s
        .char_indices()
        .take_while(|(i, _)| *i < max.saturating_sub(1))
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}…", &s[..idx])
}

fn escape_dot(s: &str) -> String {
    s.replace('"', "\\\"").replace('\n', " ").replace('\r', "")
}

fn category_label(cat: &crate::ir::ConstraintCategory) -> &'static str {
    use crate::ir::ConstraintCategory::*;
    match cat {
        SequentialOrder => "sequential",
        Exclusive => "exclusive",
        Conditional => "conditional",
        ConcurrentEvents => "concurrent",
        Global => "global",
        FixedTime => "fixed-time",
        NonFormalizable => "non-formalizable",
        PatternUngrounded => "pattern-ungrounded",
    }
}

/// Strip backticks and other markdown syntax from labels to avoid Mermaid rendering issues.
fn clean_label(s: &str) -> String {
    s.replace(['`', '"'], "'")
}
