use std::path::Path;

use clap::{Parser, Subcommand};

use veriplan::annotator;
use veriplan::checker;
use veriplan::parser;

#[derive(Parser)]
#[command(name = "veriplan", about = "Formal verification for OpenSpec plans")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run convertibility + model checking on an OpenSpec change
    Check {
        /// Change name (e.g., "veriplan-plan-verifier")
        change: String,
        /// Stop after convertibility check (Phase 1)
        #[arg(long)]
        phase: Option<String>,
        /// Output format
        #[arg(long, default_value = "human")]
        format: Option<String>,
        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },
    /// Init openspec/config.yaml with formal-verification-friendly rules
    Init {
        /// Project root (defaults to cwd)
        #[arg(long)]
        project_root: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check {
            change,
            phase,
            format,
            verbose: _verbose,
        } => run_check(&change, phase.as_deref(), format.as_deref(), _verbose),
        Commands::Init { project_root } => run_init(project_root.as_deref()),
    }
}

fn run_check(
    change_name: &str,
    phase: Option<&str>,
    format: Option<&str>,
    verbose: bool,
) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let change_dir = find_change_dir(&project_root, change_name)?;

    let plan = parser::parse_plan(&change_dir).map_err(|e| anyhow::anyhow!(e))?;

    let no_model = phase == Some("convertibility");
    // Use just the last path component for display even if user passes a full path
    let display_name = std::path::Path::new(change_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(change_name);
    let result = checker::verify(&plan, display_name, no_model);

    let annotated = annotator::annotate(&result, &plan);

    match format.unwrap_or("human") {
        "json" => println!(
            "{}",
            annotator::format_json(&result, &annotated, &plan, verbose)
        ),
        _ => print!(
            "{}",
            annotator::format_human(&result, &annotated, &plan, verbose)
        ),
    }

    // Exit codes: 0 = valid/pass, 1 = invalid/fail, 2 = blocking convertibility
    if !result.convertible {
        std::process::exit(2);
    } else if result.valid == Some(false) {
        std::process::exit(1);
    } else if no_model
        && !result
            .convertibility_report
            .as_ref()
            .is_none_or(|r| r.warnings.is_empty())
    {
        // In convertibility-only mode, warnings still count as non-clean
        // but don't exit with error
        std::process::exit(0);
    }

    Ok(())
}

fn find_change_dir(project_root: &Path, change_name: &str) -> anyhow::Result<std::path::PathBuf> {
    let direct = Path::new(change_name).to_path_buf();
    let candidates = [
        project_root
            .join("openspec")
            .join("changes")
            .join(change_name),
        project_root.join(change_name),
        direct,
    ];

    for candidate in &candidates {
        if candidate.join("tasks.md").exists() || candidate.join("specs").exists() {
            return Ok(candidate.clone());
        }
    }

    // If not found, try to find any openspec changes directory
    let changes_dir = project_root.join("openspec").join("changes");
    if changes_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&changes_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        anyhow::bail!(
            "Change '{}' not found. Available changes: {:?}",
            change_name,
            entries
        );
    }

    anyhow::bail!("Change directory not found for '{}'", change_name);
}

// ═══════════════════════════════════════════════════════════════
// Bootstrap command
// ═══════════════════════════════════════════════════════════════

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

    Ok(())
}

/// Merge formal-verification rules into existing config content.
/// Preserves existing context and rules, only adds missing pieces.
fn merge_config(existing: &str) -> String {
    let existing = existing.trim();

    // If empty, emit the full config
    if existing.is_empty() {
        return BOOTSTRAP_CONFIG.to_string();
    }

    // Parse existing to check what's already there
    let has_context = existing.contains("context:");
    let has_rules = existing.contains("rules:");

    let mut result = String::new();
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();

    // We need to be careful about YAML merging.
    // Simple approach: check if context is present, add if not
    // Check if rules keys are present, add missing ones

    if !has_context {
        // Find schema line and insert context after it
        let insert_pos = lines.iter().position(|l| l.starts_with("schema:"));
        if let Some(pos) = insert_pos {
            lines.insert(pos + 1, String::new());
            lines.insert(pos + 2, "# Added by veriplan init".to_string());
            let context_lines: Vec<&str> = BOOTSTRAP_CONTEXT.trim().lines().collect();
            let start_idx = pos + 3;
            for (i, line) in context_lines.iter().enumerate() {
                lines.insert(start_idx + i, line.to_string());
            }
        }
    }

    if !has_rules {
        // Append rules at the end
        lines.push(String::new());
        lines.push("# Rules added by veriplan init".to_string());
        let rules_lines: Vec<&str> = BOOTSTRAP_RULES.trim().lines().collect();
        for line in &rules_lines {
            lines.push(line.to_string());
        }
    } else {
        // Rules section exists — check for missing artifact keys
        // Parse existing artifact keys (owned to avoid borrow conflict)
        let existing_keys: Vec<String> = lines
            .iter()
            .filter(|l| l.starts_with("  ") && !l.starts_with("    ") && l.contains(':'))
            .map(|l| l.trim().trim_end_matches(':').trim().to_string())
            .collect();

        let wanted_keys = ["proposal", "specs", "design", "tasks"];
        let rules_line_idx = lines.iter().position(|l| l.trim() == "rules:");

        for key in &wanted_keys {
            if existing_keys.iter().any(|k| k == key) {
                continue;
            }
            if let Some(rules_line) = rules_line_idx {
                let insert_at = rules_line + 1 + existing_keys.len();
                if insert_at <= lines.len() {
                    lines.insert(insert_at, format!("  {}:", key));
                    let template_rules = get_rules_for_artifact(key);
                    for rule in &template_rules {
                        lines.push(format!("    - {}", rule));
                    }
                }
            }
        }
    }

    result.push_str(&lines.join("\n"));
    result.push('\n');
    result
}

fn get_rules_for_artifact(key: &str) -> Vec<&'static str> {
    match key {
        "proposal" => vec![
            "State the problem as a gap a state machine model can detect",
            "List non-goals to bound the formal model",
        ],
        "specs" => vec![
            "Every requirement MUST use an RFC 2119 keyword (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)",
            "Every SHALL MUST reference at least one task by N.M ID (e.g. 'T2.1 SHALL complete before T2.3')",
            "Every SHALL MUST use ONE temporal keyword: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, or AT MOST ONE",
            "Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed",
            "Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs')",
            "Avoid vague SHALLs ('be robust', 'be user-friendly')",
            "Every scenario MUST have WHEN + THEN with RFC 2119 keyword; GIVEN is optional",
        ],
        "design" => vec![
            "Each task maps to a single state variable",
            "For every requirement, note its temporal category and the task IDs involved",
            "If a constraint cannot be formalised, mark it 'human review only'",
        ],
        "tasks" => vec![
            "Every task MUST have an N.M identifier (e.g. '1.3')",
            "Group tasks under ## Phase headings",
        ],
        _ => vec![],
    }
}

// ═══════════════════════════════════════════════════════════════
// Bootstrap config template (compact)
// ═══════════════════════════════════════════════════════════════

const BOOTSTRAP_CONTEXT: &str = r#"context: |-
  Every OpenSpec artifact must be machine-parseable into a formal
  state machine model. Write tasks, requirements, and constraints
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
  - No vague verbs ("be robust", "be user-friendly")."#;

const BOOTSTRAP_RULES: &str = r#"rules:
  proposal:
    - State the problem as a gap a state machine model can detect
    - List non-goals to bound the formal model
  specs:
    - Every requirement MUST use an RFC 2119 keyword (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)
    - Every SHALL MUST reference at least one task by N.M ID (e.g. 'T2.1 SHALL complete before T2.3')
    - Every SHALL MUST use ONE temporal keyword: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, or AT MOST ONE
    - Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed
    - Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs')
    - Avoid vague SHALLs ('be robust', 'be user-friendly')
    - Every scenario MUST have WHEN + THEN with RFC 2119 keyword; GIVEN is optional
  design:
    - Each task maps to a single state variable
    - For every requirement, note its temporal category and the task IDs involved
    - If a constraint cannot be formalised, mark it 'human review only'
  tasks:
    - Every task MUST have an N.M identifier (e.g. '1.3')
    - Group tasks under ## Phase headings"#;

const BOOTSTRAP_CONFIG: &str = r#"schema: spec-driven

# Added by veriplan init
context: |-
  Every OpenSpec artifact must be machine-parseable into a formal
  state machine model. Write tasks, requirements, and constraints
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
    - Every requirement MUST use an RFC 2119 keyword (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)
    - Every SHALL MUST reference at least one task by N.M ID (e.g. 'T2.1 SHALL complete before T2.3')
    - Every SHALL MUST use ONE temporal keyword: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, or AT MOST ONE
    - Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed
    - Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs')
    - Avoid vague SHALLs ('be robust', 'be user-friendly')
    - Every scenario MUST have WHEN + THEN with RFC 2119 keyword; GIVEN is optional
  design:
    - Each task maps to a single state variable
    - For every requirement, note its temporal category and the task IDs involved
    - If a constraint cannot be formalised, mark it 'human review only'
  tasks:
    - Every task MUST have an N.M identifier (e.g. '1.3')
    - Group tasks under ## Phase headings
"#;
