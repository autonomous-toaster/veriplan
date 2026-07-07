use std::io::Write;

use clap::{Parser, Subcommand};

use veriplan::annotator;
use veriplan::checker;
use veriplan::input;

mod cli;
mod cmd_init;
mod cmd_visualize;

/// Supported plan formats.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum Format {
    /// OpenSpec specification format (default).
    Openspec,
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openspec" => Ok(Format::Openspec),
            other => Err(format!(
                "unknown format '{}'. Supported formats: openspec",
                other
            )),
        }
    }
}

#[derive(Parser)]
#[command(name = "veriplan", about = "Formal verification for OpenSpec plans")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run convertibility + model checking on a plan
    Check {
        /// Change name, file path, or directory. Use '-' for stdin. Omit to auto-detect.
        #[arg(required = false)]
        change: Option<String>,
        /// Alias for CHANGE (e.g., --change my-change)
        #[arg(long = "change", required = false)]
        change_alias: Option<String>,
        /// Stop after convertibility check (Phase 1)
        #[arg(long)]
        phase: Option<String>,
        /// Output format: human, json
        #[arg(long, default_value = "human")]
        format: Option<String>,
        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
        /// Pre-commit mode: missing SPIN is non-blocking, blockers exit 1, warnings exit 0
        #[arg(long)]
        pre_commit: bool,
        /// Read plan from stdin instead of a file
        #[arg(long)]
        stdin: bool,
        /// Strict checking: ungrounded patterns are blockers (default)
        #[arg(long)]
        strict: bool,
        /// Moderate checking: ungrounded patterns are warnings
        #[arg(long)]
        moderate: bool,
        /// Lax checking: ungrounded patterns are info
        #[arg(long)]
        lax: bool,
        /// Checker backend: spin (default) or spin-rs
        #[arg(long)]
        checker: Option<String>,
        /// Run both backends and compare results
        #[arg(long)]
        compare: bool,
    },
    /// Init openspec/config.yaml with formal-verification-friendly rules
    Init {
        /// Project root (defaults to cwd)
        #[arg(long)]
        project_root: Option<String>,
    },
    /// Visualize the plan as a state-machine diagram
    Visualize {
        /// Change name (e.g., "veriplan-plan-verifier")
        #[arg(required = false)]
        change: Option<String>,
        /// Output format: mermaid, dot, or markdown (default: mermaid)
        #[arg(long, default_value = "mermaid")]
        format: Option<String>,
        /// Output file (omit for stdout)
        #[arg(short)]
        output: Option<String>,
    },
    /// Run the LSP server over stdio (for editor integration)
    Lsp {
        /// Use stdio transport
        #[arg(long)]
        stdio: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Check {
            change,
            change_alias,
            phase,
            format,
            verbose: _verbose,
            pre_commit,
            stdin,
            strict,
            moderate,
            lax,
            checker,
            compare,
        } => run_check(
            change.or(change_alias),
            phase.as_deref(),
            format.as_deref(),
            _verbose,
            pre_commit,
            stdin,
            strict,
            moderate,
            lax,
            checker,
            compare,
        ),
        Commands::Init { project_root } => cmd_init::run_init(project_root.as_deref()),
        Commands::Visualize {
            change,
            format,
            output,
        } => cmd_visualize::run_visualize(change, format.as_deref(), output.as_deref()),
        Commands::Lsp { stdio: _stdio } => veriplan::lsp::run_lsp(),
    };

    // Flush stdio before exiting to avoid losing buffered output
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    result
}

#[allow(clippy::too_many_arguments)]
fn run_check(
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
        veriplan::input::StrictnessProfile::Lax
    } else if moderate {
        veriplan::input::StrictnessProfile::Moderate
    } else {
        veriplan::input::StrictnessProfile::Strict // default
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
    let source = veriplan::input::resolve_input(change_name.as_deref(), &project_root, stdin_flag)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let no_model = phase == Some("convertibility");

    // Detect PRE_COMMIT env var for auto-enabling pre-commit mode
    let pre_commit = pre_commit || std::env::var("PRE_COMMIT").as_deref() == Ok("1");

    // Handle MultiOpenSpec case
    if let veriplan::input::InputSource::MultiOpenSpec {
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
    if let veriplan::input::InputSource::Empty { path, reason } = source {
        let message = match reason {
            veriplan::input::EmptyReason::NoContent => {
                format!(
                    "No verifiable content found in {} — skipping verification",
                    path.display()
                )
            }
            veriplan::input::EmptyReason::NoActiveChanges => {
                format!(
                    "No active changes found in {} — skipping verification",
                    path.display()
                )
            }
        };
        println!("{}", message);
        return Ok(());
    }

    // Load plan from the resolved source
    let plan = veriplan::input::load_plan(&source).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Determine name for display
    let label = source.label();
    let is_openspec = source.is_openspec();

    // Comparison mode: run both backends and diff
    if compare {
        return run_compare(&plan, &label, no_model, pre_commit, strictness, is_openspec, format.unwrap_or("human"), verbose);
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
    strictness: veriplan::input::StrictnessProfile,
    is_openspec: bool,
    _format: &str,
    _verbose: bool,
) -> anyhow::Result<()> {
    use std::time::Instant;

    // Run spin backend
    let spin_start = Instant::now();
    let spin_result = checker::verify_with_strictness(
        plan, label, no_model, pre_commit, strictness, is_openspec,
        checker::CheckerBackend::Spin,
    );
    let spin_elapsed = spin_start.elapsed();

    // Run spin-rs backend
    let spin_rs_start = Instant::now();
    let spin_rs_result = checker::verify_with_strictness(
        plan, label, no_model, pre_commit, strictness, is_openspec,
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
    println!("{:<30} {:<10} {:<10} {:<8}", "Constraint", "spin", "spin-rs", "Match?");
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
            truncate(&c.requirement_id, 28),
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
