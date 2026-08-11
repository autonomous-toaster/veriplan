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
You are authoring an OpenSpec CHANGE. A change is a set of artifacts describing\n\
one unit of work:\n\
  - proposal.md  — WHY the change is needed (narrative)\n\
  - specs/<capability>/spec.md — the CONTRACT to be implemented (machine-verified)\n\
  - design.md    — HOW to implement it (narrative)\n\
  - tasks.md     — the work breakdown, as a task list\n\
\n\
WRITE ORDER matters:\n\
1. tasks.md first — it defines the task IDs that specs reference.\n\
2. specs/<capability>/spec.md — requirements reference those task IDs.\n\
3. proposal.md and design.md — narrative; they do NOT need task references.\n\
\n\
TASK IDs: every task has an ID in the form N.M (e.g. '1.3'), grouped under a\n\
'## Phase' heading. A spec references a task by prefixing its ID with 'T'\n\
(so task '1.3' is referenced as 'T1.3' in a spec).\n\
\n\
The spec is machine-verified, so its requirement bodies must follow a strict\n\
grammar (see the specs rules). proposal.md and design.md are free-form narrative\n\
and are NOT machine-verified — write them naturally, in plain prose.";

fn veriplan_rules() -> BTreeMap<String, Vec<String>> {
    let mut rules = BTreeMap::new();
    rules.insert(
        "proposal".to_string(),
        vec![
            "State the problem as a gap that this change should close".to_string(),
            "List non-goals to bound the scope".to_string(),
            "Narrative prose is fine here — do NOT use temporal constraint grammar".to_string(),
        ],
    );
    rules.insert(
        "specs".to_string(),
        vec![
            "Every requirement MUST use an RFC 2119 keyword: MUST, SHALL, SHOULD, MAY, MUST NOT, SHALL NOT".to_string(),
            "A requirement BODY must be a temporal constraint. Grammar: \"<task> SHALL <action> <TEMPORAL> <task> SHALL <action>\". Exactly ONE temporal keyword per requirement body.".to_string(),
            "Temporal keywords: BEFORE (one completes first), AFTER (one starts after another), CONCURRENTLY (run together), IF...THEN (failure triggers recovery), ALWAYS (invariant holds), AT MOST ONE (mutually exclusive)".to_string(),
            "Every SHALL MUST reference at least one real task ID from tasks.md, using the T prefix (e.g. T1.1)".to_string(),
            "Put the SHALL sentence in a body paragraph AFTER the heading — the heading alone is not parsed".to_string(),
            "Do NOT cram two constraints into one requirement body. One SHALL = one temporal keyword. If you need two orderings, write two requirements.".to_string(),
            "If a requirement is a capability/policy, not a temporal constraint, write the literal marker 'human review only' in the body".to_string(),
            "Give each requirement a \"#### Scenario:\" block with **GIVEN** (optional), **WHEN**, and **THEN** steps".to_string(),
            "**WHEN** and **THEN** steps SHOULD reference a task ID (e.g. \"WHEN T3.2 runs\") and use an RFC 2119 keyword".to_string(),
            "GOOD requirement body: \"T2.1 SHALL complete BEFORE T3.1 SHALL run\"  (task IDs + one temporal keyword)".to_string(),
            "GOOD scenario: \"**WHEN** T2.1 completes\" then \"**THEN** T3.1 SHALL run\"".to_string(),
            "BAD: \"The system SHALL auto-detect changes\"  (no task ID, no temporal keyword)".to_string(),
            "BAD: \"The migration SHALL happen\"  (no task ID, no temporal keyword)".to_string(),
            "BAD: 'T1.1 SHALL be done quickly'  ('quickly' is vague — define it measurably or use a temporal relation)".to_string(),
            "IF...THEN is for failure-recovery: \"IF T1.1 fails THEN T2.1 SHALL run\"".to_string(),
            "Write in ACTIVE voice and name the acting task by ID (e.g. \"T2.1 SHALL resolve the path\"), not passive prose like \"the path SHALL be resolved\"".to_string(),
            "Keep sentences short (<30 words); avoid vague words (robust, clean, good, user-friendly, efficiently)".to_string(),
            "Every spec file MUST open with a Task Reference table listing each task ID used, before the first requirement".to_string(),
        ],
    );
    rules.insert(
        "design".to_string(),
        vec![
            "Narrative prose is fine here — do NOT use temporal constraint grammar".to_string(),
            "Describe how the implementation will satisfy the spec".to_string(),
            "Note which tasks and requirements are affected".to_string(),
        ],
    );
    rules.insert(
        "tasks".to_string(),
        vec![
            "Every task MUST have an N.M identifier (e.g. 1.3)".to_string(),
            "Group tasks under \"## Phase\" headings".to_string(),
            "Write each task as a short, imperative action (one instruction per task)".to_string(),
            "Task descriptions become aliases for matching — make them descriptive but concise".to_string(),
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
        // Context block — workflow-first, tool-agnostic
        assert!(content.contains("You are authoring an OpenSpec CHANGE"));
        assert!(content.contains("WRITE ORDER matters"));
        assert!(content.contains("N.M") && content.contains("T prefix"));
        // Proposal rules — narrative
        assert!(content.contains("State the problem as a gap"));
        assert!(content.contains("List non-goals to bound the scope"));
        assert!(content.contains("Narrative prose is fine here"));
        // Specs rules — the machine-verified grammar
        assert!(content.contains("Every requirement MUST use an RFC 2119 keyword"));
        assert!(content.contains("temporal constraint"));
        assert!(content.contains("Temporal keywords: BEFORE"));
        assert!(content.contains("AT MOST ONE"));
        // Design rules — narrative
        assert!(content.contains("Describe how the implementation will satisfy the spec"));
        // Tasks rules
        assert!(content.contains("Every task MUST have an N.M identifier"));
        assert!(content.contains("## Phase"));
        // Must not cite the tool
        assert!(!content.contains("veriplan checks"));
        assert!(!content.contains("steve"));
    }

    #[test]
    fn test_merge_config_includes_lean_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openspec").join("config.yaml");
        merge_config(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // Context: workflow-first, tool-free.
        assert!(content.contains("WRITE ORDER"));
        assert!(content.contains("human review only"));
        assert!(!content.contains("steve"), "config must not cite tool names");
        assert!(!content.contains("veriplan"), "config must not cite tool names");
        assert!(!content.contains("NonFormalizable"), "no internal jargon");
        // Specs rules: core grammar present.
        assert!(content.contains("temporal constraint"));
        assert!(content.contains("AT MOST ONE"));
        assert!(content.contains("<30 words"));
        // The config must round-trip as valid YAML with all rules intact.
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        let specs = parsed["rules"]["specs"].as_sequence().unwrap();
        assert!(
            specs.iter().any(|r| r.as_str().unwrap().contains("ACTIVE voice")),
            "specs rules must include the active-voice rule"
        );
        assert!(
            specs.iter().any(|r| r.as_str().unwrap().contains("GOOD requirement body")),
            "specs rules must include a GOOD example"
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
