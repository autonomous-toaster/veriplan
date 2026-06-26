//! Diagnostics — maps `checker::check_convertibility()` CheckItems to LSP Diagnostics.

use std::path::Path;

use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

use crate::ir::{CheckItem, ConvertibilityReport};

/// Convert a ConvertibilityReport into per-file diagnostics.
/// Returns a vec of (file_path, diagnostics).
pub fn report_to_diagnostics(
    report: &ConvertibilityReport,
    project_root: &Path,
) -> Vec<(std::path::PathBuf, Vec<Diagnostic>)> {
    let mut per_file: std::collections::HashMap<std::path::PathBuf, Vec<Diagnostic>> =
        std::collections::HashMap::new();

    for item in &report.blockers {
        push_diagnostic(&mut per_file, item, DiagnosticSeverity::ERROR, project_root);
    }
    for item in &report.warnings {
        push_diagnostic(
            &mut per_file,
            item,
            DiagnosticSeverity::WARNING,
            project_root,
        );
    }
    for item in &report.info {
        push_diagnostic(
            &mut per_file,
            item,
            DiagnosticSeverity::INFORMATION,
            project_root,
        );
    }

    per_file.into_iter().collect()
}

fn push_diagnostic(
    per_file: &mut std::collections::HashMap<std::path::PathBuf, Vec<Diagnostic>>,
    item: &CheckItem,
    severity: DiagnosticSeverity,
    project_root: &Path,
) {
    let (file_path, line) = parse_location(&item.location, project_root);
    let range = Range {
        start: Position {
            line: line.saturating_sub(1) as u32,
            character: 0,
        },
        end: Position {
            line: line.saturating_sub(1) as u32,
            character: 999,
        },
    };

    let diagnostic = Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(item.check.clone())),
        code_description: None,
        source: Some("veriplan".to_string()),
        message: item.detail.clone(),
        related_information: None,
        tags: None,
        data: item.fix.as_ref().map(|f| serde_json::json!({ "fix": f })),
    };

    per_file.entry(file_path).or_default().push(diagnostic);
}

/// Parse a "file:line" location string into (absolute_path, line_number).
pub fn parse_location(location: &str, project_root: &Path) -> (std::path::PathBuf, usize) {
    if let Some((file_part, line_part)) = location.rsplit_once(':')
        && let Ok(line) = line_part.parse::<usize>()
    {
        let path = std::path::PathBuf::from(file_part);
        if path.is_absolute() {
            return (path, line);
        }
        return (project_root.join(file_part), line);
    }
    (project_root.join(location), 0)
}

/// CheckItem severity string → DiagnosticSeverity (used by transport)
pub fn severity_from_str(s: &str) -> DiagnosticSeverity {
    match s {
        "blocker" => DiagnosticSeverity::ERROR,
        "warning" => DiagnosticSeverity::WARNING,
        _ => DiagnosticSeverity::INFORMATION,
    }
}
