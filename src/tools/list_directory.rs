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
