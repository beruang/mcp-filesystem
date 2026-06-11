use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-test-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sandbox_from_root(root: &std::path::Path, mode: RootMode) -> Sandbox {
    let canonical = fs::canonicalize(root).unwrap();
    Sandbox::new(
        vec![AllowedRoot {
            original: root.to_path_buf(),
            canonical: canonical.clone(),
            mode,
        }],
        Some(canonical),
    )
}

#[test]
fn test_resolve_existing_read_allows_child() {
    let dir = temp_dir();
    let file = dir.join("test.txt");
    fs::write(&file, "hello").unwrap();

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_read(&file);
    assert!(
        result.is_ok(),
        "child file should be readable: {:?}",
        result.err()
    );
}

#[test]
fn test_resolve_existing_read_rejects_outside() {
    let dir = temp_dir();
    let outside = std::env::temp_dir().join(format!("outside-{}", std::process::id()));
    fs::create_dir_all(&outside).unwrap();
    let file = outside.join("secret.txt");
    fs::write(&file, "secret").unwrap();

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_read(&file);
    assert!(result.is_err(), "path outside root should be rejected");
}

#[test]
fn test_resolve_rejects_sibling_prefix() {
    let dir = temp_dir();
    // Create a sibling directory whose name starts with the root name
    let root_name = dir.file_name().unwrap().to_string_lossy().to_string();
    let sibling = dir.parent().unwrap().join(format!("{}Evil", root_name));
    fs::create_dir_all(&sibling).unwrap();
    let file = sibling.join("file.txt");
    fs::write(&file, "evil").unwrap();

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_read(&file);
    assert!(result.is_err(), "sibling prefix attack should be rejected");
}

#[test]
fn test_resolve_allows_deep_descendant() {
    let dir = temp_dir();
    let deep = dir.join("a/b/c/d");
    fs::create_dir_all(&deep).unwrap();
    let file = deep.join("deep.txt");
    fs::write(&file, "deep").unwrap();

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_read(&file);
    assert!(
        result.is_ok(),
        "deep descendant should be allowed: {:?}",
        result.err()
    );
}

#[test]
fn test_list_allowed_directories() {
    let dir = temp_dir();
    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let dirs = sb.list_allowed_directories();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0], dir.display().to_string());
}
