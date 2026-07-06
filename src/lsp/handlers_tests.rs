#[cfg(test)]
mod tests {
    use crate::lsp::handlers::*;
    use crate::ir::*;

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
