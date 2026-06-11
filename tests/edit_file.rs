/// edit_file tests are functional: they test the tool logic against real files.
/// They use the sandbox for path validation and verify edit operations.
use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-edit-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_config(sandbox: Sandbox) -> mcp_filesystem_rs::config::AppConfig {
    mcp_filesystem_rs::config::AppConfig {
        sandbox,
        limits: mcp_filesystem_rs::config::Limits::default(),
        behavior: mcp_filesystem_rs::config::Behavior::default(),
    }
}

#[test]
fn test_edit_file_single_replacement() {
    let dir = temp_dir();
    let file = dir.join("test.rs");
    fs::write(&file, "println!(\"hello\");\n").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sandbox = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadWrite,
        }],
        Some(dir.clone()),
    );
    let config = make_config(sandbox);

    let params = json!({
        "path": file.display().to_string(),
        "edits": [{
            "oldText": r#"println!("hello");"#,
            "newText": r#"println!("hello world");"#,
            "replaceAll": false
        }],
        "dryRun": true
    });

    let result = mcp_filesystem_rs::tools::edit_file::execute(&config.sandbox, &config, params);

    assert!(result.is_ok(), "edit should succeed: {:?}", result.err());
    let output = result.unwrap();
    assert_eq!(output["changed"], true);
    assert!(output["diff"].as_str().unwrap().contains("hello"));

    // File should not be modified (dryRun)
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("hello"));
    assert!(!content.contains("hello world"));
}

#[test]
fn test_edit_file_apply_writes_file() {
    let dir = temp_dir();
    let file = dir.join("test.md");
    fs::write(&file, "original content\n").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sandbox = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadWrite,
        }],
        Some(dir.clone()),
    );
    let config = make_config(sandbox);

    let params = json!({
        "path": file.display().to_string(),
        "edits": [{
            "oldText": "original content",
            "newText": "modified content",
            "replaceAll": false
        }],
        "dryRun": false
    });

    let result = mcp_filesystem_rs::tools::edit_file::execute(&config.sandbox, &config, params);

    assert!(result.is_ok());
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("modified content"));
}

#[test]
fn test_edit_file_pattern_not_found() {
    let dir = temp_dir();
    let file = dir.join("not_found.txt");
    fs::write(&file, "some content\n").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sandbox = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadWrite,
        }],
        Some(dir.clone()),
    );
    let config = make_config(sandbox);

    let params = json!({
        "path": file.display().to_string(),
        "edits": [{
            "oldText": "nonexistent pattern",
            "newText": "replacement",
            "replaceAll": false
        }],
        "dryRun": true
    });

    let result = mcp_filesystem_rs::tools::edit_file::execute(&config.sandbox, &config, params);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        mcp_filesystem_rs::error::FsError::EditPatternNotFound { .. }
    ));
}

#[test]
fn test_edit_file_replace_all() {
    let dir = temp_dir();
    let file = dir.join("replace_all.txt");
    fs::write(&file, "foo bar foo baz foo\n").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sandbox = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadWrite,
        }],
        Some(dir.clone()),
    );
    let config = make_config(sandbox);

    let params = json!({
        "path": file.display().to_string(),
        "edits": [{
            "oldText": "foo",
            "newText": "qux",
            "replaceAll": true
        }],
        "dryRun": false
    });

    let result = mcp_filesystem_rs::tools::edit_file::execute(&config.sandbox, &config, params);

    assert!(result.is_ok());
    let content = fs::read_to_string(&file).unwrap();
    assert!(!content.contains("foo"));
    assert_eq!(content.matches("qux").count(), 3);
}

#[test]
fn test_edit_file_ambiguous_multiple_matches() {
    let dir = temp_dir();
    let file = dir.join("ambiguous.txt");
    fs::write(&file, "dup dup\n").unwrap();

    let canonical = fs::canonicalize(&dir).unwrap();
    let sandbox = Sandbox::new(
        vec![AllowedRoot {
            original: dir.clone(),
            canonical,
            mode: RootMode::ReadWrite,
        }],
        Some(dir.clone()),
    );
    let config = make_config(sandbox);

    let params = json!({
        "path": file.display().to_string(),
        "edits": [{
            "oldText": "dup",
            "newText": "unique",
            "replaceAll": false
        }],
        "dryRun": true
    });

    let result = mcp_filesystem_rs::tools::edit_file::execute(&config.sandbox, &config, params);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        mcp_filesystem_rs::error::FsError::EditPatternAmbiguous { .. }
    ));
}
