//! `check` command implementation — extracted from main.rs to keep files
//! under the 550-line file-length gate.

use std::io::Write;

use veriplan::annotator;
use veriplan::checker;
use veriplan::input;

use crate::cli;

/// The output format, refusing an unknown one rather than silently printing human.
fn resolve_format(format: Option<&str>) -> anyhow::Result<&str> {
    match format.unwrap_or("human") {
        v @ ("openspec" | "human" | "json") => Ok(v),
        other => anyhow::bail!("unknown format '{}'. Supported formats: openspec", other),
    }
}

/// Lax and moderate are explicit flags; strict is what remains.
fn resolve_strictness(lax: bool, moderate: bool) -> input::StrictnessProfile {
    if lax {
        input::StrictnessProfile::Lax
    } else if moderate {
        input::StrictnessProfile::Moderate
    } else {
        input::StrictnessProfile::Strict
    }
}

/// The CLI flag wins over the environment variable; spin is the default.
fn resolve_backend(checker: Option<String>) -> anyhow::Result<checker::CheckerBackend> {
    let name = checker
        .or_else(|| std::env::var("VERIPLAN_CHECKER").ok())
        .unwrap_or_else(|| "spin".to_string());
    name.parse::<checker::CheckerBackend>()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Sources that are not a single plan to check: several changes, or nothing to verify at all.
///
/// Returns true when the source was handled here and the caller should stop.
fn handle_special_source(
    source: &input::InputSource,
    format: &str,
    verbose: bool,
    pre_commit: bool,
    strictness: input::StrictnessProfile,
    backend: checker::CheckerBackend,
) -> anyhow::Result<bool> {
    match source {
        input::InputSource::MultiOpenSpec {
            changes,
            project_root,
        } => {
            cli::check_all_changes(
                changes,
                project_root,
                format,
                verbose,
                pre_commit,
                strictness,
                backend,
            )?;
            Ok(true)
        }
        input::InputSource::Empty { path, reason } => {
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
            println!("{message}");
            Ok(true)
        }
        _ => Ok(false),
    }
}
#[allow(clippy::too_many_arguments)]
/// Drive a single `veriplan check` run: resolve input, verify, optionally apply `--fix`, render,
/// and set the process exit code.
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
    // Validated here so an unknown format fails before any work is done; the value is read below.
    resolve_format(format)?;
    let strictness = resolve_strictness(lax, moderate);
    let backend = resolve_backend(checker)?;
    let project_root = std::env::current_dir()?;

    // Resolve input source
    let source = input::resolve_input(change_name.as_deref(), &project_root, stdin_flag)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let no_model = phase == Some("convertibility");

    // Detect PRE_COMMIT env var for auto-enabling pre-commit mode
    let pre_commit = pre_commit || std::env::var("PRE_COMMIT").as_deref() == Ok("1");

    if handle_special_source(
        &source,
        format.unwrap_or("human"),
        verbose,
        pre_commit,
        strictness,
        backend,
    )? {
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
    if fix
        && apply_fixes(
            &source,
            &plan,
            &result,
            &annotated,
            Revalidate {
                label: &label,
                no_model,
                pre_commit,
                strictness,
                is_openspec,
                backend,
                format: format.unwrap_or("human"),
                verbose,
            },
        )?
    {
        return Ok(());
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

    let has_blocker = annotator::findings(&result, &annotated)
        .iter()
        .any(|f| f.severity == "blocker");
    if let Some(code) = exit_code(&result, pre_commit, has_blocker) {
        if pre_commit && code == 1 {
            eprintln!(
                "\nCommit blocked. Fix blockers above, or skip with: VERIPLAN_SKIP=1 git commit"
            );
        }
        cli::flush_exit(code);
    }

    Ok(())
}

/// The exit code a finished check implies, or None when the run exits zero.
///
/// Normal mode distinguishes "not convertible" (2) from "has findings" (1); pre-commit collapses
/// both onto 1 so a commit is blocked the same way whichever it hit.
fn exit_code(
    result: &veriplan::checker::VerificationResult,
    pre_commit: bool,
    has_blocker: bool,
) -> Option<i32> {
    if !result.convertible {
        return Some(if pre_commit { 1 } else { 2 });
    }
    if result.valid == Some(false) {
        return Some(1);
    }
    if result.valid == Some(true) && has_blocker {
        return Some(1);
    }
    None
}

/// Apply machine-applicable findings, revalidate and report. Returns true when the run is finished
/// (fixes were applied) so the caller stops, false when there was nothing to apply.
#[allow(clippy::too_many_arguments)]
fn apply_fixes(
    source: &input::InputSource,
    plan: &veriplan::ir::PlanIR,
    result: &checker::VerificationResult,
    annotated: &[annotator::AnnotatedViolation],
    revalidate: Revalidate<'_>,
) -> anyhow::Result<bool> {
    let findings = annotator::findings(result, annotated);
    let report = veriplan::fix::fix_plan(plan, &findings, &base_dir_for(source));

    if !report.applied.is_empty() {
        println!(
            "Applied {} machine-applicable fix(es):",
            report.applied.len()
        );
        for e in &report.applied {
            println!("  - [{}] {}", e.kind, e.description);
        }
        finish_after_fixes(source, revalidate)?;
        return Ok(true);
    }
    if !report.left_as_suggestions.is_empty() {
        println!(
            "{} finding(s) left as suggestions (structural/judgment):",
            report.left_as_suggestions.len()
        );
    }
    Ok(false)
}

/// What is needed to verify the plan again after edits have been written.
#[derive(Clone, Copy)]
struct Revalidate<'a> {
    label: &'a str,
    no_model: bool,
    pre_commit: bool,
    strictness: input::StrictnessProfile,
    is_openspec: bool,
    backend: checker::CheckerBackend,
    format: &'a str,
    verbose: bool,
}

/// The directory relative finding paths are resolved against.
fn base_dir_for(source: &input::InputSource) -> std::path::PathBuf {
    match source {
        input::InputSource::OpenSpec { change_dir, .. } => change_dir.clone(),
        input::InputSource::Directory { path, .. } => path.clone(),
        input::InputSource::SingleFile { path } => path
            .parent()
            .map_or_else(std::env::current_dir, |p| Ok(p.to_path_buf()))
            .unwrap_or_default(),
        _ => std::env::current_dir().unwrap_or_default(),
    }
}

/// Reload the edited plan, verify it again, print the report and exit on its verdict.
fn finish_after_fixes(source: &input::InputSource, v: Revalidate<'_>) -> anyhow::Result<()> {
    let reloaded = input::load_plan(source).map_err(|e| anyhow::anyhow!("{}", e))?;
    let plans = vec![(v.label.to_string(), reloaded)];
    let result = checker::verify_with_strictness(
        &plans[0].1,
        v.label,
        v.no_model,
        v.pre_commit,
        v.strictness,
        v.is_openspec,
        v.backend,
    );
    let annotated = annotator::annotate(&result, &plans);
    let rendered = match v.format {
        "json" => annotator::format_json(&result, &annotated, &plans, v.verbose),
        _ => annotator::format_human(&result, &annotated, &plans, v.verbose),
    };
    print!("{rendered}");
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    if let Some(code) = exit_code(&result, v.pre_commit, false) {
        cli::flush_exit(code);
    }
    Ok(())
}

/// Verify with one backend, and say how long it took.
#[allow(clippy::too_many_arguments)]
fn run_backend(
    plan: &veriplan::ir::PlanIR,
    label: &str,
    no_model: bool,
    pre_commit: bool,
    strictness: input::StrictnessProfile,
    is_openspec: bool,
    backend: checker::CheckerBackend,
) -> (checker::VerificationResult, std::time::Duration) {
    let started = std::time::Instant::now();
    let result = checker::verify_with_strictness(
        plan,
        label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
        backend,
    );
    (result, started.elapsed())
}

/// Compare one constraint and print its row, counting it when both backends decided it.
fn compare_one(
    c: &checker::ConstraintSummary,
    peer_result: &checker::VerificationResult,
    tally: &mut (u32, u32),
) {
    let peer = peer_result
        .constraints_summary
        .iter()
        .find(|sc| sc.requirement_id == c.requirement_id);
    let unverified = peer.map_or(peer_result.valid.is_none(), |sc| sc.unchecked);
    if unverified {
        println!(
            "{:<30} {:<10} {:<10} {:<8}",
            veriplan::util::truncate(&c.requirement_id, 30),
            if c.satisfied { "pass" } else { "FAIL" },
            "unverified",
            "—"
        );
        return;
    }
    let peer_satisfied = peer.is_some_and(|sc| sc.satisfied);
    let matched = c.satisfied == peer_satisfied;
    if !matched {
        tally.1 += 1;
    }
    tally.0 += 1;
    println!(
        "{:<30} {:<10} {:<10} {:<8}",
        veriplan::util::truncate(&c.requirement_id, 28),
        if c.satisfied { "pass" } else { "FAIL" },
        if peer_satisfied { "pass" } else { "FAIL" },
        if matched { "✓" } else { "✗" }
    );
}

/// Print one row per constraint and tally how many the two backends agree on.
///
/// A backend that produced no verdict for a constraint did not match anything; calling that a
/// mismatch would be as wrong as calling it a match, so it is printed as unverified and left out of
/// the tally.
fn compare_rows(
    spin_result: &checker::VerificationResult,
    spin_rs_result: &checker::VerificationResult,
) -> (u32, u32) {
    println!();
    println!(
        "{:<30} {:<10} {:<10} {:<8}",
        "Constraint", "spin", "spin-rs", "Match?"
    );
    println!("{}", "-".repeat(60));
    if spin_rs_result.valid.is_none() {
        println!("  spin-rs produced no verdict for this plan:");
        if let Some(reason) = &spin_rs_result.skip_reason {
            println!("    {reason}");
        }
        println!();
    }
    let mut tally = (0u32, 0u32);
    for c in &spin_result.constraints_summary {
        compare_one(c, spin_rs_result, &mut tally);
    }
    tally
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
    let (spin_result, spin_elapsed) = run_backend(
        plan,
        label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
        checker::CheckerBackend::Spin,
    );
    let (spin_rs_result, spin_rs_elapsed) = run_backend(
        plan,
        label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
        checker::CheckerBackend::SpinRs,
    );

    // Build comparison table
    let _spin_summary: std::collections::HashMap<&str, bool> = spin_result
        .constraints_summary
        .iter()
        .map(|c| (c.requirement_id.as_str(), c.satisfied))
        .collect();

    let (total, mismatches) = compare_rows(&spin_result, &spin_rs_result);

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
