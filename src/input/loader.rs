//! Directory loading and parsing helpers.

use std::path::Path;

use crate::ir::PlanIR;
use crate::parser;

/// Load a directory that may have tasks.md and/or specs/ but not the full OpenSpec layout.
pub fn load_directory(path: &Path, has_tasks: bool, has_specs: bool) -> Result<PlanIR, String> {
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
