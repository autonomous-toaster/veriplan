//! Input resolution — detect and load plan content from various sources.
//!
//! Supports OpenSpec directories (current), loose directories, single files,
//! and stdin. The InputResolver determines the source type and loads content
//! into a PlanIR for the checker.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::ir::PlanIR;
use crate::parser;

/// How strict the checker should be about findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum StrictnessProfile {
    /// Strict mode: pattern-ungrounded is BLOCKER, missing tasks/reqs in OpenSpec is BLOCKER
    #[default]
    Strict,
    /// Moderate mode: pattern-ungrounded is WARNING, missing tasks/reqs in OpenSpec is BLOCKER
    Moderate,
    /// Lax mode: pattern-ungrounded is INFO, missing tasks/reqs in OpenSpec is WARNING
    Lax,
}

impl std::str::FromStr for StrictnessProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "moderate" => Ok(Self::Moderate),
            "lax" => Ok(Self::Lax),
            other => Err(format!(
                "unknown strictness profile '{}'. Use: strict, moderate, or lax",
                other
            )),
        }
    }
}

/// The source of plan content.
#[derive(Debug, Clone)]
pub enum InputSource {
    /// OpenSpec change directory (current behavior).
    OpenSpec {
        change_dir: PathBuf,
        change_name: String,
    },
    /// A directory with tasks.md and/or specs/ but not full OpenSpec layout.
    Directory {
        path: PathBuf,
        has_tasks: bool,
        has_specs: bool,
    },
    /// A single .md file (content auto-detected).
    SingleFile { path: PathBuf },
    /// Content read from stdin.
    Stdin { content: String, label: String },
}

impl InputSource {
    /// Returns true if this is an OpenSpec change directory.
    pub fn is_openspec(&self) -> bool {
        matches!(self, Self::OpenSpec { .. })
    }

    /// Returns a human-readable label for this source.
    pub fn label(&self) -> String {
        match self {
            Self::OpenSpec { change_name, .. } => change_name.clone(),
            Self::Directory { path, .. } => path.display().to_string(),
            Self::SingleFile { path } => path.display().to_string(),
            Self::Stdin { label, .. } => label.clone(),
        }
    }
}

/// Resolve the input source from CLI arguments.
///
/// Detection priority:
/// 1. Argument is a directory with `openspec/changes/` → OpenSpec mode
/// 2. Argument is a directory with `tasks.md` or `specs/` → Directory mode
/// 3. Argument is a `.md` file → Single-file mode
/// 4. Argument is `-` or `--stdin` flag → Stdin mode
/// 5. Argument is a string (no path separators) → OpenSpec change name lookup
/// 6. No argument, CWD has `openspec/changes/` → OpenSpec auto-detect
/// 7. No argument, CWD has `tasks.md` or `specs/` → Directory mode on CWD
/// 8. None of the above → clear error message
pub fn resolve_input(
    arg: Option<&str>,
    project_root: &Path,
    stdin_flag: bool,
) -> Result<InputSource, String> {
    // Handle --stdin / - flag
    if stdin_flag {
        let content = read_stdin()?;
        return Ok(InputSource::Stdin {
            content,
            label: "<stdin>".to_string(),
        });
    }

    let arg = match arg {
        Some(a) => a,
        None => {
            // No argument: try auto-detect from CWD
            return resolve_auto(project_root);
        }
    };

    // Check for "-" as stdin
    if arg == "-" {
        let content = read_stdin()?;
        return Ok(InputSource::Stdin {
            content,
            label: "<stdin>".to_string(),
        });
    }

    let path = Path::new(arg);

    // 1. Is it a directory with openspec/changes/?
    if path.is_dir() {
        let changes_dir = path.join("openspec").join("changes");
        if changes_dir.exists() && changes_dir.is_dir() {
            // It has openspec/changes/ — treat as project root, auto-detect
            let changes = discover_changes(path)?;
            if let Some(name) = changes.first() {
                return Ok(InputSource::OpenSpec {
                    change_dir: changes_dir.join(name),
                    change_name: name.clone(),
                });
            }
            return Err(format!(
                "No active changes found in {}",
                changes_dir.display()
            ));
        }

        // 2. Is it a directory with tasks.md or specs/?
        let has_tasks = path.join("tasks.md").exists();
        let has_specs = path.join("specs").exists();

        if has_tasks || has_specs {
            return Ok(InputSource::Directory {
                path: path.to_path_buf(),
                has_tasks,
                has_specs,
            });
        }

        // Directory exists but has no verifiable content
        return Err(format!(
            "No verifiable content found in {} (no tasks.md or specs/ directory)",
            path.display()
        ));
    }

    // 3. Is it a file?
    if path.is_file() {
        // Accept any .md file, or any file for that matter (parser will detect content)
        return Ok(InputSource::SingleFile {
            path: path.to_path_buf(),
        });
    }

    // 4. Does the file not exist but might be a change name?
    // Check if it looks like a change name (no path separators)
    if !arg.contains('/') && !arg.contains('\\') {
        // Try as a change name in the project root
        let change_dir = project_root.join("openspec").join("changes").join(arg);
        if change_dir.join("tasks.md").exists() || change_dir.join("specs").exists() {
            return Ok(InputSource::OpenSpec {
                change_dir,
                change_name: arg.to_string(),
            });
        }

        // Try find_change_dir logic
        if let Ok(source) = find_change_dir(project_root, arg) {
            return Ok(source);
        }

        // List available changes
        let changes_dir = project_root.join("openspec").join("changes");
        if changes_dir.exists() {
            let entries: Vec<String> = std::fs::read_dir(&changes_dir)
                .map_err(|e| format!("Cannot read changes directory: {}", e))?
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name != "archive"
                        && (e.path().join("tasks.md").exists() || e.path().join("specs").exists())
                })
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();

            return Err(format!(
                "Change '{}' not found. Available changes: {:?}",
                arg, entries
            ));
        }

        return Err(format!(
            "No openspec/changes/ directory found at {}",
            project_root.display()
        ));
    }

    // File doesn't exist and doesn't look like a change name
    Err(format!("Path does not exist: {}", arg))
}

/// Auto-detect input source from the current working directory.
fn resolve_auto(project_root: &Path) -> Result<InputSource, String> {
    // 6. CWD has openspec/changes/ → OpenSpec auto-detect
    let changes_dir = project_root.join("openspec").join("changes");
    if changes_dir.exists() && changes_dir.is_dir() {
        let changes = discover_changes(project_root)?;
        match changes.len() {
            0 => Err(format!(
                "No active changes found in {}",
                changes_dir.display()
            )),
            1 => Ok(InputSource::OpenSpec {
                change_dir: changes_dir.join(&changes[0]),
                change_name: changes[0].clone(),
            }),
            _ => Err(format!(
                "Multiple active changes found. Specify one: {:?}",
                changes
            )),
        }
    } else {
        // 7. CWD has tasks.md or specs/ → Directory mode
        let has_tasks = project_root.join("tasks.md").exists();
        let has_specs = project_root.join("specs").exists();

        if has_tasks || has_specs {
            Ok(InputSource::Directory {
                path: project_root.to_path_buf(),
                has_tasks,
                has_specs,
            })
        } else {
            Err(format!(
                "No verifiable content found in {}. Pass a file, directory, or change name. \
                 Or pipe content via --stdin.",
                project_root.display()
            ))
        }
    }
}

/// Find a change directory by name, trying multiple lookup strategies.
fn find_change_dir(project_root: &Path, change_name: &str) -> Result<InputSource, String> {
    // First: try as a change name in the current project
    let change_path = project_root
        .join("openspec")
        .join("changes")
        .join(change_name);

    if change_path.join("tasks.md").exists() && change_path.join("specs").exists() {
        return Ok(InputSource::OpenSpec {
            change_dir: change_path,
            change_name: change_name.to_string(),
        });
    }

    // Check if the argument is a path to a directory
    let direct = Path::new(change_name);
    if direct.join("tasks.md").exists() && direct.join("specs").exists() {
        return Ok(InputSource::OpenSpec {
            change_dir: direct.to_path_buf(),
            change_name: change_name.to_string(),
        });
    }

    // Try as a project directory path
    if direct.is_dir() {
        let target_root = if direct.is_absolute() {
            direct.to_path_buf()
        } else {
            project_root.join(change_name)
        };

        let target_changes = target_root.join("openspec").join("changes");
        if target_changes.exists() && target_changes.is_dir() {
            let changes = discover_changes(&target_root)?;
            if let Some(name) = changes.first() {
                return Ok(InputSource::OpenSpec {
                    change_dir: target_changes.join(name),
                    change_name: name.clone(),
                });
            }
        }
    }

    Err(format!("Change '{}' not found", change_name))
}

/// Read all of stdin into a string.
fn read_stdin() -> Result<String, String> {
    let mut content = String::new();
    io::stdin()
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read stdin: {}", e))?;
    if content.is_empty() {
        return Err("Stdin is empty — no content to verify".to_string());
    }
    Ok(content)
}

/// Discover all active changes in a project's openspec directory.
/// Excludes the `archive/` directory.
pub fn discover_changes(project_root: &Path) -> Result<Vec<String>, String> {
    let changes_dir = project_root.join("openspec").join("changes");
    if !changes_dir.exists() || !changes_dir.is_dir() {
        return Err(format!(
            "No openspec/changes/ directory found at {}",
            changes_dir.display()
        ));
    }

    let mut changes = Vec::new();
    for entry in std::fs::read_dir(&changes_dir)
        .map_err(|e| format!("Cannot read changes directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) && name != "archive" {
            let change_path = entry.path();
            if change_path.join("tasks.md").exists() || change_path.join("specs").exists() {
                changes.push(name);
            }
        }
    }

    changes.sort();
    Ok(changes)
}

/// Parse content from any input source into a PlanIR.
///
/// For OpenSpec directories, uses the existing `parse_plan()` which requires
/// tasks.md and specs/. For all other sources, uses `parse_content()` which
/// tries both parsers and merges results.
pub fn load_plan(source: &InputSource) -> Result<PlanIR, String> {
    match source {
        InputSource::OpenSpec { change_dir, .. } => {
            parser::parse_plan(change_dir).map_err(|e| format!("Parse error: {}", e))
        }
        InputSource::Directory {
            path,
            has_tasks,
            has_specs,
        } => load_directory(path, *has_tasks, *has_specs),
        InputSource::SingleFile { path } => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            parse_content(&content, &filename)
        }
        InputSource::Stdin { content, label } => parse_content(content, label),
    }
}

/// Load a directory that may have tasks.md and/or specs/ but not the full OpenSpec layout.
fn load_directory(path: &Path, has_tasks: bool, has_specs: bool) -> Result<PlanIR, String> {
    let mut plan = PlanIR {
        tasks: Vec::new(),
        requirements: Vec::new(),
        scenarios: Vec::new(),
        phases: Vec::new(),
        source_map: crate::ir::SourceMap::default(),
    };

    let mut parser_instance = tree_sitter::Parser::new();
    let lang = tree_sitter_language_pack::get_language("markdown")
        .map_err(|e| format!("Grammar error: {}", e))?;
    parser_instance
        .set_language(&lang)
        .map_err(|e| format!("Grammar error: {}", e))?;

    if has_tasks {
        let tasks_path = path.join("tasks.md");
        let tasks_source = std::fs::read_to_string(&tasks_path)
            .map_err(|e| format!("Cannot read {}: {}", tasks_path.display(), e))?;
        let (tasks, phases) =
            parser::parse_tasks(&mut parser_instance, &tasks_source, &tasks_path)?;
        plan.tasks = tasks;
        plan.phases = phases;
    }

    if has_specs {
        let specs_dir = path.join("specs");
        let mut spec_files = Vec::new();
        collect_specs(&specs_dir, &mut spec_files)
            .map_err(|e| format!("Error reading specs directory: {}", e))?;
        spec_files.sort_by(|a, b| a.capability.cmp(&b.capability));

        for spec_file in &spec_files {
            let source = std::fs::read_to_string(&spec_file.path)
                .map_err(|e| format!("Cannot read {}: {}", spec_file.path.display(), e))?;
            let (reqs, standalone_scenarios) = parser::parse_spec(
                &mut parser_instance,
                &source,
                &spec_file.path,
                &spec_file.capability,
            )?;
            plan.requirements.extend(reqs);
            plan.scenarios.extend(standalone_scenarios);
        }
    }

    // Build source map
    for task in &plan.tasks {
        plan.source_map
            .tasks
            .insert(task.id.clone(), task.source.clone());
    }
    for req in &plan.requirements {
        plan.source_map
            .requirements
            .insert(req.id.clone(), req.source.clone());
    }

    Ok(plan)
}

/// Collect spec files from a directory tree.
fn collect_specs(dir: &Path, files: &mut Vec<parser::SpecFile>) -> Result<(), std::io::Error> {
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
            files.push(parser::SpecFile { capability, path });
        }
    }
    Ok(())
}

/// Parse arbitrary markdown content into a PlanIR.
///
/// Tries `parse_tasks()` and `parse_spec()` on the same content.
/// Either, both, or neither may produce results. If neither produces
/// anything, returns an error.
pub fn parse_content(source: &str, filename: &str) -> Result<PlanIR, String> {
    let mut parser_instance = tree_sitter::Parser::new();
    let lang = tree_sitter_language_pack::get_language("markdown")
        .map_err(|e| format!("Grammar error: {}", e))?;
    parser_instance
        .set_language(&lang)
        .map_err(|e| format!("Grammar error: {}", e))?;

    let file_path = Path::new(filename);

    // Try parsing as tasks
    let tasks_result = parser::parse_tasks(&mut parser_instance, source, file_path);
    let (tasks, phases) = tasks_result.unwrap_or_default();

    // Try parsing as spec (use filename as capability name for better IDs)
    let capability = file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("standalone");

    let (requirements, standalone_scenarios) =
        match parser::parse_spec(&mut parser_instance, source, file_path, capability) {
            Ok((reqs, scenarios)) => (reqs, scenarios),
            Err(_) => (Vec::new(), Vec::new()),
        };

    if tasks.is_empty() && requirements.is_empty() {
        return Err(format!(
            "No verifiable content found in {} — no tasks or requirements detected",
            filename
        ));
    }

    // It's OK to have only tasks or only requirements — the checker will
    // handle the severity based on input mode and strictness.
    // For example, a standalone spec.md has requirements but no tasks, and
    // a standalone tasks.md has tasks but no requirements.

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
        scenarios: standalone_scenarios,
        phases,
        source_map,
    })
}
