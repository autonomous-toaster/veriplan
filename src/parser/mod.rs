//! OpenSpec plan parser using tree-sitter for markdown.
//!
//! Parses real OpenSpec change directories:
//!   openspec/changes/<name>/
//!   ├── tasks.md (N.M task numbering)
//!   └── specs/<capability>/spec.md (Requirements + Scenarios)

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tree_sitter::Parser;
use tree_sitter_language_pack::get_language;

use crate::ir::{
    Phase, PhaseMode, PlanIR, Requirement, Rfc2119Strength, Scenario, ScenarioStep, SourceLocation, SourceMap,
    StepKind, Task,
};

/// Information about a spec file's location.
struct SpecFile {
    capability: String,
    path: std::path::PathBuf,
}

/// Locate all relevant files in an OpenSpec change directory.
pub fn locate_change(change_dir: &Path) -> Result<(std::path::PathBuf, Vec<SpecFile>), String> {
    let tasks_path = change_dir.join("tasks.md");
    if !tasks_path.exists() {
        return Err(format!(
            "No tasks.md found in change directory: {}",
            change_dir.display()
        ));
    }

    let specs_dir = change_dir.join("specs");
    if !specs_dir.exists() || !specs_dir.is_dir() {
        return Err(format!(
            "No specs/ directory found in change directory: {}",
            change_dir.display()
        ));
    }

    let mut spec_files = Vec::new();
    collect_specs(&specs_dir, &mut spec_files)
        .map_err(|e| format!("Error reading specs directory: {}", e))?;

    if spec_files.is_empty() {
        return Err(format!(
            "No spec files found under specs/ in: {}",
            change_dir.display()
        ));
    }

    // Sort by capability name for deterministic order
    spec_files.sort_by(|a, b| a.capability.cmp(&b.capability));

    Ok((tasks_path, spec_files))
}

fn collect_specs(dir: &Path, files: &mut Vec<SpecFile>) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(dir)? {
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

/// Parse a complete OpenSpec change directory into PlanIR.
pub fn parse_plan(change_dir: &Path) -> Result<PlanIR, String> {
    let mut parser = Parser::new();
    let lang = get_language("markdown").map_err(|e| format!("Grammar error: {}", e))?;
    parser
        .set_language(&lang)
        .map_err(|e| format!("Grammar error: {}", e))?;

    let (tasks_path, spec_files) = locate_change(change_dir)?;

    let tasks_source = fs::read_to_string(&tasks_path)
        .map_err(|e| format!("Cannot read {}: {}", tasks_path.display(), e))?;

    let (tasks, phases) = parse_tasks(&mut parser, &tasks_source, &tasks_path)?;

    let mut all_requirements = Vec::new();
    let mut all_scenarios = Vec::new();
    let mut source_map = SourceMap::default();

    for task in &tasks {
        source_map
            .tasks
            .insert(task.id.clone(), task.source.clone());
    }

    for spec_file in &spec_files {
        let source = fs::read_to_string(&spec_file.path)
            .map_err(|e| format!("Cannot read {}: {}", spec_file.path.display(), e))?;

        let (reqs, scenarios) =
            parse_spec(&mut parser, &source, &spec_file.path, &spec_file.capability)?;

        for req in &reqs {
            source_map
                .requirements
                .insert(req.id.clone(), req.source.clone());
            for sc in &req.scenarios {
                source_map
                    .scenarios
                    .insert((req.id.clone(), sc.name.clone()), sc.source.clone());
                all_scenarios.push(sc.clone());
            }
        }

        for sc in &scenarios {
            all_scenarios.push(sc.clone());
        }
        all_requirements.extend(reqs);
    }

    Ok(PlanIR {
        tasks,
        requirements: all_requirements,
        scenarios: all_scenarios,
        phases,
        source_map,
    })
}

/// Parse tasks from tasks.md content.
pub fn parse_tasks(
    parser: &mut Parser,
    source: &str,
    file_path: &Path,
) -> Result<(Vec<Task>, Vec<Phase>), String> {
    let tree = parser
        .parse(source, None)
        .ok_or("Failed to parse tasks.md (tree-sitter returned None)".to_string())?;
    let root = tree.root_node();

    let mut tasks = Vec::new();
    let mut phases = Vec::new();
    let mut current_phase = "default".to_string();
    // Track which phases are concurrent
    let mut concurrent_phases: std::collections::HashSet<String> = std::collections::HashSet::new();

    let file_name = file_name_str(file_path);
    let bytes = source.as_bytes();

    let cursor = &mut tree.walk();
    explore_node(
        cursor,
        &root,
        source,
        &mut |node, _depth| match node.kind() {
            "atx_heading" => {
                if let Ok(heading_text) = node_text(node, bytes) {
                    let trimmed = heading_text.trim().trim_start_matches('#').trim();
                    if !trimmed.is_empty() {
                        // Detect [concurrent] tag
                        if let Some(bracket) = trimmed.rfind("[") {
                            let tag = trimmed[bracket + 1..].trim_end_matches(']').trim();
                            if tag.eq_ignore_ascii_case("concurrent")
                                || tag.eq_ignore_ascii_case("parallel")
                            {
                                let phase_name = trimmed[..bracket].trim().to_string();
                                concurrent_phases.insert(phase_name.clone());
                                current_phase = phase_name;
                            } else {
                                current_phase = trimmed.to_string();
                            }
                        } else {
                            current_phase = trimmed.to_string();
                        }
                    }
                }
            }
            "list_item" => {
                if let Some(check_marker) = find_child(node, "task_list_marker_unchecked")
                    .or_else(|| find_child(node, "task_list_marker_checked"))
                {
                    let checked = check_marker.kind() == "task_list_marker_checked";
                    if let Some(content) = find_child(node, "paragraph")
                        && let Ok(text) = node_text(&content, bytes) {
                            let text = text.trim().trim_start_matches('[').trim();
                            let (id, desc) = extract_task_id(text);
                            let loc = SourceLocation {
                                file: file_name.clone(),
                                start_byte: node.start_byte(),
                                end_byte: node.end_byte(),
                                start_line: node.start_position().row + 1,
                                end_line: node.end_position().row + 1,
                            };
                            tasks.push(Task {
                                id,
                                description: desc.to_string(),
                                phase: current_phase.clone(),
                                checked,
                                source: loc,
                            });
                        }
                }
            }
            _ => {}
        },
    );

    // Build phases from grouping
    let mut phase_map: HashMap<String, Vec<String>> = HashMap::new();
    for task in &tasks {
        phase_map
            .entry(task.phase.clone())
            .or_default()
            .push(task.id.clone());
    }
    for (name, task_ids) in phase_map {
        phases.push(Phase {
            mode: if concurrent_phases.contains(&name) {
                PhaseMode::Concurrent
            } else {
                PhaseMode::Sequential
            },
            name,
            task_ids,
        });
    }

    Ok((tasks, phases))
}

/// Parse a single spec.md file, returning requirements and standalone scenarios.
pub fn parse_spec(
    parser: &mut Parser,
    source: &str,
    file_path: &Path,
    capability: &str,
) -> Result<(Vec<Requirement>, Vec<Scenario>), String> {
    let tree = parser
        .parse(source, None)
        .ok_or("Failed to parse spec.md (tree-sitter returned None)".to_string())?;
    let root = tree.root_node();

    let mut requirements = Vec::new();
    let mut standalone_scenarios = Vec::new();
    let file_name = file_name_str(file_path);
    let bytes = source.as_bytes();

    let cursor = &mut tree.walk();
    let mut sections: Vec<(usize, String)> = Vec::new(); // (start_byte, heading_text)

    // First pass: find all requirement and scenario headings
    explore_node(cursor, &root, source, &mut |node, _depth| {
        if node.kind() == "atx_heading"
            && let Ok(text) = node_text(node, bytes) {
                let clean = text.trim().trim_start_matches('#').trim().to_string();
                // Only record ## and ### headings as section boundaries
                // so #### Scenario: headings stay inside requirement bodies
                let level = text.trim().chars().take_while(|&c| c == '#').count();
                if level < 4 {
                    sections.push((node.start_byte(), clean));
                }
            }
    });

    // Second pass: extract content between headings
    for i in 0..sections.len() {
        let (start_byte, heading_text) = &sections[i];
        let end_byte = if i + 1 < sections.len() {
            sections[i + 1].0
        } else {
            source.len()
        };

        if heading_text.starts_with("Requirement:") {
            let req_name = heading_text
                .strip_prefix("Requirement:")
                .unwrap_or(heading_text)
                .trim()
                .to_string();
            let body = &source[*start_byte..end_byte];
            let req_source = SourceLocation {
                file: file_name.clone(),
                start_byte: *start_byte,
                end_byte,
                start_line: find_line_for_byte(source, *start_byte),
                end_line: find_line_for_byte(source, end_byte),
            };

            // Find SHALL paragraph in body
            let statement = extract_shall_statement(body, &file_name);
            let strength = detect_rfc2119(&statement);
            let (scenarios, _) = extract_scenarios(body, bytes, &file_name);

            let req = Requirement {
                id: format!("{}::{}", capability, req_name),
                statement,
                strength,
                category: crate::ir::ConstraintCategory::NonFormalizable, // Will be classified later
                ltl: None,
                scenarios,
                source: req_source,
            };
            requirements.push(req);
        } else if heading_text.starts_with("Scenario:") {
            let sc_name = heading_text
                .strip_prefix("Scenario:")
                .unwrap_or(heading_text)
                .trim()
                .to_string();
            let body = &source[*start_byte..end_byte];
            let sc_source = SourceLocation {
                file: file_name.clone(),
                start_byte: *start_byte,
                end_byte,
                start_line: find_line_for_byte(source, *start_byte),
                end_line: find_line_for_byte(source, end_byte),
            };
            let (_, steps) = extract_scenarios(body, bytes, &file_name);
            if !steps.is_empty() {
                standalone_scenarios.push(Scenario {
                    name: sc_name,
                    steps,
                    source: sc_source,
                });
            }
        } else if heading_text.starts_with("### ") || heading_text.starts_with("## ") {
            // Skip delta section headers like ## ADDED Requirements, but keep looking
        }
    }

    Ok((requirements, standalone_scenarios))
}

/// Extract SHALL statement from a requirement body paragraph.
fn extract_shall_statement(body: &str, _file: &str) -> String {
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with('#')
        {
            continue; // skip list items and headings
        }
        if trimmed.contains("SHALL")
            || trimmed.contains("MUST")
            || trimmed.contains("SHOULD")
            || trimmed.contains("MAY")
        {
            return trimmed.to_string();
        }
    }
    // If no SHALL line, return the first non-empty, non-heading line
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with('*')
        {
            continue;
        }
        return trimmed.to_string();
    }
    body.lines().next().unwrap_or("").trim().to_string()
}

/// Extract scenarios from a requirement body.
fn extract_scenarios(body: &str, _bytes: &[u8], file: &str) -> (Vec<Scenario>, Vec<ScenarioStep>) {
    let mut scenarios = Vec::new();
    let steps = Vec::new();
    let mut current_name = String::new();
    let mut current_steps: Vec<ScenarioStep> = Vec::new();

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("#### Scenario:") {
            // Save previous scenario
            if !current_name.is_empty() && !current_steps.is_empty() {
                scenarios.push(Scenario {
                    name: current_name.clone(),
                    steps: current_steps,
                    source: SourceLocation {
                        file: file.to_string(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: i,
                        end_line: i,
                    },
                });
            }
            current_name = trimmed
                .strip_prefix("#### Scenario:")
                .unwrap_or("")
                .trim()
                .to_string();
            current_steps = Vec::new();
        } else if let Some(step) = parse_step(trimmed, file, i) {
            current_steps.push(step);
        }
    }
    // Save last scenario
    if !current_name.is_empty() && !current_steps.is_empty() {
        scenarios.push(Scenario {
            name: current_name.clone(),
            steps: current_steps.clone(),
            source: SourceLocation {
                file: file.to_string(),
                start_byte: 0,
                end_byte: 0,
                start_line: body.lines().count(),
                end_line: body.lines().count(),
            },
        });
    }

    (scenarios, steps)
}

/// Parse a single scenario step from a list item.
fn parse_step(text: &str, file: &str, line: usize) -> Option<ScenarioStep> {
    let clean = text.strip_prefix("- ")?.trim();
    let (kind_str, step_text) = if let Some(rest) = clean.strip_prefix("**GIVEN**") {
        ("given", rest.trim())
    } else if let Some(rest) = clean.strip_prefix("**WHEN**") {
        ("when", rest.trim())
    } else if let Some(rest) = clean.strip_prefix("**THEN**") {
        ("then", rest.trim())
    } else if let Some(rest) = clean.strip_prefix("**AND**") {
        ("and", rest.trim())
    } else {
        return None;
    };

    let kind = match kind_str {
        "given" => StepKind::Given,
        "when" => StepKind::When,
        "then" => StepKind::Then,
        "and" => StepKind::And,
        _ => return None,
    };

    Some(ScenarioStep {
        kind,
        text: step_text.to_string(),
        source: SourceLocation {
            file: file.to_string(),
            start_byte: 0,
            end_byte: 0,
            start_line: line + 1,
            end_line: line + 1,
        },
    })
}

/// Detect RFC 2119 strength from statement text.
pub fn detect_rfc2119(statement: &str) -> Rfc2119Strength {
    let upper = statement.to_uppercase();
    if upper.contains("MUST NOT") || upper.contains("SHALL NOT") {
        Rfc2119Strength::MustNot
    } else if upper.contains("MUST") || upper.contains("SHALL") {
        Rfc2119Strength::Must
    } else if upper.contains("SHOULD") {
        Rfc2119Strength::Should
    } else if upper.contains("MAY") {
        Rfc2119Strength::May
    } else {
        Rfc2119Strength::None
    }
}

/// Extract N.M task ID and description from text.
/// "1.3 Add dependencies" → ("1.3", "Add dependencies")
/// "Some task" → (auto-generated, "Some task")
fn extract_task_id(text: &str) -> (String, String) {
    let text = text.trim();
    // Match patterns like "1.3", "2.1", "12.5"
    // Extract N.M task ID from text like "1.3 Add dependencies"
    if let Some(space_pos) = text.find(' ') {
        let candidate = &text[..space_pos];
        let rest = text[space_pos + 1..].trim();
        // Check if candidate matches N.M pattern (digits.digits)
        if let Some(dot_pos) = candidate.find('.') {
            let left = &candidate[..dot_pos];
            let right = &candidate[dot_pos + 1..];
            if !left.is_empty()
                && !right.is_empty()
                && left.chars().all(|c| c.is_ascii_digit())
                && right.chars().all(|c| c.is_ascii_digit())
            {
                return (candidate.to_string(), rest.to_string());
            }
        }
    }
    // Auto-generate ID from hash if no N.M pattern
    let id = format!("x{:x}", text.len());
    (id, text.to_string())
}

// --- helpers ---

fn file_name_str(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn node_text<'a>(node: &tree_sitter::Node, source: &'a [u8]) -> Result<&'a str, String> {
    source
        .get(node.start_byte()..node.end_byte())
        .and_then(|s| std::str::from_utf8(s).ok())
        .ok_or_else(|| "Invalid UTF-8 in source".to_string())
}

fn find_child<'a>(node: &tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|&child| child.kind() == kind)
}

fn find_line_for_byte(source: &str, byte: usize) -> usize {
    source[..byte.min(source.len())].lines().count()
}

/// Walk the tree-sitter tree calling `f` for every node with its depth.
fn explore_node<'a>(
    cursor: &mut tree_sitter::TreeCursor<'a>,
    node: &tree_sitter::Node<'a>,
    source: &str,
    f: &mut dyn FnMut(&tree_sitter::Node<'a>, usize),
) {
    f(node, cursor.depth() as usize);
    if cursor.goto_first_child() {
        loop {
            explore_node(cursor, &cursor.node(), source, f);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}
