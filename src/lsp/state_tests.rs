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

#[test]
fn test_scan_changes_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = ChangeStore::new(dir.path());
    store.scan_changes();
    assert!(store.plans.is_empty());
}

#[test]
fn test_scan_changes_with_valid_change() {
    let dir = tempfile::tempdir().unwrap();
    let changes_dir = dir.path().join("openspec").join("changes");
    std::fs::create_dir_all(&changes_dir).unwrap();
    let change_dir = changes_dir.join("my-change");
    std::fs::create_dir(&change_dir).unwrap();
    std::fs::write(change_dir.join("tasks.md"), "- [ ] 1.1 Setup\n").unwrap();
    let specs_dir = change_dir.join("specs").join("cap");
    std::fs::create_dir_all(&specs_dir).unwrap();
    std::fs::write(specs_dir.join("spec.md"), "# Spec\n").unwrap();
    let mut store = ChangeStore::new(dir.path());
    store.scan_changes();
    // scan_changes loads changes that have both tasks.md and specs/
    // Our change has both, so it should be loaded
    assert!(
        store.plans.contains_key("my-change")
            || store.file_to_change.values().any(|v| v == "my-change")
    );
}

#[test]
fn test_resolve_by_path_walk_found() {
    let dir = tempfile::tempdir().unwrap();
    let change_dir = dir
        .path()
        .join("openspec")
        .join("changes")
        .join("my-change");
    std::fs::create_dir_all(&change_dir).unwrap();
    // File inside specs/ subdirectory of the change dir (2 levels deep)
    let specs_dir = change_dir.join("specs");
    std::fs::create_dir(&specs_dir).unwrap();
    let file_path = specs_dir.join("spec.md");
    std::fs::write(&file_path, "").unwrap();
    let store = ChangeStore::new(dir.path());
    let result = store.resolve_by_path_walk(&file_path);
    assert_eq!(result, Some("my-change".to_string()));
}

#[test]
fn test_resolve_by_path_walk_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("some").join("random").join("file.md");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, "").unwrap();
    let store = ChangeStore::new(dir.path());
    let result = store.resolve_by_path_walk(&file_path);
    assert_eq!(result, None);
}

#[test]
fn test_is_valid_change_entry_valid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("tasks.md"), "").unwrap();
    std::fs::create_dir(dir.path().join("specs")).unwrap();
    // Create a DirEntry for the temp dir
    let entry = std::fs::read_dir(dir.path().parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path() == dir.path())
        .unwrap();
    let result = is_valid_change_entry(&entry);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().0,
        dir.path().file_name().unwrap().to_string_lossy()
    );
}

#[test]
fn test_is_valid_change_entry_archive() {
    let dir = tempfile::tempdir().unwrap();
    let archive_dir = dir.path().join("archive");
    std::fs::create_dir(&archive_dir).unwrap();
    let entry = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path() == archive_dir)
        .unwrap();
    let result = is_valid_change_entry(&entry);
    assert!(result.is_none());
}

#[test]
fn test_extract_change_name_from_path_found() {
    let dir = tempfile::tempdir().unwrap();
    let change_dir = dir
        .path()
        .join("openspec")
        .join("changes")
        .join("my-change");
    std::fs::create_dir_all(&change_dir).unwrap();
    // Pass a file path inside the change dir (not the change dir itself)
    let file_path = change_dir.join("tasks.md");
    std::fs::write(&file_path, "").unwrap();
    let result = extract_change_name_from_path(&file_path);
    assert_eq!(result, Some("my-change".to_string()));
}

#[test]
fn test_extract_change_name_from_path_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let result = extract_change_name_from_path(dir.path());
    assert_eq!(result, None);
}

#[test]
fn test_get_parent_and_grandparent() {
    let path = Path::new("/a/b/c/d");
    let result = get_parent_and_grandparent(path);
    assert!(result.is_some());
    let (parent, grandparent) = result.unwrap();
    assert_eq!(parent, Path::new("/a/b/c"));
    assert_eq!(grandparent, Path::new("/a/b"));
}

#[test]
fn test_get_parent_and_grandparent_root() {
    let path = Path::new("/");
    let result = get_parent_and_grandparent(path);
    assert!(result.is_none());
}
