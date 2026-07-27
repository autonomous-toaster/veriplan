//! LSP transport — stdio-based event loop using lsp-server crate.
//!
//! The main loop receives JSON-RPC messages from the client, dispatches
//! them to the appropriate handler, and sends responses/notifications.

use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use lsp_server::{Connection, Message, Notification};
use lsp_types::*;

use super::state::ChangeStore;

/// Run the LSP server over stdio. Blocks until the client disconnects.
pub fn run_lsp() -> Result<()> {
    eprintln!("[veriplan-lsp] Starting LSP server over stdio...");

    let (connection, io_threads) = Connection::stdio();
    let (store, init_params) = initialize_lsp(&connection)?;

    // Main event loop
    main_loop(&connection, &store, &init_params)?;

    io_threads.join().context("LSP IO threads failed")?;
    Ok(())
}

fn initialize_lsp(
    connection: &Connection,
) -> Result<(Arc<RwLock<ChangeStore>>, serde_json::Value)> {
    let project_root = std::env::current_dir().context("Failed to get cwd")?;
    eprintln!("[veriplan-lsp] Project root: {}", project_root.display());

    let openspec_dir = project_root.join("openspec");
    if !openspec_dir.exists() {
        eprintln!("[veriplan-lsp] Warning: No openspec/ directory found at project root");
    }

    let store = Arc::new(RwLock::new(ChangeStore::new(&project_root)));
    let capabilities = build_server_capabilities();
    let server_capabilities =
        serde_json::to_value(&capabilities).context("Failed to serialize server capabilities")?;

    let init_params = connection
        .initialize(server_capabilities)
        .context("LSP initialize failed")?;

    let initialized_notif = Notification::new("initialized".to_string(), serde_json::json!({}));
    connection
        .sender
        .send(Message::Notification(initialized_notif))?;

    Ok((store, init_params))
}

fn build_server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(TextDocumentSyncSaveOptions::Supported(true)),
            },
        )),
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
    }
}

/// The main message dispatch loop.
fn main_loop(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    _init_params: &serde_json::Value,
) -> Result<()> {
    for msg in &connection.receiver {
        handle_message(connection, store, msg)?;
    }
    Ok(())
}

fn handle_message(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    msg: Message,
) -> Result<()> {
    match msg {
        Message::Request(req) => handle_request_message(connection, store, req),
        Message::Notification(not) => {
            if let Err(e) = super::handlers::handle_notification(connection, store, not) {
                eprintln!("[veriplan-lsp] Error handling notification: {e:#}");
            }
            Ok(())
        }
        Message::Response(_) => Ok(()),
    }
}

fn handle_request_message(
    connection: &Connection,
    store: &Arc<RwLock<ChangeStore>>,
    req: lsp_server::Request,
) -> Result<()> {
    if connection.handle_shutdown(&req).unwrap_or(false) {
        eprintln!("[veriplan-lsp] Shutdown requested");
        return Ok(());
    }
    if let Err(e) = super::handlers::handle_request(connection, store, req) {
        eprintln!("[veriplan-lsp] Error handling request: {e:#}");
    }
    Ok(())
}
