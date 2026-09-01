//! `check` command implementation — extracted from main.rs to keep files
//! under the 550-line file-length gate.

use std::io::Write;

use veriplan::annotator;
use veriplan::checker;
use veriplan::input;

use crate::cli;

/// Drive a single `veriplan check` run: resolve input, verify, optionally
/// apply `--fix`, render, and set the process exit code.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_check(
    change_name: Option<String>,
    phase: Option<&str>,
    format: Option<&str>,
    verbose: bool,
    pre_commit: bool,
    stdin_flag: bool,
    _strict: bool,
    moderate: bool,
    lax: bool,
    checker: Option<String>,
    compare: bool,
    fix: bool,
) -> anyhow::Result<()> {
    // Validate plan format if provided
    let format_val = format.unwrap_or("human");
    if format_val != "openspec" && format_val != "human" && format_val != "json" {
        anyhow::bail!(
            "unknown format '{}'. Supported formats: openspec",
            format_val
        );
    }

    // Resolve strictness profile
    let strictness = if lax {
        input::StrictnessProfile::Lax
    } else if moderate {
        input::StrictnessProfile::Moderate
    } else {
        input::StrictnessProfile::Strict // default
    };

    // Resolve checker backend: CLI flag overrides env var, default is spin
    let backend_str = checker
        .or_else(|| std::env::var("VERIPLAN_CHECKER").ok())
        .unwrap_or_else(|| "spin".to_string());
    let backend = backend_str
        .parse::<checker::CheckerBackend>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let project_root = std::env::current_dir()?;

    // Resolve input source
    let source = input::resolve_input(change_name.as_deref(), &project_root, stdin_flag)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let no_model = phase == Some("convertibility");

    // Detect PRE_COMMIT env var for auto-enabling pre-commit mode
    let pre_commit = pre_commit || std::env::var("PRE_COMMIT").as_deref() == Ok("1");

    // Handle MultiOpenSpec case
    if let input::InputSource::MultiOpenSpec {
        changes,
        project_root,
    } = source
    {
        cli::check_all_changes(
            &changes,
            &project_root,
            format.unwrap_or("human"),
            verbose,
            pre_commit,
            strictness,
            backend,
        )?;
        return Ok(());
    }

    // Handle Empty case - graceful success with informational message
    if let input::InputSource::Empty { path, reason } = source {
        let message = match reason {
            input::EmptyReason::NoContent => format!(
                "No verifiable content found in {} — skipping verification",
                path.display()
            ),
            input::EmptyReason::NoActiveChanges => format!(
                "No active changes found in {} — skipping verification",
                path.display()
            ),
        };
        println!("{}", message);
        return Ok(());
    }

    // Load plan from the resolved source
    let plan = input::load_plan(&source).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Determine name for display
    let label = source.label();
    let is_openspec = source.is_openspec();

    // Comparison mode: run both backends and diff
    if compare {
        return run_compare(
            &plan,
            &label,
            no_model,
            pre_commit,
            strictness,
            is_openspec,
            format.unwrap_or("human"),
            verbose,
        );
    }

    // Run checker with strictness profile
    let result = checker::verify_with_strictness(
        &plan,
        &label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
        backend,
    );
    let annotated = annotator::annotate(&result, &[(label.clone(), plan.clone())]);

    // `--fix`: apply machine-applicable (`local`) findings, then revalidate
    // (design D3, task 6.3). Structural/judgment findings are left as
    // suggestions.
    if fix {
        let all_findings = annotator::findings(&result, &annotated);
        // Resolve the change directory for relative file paths.
        let base_dir = match &source {
            input::InputSource::OpenSpec { change_dir, .. } => change_dir.clone(),
            input::InputSource::Directory { path, .. } => path.clone(),
            input::InputSource::SingleFile { path } => path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
            _ => std::env::current_dir().unwrap_or_default(),
        };
        let fix_report = veriplan::fix::fix_plan(&plan, &all_findings, &base_dir);
        if !fix_report.applied.is_empty() {
            println!(
                "Applied {} machine-applicable fix(es):",
                fix_report.applied.len()
            );
            for e in &fix_report.applied {
                println!("  - [{}] {}", e.kind, e.description);
            }
            // Revalidate the plan after applying edits.
            let reloaded = input::load_plan(&source).map_err(|e| anyhow::anyhow!("{}", e))?;
            let re_result = checker::verify_with_strictness(
                &reloaded,
                &label,
                no_model,
                pre_commit,
                strictness,
                is_openspec,
                backend,
            );
            let re_annotated =
                annotator::annotate(&re_result, &[(label.clone(), reloaded.clone())]);
            match format.unwrap_or("human") {
                "json" => println!(
                    "{}",
                    annotator::format_json(
                        &re_result,
                        &re_annotated,
                        &[(label, reloaded)],
                        verbose
                    )
                ),
                _ => print!(
                    "{}",
                    annotator::format_human(
                        &re_result,
                        &re_annotated,
                        &[(label, reloaded.clone())],
                        verbose
                    )
                ),
            }
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
            if !re_result.convertible {
                cli::flush_exit(2);
            } else if re_result.valid == Some(false) {
                cli::flush_exit(1);
            }
            return Ok(());
        }
        if !fix_report.left_as_suggestions.is_empty() {
            println!(
                "{} finding(s) left as suggestions (structural/judgment):",
                fix_report.left_as_suggestions.len()
            );
        }
    }

    match format.unwrap_or("human") {
        "json" => println!(
            "{}",
            annotator::format_json(&result, &annotated, &[(label, plan)], verbose)
        ),
        _ => print!(
            "{}",
            annotator::format_human(&result, &annotated, &[(label, plan.clone())], verbose)
        ),
    }

    // Flush output before exit to avoid losing buffered content
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    // Exit codes depend on mode:
    //   Normal:      0 = valid, 1 = violations, 2 = not convertible / missing SPIN
    //   Pre-commit:  0 = valid/warnings/missing-SPIN, 1 = blockers/violations
    if !result.convertible {
        if pre_commit {
            // In pre-commit mode, blockers exit 1 (not 2)
            eprintln!(
                "\nCommit blocked. Fix blockers above, or skip with: VERIPLAN_SKIP=1 git commit"
            );
            cli::flush_exit(1);
        } else {
            cli::flush_exit(2);
        }
    } else if result.valid == Some(false) {
        if pre_commit {
            eprintln!(
                "\nCommit blocked. Fix violations above, or skip with: VERIPLAN_SKIP=1 git commit"
            );
        }
        cli::flush_exit(1);
    } else if result.valid == Some(true)
        && annotator::findings(&result, &annotated)
            .iter()
            .any(|f| f.severity == "blocker")
    {
        // Prose blockers (Strict only) are convertible + valid but carry a
        // `blocker` finding. Exit non-zero so a blocker is a blocker everywhere,
        // including pre-commit (consistent with other blockers).
        if pre_commit {
            eprintln!(
                "\nCommit blocked. Fix blockers above, or skip with: VERIPLAN_SKIP=1 git commit"
            );
        }
        cli::flush_exit(1);
    } else if let Some(_reason) = &result.skip_reason {
        // Missing SPIN or other non-blocking skip: plan is convertible,
        // just can't model-check. Exit 0 since the plan is valid.
        return Ok(());
    } else if no_model
        && !result
            .convertibility_report
            .as_ref()
            .is_none_or(|r| r.warnings.is_empty())
    {
        cli::flush_exit(0);
    }

    Ok(())
}

/// Run both backends and compare results.
#[allow(clippy::too_many_arguments)]
fn run_compare(
    plan: &veriplan::ir::PlanIR,
    label: &str,
    no_model: bool,
    pre_commit: bool,
    strictness: input::StrictnessProfile,
    is_openspec: bool,
    _format: &str,
    _verbose: bool,
) -> anyhow::Result<()> {
    use std::time::Instant;

    // Run spin backend
    let spin_start = Instant::now();
    let spin_result = checker::verify_with_strictness(
        plan,
        label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
        checker::CheckerBackend::Spin,
    );
    let spin_elapsed = spin_start.elapsed();

    // Run spin-rs backend
    let spin_rs_start = Instant::now();
    let spin_rs_result = checker::verify_with_strictness(
        plan,
        label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
        checker::CheckerBackend::SpinRs,
    );
    let spin_rs_elapsed = spin_rs_start.elapsed();

    // Build comparison table
    let _spin_summary: std::collections::HashMap<&str, bool> = spin_result
        .constraints_summary
        .iter()
        .map(|c| (c.requirement_id.as_str(), c.satisfied))
        .collect();

    let mut mismatches = 0u32;
    let mut total = 0u32;

    println!("═══ Backend Comparison: {} ═══", label);
    println!();
    println!(
        "{:<30} {:<10} {:<10} {:<8}",
        "Constraint", "spin", "spin-rs", "Match?"
    );
    println!("{}", "-".repeat(60));

    for c in &spin_result.constraints_summary {
        let spin_rs_satisfied = spin_rs_result
            .constraints_summary
            .iter()
            .find(|sc| sc.requirement_id == c.requirement_id)
            .map(|sc| sc.satisfied)
            .unwrap_or(false);

        let matched = c.satisfied == spin_rs_satisfied;
        if !matched {
            mismatches += 1;
        }
        total += 1;

        let spin_status = if c.satisfied { "pass" } else { "FAIL" };
        let spin_rs_status = if spin_rs_satisfied { "pass" } else { "FAIL" };
        let match_icon = if matched { "✓" } else { "✗" };

        println!(
            "{:<30} {:<10} {:<10} {:<8}",
            veriplan::util::truncate(&c.requirement_id, 28),
            spin_status,
            spin_rs_status,
            match_icon,
        );
    }

    println!();
    println!(
        "spin:    {:.2}s  |  valid={}  |  violations={}",
        spin_elapsed.as_secs_f64(),
        format_valid(spin_result.valid),
        spin_result.violations.len(),
    );
    println!(
        "spin-rs: {:.2}s  |  valid={}  |  violations={}",
        spin_rs_elapsed.as_secs_f64(),
        format_valid(spin_rs_result.valid),
        spin_rs_result.violations.len(),
    );
    println!();
    println!(
        "{}/{} constraints match, {} mismatches",
        total - mismatches,
        total,
        mismatches,
    );

    if mismatches > 0 {
        eprintln!("⚠ Backends disagree on {} constraint(s)", mismatches);
    }

    Ok(())
}

fn format_valid(valid: Option<bool>) -> String {
    match valid {
        Some(true) => "✓".into(),
        Some(false) => "✗".into(),
        None => "?".into(),
    }
}
