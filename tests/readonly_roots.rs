use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-ro-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_read_allowed_on_readonly_root() {
    let dir = temp_dir();
    let file = dir.join("readable.txt");
    fs::write(&file, "hello").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sb = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadOnly,
        }],
        Some(dir.clone()),
    );

    let result = sb.resolve_existing_read(&file);
    assert!(
        result.is_ok(),
        "read on readOnly root should be allowed: {:?}",
        result.err()
    );
}

#[test]
fn test_write_rejected_on_readonly_root() {
    let dir = temp_dir();
    let file = dir.join("readable.txt");
    fs::write(&file, "hello").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sb = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadOnly,
        }],
        Some(dir.clone()),
    );

    let result = sb.resolve_existing_write(&file);
    assert!(result.is_err(), "write on readOnly root should be rejected");

    if let Err(e) = result {
        assert_eq!(
            e.error_code(),
            "read_only_root",
            "should return read_only_root error"
        );
    }
}

#[test]
fn test_create_write_rejected_on_readonly_root() {
    let dir = temp_dir();
    let new_file = dir.join("new.txt");

    let canonical = fs::canonicalize(&dir).unwrap();
    let sb = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadOnly,
        }],
        Some(dir.clone()),
    );

    let result = sb.resolve_create_write(&new_file);
    assert!(
        result.is_err(),
        "create on readOnly root should be rejected"
    );
}
