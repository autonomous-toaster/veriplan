use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use anyhow::Context;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::*;

use super::code_actions;
use super::completions;
use super::diagnostics as diag;
use super::navigation;
use super::state::ChangeStore;
use super::symbols;

pub(crate) fn handle_request(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    req: Request,
) -> Result<()> {
    match req.method.as_str() {
        "textDocument/completion" => {
            let params: CompletionParams =
                serde_json::from_value(req.params).context("Bad completion params")?;
            let result = handle_completion(store, &params);
            let response = Response::new_ok(req.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        "textDocument/definition" => {
            let params: GotoDefinitionParams =
                serde_json::from_value(req.params).context("Bad goto-def params")?;
            let result = handle_goto_definition(store, &params);
            let response = Response::new_ok(req.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        "textDocument/hover" => {
            let params: HoverParams =
                serde_json::from_value(req.params).context("Bad hover params")?;
            let result = handle_hover(store, &params);
            let response = Response::new_ok(req.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        "textDocument/documentSymbol" => {
            let params: DocumentSymbolParams =
                serde_json::from_value(req.params).context("Bad symbol params")?;
            let result = handle_document_symbols(store, &params);
            let response = Response::new_ok(req.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        "textDocument/codeAction" => {
            let params: CodeActionParams =
                serde_json::from_value(req.params).context("Bad code action params")?;
            let result = handle_code_action(store, &params);
            let response = Response::new_ok(req.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        _ => {
            // Unknown method — respond with MethodNotFound
            let response = Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                format!("Unknown method: {}", req.method),
            );
            connection.sender.send(Message::Response(response))?;
        }
    }
    Ok(())
}

pub(crate) fn handle_notification(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    not: Notification,
) -> Result<()> {
    match not.method.as_str() {
        "textDocument/didOpen" => handle_did_open(connection, store, not)?,
        "textDocument/didChange" => handle_did_change(connection, store, not)?,
        "textDocument/didSave" => handle_did_save(connection, store, not)?,
        _ => {}
    }
    Ok(())
}

/// Handle textDocument/didOpen notification.
fn handle_did_open(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    not: Notification,
) -> Result<()> {
    let params: DidOpenTextDocumentParams =
        serde_json::from_value(not.params).context("Bad didOpen params")?;
    let file_path = params.text_document.uri.to_file_path().unwrap_or_default();
    eprintln!("[veriplan-lsp] didOpen: {}", file_path.display());

    let change_name = resolve_change_with_rescan(store, &file_path);

    if let Some(change) = change_name {
        publish_change_diagnostics(connection, store, &change);
    } else {
        handle_standalone_open(connection, store, &file_path);
    }
    Ok(())
}

/// Handle textDocument/didChange notification.
fn handle_did_change(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    not: Notification,
) -> Result<()> {
    let params: DidChangeTextDocumentParams =
        serde_json::from_value(not.params).context("Bad didChange params")?;
    let file_path = params.text_document.uri.to_file_path().unwrap_or_default();
    eprintln!("[veriplan-lsp] didChange: {}", file_path.display());

    let change_name = resolve_change_with_rescan(store, &file_path);
    let diagnostics_per_file = get_diagnostics_for_file(store, &file_path, change_name.as_deref());

    publish_diagnostics(connection, &diagnostics_per_file);
    clear_stale_diagnostics(connection, store, &file_path, &diagnostics_per_file);
    Ok(())
}

/// Handle textDocument/didSave notification.
fn handle_did_save(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    not: Notification,
) -> Result<()> {
    let params: DidSaveTextDocumentParams =
        serde_json::from_value(not.params).context("Bad didSave params")?;
    let file_path = params.text_document.uri.to_file_path().unwrap_or_default();
    eprintln!("[veriplan-lsp] didSave: {}", file_path.display());

    let change_name = resolve_change_with_rescan(store, &file_path);
    let diagnostics_per_file = get_diagnostics_for_file(store, &file_path, change_name.as_deref());

    let published_uris = publish_diagnostics_and_collect(connection, &diagnostics_per_file);
    clear_stale_diagnostics_filtered(connection, store, &file_path, &published_uris);
    Ok(())
}

/// Resolve a change for a file path, with automatic rescan if not found.
fn resolve_change_with_rescan(
    store: &Arc<RwLock<ChangeStore>>,
    file_path: &std::path::Path,
) -> Option<String> {
    let read_store = store.read().unwrap();
    let resolved = read_store.resolve_change(file_path);
    drop(read_store);
    if resolved.is_none() {
        store.write().unwrap().rescan();
        store.read().unwrap().resolve_change(file_path)
    } else {
        resolved
    }
}

/// Get diagnostics for a file, either from a change or standalone mode.
fn get_diagnostics_for_file(
    store: &Arc<RwLock<ChangeStore>>,
    file_path: &std::path::Path,
    change_name: Option<&str>,
) -> Vec<(std::path::PathBuf, Vec<lsp_types::Diagnostic>)> {
    if let Some(change) = change_name {
        eprintln!("[veriplan-lsp] resolved change '{}', refreshing...", change);
        store.write().unwrap().refresh(change)
    } else {
        eprintln!("[veriplan-lsp] file not in any change, trying standalone");
        let mut write_store = store.write().unwrap();
        if write_store.refresh_standalone(file_path).is_some() {
            vec![(
                file_path.to_path_buf(),
                write_store
                    .get_standalone_diagnostics(file_path)
                    .unwrap_or_default(),
            )]
        } else {
            Vec::new()
        }
    }
}

/// Publish diagnostics for all files.
fn publish_diagnostics(connection: &Connection, diagnostics: &[(std::path::PathBuf, Vec<lsp_types::Diagnostic>)]) {
    for (path, diags) in diagnostics {
        if let Ok(uri) = lsp_types::Url::from_file_path(path) {
            eprintln!("[veriplan-lsp] publishDiagnostics: {} ({} diags)", uri, diags.len());
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics: diags.clone(),
                version: None,
            };
            let notif = Notification::new("textDocument/publishDiagnostics".to_string(), params);
            let _ = connection.sender.send(Message::Notification(notif));
        }
    }
}

/// Publish diagnostics and return the list of published URIs.
fn publish_diagnostics_and_collect(
    connection: &Connection,
    diagnostics: &[(std::path::PathBuf, Vec<lsp_types::Diagnostic>)],
) -> Vec<std::path::PathBuf> {
    let mut uris = Vec::new();
    for (path, diags) in diagnostics {
        if let Ok(uri) = lsp_types::Url::from_file_path(path) {
            uris.push(path.clone());
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics: diags.clone(),
                version: None,
            };
            let notif = Notification::new("textDocument/publishDiagnostics".to_string(), params);
            let _ = connection.sender.send(Message::Notification(notif));
        }
    }
    uris
}

/// Clear stale diagnostics for files in the change that weren't published.
fn clear_stale_diagnostics(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    file_path: &std::path::Path,
    published: &[(std::path::PathBuf, Vec<lsp_types::Diagnostic>)],
) {
    if let Some(change) = store.read().unwrap().resolve_change(file_path) {
        let change_dir = store
            .read()
            .unwrap()
            .project_root()
            .join("openspec")
            .join("changes")
            .join(&change);
        if let Ok(entries) = walk_files_for_clear(&change_dir) {
            let published_uris: Vec<_> = published.iter().map(|(p, _)| p.clone()).collect();
            for path in entries {
                if !published_uris.contains(&path)
                    && let Ok(uri) = lsp_types::Url::from_file_path(&path)
                {
                    let params = PublishDiagnosticsParams {
                        uri,
                        diagnostics: Vec::new(),
                        version: None,
                    };
                    let notif = Notification::new(
                        "textDocument/publishDiagnostics".to_string(),
                        params,
                    );
                    let _ = connection.sender.send(Message::Notification(notif));
                }
            }
        }
    }
}

/// Clear stale diagnostics using a pre-collected list of published URIs.
fn clear_stale_diagnostics_filtered(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    file_path: &std::path::Path,
    published_uris: &[std::path::PathBuf],
) {
    if let Some(change) = store.read().unwrap().resolve_change(file_path) {
        let change_dir = store
            .read()
            .unwrap()
            .project_root()
            .join("openspec")
            .join("changes")
            .join(&change);
        if let Ok(entries) = walk_files_for_clear(&change_dir) {
            for path in entries {
                if !published_uris.contains(&path)
                    && let Ok(uri) = lsp_types::Url::from_file_path(&path)
                {
                    let params = PublishDiagnosticsParams {
                        uri,
                        diagnostics: Vec::new(),
                        version: None,
                    };
                    let notif = Notification::new(
                        "textDocument/publishDiagnostics".to_string(),
                        params,
                    );
                    let _ = connection.sender.send(Message::Notification(notif));
                }
            }
        }
    }
}

/// Handle standalone file open (not in any change).
fn handle_standalone_open(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    file_path: &std::path::Path,
) {
    eprintln!("[veriplan-lsp] didOpen: file not in any change, trying standalone mode");
    let mut write_store = store.write().unwrap();
    if write_store.load_standalone(file_path) {
        let diagnostics = write_store
            .get_standalone_diagnostics(file_path)
            .unwrap_or_default();
        eprintln!("[veriplan-lsp] didOpen: loaded as standalone, {} diagnostics", diagnostics.len());
        if let Ok(uri) = lsp_types::Url::from_file_path(file_path) {
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics,
                version: None,
            };
            let notif = Notification::new("textDocument/publishDiagnostics".to_string(), params);
            let _ = connection.sender.send(Message::Notification(notif));
        }
    } else {
        eprintln!("[veriplan-lsp] didOpen: not a valid standalone file, publishing empty diagnostics");
        if let Ok(uri) = lsp_types::Url::from_file_path(file_path) {
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics: Vec::new(),
                version: None,
            };
            let notif = Notification::new("textDocument/publishDiagnostics".to_string(), params);
            let _ = connection.sender.send(Message::Notification(notif));
        }
    }
}

/// Publish diagnostics for a change (used by didOpen).
fn publish_change_diagnostics(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    change: &str,
) {
    let diagnostics = store.write().unwrap().refresh(change);
    eprintln!(
        "[veriplan-lsp] didOpen: resolved change '{}', {} diagnostic files",
        change,
        diagnostics.len()
    );
    for (path, diags) in diagnostics {
        if let Ok(uri) = lsp_types::Url::from_file_path(&path) {
            eprintln!("[veriplan-lsp] publishDiagnostics: {} ({} diagnostics)", uri, diags.len());
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics: diags,
                version: None,
            };
            let notif = Notification::new("textDocument/publishDiagnostics".to_string(), params);
            let _ = connection.sender.send(Message::Notification(notif));
        }
    }
}

// ── Request handlers ──

pub(crate) fn handle_completion(
    store: &Arc<RwLock<ChangeStore>>,
    params: &CompletionParams,
) -> Option<CompletionResponse> {
    let file_path = params
        .text_document_position
        .text_document
        .uri
        .to_file_path()
        .ok()?;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;
    let plan = store.read().ok()?.get_plan(&change_name)?.clone();

    // Read the current line to determine context
    let pos = params.text_document_position.position;
    // We don't have the line text without re-reading, but completions can work
    // with just the plan context
    let completions = completions::get_completions(
        &plan,
        "", // line text (simplified)
        pos.character as usize,
    )?;

    Some(CompletionResponse::List(completions))
}

pub(crate) fn handle_goto_definition(
    store: &Arc<RwLock<ChangeStore>>,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let file_path = uri.to_file_path().ok()?;
    let pos = params.text_document_position_params.position;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;
    let plan = store.read().ok()?.get_plan(&change_name)?.clone();

    // Read the current line from the file for cursor context
    let line_text = read_line(&file_path, pos.line as usize)?;

    navigation::goto_definition(&plan, uri, &pos, &line_text)
}

pub(crate) fn handle_hover(store: &Arc<RwLock<ChangeStore>>, params: &HoverParams) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let file_path = uri.to_file_path().ok()?;
    let pos = params.text_document_position_params.position;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;
    let plan = store.read().ok()?.get_plan(&change_name)?.clone();

    let line_text = read_line(&file_path, pos.line as usize)?;
    navigation::hover(&plan, &pos, &line_text)
}

pub(crate) fn handle_document_symbols(
    store: &Arc<RwLock<ChangeStore>>,
    params: &DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let file_path = params.text_document.uri.to_file_path().ok()?;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;
    let plan = store.read().ok()?.get_plan(&change_name)?.clone();

    let file_name = file_path.file_name()?.to_string_lossy().to_string();

    match file_name.as_str() {
        "tasks.md" => symbols::tasks_document_symbols(&plan),
        _ => {
            // spec file — gather requirements for this specific file
            let file_str = file_path.to_string_lossy().to_string();
            let requirements: Vec<_> = plan
                .requirements
                .iter()
                .filter(|r| r.source.file == file_str || file_str.contains(&r.source.file))
                .cloned()
                .collect();

            if requirements.is_empty() {
                return None;
            }

            // Get category labels for each requirement
            let categories: Vec<String> = requirements
                .iter()
                .map(|r| format!("{:?}", r.category))
                .collect();

            symbols::spec_document_symbols_with_labels(&requirements, &categories)
        }
    }
}

pub(crate) fn handle_code_action(
    store: &Arc<RwLock<ChangeStore>>,
    params: &CodeActionParams,
) -> Option<Vec<CodeActionOrCommand>> {
    let uri = &params.text_document.uri;
    let file_path = uri.to_file_path().ok()?;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;

    // Get diagnostics for this file from the store
    let report = store.read().ok()?.get_report(&change_name)?.clone();
    let project_root = store.read().ok()?.project_root().to_path_buf();

    let diagnostics = diag::report_to_diagnostics(&report, &project_root);
    let file_diags: Vec<_> = diagnostics
        .into_iter()
        .find(|(path, _)| *path == file_path)
        .map(|(_, diags)| diags)
        .unwrap_or_default();

    let actions = code_actions::code_actions_for_diagnostics(uri, &file_diags);
    if actions.is_empty() {
        return None;
    }
    Some(actions)
}

// ── Helpers ──

/// Read a specific line from a file.
pub(crate) fn read_line(path: &Path, line: usize) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().nth(line).map(|s| s.to_string())
}

/// Walk files in a change directory (for clearing diagnostics).
pub(crate) fn walk_files_for_clear(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_file() {
        files.push(dir.to_path_buf());
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(walk_files_for_clear(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_files_for_clear_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(&file_path, "content").unwrap();
        let files = walk_files_for_clear(&file_path).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_path);
    }

    #[test]
    fn test_walk_files_for_clear_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("a.md");
        let file2 = dir.path().join("b.md");
        std::fs::write(&file1, "content").unwrap();
        std::fs::write(&file2, "content").unwrap();
        let files = walk_files_for_clear(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_walk_files_for_clear_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let files = walk_files_for_clear(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_read_line() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(&file_path, "line1\nline2\nline3").unwrap();
        assert_eq!(read_line(&file_path, 0), Some("line1".to_string()));
        assert_eq!(read_line(&file_path, 1), Some("line2".to_string()));
        assert_eq!(read_line(&file_path, 5), None);
    }
}
