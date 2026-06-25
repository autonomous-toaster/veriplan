//! Visualization command handler — extracted from main.rs.

use std::path::Path;

use veriplan::ir::PlanIR;
use veriplan::parser;
use veriplan::translator;
use veriplan::visualizer;

pub fn run_visualize(
    change_name: Option<String>,
    format: Option<&str>,
    output: Option<&str>,
) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;

    // Determine which change to visualize
    let change_dir = if let Some(name) = &change_name {
        find_change_dir(&project_root, name)?
    } else {
        // Auto-detect: if exactly one active change, use it
        let changes = discover_changes(&project_root)?;
        match changes.len() {
            0 => anyhow::bail!("No active changes found — specify a change name"),
            1 => project_root.join("openspec/changes").join(&changes[0]),
            _ => anyhow::bail!("Multiple active changes found. Specify one: {:?}", changes),
        }
    };

    // Parse plan
    let plan: PlanIR = parser::parse_plan(&change_dir).map_err(|e| anyhow::anyhow!(e))?;

    // Translate constraints
    let constraints = translator::translate_all(&plan);

    // Generate output
    let format = format.unwrap_or("mermaid");
    let diagram = match format {
        "mermaid" => visualizer::format_mermaid(&plan, &constraints),
        "dot" => visualizer::format_dot(&plan, &constraints),
        "markdown" => visualizer::format_markdown(&plan, &constraints),
        other => anyhow::bail!("Unknown format '{}'. Use: mermaid, dot, or markdown", other),
    };

    if let Some(path) = output {
        std::fs::write(path, &diagram)?;
        println!("✓ Visualization written to {}", path);
    } else {
        print!("{}", diagram);
    }

    Ok(())
}

fn find_change_dir(project_root: &Path, change_name: &str) -> anyhow::Result<std::path::PathBuf> {
    // First: try as a change name in the current project
    let change_path = project_root
        .join("openspec")
        .join("changes")
        .join(change_name);

    if change_path.join("tasks.md").exists() && change_path.join("specs").exists() {
        return Ok(change_path);
    }

    // Check if the argument is a change name directly in CWD
    let direct = Path::new(change_name);
    if direct.join("tasks.md").exists() && direct.join("specs").exists() {
        return Ok(direct.to_path_buf());
    }

    // Second: disambiguation — check if it looks like a path
    // (contains separator or exists as a directory)
    let looks_like_path =
        change_name.contains('/') || change_name.contains('\\') || direct.exists();

    if looks_like_path {
        // Treat as a project directory path — scan for openspec inside it
        let target_root = if direct.is_absolute() {
            direct.to_path_buf()
        } else {
            project_root.join(change_name)
        };

        let target_changes = target_root.join("openspec").join("changes");
        if target_changes.exists() && target_changes.is_dir() {
            let changes = discover_changes(&target_root)?;
            if let Some(first) = changes.first() {
                return Ok(target_changes.join(first));
            }
            anyhow::bail!(
                "Directory '{}' has openspec/changes/ but no active changes found",
                target_root.display()
            );
        }
    }

    // Not found anywhere — show available changes
    let changes_dir = project_root.join("openspec").join("changes");
    if changes_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&changes_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        if looks_like_path {
            anyhow::bail!(
                "No openspec change or project directory found for '{}'. Available changes: {:?}",
                change_name,
                entries
            );
        } else {
            anyhow::bail!(
                "Change '{}' not found. Available changes: {:?}",
                change_name,
                entries
            );
        }
    }

    anyhow::bail!("Change directory not found for '{}'", change_name);
}

/// Discover all active changes in a project's openspec directory.
/// Excludes the `archive/` directory.
fn discover_changes(project_root: &Path) -> anyhow::Result<Vec<String>> {
    let changes_dir = project_root.join("openspec").join("changes");
    if !changes_dir.exists() || !changes_dir.is_dir() {
        anyhow::bail!(
            "No openspec/changes/ directory found at {}",
            changes_dir.display()
        );
    }

    let mut changes = Vec::new();
    for entry in std::fs::read_dir(&changes_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.file_type()?.is_dir() && !is_archive_dir(&name) {
            // Verify it's a valid change dir (has tasks.md or specs/)
            let change_path = entry.path();
            if change_path.join("tasks.md").exists() || change_path.join("specs").exists() {
                changes.push(name);
            }
        }
    }

    changes.sort();
    Ok(changes)
}

/// Check if a directory name is the archive directory.
fn is_archive_dir(name: &str) -> bool {
    name == "archive"
}
