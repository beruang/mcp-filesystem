use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};
use std::fs;
use std::os::unix;
use std::path::PathBuf;

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-sym-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sandbox_from_root(root: &std::path::Path, mode: RootMode) -> Sandbox {
    let canonical = fs::canonicalize(root).unwrap();
    Sandbox::new(
        vec![AllowedRoot { original: root.to_path_buf(), canonical: canonical.clone(), mode }],
        Some(canonical),
    )
}

#[test]
fn test_symlink_outside_rejected_read() {
    let dir = temp_dir();
    let outside = temp_dir();
    let outside_file = outside.join("secret.txt");
    fs::write(&outside_file, "secret").unwrap();

    let link = dir.join("escape_link");
    unix::fs::symlink(&outside_file, &link).unwrap();

    // The link itself is under the root but points outside
    // Reading via canonical path should detect the escape
    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_read(&link);

    // The canonical path of the link will be the outside file
    // which is NOT under the root, so it should be rejected
    assert!(result.is_err(), "symlink pointing outside root should be rejected");
}

#[test]
fn test_symlink_outside_rejected_write() {
    let dir = temp_dir();
    let outside = temp_dir();
    let outside_file = outside.join("new_secret.txt");

    let link = dir.join("escape_link");
    unix::fs::symlink(&outside_file, &link).unwrap();

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_write(&link);

    assert!(result.is_err(), "write via symlink pointing outside should be rejected");
}

#[test]
fn test_symlink_inside_allowed() {
    let dir = temp_dir();
    let real_dir = dir.join("real");
    fs::create_dir_all(&real_dir).unwrap();
    let real_file = real_dir.join("file.txt");
    fs::write(&real_file, "content").unwrap();

    let link_dir = dir.join("link_dir");
    unix::fs::symlink(&real_dir, &link_dir).unwrap();
    let link_file = link_dir.join("file.txt");

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_existing_read(&link_file);

    // Canonical path resolves to dir/real/file.txt which is inside root
    assert!(result.is_ok(), "symlink pointing inside root should be allowed: {:?}", result.err());
}

#[test]
fn test_symlink_to_nonexistent_inside_allowed_for_create() {
    let dir = temp_dir();
    let real_dir = dir.join("real");
    fs::create_dir_all(&real_dir).unwrap();

    let link_dir = dir.join("link_dir");
    unix::fs::symlink(&real_dir, &link_dir).unwrap();
    let new_file = link_dir.join("new_file.txt");

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    let result = sb.resolve_create_write(&new_file);

    assert!(result.is_ok(), "create via internal symlink should be allowed: {:?}", result.err());
}

#[test]
fn test_symlink_chain_outside_rejected() {
    let dir = temp_dir();
    let outside = temp_dir();

    let first_link = dir.join("first");
    let second_link = outside.join("second");
    let outside_file = outside.join("secret.txt");
    fs::write(&outside_file, "secret").unwrap();

    unix::fs::symlink(&second_link, &first_link).unwrap();
    unix::fs::symlink(&outside_file, &second_link).unwrap();

    let sb = sandbox_from_root(&dir, RootMode::ReadWrite);
    // Reading first_link -> second_link -> outside_file
    // Canonical resolution should catch this
    let result = sb.resolve_existing_read(&first_link);

    // The first link points to second_link which doesn't exist within the root,
    // so canonicalize will fail or resolve to outside
    assert!(result.is_err(), "symlink chain pointing outside should be rejected");
}
