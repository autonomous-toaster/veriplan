//! CLI command handlers — extracted from main.rs for maintainability.

use std::path::Path;

use crate::checker;
use crate::input;

/// Check all changes sequentially and print summary.
pub fn check_all_changes(
    changes: &[String],
    project_root: &Path,
    format: &str,
    verbose: bool,
    pre_commit: bool,
    strictness: input::StrictnessProfile,
) -> anyhow::Result<()> {
    let mut results = Vec::new();

    for change in changes {
        let source = input::InputSource::OpenSpec {
            change_dir: project_root.join("openspec/changes").join(change),
            change_name: change.clone(),
        };

        let plan = input::load_plan(&source).map_err(|e| anyhow::anyhow!("{}", e))?;
        let result = checker::verify_with_strictness(
            &plan, change, false, // no_model
            pre_commit, strictness, true, // is_openspec
        );
        results.push((change.clone(), result));
    }

    match format {
        "json" => print_multi_json(&results),
        _ => print_multi_human(&results, verbose),
    }

    // Exit code based on results
    // Only exit 1 if there are actual violations (valid==Some(false)) or blockers (not convertible)
    let has_issues = results
        .iter()
        .any(|(_, r)| r.valid == Some(false) || !r.convertible);
    if has_issues {
        flush_exit(1);
    } else {
        Ok(())
    }
}

/// Print human-readable output for multiple changes.
pub fn print_multi_human(results: &[(String, checker::VerificationResult)], _verbose: bool) {
    let total = results.len();
    let invalid: Vec<_> = results
        .iter()
        .filter(|(_, r)| r.valid == Some(false) || !r.convertible)
        .collect();

    if invalid.is_empty() {
        println!("✓ All {} changes valid", total);
    } else {
        eprintln!("✗ {}/{} changes invalid", invalid.len(), total);
        for (name, _) in &invalid {
            eprintln!("  - {}: INVALID", name);
        }
        eprintln!();
        eprintln!("Run:");
        for (name, _) in &invalid {
            eprintln!("  veriplan check {}", name);
        }
    }
}

/// Print JSON output for multiple changes.
pub fn print_multi_json(results: &[(String, checker::VerificationResult)]) {
    let mut changes_json = Vec::new();
    let mut invalid_changes = Vec::new();

    for (name, result) in results {
        changes_json.push(serde_json::json!({
            "name": name,
            "valid": result.valid,
            "plan_name": result.plan_name,
        }));

        if result.valid == Some(false) || !result.convertible {
            invalid_changes.push(name);
        }
    }

    let output = serde_json::json!({
        "changes": changes_json,
        "all_valid": invalid_changes.is_empty(),
        "invalid_changes": invalid_changes,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// Flush stdio and exit with the given code.
pub fn flush_exit(code: i32) -> ! {
    std::process::exit(code);
}
