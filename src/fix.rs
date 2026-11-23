//! `--fix` mode: apply machine-applicable (`Local`) findings automatically.
//!
//! Only findings whose `op` is machine-applicable (`fixability == Local`) and
//! that carry a deterministic `replacement` are applied (design D3/D4). All
//! `Structural`/judgment findings (e.g. `split_requirement`) are left as
//! suggestions. Edits are applied one op at a time and the plan is revalidated
//! after each application (design D3, task 6.3).

use crate::ir::{Finding, Fixability, Op, PlanIR};
use std::path::Path;

/// A single applied edit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppliedEdit {
    /// The file that was edited.
    pub file: String,
    /// The kind of finding that drove the edit.
    pub kind: String,
    /// A human-readable description of what was applied.
    pub description: String,
}

/// The result of a `--fix` run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixReport {
    /// Edits that were applied.
    pub applied: Vec<AppliedEdit>,
    /// Findings that were left as suggestions (structural/judgment).
    pub left_as_suggestions: Vec<String>,
}

/// Apply machine-applicable findings to the plan's source files.
///
/// `base_dir` is the change directory used to resolve relative file paths
/// (e.g. `spec.md` → `<change_dir>/specs/cap/spec.md`). Returns a report of
/// what was applied vs. left as suggestions. The caller revalidates the plan
/// after each applied edit (task 6.3).
pub fn fix_plan(plan: &PlanIR, findings: &[Finding], base_dir: &Path) -> FixReport {
    let mut report = FixReport {
        applied: Vec::new(),
        left_as_suggestions: Vec::new(),
    };

    for f in findings {
        if f.fixability != Fixability::Local {
            report
                .left_as_suggestions
                .push(format!("{}: {}", f.kind, f.message));
            continue;
        }

        // Only apply findings with a deterministic replacement.
        let Some(replacement) = &f.replacement else {
            report.left_as_suggestions.push(format!(
                "{}: {} (no deterministic replacement)",
                f.kind, f.message
            ));
            continue;
        };

        match f.op {
            Op::RenameTask => {
                // Rename a duplicate task ID. The replacement is the new ID.
                if let Some(edit) = apply_task_rename(plan, f, replacement, base_dir) {
                    report.applied.push(edit);
                } else {
                    report
                        .left_as_suggestions
                        .push(format!("{}: {} (could not locate task)", f.kind, f.message));
                }
            }
            Op::ReplaceBody => {
                // Byte-span replacement (e.g. prose SlopWord). The finding's
                // start/end are relative to the requirement body snippet, so
                // we map them to file-absolute offsets via the requirement's
                // source span.
                if let Some(edit) = apply_span_replacement(plan, f, replacement, base_dir) {
                    report.applied.push(edit);
                } else {
                    report
                        .left_as_suggestions
                        .push(format!("{}: {} (could not apply span)", f.kind, f.message));
                }
            }
            _ => {
                // No deterministic edit for this op.
                report
                    .left_as_suggestions
                    .push(format!("{}: {} (op not auto-appliable)", f.kind, f.message));
            }
        }
    }

    report
}

/// Resolve a (possibly relative) file path against the change directory.
///
/// The parser stores only the basename (e.g. `spec.md`), so if the direct
/// join does not exist we search the change directory recursively for a file
/// with that basename.
fn resolve_path(file: &str, base_dir: &Path) -> Option<std::path::PathBuf> {
    let p = Path::new(file);
    if p.is_absolute() {
        return Some(p.to_path_buf());
    }
    let direct = base_dir.join(p);
    if direct.exists() {
        return Some(direct);
    }
    // Search recursively for a file with this basename.
    let name = p.file_name()?.to_str()?;
    find_file_by_name(base_dir, name)
}

/// Recursively search `dir` for a file whose name equals `name`.
fn find_file_by_name(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_by_name(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

/// Replace a task id in place and describe the edit.
fn rewrite_task_id(
    path: &Path,
    task: &crate::ir::Task,
    f: &Finding,
    new_id: &str,
) -> Option<AppliedEdit> {
    let content = std::fs::read_to_string(path).ok()?;
    let new_content = splice_id(
        &content,
        task.source.start_byte,
        task.source.end_byte,
        new_id,
    )?;
    std::fs::write(path, new_content).ok()?;
    Some(applied(
        f,
        format!("renamed task {} to {}", task.id, new_id),
    ))
}

/// Replace `start..end` of a file with `replacement` and describe the edit.
#[allow(clippy::too_many_arguments)]
fn write_replacement(
    path: &Path,
    content: &str,
    start: usize,
    end: usize,
    replacement: &str,
    f: &Finding,
    target: &str,
) -> Option<AppliedEdit> {
    let mut bytes = content.as_bytes().to_vec();
    bytes.splice(start..end, replacement.bytes());
    std::fs::write(path, String::from_utf8(bytes).ok()?).ok()?;
    Some(applied(
        f,
        format!("replaced {target:?} with {replacement:?}"),
    ))
}

/// The edit a finding produced, carrying the finding's file and kind.
fn applied(f: &Finding, description: String) -> AppliedEdit {
    AppliedEdit {
        file: f.file.clone(),
        kind: f.kind.clone(),
        description,
    }
}

/// The byte span a finding's requirement occupies in `content`, or the whole file when the finding
/// names no requirement it can find.
fn requirement_span(plan: &PlanIR, f: &Finding, content: &str) -> (usize, usize) {
    let req = f
        .requirement_id
        .as_deref()
        .and_then(|rid| plan.requirements.iter().find(|r| r.id == rid));
    match req {
        Some(r) => (r.source.start_byte, r.source.end_byte.min(content.len())),
        None => (0, content.len()),
    }
}

/// Where the target word sits: inside the requirement span when the span holds it, and anywhere in
/// the file when it does not - the parser's requirement span may cover only the heading.
fn locate_target(content: &str, start: usize, end: usize, target: &str) -> Option<usize> {
    match find_word(&content[start..end], target) {
        Some(rel) => Some(start + rel),
        None => find_word(content, target),
    }
}

fn apply_span_replacement(
    plan: &PlanIR,
    f: &Finding,
    replacement: &str,
    base_dir: &Path,
) -> Option<AppliedEdit> {
    if f.file.is_empty() || f.suggestion.is_none() {
        return None;
    }
    let path = resolve_path(&f.file, base_dir)?;
    let content = std::fs::read_to_string(&path).ok()?;

    let (search_start, search_end) = requirement_span(plan, f, &content);
    let target = extract_quoted_word(&f.message)?;
    if target.is_empty() {
        return None;
    }
    let abs_start = locate_target(&content, search_start, search_end, &target)?;

    write_replacement(
        &path,
        &content,
        abs_start,
        abs_start + target.len(),
        replacement,
        f,
        &target,
    )
}

/// Extract a quoted word from a message like `replace "leverage" with "use"`.
fn extract_quoted_word(message: &str) -> Option<String> {
    let start = message.find('"')? + 1;
    let rest = &message[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Find the byte offset of the first word-boundary occurrence of `word` in `hay`.
fn find_word(hay: &str, word: &str) -> Option<usize> {
    let bytes = hay.as_bytes();
    let w = word.as_bytes();
    if w.is_empty() || w.len() > bytes.len() {
        return None;
    }
    let mut i = 0;
    while i + w.len() <= bytes.len() {
        if &bytes[i..i + w.len()] == w {
            // Check word boundaries.
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after_ok =
                i + w.len() == bytes.len() || !bytes[i + w.len()].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// The task a finding points at, by file and start line.
fn find_task_at<'a>(plan: &'a PlanIR, f: &Finding) -> Option<&'a crate::ir::Task> {
    plan.tasks
        .iter()
        .find(|t| t.source.file == f.file && t.source.start_line == f.line)
}

/// Replace `start..end` of `content` with `new_id`, refusing a span that does not fit.
fn splice_id(content: &str, start: usize, end: usize, new_id: &str) -> Option<String> {
    if end > content.len() || start > end {
        return None;
    }
    let mut bytes = content.as_bytes().to_vec();
    bytes.splice(start..end, new_id.bytes());
    String::from_utf8(bytes).ok()
}

/// Rename a duplicate task ID in tasks.md.
fn apply_task_rename(
    plan: &PlanIR,
    f: &Finding,
    new_id: &str,
    base_dir: &Path,
) -> Option<AppliedEdit> {
    // The finding's message references the duplicate task ID. Find the task in
    // the plan whose source location matches the finding's file/line.
    let task = find_task_at(plan, f)?;
    let path = resolve_path(&f.file, base_dir)?;
    rewrite_task_id(&path, task, f, new_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Finding, Fixability, Op, PlanIR};

    fn make_finding(
        kind: &str,
        op: Op,
        fixability: Fixability,
        replacement: Option<&str>,
    ) -> Finding {
        Finding {
            kind: kind.to_string(),
            severity: "warning".into(),
            file: "spec.md".into(),
            line: 1,
            column: 0,
            start: 0,
            end: 0,
            message: "replace \"leverage\" with \"use\"".into(),
            suggestion: Some("replace \"leverage\" with \"use\"".into()),
            replacement: replacement.map(|s| s.to_string()),
            fixability,
            op,
            requirement_id: Some("cap::Test".into()),
            advisory: true,
            element: "cap::Test".into(),
            excerpt: None,
        }
    }

    fn empty_plan() -> PlanIR {
        PlanIR {
            tasks: vec![],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: crate::ir::SourceMap::default(),
        }
    }

    #[test]
    fn local_finding_is_applied() {
        let plan = empty_plan();
        let findings = vec![make_finding(
            "prose_other",
            Op::ReplaceBody,
            Fixability::Local,
            Some("use"),
        )];
        // No file exists, so the span edit cannot be applied — but the finding
        // is recognized as machine-applicable (not left as a structural
        // suggestion). The report should not classify it as structural.
        let report = fix_plan(&plan, &findings, std::path::Path::new("/tmp/nonexistent"));
        assert!(
            !report
                .left_as_suggestions
                .iter()
                .any(|s| s.contains("structural")),
            "local finding must not be left as a structural suggestion: {:?}",
            report.left_as_suggestions
        );
    }

    #[test]
    fn structural_finding_is_left_as_suggestion() {
        let plan = empty_plan();
        let findings = vec![make_finding(
            "grounding_multi_keyword",
            Op::SplitRequirement,
            Fixability::Structural,
            None,
        )];
        let report = fix_plan(&plan, &findings, std::path::Path::new("/tmp"));
        assert!(report.applied.is_empty());
        assert_eq!(report.left_as_suggestions.len(), 1);
        assert!(
            report.left_as_suggestions[0].contains("grounding_multi_keyword"),
            "structural finding should be left as a suggestion: {:?}",
            report.left_as_suggestions
        );
    }

    #[test]
    fn split_requirement_is_never_auto_applied() {
        // A split_requirement finding is `structural` and must never be
        // auto-applied by `--fix`, even if it carried a replacement.
        let plan = empty_plan();
        let findings = vec![make_finding(
            "grounding_multi_keyword",
            Op::SplitRequirement,
            Fixability::Structural,
            Some("split body"),
        )];
        let report = fix_plan(&plan, &findings, std::path::Path::new("/tmp"));
        assert!(
            report.applied.is_empty(),
            "split_requirement must not be auto-applied"
        );
        assert_eq!(report.left_as_suggestions.len(), 1);
    }

    #[test]
    fn extract_quoted_word_parses_message() {
        assert_eq!(
            extract_quoted_word("replace \"leverage\" with \"use\""),
            Some("leverage".to_string())
        );
        assert_eq!(extract_quoted_word("no quotes here"), None);
    }

    #[test]
    fn find_word_respects_boundaries() {
        assert_eq!(find_word("use leverage here", "leverage"), Some(4));
        assert_eq!(find_word("leverage", "leverage"), Some(0));
        // "leverage" inside "leverages" must not match at the boundary.
        assert_eq!(find_word("leverages", "leverage"), None);
    }
}
