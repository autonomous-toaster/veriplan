//! Input resolution — detect and load plan content from various sources.
//!
//! Supports OpenSpec directories (current), loose directories, single files,
//! and stdin. The InputResolver determines the source type and loads content
//! into a PlanIR for the checker.

mod loader;
mod resolve;

pub use loader::{load_directory, parse_content};
pub use resolve::{find_change_dir, read_stdin, resolve_auto};

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
    /// Multiple OpenSpec changes detected — check all sequentially.
    MultiOpenSpec {
        changes: Vec<String>,
        project_root: PathBuf,
    },
    /// No verifiable content found — graceful empty state.
    Empty { path: PathBuf, reason: EmptyReason },
}

/// Reason why no verifiable content was found.
#[derive(Debug, Clone)]
pub enum EmptyReason {
    /// No openspec/changes/, tasks.md, or specs/ found
    NoContent,
    /// openspec/changes/ exists but contains no active changes
    NoActiveChanges,
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
            Self::MultiOpenSpec { changes, .. } => {
                format!("{} changes", changes.len())
            }
            Self::Empty { path, .. } => path.display().to_string(),
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
        InputSource::MultiOpenSpec { .. } => Err(
            "Cannot load plan from multiple changes — use check_all_changes() instead".to_string(),
        ),
        InputSource::Empty { .. } => {
            Err("Cannot load plan from empty directory — no verifiable content found".to_string())
        }
    }
}
