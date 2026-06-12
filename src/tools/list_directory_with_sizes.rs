use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct DirEntryWithSize {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Serialize)]
pub struct ListDirectoryWithSizesOutput {
    pub entries: Vec<DirEntryWithSize>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "list_directory_with_sizes".to_string(),
        description:
            "List direct children of a directory including file sizes. Directory sizes are null."
                .to_string(),
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

    let mut entries: Vec<DirEntryWithSize> = Vec::new();
    let dir_iter = std::fs::read_dir(&resolved.canonical)?;

    for entry in dir_iter.take(config.limits.max_directory_entries) {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().display().to_string();
        let file_type = entry.file_type()?;
        let metadata = entry.metadata().ok();

        let entry_type = if file_type.is_dir() {
            "directory"
        } else if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };

        let size = if file_type.is_file() { metadata.map(|m| m.len()) } else { None };

        entries.push(DirEntryWithSize { name, path, entry_type: entry_type.to_string(), size });
    }

    serde_json::to_value(ListDirectoryWithSizesOutput { entries })
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
        let dir = std::env::temp_dir().join(format!("ldws-{}-{}", std::process::id(), n));
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
        let def = definition();
        assert_eq!(def.name, "list_directory_with_sizes");
    }

    #[test]
    fn test_execute_lists_with_sizes() {
        let (dir, sandbox, config) = setup();
        fs::write(dir.join("file.txt"), "hello world").unwrap();
        fs::create_dir_all(dir.join("subdir")).unwrap();

        let result =
            execute(&sandbox, &config, json!({"path": dir.display().to_string()})).unwrap();
        let entries = result["entries"].as_array().unwrap();
        assert!(entries.len() >= 2);

        let file_entry = entries.iter().find(|e| e["name"] == "file.txt").unwrap();
        assert_eq!(file_entry["type"], "file");
        assert_eq!(file_entry["size"], 11);

        let dir_entry = entries.iter().find(|e| e["name"] == "subdir").unwrap();
        assert_eq!(dir_entry["type"], "directory");
        assert!(dir_entry["size"].is_null());
    }

    #[test]
    fn test_execute_not_a_directory() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("file.txt");
        fs::write(&file, "data").unwrap();
        let result = execute(&sandbox, &config, json!({"path": file.display().to_string()}));
        assert!(result.is_err());
    }
}
