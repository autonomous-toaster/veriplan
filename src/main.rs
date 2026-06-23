use std::io::Write;
use std::path::Path;

use clap::{Parser, Subcommand};

use veriplan::annotator;
use veriplan::checker;
use veriplan::ir::PlanIR;
use veriplan::parser;
use veriplan::translator;
use veriplan::visualizer;

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
        #[arg(long, visible_alias = "moderate")]
        lax: bool,
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
            phase,
            format,
            verbose: _verbose,
            pre_commit,
            stdin,
            strict,
            lax,
        } => run_check(
            change,
            phase.as_deref(),
            format.as_deref(),
            _verbose,
            pre_commit,
            stdin,
            strict,
            lax,
        ),
        Commands::Init { project_root } => run_init(project_root.as_deref()),
        Commands::Visualize {
            change,
            format,
            output,
        } => run_visualize(change, format.as_deref(), output.as_deref()),
        Commands::Lsp { stdio: _stdio } => veriplan::lsp::run_lsp(),
    };

    // Flush stdio before exiting to avoid losing buffered output
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    result
}

/// Flush stdio and exit with the given code.
/// Always flushes stdout and stderr before calling process::exit.
fn flush_exit(code: i32) -> ! {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}

fn run_check(
    change_name: Option<String>,
    phase: Option<&str>,
    format: Option<&str>,
    verbose: bool,
    pre_commit: bool,
    stdin_flag: bool,
    strict: bool,
    lax: bool,
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
    } else if strict {
        veriplan::input::StrictnessProfile::Strict
    } else {
        veriplan::input::StrictnessProfile::Strict // default
    };

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
        check_all_changes(
            &changes,
            &project_root,
            format.unwrap_or("human"),
            verbose,
            pre_commit,
            strictness,
        )?;
        return Ok(());
    }

    // Load plan from the resolved source
    let plan = veriplan::input::load_plan(&source).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Determine name for display
    let label = source.label();
    let is_openspec = source.is_openspec();

    // Run checker with strictness profile
    let result = checker::verify_with_strictness(
        &plan,
        &label,
        no_model,
        pre_commit,
        strictness,
        is_openspec,
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
            flush_exit(1);
        } else {
            flush_exit(2);
        }
    } else if result.valid == Some(false) {
        if pre_commit {
            eprintln!(
                "\nCommit blocked. Fix violations above, or skip with: VERIPLAN_SKIP=1 git commit"
            );
        }
        flush_exit(1);
    } else if let Some(_reason) = &result.skip_reason {
        if pre_commit {
            // Missing SPIN in pre-commit mode: warn but don't block
            eprintln!(
                "⚠ SPIN not found — skipping model checking. Install SPIN for full verification."
            );
            flush_exit(0);
        } else {
            flush_exit(2);
        }
    } else if no_model
        && !result
            .convertibility_report
            .as_ref()
            .is_none_or(|r| r.warnings.is_empty())
    {
        flush_exit(0);
    }

    Ok(())
}

fn run_visualize(
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

// ═══════════════════════════════════════════════════════════════
// Bootstrap command
// ═══════════════════════════════════════════════════════════════

/// Check all changes sequentially and print summary.
fn check_all_changes(
    changes: &[std::string::String],
    project_root: &Path,
    format: &str,
    verbose: bool,
    pre_commit: bool,
    strictness: veriplan::input::StrictnessProfile,
) -> anyhow::Result<()> {
    let mut results = Vec::new();

    for change in changes {
        let source = veriplan::input::InputSource::OpenSpec {
            change_dir: project_root.join("openspec/changes").join(change),
            change_name: change.clone(),
        };

        let plan = veriplan::input::load_plan(&source).map_err(|e| anyhow::anyhow!("{}", e))?;
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
    if results.iter().all(|(_, r)| r.valid.unwrap_or(false)) {
        Ok(())
    } else {
        flush_exit(1);
    }
}

/// Print human-readable output for multiple changes.
fn print_multi_human(results: &[(String, checker::VerificationResult)], _verbose: bool) {
    let total = results.len();
    let invalid: Vec<_> = results
        .iter()
        .filter(|(_, r)| !r.valid.unwrap_or(false))
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
fn print_multi_json(results: &[(String, checker::VerificationResult)]) {
    let mut changes_json = Vec::new();
    let mut invalid_changes = Vec::new();

    for (name, result) in results {
        changes_json.push(serde_json::json!({
            "name": name,
            "valid": result.valid,
            "plan_name": result.plan_name,
        }));

        if !result.valid.unwrap_or(false) {
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

fn run_init(project_root: Option<&str>) -> anyhow::Result<()> {
    let root = project_root
        .map(|p| Path::new(p).to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let config_path = root.join("openspec").join("config.yaml");

    // Read existing config if any
    let mut existing_content = String::new();
    if config_path.exists() {
        existing_content = std::fs::read_to_string(&config_path)?;
    }

    let merged = merge_config(&existing_content);
    std::fs::write(&config_path, &merged)?;

    println!("✓ Init complete: {}", config_path.display());
    if existing_content.is_empty() {
        println!("  Created new config.yaml with formal-verification rules");
    } else {
        println!("  Merged rules into existing config.yaml (no duplicates)");
    }

    // Update .gitignore with SPIN trail files
    update_gitignore(&root)?;

    Ok(())
}

/// Add SPIN trail-file entries to .gitignore if missing.
fn update_gitignore(root: &Path) -> anyhow::Result<()> {
    let gitignore_path = root.join(".gitignore");
    let mut content = String::new();
    let mut has_veriplan_marker = false;

    if gitignore_path.exists() {
        content = std::fs::read_to_string(&gitignore_path)?;
        has_veriplan_marker = content.contains("# veriplan init");
    }

    if has_veriplan_marker {
        return Ok(()); // already configured
    }

    // Append SPIN-related entries
    let entries = vec![
        "",
        "# veriplan init — SPIN model checker artifacts",
        "*.trail",
        "pan.*",
    ];

    // If .gitignore is non-empty and doesn't end with newline, add one
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    for line in &entries {
        content.push_str(line);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, &content)?;
    println!("  Added SPIN trail-file entries to .gitignore");

    Ok(())
}

/// Merge formal-verification rules into existing config content.
/// Preserves existing context and rules, only adds missing pieces.
fn merge_config(existing: &str) -> String {
    let existing = existing.trim();

    // Empty → full config
    if existing.is_empty() {
        return BOOTSTRAP_CONFIG.to_string();
    }

    // If our marker exists, replace everything from it onward (reentrant)
    // Try full old marker first, then new marker (substring-safe)
    let marker_pos = existing
        .find("# Added by veriplan init")
        .or_else(|| existing.find(VERIPLAN_MARKER));

    if let Some(pos) = marker_pos {
        let before = existing[..pos].trim_end();
        return format!("{}\n\n{}", before, BOOTSTRAP_SUFFIX.trim());
    }

    // No marker yet — append the suffix at end
    format!("{}\n\n{}", existing.trim_end(), BOOTSTRAP_SUFFIX.trim())
}

const VERIPLAN_MARKER: &str = "# veriplan init";

const BOOTSTRAP_SUFFIX: &str = r#"# veriplan init
context: |-
  Every OpenSpec artifact must be machine-parseable into a formal
  state machine model AND clearly readable by a human reviewer.
  Write tasks, requirements, and constraints
  so they translate directly to states, transitions, and invariants.

  Structural rules:
  - Every task MUST have a unique N.M ID and belong to a named phase.
  - Phases execute in order. Tasks within a phase execute one at a time.
    Mark a phase heading with [concurrent] if tasks are meant to run simultaneously.
  - Every requirement MUST use RFC 2119 keywords (MUST/SHALL/SHOULD/MAY/MUST NOT).
  - Every SHALL MUST reference at least one task by N.M ID (e.g. 'T3.2 SHALL complete before T3.9').
  - Every SHALL MUST use ONE temporal keyword: BEFORE (sequential),
    AT MOST ONE (exclusive), IF...THEN (conditional), CONCURRENTLY (concurrent),
    or ALWAYS (global invariant).
  - Put the SHALL sentence in a body paragraph — the heading alone is not parsed.
  - Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs').
  - Every scenario MUST have WHEN + THEN with an RFC 2119 keyword.
  - No vague verbs ("be robust", "be user-friendly").

rules:
  proposal:
    - State the problem as a gap a state machine model can detect
    - List non-goals to bound the formal model
  specs:
    - "Every requirement MUST use an RFC 2119 keyword (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)"
    - "Every SHALL MUST reference at least one task by N.M ID (e.g. 'T2.1 SHALL complete before T2.3')"
    - "Every SHALL MUST use ONE temporal keyword: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, or AT MOST ONE"
    - "Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed"
    - "Every spec file MUST open with a Task Reference section — a table listing each T N.M ID used in the file with a one-line description, placed before the first requirement heading. This helps human reviewers see which tasks are involved at a glance."
    - "Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs')"
    - "Avoid vague SHALLs ('be robust', 'be user-friendly')"
    - "GOOD: T2.1 SHALL complete BEFORE T3.1 SHALL run (references task IDs + temporal keyword)"
    - "BAD: The system SHALL auto-detect changes (no task ID, no temporal keyword — NonFormalizable)"
    - "IF...THEN is for failure-recovery: IF T1.1 fails THEN T2.1 SHALL run"
    - "For branching/decision logic, use BEFORE instead: T1.5 SHALL complete BEFORE T1.4"
    - "Every scenario MUST have WHEN + THEN with RFC 2119 keyword; GIVEN is optional"
  design:
    - Each task maps to a single state variable
    - "For every requirement, note its temporal category and the task IDs involved"
    - "If a constraint cannot be formalised, mark it 'human review only'"
  tasks:
    - "Every task MUST have an N.M identifier (e.g. '1.3')"
    - "Group tasks under ## Phase headings"
"#;

const BOOTSTRAP_CONFIG: &str = r#"schema: spec-driven

# veriplan init
context: |-
  Every OpenSpec artifact must be machine-parseable into a formal
  state machine model AND clearly readable by a human reviewer.
  Write tasks, requirements, and constraints
  so they translate directly to states, transitions, and invariants.

  Structural rules:
  - Every task MUST have a unique N.M ID and belong to a named phase.
  - Phases execute in order. Tasks within a phase execute one at a time.
    Mark a phase heading with [concurrent] if tasks are meant to run simultaneously.
  - Every requirement MUST use RFC 2119 keywords (MUST/SHALL/SHOULD/MAY/MUST NOT).
  - Every SHALL MUST reference at least one task by N.M ID (e.g. 'T3.2 SHALL complete before T3.9').
  - Every SHALL MUST use ONE temporal keyword: BEFORE (sequential),
    AT MOST ONE (exclusive), IF...THEN (conditional), CONCURRENTLY (concurrent),
    or ALWAYS (global invariant).
  - Put the SHALL sentence in a body paragraph — the heading alone is not parsed.
  - Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs').
  - Every scenario MUST have WHEN + THEN with an RFC 2119 keyword.
  - No vague verbs ("be robust", "be user-friendly").

rules:
  proposal:
    - State the problem as a gap a state machine model can detect
    - List non-goals to bound the formal model
  specs:
    - "Every requirement MUST use an RFC 2119 keyword (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)"
    - "Every SHALL MUST reference at least one task by N.M ID (e.g. 'T2.1 SHALL complete before T2.3')"
    - "Every SHALL MUST use ONE temporal keyword: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, or AT MOST ONE"
    - "Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed"
    - "Every spec file MUST open with a Task Reference section — a table listing each T N.M ID used in the file with a one-line description, placed before the first requirement heading. This helps human reviewers see which tasks are involved at a glance."
    - "Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs')"
    - "Avoid vague SHALLs ('be robust', 'be user-friendly')"
    - "GOOD: T2.1 SHALL complete BEFORE T3.1 SHALL run (references task IDs + temporal keyword)"
    - "BAD: The system SHALL auto-detect changes (no task ID, no temporal keyword — NonFormalizable)"
    - "IF...THEN is for failure-recovery: IF T1.1 fails THEN T2.1 SHALL run"
    - "For branching/decision logic, use BEFORE instead: T1.5 SHALL complete BEFORE T1.4"
    - "Every scenario MUST have WHEN + THEN with RFC 2119 keyword; GIVEN is optional"
  design:
    - Each task maps to a single state variable
    - "For every requirement, note its temporal category and the task IDs involved"
    - "If a constraint cannot be formalised, mark it 'human review only'"
  tasks:
    - "Every task MUST have an N.M identifier (e.g. '1.3')"
    - "Group tasks under ## Phase headings"
"#;
