//! Input resolution strategies — helper functions for resolve_input.

use std::io::{self, Read};
use std::path::Path;

use super::{EmptyReason, InputSource, discover_changes};

/// Auto-detect input source from the current working directory.
pub fn resolve_auto(project_root: &Path) -> Result<InputSource, String> {
    // 6. CWD has openspec/changes/ → OpenSpec auto-detect
    let changes_dir = project_root.join("openspec").join("changes");
    if changes_dir.exists() && changes_dir.is_dir() {
        let changes = discover_changes(project_root)?;
        match changes.len() {
            0 => Ok(InputSource::Empty {
                path: project_root.to_path_buf(),
                reason: EmptyReason::NoActiveChanges,
            }),
            1 => Ok(InputSource::OpenSpec {
                change_dir: changes_dir.join(&changes[0]),
                change_name: changes[0].clone(),
            }),
            _ => Ok(InputSource::MultiOpenSpec {
                changes,
                project_root: project_root.to_path_buf(),
            }), // multiple changes is valid, not an error
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
            Ok(InputSource::Empty {
                path: project_root.to_path_buf(),
                reason: EmptyReason::NoContent,
            })
        }
    }
}

/// Find a change directory by name, trying multiple lookup strategies.
pub fn find_change_dir(project_root: &Path, change_name: &str) -> Result<InputSource, String> {
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
pub fn read_stdin() -> Result<String, String> {
    let mut content = String::new();
    io::stdin()
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read stdin: {}", e))?;
    if content.is_empty() {
        return Err("Stdin is empty — no content to verify".to_string());
    }
    Ok(content)
}
