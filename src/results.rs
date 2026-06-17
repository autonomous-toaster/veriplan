//! Results cache for `veriplan visualize`.
//! Written by `check` after SPIN model check, read by `visualize` for pass/fail overlay.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Per-constraint result (simplified from ConstraintSummary + violation info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintResult {
    pub requirement_id: String,
    pub ltl: String,
    pub category: String,
    pub passed: bool,
    pub violated: bool,
    pub timed_out: bool,
}

/// Full check results written to `.veriplan/results.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResults {
    pub plan_name: String,
    pub valid: bool,
    pub total: usize,
    pub satisfied: usize,
    pub constraints: Vec<ConstraintResult>,
}

impl CheckResults {
    /// Look up a constraint result by requirement_id.
    pub fn for_requirement(&self, id: &str) -> Option<&ConstraintResult> {
        self.constraints.iter().find(|c| c.requirement_id == id)
    }
}

/// Path to `.veriplan/results.json` under a project root.
pub fn results_path(project_root: &Path) -> PathBuf {
    project_root.join(".veriplan").join("results.json")
}

/// Write check results to `.veriplan/results.json`.
pub fn write_results(results: &CheckResults, project_root: &Path) -> anyhow::Result<()> {
    let dir = project_root.join(".veriplan");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("results.json");
    let json = serde_json::to_string_pretty(results)?;
    std::fs::write(&path, &json)?;
    Ok(())
}

/// Read check results from `.veriplan/results.json` (if it exists).
pub fn read_results(project_root: &Path) -> Option<CheckResults> {
    let path = results_path(project_root);
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    }
}
