use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String,
}

#[derive(Serialize)]
pub struct ListDirectoryOutput {
    pub entries: Vec<DirEntry>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "list_directory".to_string(),
        description: "List direct children of a directory. Returns name, path, and type (file, directory, symlink, other) for each entry.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the directory to list"}
            }),
            vec!["path"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let requested = Path::new(path_str);

    let resolved = sandbox.resolve_existing_read(requested)?;

    if !resolved.canonical.is_dir() {
        return Err(FsError::NotADirectory { path: requested.to_path_buf() });
    }

    let mut entries: Vec<DirEntry> = Vec::new();
    let dir_iter = std::fs::read_dir(&resolved.canonical)?;

    for entry in dir_iter.take(config.limits.max_directory_entries) {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().display().to_string();
        let file_type = entry.file_type()?;

        let entry_type = if file_type.is_dir() {
            "directory"
        } else if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };

        entries.push(DirEntry { name, path, entry_type: entry_type.to_string() });
    }

    // Check if we hit the limit
    let mut dir_iter = std::fs::read_dir(&resolved.canonical)?;
    if dir_iter.nth(config.limits.max_directory_entries).is_some() {
        Err(FsError::TooManyResults {
            count: config.limits.max_directory_entries + 1,
            max: config.limits.max_directory_entries,
        })?;
    }

    serde_json::to_value(ListDirectoryOutput { entries })
        .map_err(|e| FsError::SerializationError { message: e.to_string() })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    use super::*;
    use crate::config::{AppConfig, Behavior, Limits};
    use crate::sandbox::{AllowedRoot, RootMode, Sandbox};
    use serde_json::json;
    use std::fs;

    fn setup() -> (std::path::PathBuf, Sandbox, AppConfig) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("ld-t-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        let canonical = fs::canonicalize(&dir).unwrap();
        let sandbox = Sandbox::new(
            vec![AllowedRoot { original: dir.clone(), canonical, mode: RootMode::ReadWrite }],
            Some(dir.clone()),
        );
        let config = AppConfig {
            sandbox: sandbox.clone(),
            limits: Limits::default(),
            behavior: Behavior::default(),
        };
        (dir, sandbox, config)
    }

    #[test]
    fn test_definition() {
        assert_eq!(definition().name, "list_directory");
    }

    #[test]
    fn test_execute_empty_directory() {
        let (dir, sandbox, config) = setup();
        let result =
            execute(&sandbox, &config, json!({"path": dir.display().to_string()})).unwrap();
        assert!(result["entries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_execute_not_a_directory() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("file.txt");
        fs::write(&file, "data").unwrap();
        let result = execute(&sandbox, &config, json!({"path": file.display().to_string()}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_code(), "not_a_directory");
    }

    #[test]
    fn test_execute_symlink_entry() {
        let (dir, sandbox, config) = setup();
        fs::write(dir.join("real.txt"), "real").unwrap();
        std::os::unix::fs::symlink(dir.join("real.txt"), dir.join("link.txt")).ok();
        let result =
            execute(&sandbox, &config, json!({"path": dir.display().to_string()})).unwrap();
        let entries = result["entries"].as_array().unwrap();
        assert!(entries.iter().any(|e| e["type"] == "file"));
    }
}
