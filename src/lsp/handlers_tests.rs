use crate::lsp::handlers::*;
use crate::lsp::state::ChangeStore;
use std::sync::{Arc, RwLock};

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

#[test]
fn test_handle_document_symbols_tasks_md() {
    let dir = tempfile::tempdir().unwrap();
    let changes_dir = dir.path().join("openspec").join("changes");
    std::fs::create_dir_all(&changes_dir).unwrap();
    let change_dir = changes_dir.join("my-change");
    std::fs::create_dir(&change_dir).unwrap();
    // tasks.md with a phase and a task
    std::fs::write(change_dir.join("tasks.md"), "## Phase 1\n- [ ] 1.1 Setup\n").unwrap();
    let specs_dir = change_dir.join("specs").join("cap");
    std::fs::create_dir_all(&specs_dir).unwrap();
    std::fs::write(specs_dir.join("spec.md"), "# Spec\n").unwrap();

    let store = Arc::new(RwLock::new(ChangeStore::new(dir.path())));
    let uri = lsp_types::Url::from_file_path(change_dir.join("tasks.md")).unwrap();
    let params = lsp_types::DocumentSymbolParams {
        text_document: lsp_types::TextDocumentIdentifier { uri },
        work_done_progress_params: lsp_types::WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: lsp_types::PartialResultParams {
            partial_result_token: None,
        },
    };

    let result = handle_document_symbols(&store, &params);
    assert!(result.is_some());
}
