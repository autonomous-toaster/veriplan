//! LSP transport — stdio-based event loop using lsp-server crate.
//!
//! The main loop receives JSON-RPC messages from the client, dispatches
//! them to the appropriate handler, and sends responses/notifications.

use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use lsp_server::{Connection, Message, Notification, Request, Response, ErrorCode};
use lsp_types::*;


use super::code_actions;
use super::completions;
use super::diagnostics as diag;
use super::navigation;
use super::state::ChangeStore;
use super::symbols;

/// Run the LSP server over stdio. Blocks until the client disconnects.
pub fn run_lsp() -> Result<()> {
    eprintln!("[veriplan-lsp] Starting LSP server over stdio...");

    let (connection, io_threads) = Connection::stdio();

    // Determine project root from the client's workspace folders or cwd
    let project_root = std::env::current_dir().context("Failed to get cwd")?;
    eprintln!("[veriplan-lsp] Project root: {}", project_root.display());

    // Check if openspec directory exists
    let openspec_dir = project_root.join("openspec");
    if !openspec_dir.exists() {
        eprintln!("[veriplan-lsp] Warning: No openspec/ directory found at project root");
    }

    // Initialize state
    let store = Arc::new(RwLock::new(ChangeStore::new(&project_root)));

    // Server capabilities
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(TextDocumentSyncSaveOptions::Supported(true)),
            })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["T".to_string(), "t".to_string()]),
            all_commit_characters: None,
            resolve_provider: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            completion_item: None,
        }),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    };

    let server_capabilities = serde_json::to_value(&capabilities).unwrap();

    // Initialize handshake
    let init_params = connection
        .initialize(server_capabilities)
        .context("LSP initialize failed")?;

    // Send initialized notification
    let initialized_notif = Notification::new("initialized".to_string(), serde_json::json!({}));
    connection
        .sender
        .send(Message::Notification(initialized_notif))?;

    // Main event loop
    main_loop(&connection, &store, &init_params)?;

    io_threads.join().context("LSP IO threads failed")?;
    Ok(())
}

/// The main message dispatch loop.
fn main_loop(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    _init_params: &serde_json::Value,
) -> Result<()> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection
                    .handle_shutdown(&req)
                    .unwrap_or(false)
                {
                    eprintln!("[veriplan-lsp] Shutdown requested");
                    return Ok(());
                }
                if let Err(e) = handle_request(connection, store, req) {
                    eprintln!("[veriplan-lsp] Error handling request: {e:#}");
                }
            }
            Message::Notification(not) => {
                if let Err(e) = handle_notification(connection, store, not) {
                    eprintln!("[veriplan-lsp] Error handling notification: {e:#}");
                }
            }
            Message::Response(_) => {
                // We don't send requests to the client, so ignore responses
            }
        }
    }
    Ok(())
}

fn handle_request(
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
            connection
                .sender
                .send(Message::Response(response))?;
        }
        "textDocument/definition" => {
            let params: GotoDefinitionParams =
                serde_json::from_value(req.params).context("Bad goto-def params")?;
            let result = handle_goto_definition(store, &params);
            let response = Response::new_ok(req.id, result);
            connection
                .sender
                .send(Message::Response(response))?;
        }
        "textDocument/hover" => {
            let params: HoverParams =
                serde_json::from_value(req.params).context("Bad hover params")?;
            let result = handle_hover(store, &params);
            let response = Response::new_ok(req.id, result);
            connection
                .sender
                .send(Message::Response(response))?;
        }
        "textDocument/documentSymbol" => {
            let params: DocumentSymbolParams =
                serde_json::from_value(req.params).context("Bad symbol params")?;
            let result = handle_document_symbols(store, &params);
            let response = Response::new_ok(req.id, result);
            connection
                .sender
                .send(Message::Response(response))?;
        }
        "textDocument/codeAction" => {
            let params: CodeActionParams =
                serde_json::from_value(req.params).context("Bad code action params")?;
            let result = handle_code_action(store, &params);
            let response = Response::new_ok(req.id, result);
            connection
                .sender
                .send(Message::Response(response))?;
        }
        _ => {
            // Unknown method — respond with MethodNotFound
            let response = Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                format!("Unknown method: {}", req.method),
            );
            connection
                .sender
                .send(Message::Response(response))?;
        }
    }
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    not: Notification,
) -> Result<()> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams =
                serde_json::from_value(not.params).context("Bad didOpen params")?;
            let file_path = params.text_document.uri.to_file_path().unwrap_or_default();
            eprintln!("[veriplan-lsp] didOpen: {}", file_path.display());

            // Try to resolve the change. If not found, rescan — the change
            // may have been created after the LSP started.
            let change_name = {
                let read_store = store.read().unwrap();
                let resolved = read_store.resolve_change(&file_path);
                drop(read_store);
                if resolved.is_none() {
                    // Re-scan to pick up newly created change directories
                    store.write().unwrap().rescan();
                    store.read().unwrap().resolve_change(&file_path)
                } else {
                    resolved
                }
            };

            if let Some(change) = change_name {
                // Refresh: re-parse and re-check this change, then publish diagnostics
                let diagnostics = store.write().unwrap().refresh(&change);
                eprintln!(
                    "[veriplan-lsp] didOpen: resolved change '{}', {} diagnostic files",
                    change,
                    diagnostics.len()
                );
                for (path, diags) in diagnostics {
                    if let Ok(uri) = lsp_types::Url::from_file_path(&path) {
                        eprintln!(
                            "[veriplan-lsp] publishDiagnostics: {} ({} diagnostics)",
                            uri,
                            diags.len()
                        );
                        let params = PublishDiagnosticsParams {
                            uri,
                            diagnostics: diags,
                            version: None,
                        };
                        let notif =
                            Notification::new("textDocument/publishDiagnostics".to_string(), params);
                        let _ = connection.sender.send(Message::Notification(notif));
                    }
                }
            } else {
                eprintln!("[veriplan-lsp] didOpen: file not in any change, publishing empty diagnostics");
                // File not in any change — publish empty diagnostics to clear stale markers
                if let Ok(uri) = lsp_types::Url::from_file_path(&file_path) {
                    let params = PublishDiagnosticsParams {
                        uri,
                        diagnostics: Vec::new(),
                        version: None,
                    };
                    let notif =
                        Notification::new("textDocument/publishDiagnostics".to_string(), params);
                    let _ = connection.sender.send(Message::Notification(notif));
                }
            }
        }
        "textDocument/didChange" => {
            // Treat didChange like didSave — refresh diagnostics for the
            // affected change. pi-lens sends didChange after didOpen when
            // the file content is synced; we re-parse and republish.
            let params: DidChangeTextDocumentParams =
                serde_json::from_value(not.params).context("Bad didChange params")?;
            let file_path = params.text_document.uri.to_file_path().unwrap_or_default();
            eprintln!("[veriplan-lsp] didChange: {}", file_path.display());

            // Try to resolve the change. If not found, rescan.
            let change_name = {
                let read_store = store.read().unwrap();
                let resolved = read_store.resolve_change(&file_path);
                drop(read_store);
                if resolved.is_none() {
                    store.write().unwrap().rescan();
                    store.read().unwrap().resolve_change(&file_path)
                } else {
                    resolved
                }
            };

            let diagnostics_per_file = if let Some(change) = change_name {
                eprintln!("[veriplan-lsp] didChange: resolved change '{}', refreshing...", change);
                store.write().unwrap().refresh(&change)
            } else {
                eprintln!("[veriplan-lsp] didChange: file not in any change");
                Vec::new()
            };

            for (path, diags) in &diagnostics_per_file {
                if let Ok(uri) = lsp_types::Url::from_file_path(path) {
                    eprintln!("[veriplan-lsp] publishDiagnostics: {} ({} diagnostics)", uri, diags.len());
                    let params = PublishDiagnosticsParams {
                        uri,
                        diagnostics: diags.clone(),
                        version: None,
                    };
                    let notif =
                        Notification::new("textDocument/publishDiagnostics".to_string(), params);
                    let _ = connection.sender.send(Message::Notification(notif));
                }
            }

            // Clear diagnostics for files in the change that didn't get diagnostics
            if let Some(change) = store.read().unwrap().resolve_change(&file_path) {
                let change_dir = store
                    .read()
                    .unwrap()
                    .project_root()
                    .join("openspec")
                    .join("changes")
                    .join(&change);
                if let Ok(entries) = walk_files_for_clear(&change_dir) {
                    let published_uris: Vec<_> = diagnostics_per_file
                        .iter()
                        .map(|(p, _)| p.clone())
                        .collect();
                    for path in entries {
                        if !published_uris.contains(&path)
                            && let Ok(uri) = lsp_types::Url::from_file_path(&path) {
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
        "textDocument/didSave" => {
            let params: DidSaveTextDocumentParams =
                serde_json::from_value(not.params).context("Bad didSave params")?;
            let file_path = params.text_document.uri.to_file_path().unwrap_or_default();
            eprintln!("[veriplan-lsp] didSave: {}", file_path.display());

            // Try to resolve the change. If not found, rescan.
            let change_name = {
                let read_store = store.read().unwrap();
                let resolved = read_store.resolve_change(&file_path);
                drop(read_store);
                if resolved.is_none() {
                    store.write().unwrap().rescan();
                    store.read().unwrap().resolve_change(&file_path)
                } else {
                    resolved
                }
            };

            let diagnostics_per_file = if let Some(change) = change_name {
                eprintln!("[veriplan-lsp] didSave: resolved change '{}', refreshing...", change);
                // Refresh and get diagnostics
                store.write().unwrap().refresh(&change)
            } else {
                eprintln!("[veriplan-lsp] didSave: file not in any change");
                Vec::new()
            };

            // Publish diagnostics for all affected files
            let mut published_uris = Vec::new();
            for (path, diags) in &diagnostics_per_file {
                if let Ok(uri) = lsp_types::Url::from_file_path(path) {
                    published_uris.push(path.clone());
                    let params = PublishDiagnosticsParams {
                        uri,
                        diagnostics: diags.clone(),
                        version: None,
                    };
                    let notif =
                        Notification::new("textDocument/publishDiagnostics".to_string(), params);
                    let _ = connection.sender.send(Message::Notification(notif));
                }
            }

            // Clear diagnostics for files that were in the change but didn't get diagnostics
            if let Some(change) = store.read().unwrap().resolve_change(&file_path) {
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
                            && let Ok(uri) = lsp_types::Url::from_file_path(&path) {
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
        _ => {
            // Unknown notification — ignore
        }
    }
    Ok(())
}

// ── Request handlers ──

fn handle_completion(
    store: &Arc<RwLock<ChangeStore>>,
    params: &CompletionParams,
) -> Option<CompletionResponse> {
    let file_path = params.text_document_position.text_document.uri.to_file_path().ok()?;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;
    let plan = store.read().ok()?.get_plan(&change_name)?.clone();

    // Read the current line to determine context
    let pos = params.text_document_position.position;
    // We don't have the line text without re-reading, but completions can work
    // with just the plan context
    let completions = completions::get_completions(
        &plan,
        "",   // line text (simplified)
        pos.character as usize,
    )?;

    Some(CompletionResponse::List(completions))
}

fn handle_goto_definition(
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

fn handle_hover(
    store: &Arc<RwLock<ChangeStore>>,
    params: &HoverParams,
) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let file_path = uri.to_file_path().ok()?;
    let pos = params.text_document_position_params.position;
    let change_name = store.read().ok()?.resolve_change(&file_path)?;
    let plan = store.read().ok()?.get_plan(&change_name)?.clone();

    let line_text = read_line(&file_path, pos.line as usize)?;
    navigation::hover(&plan, &pos, &line_text)
}

fn handle_document_symbols(
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

fn handle_code_action(
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
fn read_line(path: &Path, line: usize) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().nth(line).map(|s| s.to_string())
}

/// Walk files in a change directory (for clearing diagnostics).
fn walk_files_for_clear(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_file() {
        files.push(dir.to_path_buf());
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files_for_clear(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
