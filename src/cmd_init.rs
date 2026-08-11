use std::collections::BTreeMap;
use std::path::Path;

pub fn run_init(project_root: Option<&str>) -> anyhow::Result<()> {
    let root = match project_root {
        Some(p) => Path::new(p).to_path_buf(),
        None => std::env::current_dir()?,
    };

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

    let output = format!("# OpenSpec configuration\n{}", output);
    std::fs::write(path, output)?;
    Ok(())
}

const VERIPLAN_CONTEXT: &str = "\
Every artifact must be machine-parseable into a formal state machine model\n\
AND clearly readable by a human. Write so that tasks, requirements, and\n\
constraints translate directly to states, transitions, and invariants.\n\
\n\
Two rules apply across all artifacts:\n\
- TASK IDS: a task '1.3' in tasks.md is referenced as 'T1.3' in a spec\n\
  requirement. The T prefix is REQUIRED in spec references.\n\
- 'human review only': a capability/policy that is NOT a temporal constraint\n\
  must be marked 'human review only' in the requirement body. It is then\n\
  treated as informational (INFO), not a blocker.\n\
\n\
Per-artifact rules below. Write requirements that an objective test can verify.\n\
Avoid vague verbs (\"be robust\", \"be user-friendly\").";
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
            "Reference tasks by explicit ID: 'T2.1' not 'the migration step'".to_string(),
            "Use parenthetical syntax for task IDs: 'the migration step (T2.1)'".to_string(),
            "BEFORE requires two task IDs: 'T2.1 SHALL complete BEFORE T3.1 SHALL run'".to_string(),
            "ALWAYS requires one task ID: 'ALWAYS T2.1 SHALL validate input'".to_string(),
            "Write requirement bodies in ACTIVE voice and name the acting task by ID (e.g. 'T2.1 SHALL resolve the path'), not passive prose like 'the path SHALL be resolved' — passive/hedged prose grounds poorly".to_string(),
            "Keep one temporal constraint per SHALL statement — do not cram two constraints into one requirement body".to_string(),
            "Keep each sentence under 30 words; split long requirement bodies into shorter sentences".to_string(),
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
            "Write descriptive task descriptions — they become aliases for grounding".to_string(),
            "Keep task descriptions terse and imperative (one instruction per sentence)".to_string(),
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
    let output = serde_yaml::to_string(&merged).unwrap_or_else(|e| {
        eprintln!("Failed to serialize config: {e}");
        String::new()
    });
    format!("# OpenSpec configuration\n{}", output)
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
        assert!(content.contains("# OpenSpec configuration"));
        assert!(content.contains("State the problem as a gap"));
        assert!(content.contains("Every task MUST have an N.M identifier"));
        // Config must not cite any tool
        assert!(!content.contains("veriplan"), "config must not name the tool");
        assert!(!content.contains("steve"), "config must not name tools");
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
        // Context block — lean and tool-agnostic
        assert!(content.contains("Every artifact must be machine-parseable"));
        assert!(content.contains("TASK IDS"));
        assert!(content.contains("Avoid vague verbs"));
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
    fn test_merge_config_includes_lean_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openspec").join("config.yaml");
        merge_config(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // Context: lean, tool-free, no tool citations.
        assert!(content.contains("TASK IDS"));
        assert!(content.contains("human review only"));
        assert!(!content.contains("steve"), "config must not cite tool names");
        // Specs rules: core guidance present.
        assert!(content.contains("one temporal constraint per SHALL statement"));
        assert!(content.contains("sentence under 30 words"));
        // Tasks rules present.
        assert!(content.contains("terse and imperative"));
        // The config must round-trip as valid YAML with all rules intact.
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        let specs = parsed["rules"]["specs"].as_sequence().unwrap();
        assert!(
            specs.iter().any(|r| r.as_str().unwrap().contains("ACTIVE voice")),
            "specs rules must include the active-voice rule"
        );
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

    #[test]
    fn test_update_gitignore_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = update_gitignore(dir.path());
        assert!(result.is_ok());
        let gitignore = dir.path().join(".gitignore");
        assert!(gitignore.exists());
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("veriplan init"));
        assert!(content.contains("*.trail"));
    }

    #[test]
    fn test_update_gitignore_already_has_marker() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        std::fs::write(&gitignore, "# veriplan init\n*.trail\n").unwrap();
        let result = update_gitignore(dir.path());
        assert!(result.is_ok());
        // Content should not be duplicated
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert_eq!(content.lines().filter(|l| *l == "*.trail").count(), 1);
    }

    #[test]
    fn test_update_gitignore_appends_to_existing() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        std::fs::write(&gitignore, "target/\n").unwrap();
        let result = update_gitignore(dir.path());
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("target/"));
        assert!(content.contains("veriplan init"));
    }
}
