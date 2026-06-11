#![allow(clippy::redundant_clone)]
use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-ov-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_broad_readonly_narrow_readwrite_allows_write_in_narrow() {
    let dir = temp_dir();
    let broad = dir.join("broad");
    let narrow = broad.join("narrow");
    fs::create_dir_all(&narrow).unwrap();

    let narrow_file = narrow.join("file.txt");
    fs::write(&narrow_file, "content").unwrap();

    let broad_canonical = fs::canonicalize(&broad).unwrap();
    let narrow_canonical = fs::canonicalize(&narrow).unwrap();

    let sb = Sandbox::new(
        vec![
            AllowedRoot {
                original: broad.clone(),
                canonical: broad_canonical,
                mode: RootMode::ReadOnly,
            },
            AllowedRoot {
                original: narrow.clone(),
                canonical: narrow_canonical,
                mode: RootMode::ReadWrite,
            },
        ],
        None, // multi-root, no relative paths
    );

    // Write in narrow should be allowed (most specific = readWrite)
    let result = sb.resolve_existing_write(&narrow_file);
    assert!(result.is_ok(), "write in narrow readWrite root should be allowed: {:?}", result.err());
}

#[test]
fn test_broad_readonly_narrow_readwrite_rejects_write_in_broad() {
    let dir = temp_dir();
    let broad = dir.join("broad");
    let narrow = broad.join("narrow");
    fs::create_dir_all(&narrow).unwrap();

    let broad_file = broad.join("outside_narrow.txt");
    fs::write(&broad_file, "content").unwrap();

    let broad_canonical = fs::canonicalize(&broad).unwrap();
    let narrow_canonical = fs::canonicalize(&narrow).unwrap();

    let sb = Sandbox::new(
        vec![
            AllowedRoot {
                original: broad.clone(),
                canonical: broad_canonical,
                mode: RootMode::ReadOnly,
            },
            AllowedRoot {
                original: narrow.clone(),
                canonical: narrow_canonical,
                mode: RootMode::ReadWrite,
            },
        ],
        None,
    );

    // Write in broad (outside narrow) should be rejected (most specific = readOnly)
    let result = sb.resolve_existing_write(&broad_file);
    assert!(result.is_err(), "write in broad readOnly root (outside narrow) should be rejected");
}

#[test]
fn test_broad_readwrite_narrow_readonly_rejects_write_in_narrow() {
    let dir = temp_dir();
    let broad = dir.join("broad");
    let narrow = broad.join("narrow");
    fs::create_dir_all(&narrow).unwrap();

    let narrow_file = narrow.join("file.txt");
    fs::write(&narrow_file, "content").unwrap();

    let broad_canonical = fs::canonicalize(&broad).unwrap();
    let narrow_canonical = fs::canonicalize(&narrow).unwrap();

    let sb = Sandbox::new(
        vec![
            AllowedRoot {
                original: broad.clone(),
                canonical: broad_canonical,
                mode: RootMode::ReadWrite,
            },
            AllowedRoot {
                original: narrow.clone(),
                canonical: narrow_canonical,
                mode: RootMode::ReadOnly,
            },
        ],
        None,
    );

    // Write in narrow should be rejected (most specific = readOnly)
    let result = sb.resolve_existing_write(&narrow_file);
    assert!(
        result.is_err(),
        "write in narrow readOnly root should be rejected even though broad is readWrite"
    );
}

#[test]
fn test_read_allowed_in_all_overlaps() {
    let dir = temp_dir();
    let broad = dir.join("broad");
    let narrow = broad.join("narrow");
    fs::create_dir_all(&narrow).unwrap();

    let broad_file = broad.join("b.txt");
    let narrow_file = narrow.join("n.txt");
    fs::write(&broad_file, "b").unwrap();
    fs::write(&narrow_file, "n").unwrap();

    let broad_canonical = fs::canonicalize(&broad).unwrap();
    let narrow_canonical = fs::canonicalize(&narrow).unwrap();

    let sb = Sandbox::new(
        vec![
            AllowedRoot {
                original: broad.clone(),
                canonical: broad_canonical,
                mode: RootMode::ReadOnly,
            },
            AllowedRoot {
                original: narrow.clone(),
                canonical: narrow_canonical,
                mode: RootMode::ReadOnly,
            },
        ],
        None,
    );

    assert!(sb.resolve_existing_read(&broad_file).is_ok());
    assert!(sb.resolve_existing_read(&narrow_file).is_ok());
}
