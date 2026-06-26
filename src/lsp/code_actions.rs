//! Code actions — quick fixes surfaced from CheckItem.fix suggestions.

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, TextEdit, WorkspaceEdit,
};

/// Build code actions from diagnostics that have fix data.
pub fn code_actions_for_diagnostics(
    uri: &lsp_types::Url,
    diagnostics: &[Diagnostic],
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    for diagnostic in diagnostics {
        // Only produce code actions for diagnostics with fix data
        let fix_data = match &diagnostic.data {
            Some(data) => data,
            None => continue,
        };

        let fix_text = match fix_data.get("fix").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => continue,
        };

        // Create a text edit that replaces the diagnostic range with the fix
        let text_edit = TextEdit {
            range: diagnostic.range,
            new_text: fix_text.to_string(),
        };

        let edit = WorkspaceEdit {
            changes: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(uri.clone(), vec![text_edit]);
                map
            }),
            document_changes: None,
            change_annotations: None,
        };

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: fix_text.to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(edit),
            is_preferred: None,
            disabled: None,
            data: None,
            command: None,
        }));
    }

    actions
}
