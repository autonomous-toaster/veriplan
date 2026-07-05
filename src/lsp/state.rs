//! ChangeStore — in-memory workspace state for the LSP server.
//!
//! Maps file paths to their containing OpenSpec change directories,
//! caches parsed PlanIR and convertibility results per change.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::checker;
use crate::input::{InputSource, load_plan};
use crate::ir::{ConvertibilityReport, PlanIR};
use crate::parser;

/// In-memory workspace state, behind Arc<RwLock<>> for concurrent access.
pub struct ChangeStore {
    /// change name → parsed PlanIR
    plans: HashMap<String, PlanIR>,
    /// change name → cached convertibility report
    reports: HashMap<String, ConvertibilityReport>,
    /// file path → change name (reverse index)
    file_to_change: HashMap<PathBuf, String>,
    /// project root (directory containing openspec/)
    project_root: PathBuf,
    /// Standalone files: file path → (PlanIR, ConvertibilityReport)
    /// Used for files not in an OpenSpec change directory
    standalone: HashMap<PathBuf, (PlanIR, ConvertibilityReport)>,
}

impl ChangeStore {
    /// Create a new store. Scans the project root for existing changes.
    pub fn new(project_root: &Path) -> Self {
        let mut store = Self {
            plans: HashMap::new(),
            reports: HashMap::new(),
            file_to_change: HashMap::new(),
            project_root: project_root.to_path_buf(),
            standalone: HashMap::new(),
        };
        store.scan_changes();
        store
    }

    /// Scan the `openspec/changes/` directory and load all active changes.
    fn scan_changes(&mut self) {
        let changes_dir = self.project_root.join("openspec").join("changes");
        if !changes_dir.exists() || !changes_dir.is_dir() {
            eprintln!("[veriplan-lsp] No openspec/changes/ directory found");
            return;
        }
        for entry in std::fs::read_dir(&changes_dir).ok().into_iter().flatten() {
            let entry = match entry {
                Ok(e) => e,
                _ => continue,
            };
            if let Some((name, change_path)) = is_valid_change_entry(&entry) {
                self.load_change(&name, &change_path);
            }
        }
    }

    /// Load (or reload) a single change directory into the store.
    fn load_change(&mut self, name: &str, path: &Path) {
        let plan = match parser::parse_plan(path) {
            Ok(p) => p,
            Err(_) => return,
        };
        let report = checker::check_convertibility(&plan, true); // LSP always processes OpenSpec changes

        // Build file index: map every file under the change dir to this change name
        if let Ok(entries) = walk_files(path) {
            for file in entries {
                self.file_to_change.insert(file, name.to_string());
            }
        }

        self.plans.insert(name.to_string(), plan);
        self.reports.insert(name.to_string(), report);
    }

    /// Given a file path, determine which OpenSpec change it belongs to.
    pub fn resolve_change(&self, path: &Path) -> Option<String> {
        // Fast path: reverse index
        if let Some(name) = self.file_to_change.get(path) {
            return Some(name.clone());
        }
        // Slow path: walk parents looking for openspec/changes/<name>/ pattern
        self.resolve_by_path_walk(path)
    }

    fn resolve_by_path_walk(&self, path: &Path) -> Option<String> {
        let mut current = path.parent()?;
        loop {
            if let Some(name) = extract_change_name_from_path(current) {
                return Some(name);
            }
            if current == self.project_root || !current.parent().is_none_or(|p| p.exists()) {
                return None;
            }
            current = current.parent()?;
        }
    }
}

/// Given a path, check if it's inside an `openspec/changes/<name>/` directory
/// and return the change name if so.
fn is_inside_changes_dir(path: &Path) -> bool {
    path.ends_with("openspec/changes") || path.to_string_lossy().contains("openspec/changes/")
}

fn get_parent_and_grandparent(path: &Path) -> Option<(&Path, &Path)> {
    let parent = path.parent()?;
    let grandparent = parent.parent()?;
    Some((parent, grandparent))
}

fn extract_change_name_from_path(current: &Path) -> Option<String> {
    if !is_inside_changes_dir(current) {
        return None;
    }
    let (parent, grandparent) = get_parent_and_grandparent(current)?;
    if !is_inside_changes_dir(grandparent) {
        return None;
    }
    let name = parent.file_name()?.to_string_lossy().to_string();
    if name == "archive" { None } else { Some(name) }
}

impl ChangeStore {
/// Re-parse a change directory and re-run convertibility check.
    /// Returns a vec of all CheckItems converted to LSP diagnostics.
    pub fn refresh(&mut self, change: &str) -> Vec<(PathBuf, Vec<lsp_types::Diagnostic>)> {
        let change_dir = self
            .project_root
            .join("openspec")
            .join("changes")
            .join(change);
        let plan = match parser::parse_plan(&change_dir) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let report = checker::check_convertibility(&plan, true); // LSP always processes OpenSpec changes

        // Update caches
        self.plans.insert(change.to_string(), plan);
        self.reports.insert(change.to_string(), report);

        // Rebuild file index
        self.file_to_change.retain(|_, v| v != change);
        if let Ok(entries) = walk_files(&change_dir) {
            for file in entries {
                self.file_to_change.insert(file, change.to_string());
            }
        }

        // Build diagnostics per file
        self.diagnostics_for_change(change)
    }

    /// Get the cached plan for a given change.
    pub fn get_plan(&self, change: &str) -> Option<&PlanIR> {
        self.plans.get(change)
    }

    /// Get the cached convertibility report for a given change.
    pub fn get_report(&self, change: &str) -> Option<&ConvertibilityReport> {
        self.reports.get(change)
    }

    /// Re-scan the changes directory for newly added change directories.
    /// Call this when a file belongs to no known change — the change
    /// may have been created after the LSP started.
    pub fn rescan(&mut self) {
        self.scan_changes();
    }

    /// Check if a file path belongs to a known change.
    pub fn has_change(&self, path: &Path) -> bool {
        self.resolve_change(path).is_some()
    }

    /// Load a standalone file (not in an OpenSpec change) into the cache.
    /// Returns true if successful.
    pub fn load_standalone(&mut self, file_path: &Path) -> bool {
        if !file_path.exists() {
            eprintln!("[veriplan-lsp] File not found: {}", file_path.display());
            return false;
        }

        // Use InputSource::SingleFile to load the plan
        let source = InputSource::SingleFile {
            path: file_path.to_path_buf(),
        };

        let plan = match load_plan(&source) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[veriplan-lsp] Failed to parse standalone file: {}", e);
                return false;
            }
        };

        let report = checker::check_convertibility(&plan, false); // Standalone files are not OpenSpec
        self.standalone
            .insert(file_path.to_path_buf(), (plan, report));
        true
    }

    /// Get diagnostics for a standalone file.
    pub fn get_standalone_diagnostics(
        &self,
        file_path: &Path,
    ) -> Option<Vec<lsp_types::Diagnostic>> {
        let (_plan, report) = self.standalone.get(file_path)?;
        Some(self.report_to_diagnostics_for_standalone(report, file_path))
    }

    /// Refresh a standalone file after edit.
    pub fn refresh_standalone(&mut self, file_path: &Path) -> Option<Vec<lsp_types::Diagnostic>> {
        if !self.load_standalone(file_path) {
            return None;
        }
        self.get_standalone_diagnostics(file_path)
    }

    /// Build diagnostics for a standalone file from its report.
    fn report_to_diagnostics_for_standalone(
        &self,
        report: &ConvertibilityReport,
        file_path: &Path,
    ) -> Vec<lsp_types::Diagnostic> {
        let mut diagnostics = Vec::new();

        for item in &report.blockers {
            diagnostics.push(self.check_item_to_diagnostic(
                item,
                lsp_types::DiagnosticSeverity::ERROR,
                file_path,
            ));
        }
        for item in &report.warnings {
            diagnostics.push(self.check_item_to_diagnostic(
                item,
                lsp_types::DiagnosticSeverity::WARNING,
                file_path,
            ));
        }
        for item in &report.info {
            diagnostics.push(self.check_item_to_diagnostic(
                item,
                lsp_types::DiagnosticSeverity::INFORMATION,
                file_path,
            ));
        }

        diagnostics
    }

    /// Convert a CheckItem to an LSP Diagnostic.
    fn check_item_to_diagnostic(
        &self,
        item: &crate::ir::CheckItem,
        severity: lsp_types::DiagnosticSeverity,
        _file_path: &Path,
    ) -> lsp_types::Diagnostic {
        let (_file_path, line) = parse_location(&item.location);
        let range = if line > 0 {
            lsp_types::Range {
                start: lsp_types::Position {
                    line: (line - 1) as u32,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: (line - 1) as u32,
                    character: 999,
                },
            }
        } else {
            // Fallback: use the file path from the standalone cache
            lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 999,
                },
            }
        };

        lsp_types::Diagnostic {
            range,
            severity: Some(severity),
            code: Some(lsp_types::NumberOrString::String(item.check.clone())),
            code_description: None,
            source: Some("veriplan".to_string()),
            message: item.detail.clone(),
            related_information: None,
            tags: None,
            data: item.fix.as_ref().map(|f| serde_json::json!({ "fix": f })),
        }
    }

    /// Get the project root.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Build per-file diagnostics for a change from the cached report.
    fn diagnostics_for_change(&self, change: &str) -> Vec<(PathBuf, Vec<lsp_types::Diagnostic>)> {
        let report = match self.reports.get(change) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let all_items: Vec<_> = report
            .blockers
            .iter()
            .map(|i| (i, lsp_types::DiagnosticSeverity::ERROR))
            .chain(
                report
                    .warnings
                    .iter()
                    .map(|i| (i, lsp_types::DiagnosticSeverity::WARNING)),
            )
            .chain(
                report
                    .info
                    .iter()
                    .map(|i| (i, lsp_types::DiagnosticSeverity::INFORMATION)),
            )
            .collect();

        // Group by file
        let mut per_file: HashMap<PathBuf, Vec<lsp_types::Diagnostic>> = HashMap::new();
        for (item, severity) in &all_items {
            let (file_path, line) = parse_location(&item.location);
            let full_path = self.project_root.join(&file_path);
            let range = if line > 0 {
                lsp_types::Range {
                    start: lsp_types::Position {
                        line: (line - 1) as u32,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: (line - 1) as u32,
                        character: 999,
                    },
                }
            } else {
                lsp_types::Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: 0,
                        character: 999,
                    },
                }
            };

            let diagnostic = lsp_types::Diagnostic {
                range,
                severity: Some(*severity),
                code: Some(lsp_types::NumberOrString::String(item.check.clone())),
                code_description: None,
                source: Some("veriplan".to_string()),
                message: item.detail.clone(),
                related_information: None,
                tags: None,
                data: item.fix.as_ref().map(|f| serde_json::json!({ "fix": f })),
            };
            per_file.entry(full_path).or_default().push(diagnostic);
        }

        per_file.into_iter().collect()
    }
}

/// Check if a directory entry is a valid change directory and return (name, path).
fn is_valid_change_entry(entry: &std::fs::DirEntry) -> Option<(String, std::path::PathBuf)> {
    let name = entry.file_name().to_string_lossy().to_string();
    if name == "archive" || !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
        return None;
    }
    let change_path = entry.path();
    if !change_path.join("tasks.md").exists() || !change_path.join("specs").exists() {
        return None;
    }
    Some((name, change_path))
}

/// Walk a directory recursively, returning all file paths.
fn walk_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    if dir.is_file() {
        return Ok(vec![dir.to_path_buf()]);
    }
    collect_dir_files(dir)
}

fn collect_dir_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

/// Parse a "file:line" location string into (path_buf, line_number).
/// Handles both relative paths and absolute paths.
fn parse_location(location: &str) -> (PathBuf, usize) {
    if let Some((file_part, line_part)) = location.rsplit_once(':')
        && let Ok(line) = line_part.parse::<usize>()
    {
        return (PathBuf::from(file_part), line);
    }
    (PathBuf::from(location), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_inside_changes_dir_ends_with() {
        let p = Path::new("/project/openspec/changes");
        assert!(is_inside_changes_dir(p));
    }

    #[test]
    fn test_is_inside_changes_dir_contains() {
        let p = Path::new("/project/openspec/changes/my-change/tasks.md");
        assert!(is_inside_changes_dir(p));
    }

    #[test]
    fn test_is_inside_changes_dir_not() {
        let p = Path::new("/project/src/main.rs");
        assert!(!is_inside_changes_dir(p));
    }

    #[test]
    fn test_collect_dir_files_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let files = collect_dir_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_collect_dir_files_with_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "").unwrap();
        std::fs::write(dir.path().join("b.md"), "").unwrap();
        let files = collect_dir_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_parse_location_with_line() {
        let (path, line) = parse_location("file.md:42");
        assert_eq!(path, PathBuf::from("file.md"));
        assert_eq!(line, 42);
    }

    #[test]
    fn test_parse_location_without_line() {
        let (path, line) = parse_location("file.md");
        assert_eq!(path, PathBuf::from("file.md"));
        assert_eq!(line, 0);
    }
}
