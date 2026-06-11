use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-create-{}-{}", std::process::id(), n));
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
fn test_resolve_create_write_allows_new_file() {
    let dir = temp_dir();
    let new_file = dir.join("new/subdir/file.txt");

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_create_write(&new_file);
    assert!(
        result.is_ok(),
        "creating new file under root should be allowed: {:?}",
        result.err()
    );
}

#[test]
fn test_resolve_create_write_rejects_outside() {
    let dir = temp_dir();
    let outside = std::env::temp_dir().join(format!("outside-create-{}", std::process::id()));
    fs::create_dir_all(&outside).unwrap();
    let new_file = outside.join("new.txt");

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_create_write(&new_file);
    assert!(
        result.is_err(),
        "creating file outside root should be rejected"
    );
}

#[test]
fn test_resolve_create_write_rejects_traversal() {
    let dir = temp_dir();
    let traversal = dir.join("child/../../../escape.txt");

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_create_write(&traversal);
    // The canonical parent of ../../.. would resolve outside the root
    assert!(
        result.is_err(),
        "path traversal in create should be rejected"
    );
}

#[test]
fn test_resolve_create_write_finds_nearest_parent() {
    let dir = temp_dir();
    let existing = dir.join("exists");
    fs::create_dir_all(&existing).unwrap();
    let new_file = existing.join("deep/path/file.txt");

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_create_write(&new_file);
    assert!(
        result.is_ok(),
        "create with nearest existing parent should work: {:?}",
        result.err()
    );
}
