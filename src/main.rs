use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use clap::{Parser, Subcommand};

use veriplan::annotator;
use veriplan::checker;
use veriplan::input;

mod cli;
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
            moderate,
            lax,
        } => run_check(
            change,
            phase.as_deref(),
            format.as_deref(),
            _verbose,
            pre_commit,
            stdin,
            strict,
            moderate,
            lax,
        ),
        Commands::Init { project_root } => run_init(project_root.as_deref()),
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

fn run_check(
    change_name: Option<String>,
    phase: Option<&str>,
    format: Option<&str>,
    verbose: bool,
    pre_commit: bool,
    stdin_flag: bool,
    strict: bool,
    moderate: bool,
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
    } else if moderate {
        veriplan::input::StrictnessProfile::Moderate
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
        cli::check_all_changes(
            &changes,
            &project_root,
            format.unwrap_or("human"),
            verbose,
            pre_commit,
            strictness,
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

fn run_init(project_root: Option<&str>) -> anyhow::Result<()> {
    let root = project_root
        .map(|p| Path::new(p).to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let config_path = root.join("openspec").join("config.yaml");
    merge_config(&config_path)?;

    println!("✓ Init complete: {}", config_path.display());

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

/// Merge veriplan's context and rules into openspec/config.yaml using YAML-aware merge.
fn merge_config(path: &std::path::Path) -> anyhow::Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let config = create_fresh_config();
        std::fs::write(path, config)?;
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    let existing: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))?;

    let merged = yaml_merge(&existing, VERIPLAN_CONTEXT, &veriplan_rules());
    let output = serde_yaml::to_string(&merged)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

    let output = format!("# veriplan init\n{}", output);
    std::fs::write(path, output)?;
    Ok(())
}

const VERIPLAN_CONTEXT: &str = "\
Every OpenSpec artifact must be machine-parseable into a formal\n\
state machine model AND clearly readable by a human reviewer.\n\
Write tasks, requirements, and constraints\n\
so they translate directly to states, transitions, and invariants.\n\
\n\
Structural rules:\n\
- Every task MUST have a unique N.M ID and belong to a named phase.\n\
- Phases execute in order. Tasks within a phase execute one at a time.\n\
  Mark a phase heading with [concurrent] if tasks are meant to run simultaneously.\n\
- Every requirement MUST use RFC 2119 keywords (MUST/SHALL/SHOULD/MAY/MUST NOT).\n\
- Every SHALL MUST reference at least one task by N.M ID (e.g. 'T3.2 SHALL complete before T3.9').\n\
- Every SHALL MUST use ONE temporal keyword: BEFORE (sequential),\n\
  AT MOST ONE (exclusive), IF...THEN (conditional), CONCURRENTLY (concurrent),\n\
  or ALWAYS (global invariant).\n\
- Put the SHALL sentence in a body paragraph — the heading alone is not parsed.\n\
- Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs').\n\
- Every scenario MUST have WHEN + THEN with an RFC 2119 keyword.\n\
- No vague verbs (\"be robust\", \"be user-friendly\").";

fn veriplan_rules() -> BTreeMap<String, Vec<String>> {
    let mut rules = BTreeMap::new();
    rules.insert(
        "proposal".to_string(),
        vec![
            "State the problem as a gap a state machine model can detect".to_string(),
            "List non-goals to bound the formal model".to_string(),
        ],
    );
    rules.insert(
        "specs".to_string(),
        vec![
            "Every requirement MUST use an RFC 2119 keyword (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)".to_string(),
            "Every SHALL MUST reference at least one task by N.M ID (e.g. 'T2.1 SHALL complete before T2.3')".to_string(),
            "Every SHALL MUST use ONE temporal keyword: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, or AT MOST ONE".to_string(),
            "Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed".to_string(),
            "Every spec file MUST open with a Task Reference section — a table listing each T N.M ID used in the file with a one-line description, placed before the first requirement heading. This helps human reviewers see which tasks are involved at a glance.".to_string(),
            "Every WHEN and THEN step SHOULD reference a task ID (e.g. 'WHEN T3.2 runs')".to_string(),
            "Avoid vague SHALLs ('be robust', 'be user-friendly')".to_string(),
            "GOOD: T2.1 SHALL complete BEFORE T3.1 SHALL run (references task IDs + temporal keyword)".to_string(),
            "BAD: The system SHALL auto-detect changes (no task ID, no temporal keyword — NonFormalizable)".to_string(),
            "IF...THEN is for failure-recovery: IF T1.1 fails THEN T2.1 SHALL run".to_string(),
            "For branching/decision logic, use BEFORE instead: T1.5 SHALL complete BEFORE T1.4".to_string(),
            "Every scenario MUST have WHEN + THEN with RFC 2119 keyword; GIVEN is optional".to_string(),
        ],
    );
    rules.insert(
        "design".to_string(),
        vec![
            "Each task maps to a single state variable".to_string(),
            "For every requirement, note its temporal category and the task IDs involved"
                .to_string(),
            "If a constraint cannot be formalised, mark it 'human review only'".to_string(),
        ],
    );
    rules.insert(
        "tasks".to_string(),
        vec![
            "Every task MUST have an N.M identifier (e.g. '1.3')".to_string(),
            "Group tasks under ## Phase headings".to_string(),
        ],
    );
    rules
}

/// Create a fresh config with schema + veriplan context and rules.
fn create_fresh_config() -> String {
    let mut config = serde_yaml::Mapping::new();
    config.insert(
        serde_yaml::Value::String("schema".to_string()),
        serde_yaml::Value::String("spec-driven".to_string()),
    );
    let merged = yaml_merge(
        &serde_yaml::Value::Mapping(config),
        VERIPLAN_CONTEXT,
        &veriplan_rules(),
    );
    let output = serde_yaml::to_string(&merged).unwrap();
    format!("# veriplan init\n{}", output)
}

/// Merge new context and rules into an existing YAML value.
/// Context is appended with a blank line separator, skip if already present.
/// Rules are merged per artifact type, deduplicating by exact string match.
fn yaml_merge(
    existing: &serde_yaml::Value,
    new_context: &str,
    new_rules: &BTreeMap<String, Vec<String>>,
) -> serde_yaml::Value {
    let mut merged = existing.clone();

    // Merge context: append with blank line separator, skip if already present
    match merged.get_mut("context") {
        Some(serde_yaml::Value::String(ctx)) => {
            let trimmed_new = new_context.trim();
            if !ctx.contains(trimmed_new) {
                let combined = format!("{}\n\n{}", ctx.trim(), trimmed_new);
                *ctx = combined;
            }
        }
        _ => {
            merged["context"] = serde_yaml::Value::String(new_context.to_string());
        }
    }

    // Merge rules: add new items per artifact type, deduplicating
    let mut rules = match merged.get("rules") {
        Some(serde_yaml::Value::Mapping(m)) => m.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    for (artifact_type, new_items) in new_rules {
        let key = serde_yaml::Value::String(artifact_type.clone());
        let existing_items = match rules.get(&key) {
            Some(serde_yaml::Value::Sequence(s)) => s.clone(),
            _ => serde_yaml::Sequence::new(),
        };

        let mut merged_items = existing_items.clone();
        for item in new_items {
            let item_val = serde_yaml::Value::String(item.clone());
            if !merged_items.contains(&item_val) {
                merged_items.push(item_val);
            }
        }

        rules.insert(key, serde_yaml::Value::Sequence(merged_items));
    }

    merged["rules"] = serde_yaml::Value::Mapping(rules);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_merge_config_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openspec").join("config.yaml");
        assert!(!path.exists());
        merge_config(&path).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# veriplan init"));
        assert!(content.contains("State the problem as a gap"));
        assert!(content.contains("Every task MUST have an N.M identifier"));
        // Context must not cite the tool
        assert!(!content.contains("veriplan checks"));
    }

    #[test]
    fn test_merge_config_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openspec").join("config.yaml");
        merge_config(&path).unwrap();
        let after_first = std::fs::read_to_string(&path).unwrap();
        // Run again — should not add duplicates
        merge_config(&path).unwrap();
        let after_second = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn test_merge_config_preserves_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openspec").join("config.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let existing = "schema: spec-driven\ncontext: |-\n  Use conventional commits\n";
        std::fs::write(&path, existing).unwrap();
        merge_config(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // Original content preserved
        assert!(content.contains("Use conventional commits"));
        // New content added
        assert!(content.contains("State the problem as a gap"));
        assert!(content.contains("Every task MUST have an N.M identifier"));
        // Context must not cite the tool
        assert!(!content.contains("veriplan checks"));
    }

    #[test]
    fn test_merge_config_rules_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openspec").join("config.yaml");
        merge_config(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // Context block — tool-agnostic
        assert!(content.contains("Every OpenSpec artifact must be machine-parseable"));
        assert!(content.contains("Every task MUST have a unique N.M ID"));
        assert!(content.contains("No vague verbs"));
        // Proposal rules
        assert!(content.contains("State the problem as a gap"));
        assert!(content.contains("List non-goals to bound the formal model"));
        // Specs rules
        assert!(content.contains("Every requirement MUST use an RFC 2119 keyword"));
        assert!(content.contains("Every SHALL MUST reference at least one task"));
        // Design rules
        assert!(content.contains("Each task maps to a single state variable"));
        // Tasks rules
        assert!(content.contains("Every task MUST have an N.M identifier"));
        assert!(content.contains("Group tasks under ## Phase headings"));
        // Must not cite the tool
        assert!(!content.contains("veriplan checks"));
    }

    #[test]
    fn test_yaml_merge_context_appends() {
        let existing: serde_yaml::Value =
            serde_yaml::from_str("context: |-\n  Original context\n").unwrap();
        let mut rules = BTreeMap::new();
        rules.insert("specs".to_string(), vec![]);
        let merged = yaml_merge(&existing, "New context", &rules);
        let ctx = merged["context"].as_str().unwrap();
        assert!(ctx.contains("Original context"));
        assert!(ctx.contains("New context"));
    }

    #[test]
    fn test_yaml_merge_rules_dedup() {
        let existing: serde_yaml::Value =
            serde_yaml::from_str("rules:\n  specs:\n    - \"Existing rule\"\n").unwrap();
        let mut rules = BTreeMap::new();
        rules.insert(
            "specs".to_string(),
            vec!["Existing rule".to_string(), "New rule".to_string()],
        );
        let merged = yaml_merge(&existing, "", &rules);
        let specs = merged["rules"]["specs"].as_sequence().unwrap();
        assert_eq!(specs.len(), 2);
        assert!(specs.iter().any(|v| v.as_str() == Some("Existing rule")));
        assert!(specs.iter().any(|v| v.as_str() == Some("New rule")));
    }
}
