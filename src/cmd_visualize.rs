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
    let change_dir = resolve_change_dir(&project_root, change_name.as_deref())?;

    // Parse plan
    let plan: PlanIR = parser::parse_plan(&change_dir).map_err(|e| anyhow::anyhow!(e))?;

    // Translate constraints
    let constraints = translator::translate_all(&plan);

    // Generate output
    let diagram = render_diagram(format, &plan, &constraints)?;
    write_diagram_output(&diagram, output)
}

fn write_diagram_output(diagram: &str, output: Option<&str>) -> anyhow::Result<()> {
    if let Some(path) = output {
        std::fs::write(path, diagram)?;
        println!("✓ Visualization written to {}", path);
    } else {
        print!("{}", diagram);
    }
    Ok(())
}

fn resolve_change_dir(
    project_root: &Path,
    change_name: Option<&str>,
) -> anyhow::Result<std::path::PathBuf> {
    if let Some(name) = change_name {
        find_change_dir(project_root, name)
    } else {
        let changes = discover_changes(project_root)?;
        match changes.len() {
            0 => anyhow::bail!("No active changes found — specify a change name"),
            1 => Ok(project_root.join("openspec/changes").join(&changes[0])),
            _ => anyhow::bail!("Multiple active changes found. Specify one: {:?}", changes),
        }
    }
}

fn render_diagram(
    format: Option<&str>,
    plan: &PlanIR,
    constraints: &[veriplan::translator::TranslatedConstraint],
) -> anyhow::Result<String> {
    let format = format.unwrap_or("mermaid");
    match format {
        "mermaid" => Ok(visualizer::format_mermaid(plan, constraints)),
        "dot" => Ok(visualizer::format_dot(plan, constraints)),
        "markdown" => Ok(visualizer::format_markdown(plan, constraints)),
        other => anyhow::bail!("Unknown format '{}'. Use: mermaid, dot, or markdown", other),
    }
}

fn find_change_dir(project_root: &Path, change_name: &str) -> anyhow::Result<std::path::PathBuf> {
    // First: try as a change name in the current project
    let change_path = project_root
        .join("openspec")
        .join("changes")
        .join(change_name);

    if is_valid_change_dir(&change_path) {
        return Ok(change_path);
    }

    // Check if the argument is a change name directly in CWD
    let direct = Path::new(change_name);
    if is_valid_change_dir(direct) {
        return Ok(direct.to_path_buf());
    }

    // Second: disambiguation
    let looks_like_path =
        change_name.contains('/') || change_name.contains('\\') || direct.exists();

    if looks_like_path && let Some(found) = find_change_in_path(project_root, change_name, direct)?
    {
        return Ok(found);
    }

    // Not found anywhere
    show_available_changes(project_root, change_name, looks_like_path)
}

fn is_valid_change_dir(dir: &Path) -> bool {
    dir.join("tasks.md").exists() && dir.join("specs").exists()
}

fn find_change_in_path(
    project_root: &Path,
    change_name: &str,
    direct: &Path,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    let target_root = if direct.is_absolute() {
        direct.to_path_buf()
    } else {
        project_root.join(change_name)
    };

    let target_changes = target_root.join("openspec").join("changes");
    if target_changes.exists() && target_changes.is_dir() {
        let changes = discover_changes(&target_root)?;
        if let Some(first) = changes.first() {
            return Ok(Some(target_changes.join(first)));
        }
        anyhow::bail!(
            "Directory '{}' has openspec/changes/ but no active changes found",
            target_root.display()
        );
    }
    Ok(None)
}

fn show_available_changes(
    project_root: &Path,
    change_name: &str,
    looks_like_path: bool,
) -> anyhow::Result<std::path::PathBuf> {
    let changes_dir = project_root.join("openspec").join("changes");
    if changes_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&changes_dir)?
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
        if is_active_change_dir(&entry) {
            let name = entry.file_name().to_string_lossy().to_string();
            changes.push(name);
        }
    }

    changes.sort();
    Ok(changes)
}

fn is_active_change_dir(entry: &std::fs::DirEntry) -> bool {
    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
        return false;
    }
    let name = entry.file_name().to_string_lossy().to_string();
    if name == "archive" {
        return false;
    }
    let change_path = entry.path();
    change_path.join("tasks.md").exists() || change_path.join("specs").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_change_dir_true() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tasks.md"), "").unwrap();
        std::fs::create_dir(dir.path().join("specs")).unwrap();
        assert!(is_valid_change_dir(dir.path()));
    }

    #[test]
    fn test_is_valid_change_dir_false_no_tasks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("specs")).unwrap();
        assert!(!is_valid_change_dir(dir.path()));
    }

    #[test]
    fn test_is_valid_change_dir_false_no_specs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tasks.md"), "").unwrap();
        assert!(!is_valid_change_dir(dir.path()));
    }

    #[test]
    fn test_is_valid_change_dir_false_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_valid_change_dir(dir.path()));
    }

    #[test]
    fn test_resolve_change_dir_with_name() {
        let dir = tempfile::tempdir().unwrap();
        let changes = dir.path().join("openspec").join("changes");
        std::fs::create_dir_all(&changes).unwrap();
        let change_dir = changes.join("my-change");
        std::fs::create_dir(&change_dir).unwrap();
        std::fs::write(change_dir.join("tasks.md"), "").unwrap();
        std::fs::create_dir(change_dir.join("specs")).unwrap();
        let result = resolve_change_dir(dir.path(), Some("my-change"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), change_dir);
    }

    #[test]
    fn test_resolve_change_dir_no_name_no_changes() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_change_dir(dir.path(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_change_in_path_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let direct = Path::new("nonexistent");
        let result = find_change_in_path(dir.path(), "nonexistent", direct);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_render_diagram_mermaid() {
        let plan = PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: veriplan::ir::SourceMap::default(),
        };
        let _constraints: Vec<veriplan::translator::TranslatedConstraint> = vec![];
        let result = render_diagram(Some("mermaid"), &plan, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("flowchart"));
    }

    #[test]
    fn test_render_diagram_unknown_format() {
        let plan = PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: veriplan::ir::SourceMap::default(),
        };
        let _constraints: Vec<veriplan::translator::TranslatedConstraint> = vec![];
        let result = render_diagram(Some("bogus"), &plan, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_diagram_default_mermaid() {
        let plan = PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: veriplan::ir::SourceMap::default(),
        };
        let _constraints: Vec<veriplan::translator::TranslatedConstraint> = vec![];
        let result = render_diagram(None, &plan, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("flowchart"));
    }
}
