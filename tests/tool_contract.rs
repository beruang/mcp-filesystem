/// Integration tests for the full tool contract via the sandbox + tools API.
use mcp_filesystem_rs::config::{AppConfig, Behavior, Limits};
use mcp_filesystem_rs::sandbox::{AllowedRoot, RootMode, Sandbox};
use mcp_filesystem_rs::tools;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mcp-fs-tc-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup(dir: &std::path::Path) -> (Sandbox, AppConfig) {
    let canonical = fs::canonicalize(dir).unwrap();
    let sandbox = Sandbox::new(
        vec![AllowedRoot {
            original: dir.to_path_buf(),
            canonical,
            mode: RootMode::ReadWrite,
        }],
        Some(dir.to_path_buf()),
    );
    let config = AppConfig {
        sandbox: sandbox.clone(),
        limits: Limits::default(),
        behavior: Behavior::default(),
    };
    (sandbox, config)
}

#[test]
fn test_list_allowed_directories() {
    let dir = temp_dir();
    let (sandbox, config) = setup(&dir);

    let result = tools::list_allowed_directories::execute(&sandbox, &config, json!({}));
    assert!(result.is_ok());
    let output = result.unwrap();
    let dirs: Vec<String> = serde_json::from_value(output["directories"].clone()).unwrap();
    assert!(!dirs.is_empty());
}

#[test]
fn test_read_text_file() {
    let dir = temp_dir();
    let file = dir.join("readme.txt");
    fs::write(&file, "Hello, world!\n").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::read_text_file::execute(
        &sandbox,
        &config,
        json!({"path": file.display().to_string()}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["content"], "Hello, world!\n");
}

#[test]
fn test_read_text_file_with_head() {
    let dir = temp_dir();
    let file = dir.join("lines.txt");
    fs::write(&file, "line1\nline2\nline3\nline4\nline5\n").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::read_text_file::execute(
        &sandbox,
        &config,
        json!({"path": file.display().to_string(), "head": 2}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    let lines: Vec<&str> = output["content"].as_str().unwrap().lines().collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_list_directory() {
    let dir = temp_dir();
    fs::write(dir.join("a.txt"), "a").unwrap();
    fs::create_dir_all(dir.join("sub")).unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::list_directory::execute(
        &sandbox,
        &config,
        json!({"path": dir.display().to_string()}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output["entries"].as_array().unwrap().len() >= 2);
}

#[test]
fn test_get_file_info() {
    let dir = temp_dir();
    let file = dir.join("info.txt");
    fs::write(&file, "data").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::get_file_info::execute(
        &sandbox,
        &config,
        json!({"path": file.display().to_string()}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["type"], "file");
    assert_eq!(output["size"], 4);
}

#[test]
fn test_write_file() {
    let dir = temp_dir();
    let file = dir.join("new_file.txt");
    let (sandbox, config) = setup(&dir);

    let result = tools::write_file::execute(
        &sandbox,
        &config,
        json!({"path": file.display().to_string(), "content": "created content"}),
    );
    assert!(result.is_ok());
    assert!(file.exists());
    assert_eq!(fs::read_to_string(&file).unwrap(), "created content");
}

#[test]
fn test_write_file_create_parents() {
    let dir = temp_dir();
    let file = dir.join("deep/nested/file.txt");
    let (sandbox, config) = setup(&dir);

    let result = tools::write_file::execute(
        &sandbox,
        &config,
        json!({"path": file.display().to_string(), "content": "deep", "createParents": true}),
    );
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(file.exists());
}

#[test]
fn test_create_directory() {
    let dir = temp_dir();
    let new_dir = dir.join("created_dir/nested");
    let (sandbox, config) = setup(&dir);

    let result = tools::create_directory::execute(
        &sandbox,
        &config,
        json!({"path": new_dir.display().to_string()}),
    );
    assert!(result.is_ok());
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

#[test]
fn test_move_file() {
    let dir = temp_dir();
    let src = dir.join("source.txt");
    let dst = dir.join("dest.txt");
    fs::write(&src, "movable").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::move_file::execute(
        &sandbox,
        &config,
        json!({"source": src.display().to_string(), "destination": dst.display().to_string()}),
    );
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(!src.exists());
    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "movable");
}

#[test]
fn test_read_multiple_files() {
    let dir = temp_dir();
    fs::write(dir.join("a.txt"), "aaa").unwrap();
    fs::write(dir.join("b.txt"), "bbb").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::read_multiple_files::execute(
        &sandbox,
        &config,
        json!({"paths": [
            dir.join("a.txt").display().to_string(),
            dir.join("b.txt").display().to_string(),
            dir.join("nonexistent.txt").display().to_string()
        ]}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["files"].as_array().unwrap().len(), 2);
    assert_eq!(output["errors"].as_array().unwrap().len(), 1);
}

#[test]
fn test_read_media_file() {
    let dir = temp_dir();
    let file = dir.join("image.png");
    fs::write(&file, [0x89, b'P', b'N', b'G', 0, 0, 0, 0]).unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::read_media_file::execute(
        &sandbox,
        &config,
        json!({"path": file.display().to_string()}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["mimeType"], "image/png");
    assert!(!output["data"].as_str().unwrap().is_empty());
}

#[test]
fn test_directory_tree() {
    let dir = temp_dir();
    let sub = dir.join("src");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::directory_tree::execute(
        &sandbox,
        &config,
        json!({"path": dir.display().to_string(), "maxDepth": 3}),
    );
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_search_files() {
    let dir = temp_dir();
    fs::write(dir.join("a.rs"), "// a").unwrap();
    fs::write(dir.join("b.rs"), "// b").unwrap();
    fs::write(dir.join("c.txt"), "c").unwrap();
    let (sandbox, config) = setup(&dir);

    let result = tools::search_files::execute(
        &sandbox,
        &config,
        json!({
            "path": dir.display().to_string(),
            "pattern": "*.rs"
        }),
    );
    assert!(result.is_ok(), "{:?}", result.err());
    let output = result.unwrap();
    let matches = output["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);
}
