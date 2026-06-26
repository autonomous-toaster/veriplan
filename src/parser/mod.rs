//! OpenSpec markdown parser — parse tasks.md and spec.md files into PlanIR.

mod helpers;

use std::path::{Path, PathBuf};

use crate::ir::{PlanIR, Scenario};

/// A spec file with its capability name.
#[derive(Debug, Clone)]
pub struct SpecFile {
    pub capability: String,
    pub path: PathBuf,
}

/// Parse an OpenSpec change directory into a PlanIR.
pub fn parse_plan(change_dir: &Path) -> Result<PlanIR, String> {
    let tasks_path = change_dir.join("tasks.md");
    let specs_dir = change_dir.join("specs");

    if !tasks_path.exists() {
        return Err(format!("tasks.md not found in {}", change_dir.display()));
    }

    let mut parser_instance = tree_sitter::Parser::new();
    let lang = tree_sitter_language_pack::get_language("markdown")
        .map_err(|e| format!("Grammar error: {}", e))?;
    parser_instance
        .set_language(&lang)
        .map_err(|e| format!("Grammar error: {}", e))?;

    let tasks_source =
        std::fs::read_to_string(&tasks_path).map_err(|e| format!("Cannot read tasks.md: {}", e))?;
    let (tasks, phases) = parse_tasks(&mut parser_instance, &tasks_source, &tasks_path)?;

    let mut requirements = Vec::new();
    let mut scenarios = Vec::new();

    if specs_dir.exists() {
        let mut spec_files = Vec::new();
        collect_specs(&specs_dir, &mut spec_files)
            .map_err(|e| format!("Error reading specs directory: {}", e))?;
        spec_files.sort_by(|a, b| a.capability.cmp(&b.capability));

        for spec_file in &spec_files {
            let source = std::fs::read_to_string(&spec_file.path)
                .map_err(|e| format!("Cannot read {}: {}", spec_file.path.display(), e))?;
            let (reqs, standalone_scenarios) = parse_spec(
                &mut parser_instance,
                &source,
                &spec_file.path,
                &spec_file.capability,
            )?;
            requirements.extend(reqs);
            scenarios.extend(standalone_scenarios);
        }
    }

    // Build source map
    let mut source_map = crate::ir::SourceMap::default();
    for task in &tasks {
        source_map
            .tasks
            .insert(task.id.clone(), task.source.clone());
    }
    for req in &requirements {
        source_map
            .requirements
            .insert(req.id.clone(), req.source.clone());
    }

    Ok(PlanIR {
        tasks,
        requirements,
        scenarios,
        phases,
        source_map,
    })
}

/// Collect spec files from a directory tree.
fn collect_specs(dir: &Path, files: &mut Vec<SpecFile>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_specs(&path, files)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md")
            && path.file_stem().and_then(|s| s.to_str()) == Some("spec")
        {
            let capability = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            files.push(SpecFile { capability, path });
        }
    }
    Ok(())
}

/// Walk the tree-sitter tree (non-debug version: cursor-based with callback).
#[allow(clippy::only_used_in_recursion)]
fn explore_tree<'a>(
    cursor: &mut tree_sitter::TreeCursor<'a>,
    node: &tree_sitter::Node<'a>,
    _source: &str,
    f: &mut dyn FnMut(&tree_sitter::Node<'a>, usize),
) {
    f(node, cursor.depth() as usize);
    if cursor.goto_first_child() {
        loop {
            explore_tree(cursor, &cursor.node(), _source, f);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Parse tasks.md into a list of tasks and phases.
pub fn parse_tasks(
    parser: &mut tree_sitter::Parser,
    source: &str,
    path: &Path,
) -> Result<(Vec<crate::ir::Task>, Vec<crate::ir::Phase>), String> {
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse markdown".to_string())?;
    let root = tree.root_node();

    let mut tasks = Vec::new();
    let mut phases = Vec::new();
    let mut current_phase_name = String::new();
    let mut phase_order: Vec<String> = Vec::new();
    let mut phase_task_ids: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut concurrent_phases: std::collections::HashSet<String> = std::collections::HashSet::new();

    let file_name = helpers::file_name_str(path);
    let bytes = source.as_bytes();

    let mut cursor = root.walk();
    explore_tree(
        &mut cursor,
        &root,
        source,
        &mut |node, _depth| match node.kind() {
            "atx_heading" => {
                if let Ok(heading_text) = helpers::node_text(node, bytes) {
                    let trimmed = heading_text.trim().trim_start_matches('#').trim();
                    if trimmed.is_empty() {
                        return;
                    }

                    let heading_level = node
                        .child(0)
                        .map(|n| n.kind())
                        .filter(|k| k.starts_with("atx_h"))
                        .map(|k| k.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);

                    if heading_level == 2 && trimmed.starts_with("Phase") {
                        // Detect [concurrent] marker
                        if let Some(bracket) = trimmed.rfind('[') {
                            let tag = trimmed[bracket + 1..].trim_end_matches(']').trim();
                            if tag.eq_ignore_ascii_case("concurrent")
                                || tag.eq_ignore_ascii_case("parallel")
                            {
                                let phase_name = trimmed[..bracket].trim().to_string();
                                concurrent_phases.insert(phase_name.clone());
                                current_phase_name = phase_name;
                            } else {
                                current_phase_name = trimmed.to_string();
                            }
                        } else {
                            current_phase_name = trimmed.to_string();
                        }
                        phase_order.push(current_phase_name.clone());
                    } else if heading_level == 3 && trimmed.contains('.') {
                        // Task sub-heading — extract task ID from it
                        // Tasks under this heading don't have list markers;
                        // they use the heading itself for source location
                    }
                }
            }
            "list_item" => {
                if let Some(check_marker) = helpers::find_child(node, "task_list_marker_unchecked")
                    .or_else(|| helpers::find_child(node, "task_list_marker_checked"))
                {
                    let checked = check_marker.kind() == "task_list_marker_checked";
                    if let Some(content) = helpers::find_child(node, "paragraph")
                        && let Ok(text) = helpers::node_text(&content, bytes)
                    {
                        let text = text.trim();
                        let (id, desc) = helpers::extract_task_id(text);
                        if !id.is_empty() {
                            let loc = crate::ir::SourceLocation {
                                file: file_name.clone(),
                                start_byte: node.byte_range().start,
                                end_byte: node.byte_range().end,
                                start_line: helpers::find_line_for_byte(
                                    source,
                                    node.byte_range().start,
                                ),
                                end_line: helpers::find_line_for_byte(
                                    source,
                                    node.byte_range().end,
                                ),
                            };
                            phase_task_ids
                                .entry(current_phase_name.clone())
                                .or_default()
                                .push(id.clone());
                            tasks.push(crate::ir::Task {
                                id,
                                description: desc.to_string(),
                                phase: current_phase_name.clone(),
                                checked,
                                source: loc,
                            });
                        }
                    }
                }
            }
            _ => {}
        },
    );

    // Build phase list preserving insertion order
    let mut seen_phases = std::collections::HashSet::new();
    for name in &phase_order {
        if seen_phases.insert(name.clone()) {
            let mode = if concurrent_phases.contains(name) {
                crate::ir::PhaseMode::Concurrent
            } else {
                crate::ir::PhaseMode::Sequential
            };
            phases.push(crate::ir::Phase {
                name: name.clone(),
                mode,
                task_ids: phase_task_ids.remove(name).unwrap_or_default(),
            });
        }
    }
    // Any remaining tasks not in a named phase go into a default phase
    if let Some(default_ids) = phase_task_ids.remove("")
        && !default_ids.is_empty()
    {
        phases.push(crate::ir::Phase {
            name: "default".to_string(),
            mode: crate::ir::PhaseMode::Sequential,
            task_ids: default_ids,
        });
    }

    Ok((tasks, phases))
}

/// Parse a spec.md file into requirements and scenarios.
pub fn parse_spec(
    parser: &mut tree_sitter::Parser,
    source: &str,
    path: &Path,
    capability: &str,
) -> Result<(Vec<crate::ir::Requirement>, Vec<Scenario>), String> {
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse markdown".to_string())?;
    let root = tree.root_node();

    let mut requirements = Vec::new();
    let mut standalone_scenarios = Vec::new();

    let file_name = helpers::file_name_str(path);
    let bytes = source.as_bytes();

    let mut cursor = root.walk();
    explore_tree(&mut cursor, &root, source, &mut |node, _depth| {
        if node.kind() != "atx_heading" {
            return;
        }
        let heading_level = node
            .child(0)
            .map(|n| n.kind())
            .filter(|k| k.starts_with("atx_h"))
            .map(|k| k.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        if heading_level != 3 {
            return;
        }

        let Ok(heading_text) = helpers::node_text(node, bytes) else {
            return;
        };
        let heading_text = heading_text.trim().trim_start_matches('#').trim();

        if !heading_text.starts_with("Requirement:") && !heading_text.starts_with("requirement:") {
            return;
        }

        let req_name = heading_text
            .strip_prefix("Requirement:")
            .or_else(|| heading_text.strip_prefix("requirement:"))
            .unwrap_or("")
            .trim();

        let start_line = helpers::find_line_for_byte(source, node.byte_range().start);

        // Find the body paragraphs after this heading
        let mut body = String::new();
        let mut end_line = start_line;

        let mut sibling = node.next_sibling();
        while let Some(sib) = sibling {
            if sib.kind() == "paragraph" {
                if let Ok(text) = helpers::node_text(&sib, bytes) {
                    if !body.is_empty() {
                        body.push('\n');
                    }
                    body.push_str(text);
                    end_line = helpers::find_line_for_byte(source, sib.byte_range().end);
                }
            } else if sib.kind() == "atx_heading" {
                break;
            }
            sibling = sib.next_sibling();
        }

        let statement = helpers::extract_shall_statement(&body, &file_name);
        let strength = helpers::detect_rfc2119(&statement);
        let category = crate::translator::classify(&statement);

        let req_id = format!("{}::{}", capability, req_name);

        requirements.push(crate::ir::Requirement {
            id: req_id,
            statement: statement.clone(),
            strength,
            category,
            ltl: None,
            scenarios: Vec::new(),
            source: crate::ir::SourceLocation {
                file: file_name.clone(),
                start_byte: node.byte_range().start,
                end_byte: node.byte_range().end,
                start_line,
                end_line,
            },
        });

        // Extract scenarios from body
        let (req_scenarios, _) = helpers::extract_scenarios(&body, bytes, &file_name);
        for mut sc in req_scenarios {
            sc.source.file = file_name.clone();
            standalone_scenarios.push(sc);
        }
    });

    Ok((requirements, standalone_scenarios))
}

pub use helpers::{
    detect_rfc2119, explore_node, extract_scenarios, extract_shall_statement, extract_task_id,
    file_name_str, find_child, find_line_for_byte, node_text, parse_step,
};
