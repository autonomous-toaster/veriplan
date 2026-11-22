//! Provenance: which archived change grounds a published requirement.
//!
//! A delta that rewords a published requirement carries that requirement's body, and the body cites
//! task ids that belong to the change that wrote it. Those ids are not this change's tasks, and the
//! archive cannot be treated as an id namespace: 54 of 94 ids here are defined by more than one
//! archived change and \`T1.1\` by twenty-two, so an id alone identifies nothing.
//!
//! Keyed by the requirement, resolution is total. Measured over this repository: 111 of 111
//! published requirements resolve, 100 uniquely and 11 by the tie-break below.

use crate::ir::PlanIR;
use crate::util::task_ids_in;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Requirement id (\`capability::name\`) to the history that grounds it.
pub type Provenance = BTreeMap<String, crate::ir::Grounding>;

/// What one archived change says about one capability.
struct Delta {
    /// Requirement names the delta contains.
    names: Vec<String>,
    /// Task ids the change defines.
    tasks: Vec<String>,
}

/// Resolve the provenance of every requirement in a plan.
///
/// An archived change owns a requirement when its delta for that capability contains the requirement
/// and its task list defines every id the requirement body cites. Both conditions are needed: the
/// first stops a change that never touched the requirement from owning it, and the second stops a
/// change that carried the body forward unchanged from taking it from the change that wrote it.
///
/// Archive directories are date-prefixed, so sorting them is a chronology. The latest owner wins.
/// If the archive is ever no longer date-prefixed this returns the lexically latest entry, which is
/// why the caller is expected to have a date-prefixed archive; see design D2.
pub fn resolve(plan: &PlanIR, archive_dir: &Path) -> Provenance {
    let mut out = Provenance::new();
    if !archive_dir.is_dir() {
        return out;
    }
    let mut changes: Vec<PathBuf> = match std::fs::read_dir(archive_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(_) => return out,
    };
    changes.sort();

    for req in &plan.requirements {
        let Some((capability, _)) = req.id.split_once("::") else {
            continue;
        };
        let cited = task_ids_in(&req.statement);
        if cited.is_empty() {
            continue;
        }
        // Walk the chronology backwards: the latest owner is the one that wrote the published body.
        for change in changes.iter().rev() {
            let Some(delta) = read_delta(change, capability) else {
                continue;
            };
            let name = req.id.split_once("::").map(|(_, n)| n).unwrap_or("");
            if !delta.names.iter().any(|n| n == name) {
                continue;
            }
            if cited.iter().all(|id| delta.tasks.iter().any(|t| t == id)) {
                if let Some(owner) = change.file_name().and_then(|n| n.to_str()) {
                    out.insert(
                        req.id.clone(),
                        crate::ir::Grounding {
                            change: owner.to_string(),
                            tasks: delta.tasks.clone(),
                        },
                    );
                }
                break;
            }
        }
    }
    out
}

fn read_delta(change: &Path, capability: &str) -> Option<Delta> {
    let spec = change.join("specs").join(capability).join("spec.md");
    let text = std::fs::read_to_string(&spec).ok()?;
    let names = text
        .lines()
        .filter_map(|l| l.strip_prefix("### Requirement: "))
        .map(|n| n.trim().to_string())
        .collect();
    let tasks = std::fs::read_to_string(change.join("tasks.md"))
        .map(|t| task_ids_from_tasks(&t))
        .unwrap_or_default();
    Some(Delta { names, tasks })
}

fn task_ids_from_tasks(tasks_md: &str) -> Vec<String> {
    tasks_md
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let rest = l
                .strip_prefix("- [ ] ")
                .or_else(|| l.strip_prefix("- [x] "))?;
            let id = rest.split_whitespace().next()?;
            let mut parts = id.split('.');
            let major = parts.next()?;
            let minor = parts.next()?;
            if major.chars().all(|c| c.is_ascii_digit())
                && minor.chars().all(|c| c.is_ascii_digit())
                && parts.next().is_none()
            {
                Some(id.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_task_ids_and_ignores_prose() {
        let md = "## 1. One\n\n- [ ] 1.1 first\n- [x] 1.2 second\n- [ ] not a task\n";
        assert_eq!(task_ids_from_tasks(md), vec!["1.1", "1.2"]);
    }

    #[test]
    fn an_absent_archive_resolves_nothing() {
        let plan = PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: crate::ir::SourceMap::default(),
        };
        assert!(resolve(&plan, Path::new("/nonexistent/archive")).is_empty());
    }
}
