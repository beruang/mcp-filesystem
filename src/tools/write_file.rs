use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct WriteFileOutput {
    pub path: String,
    #[serde(rename = "bytesWritten")]
    pub bytes_written: u64,
}

pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "write_file".to_string(),
        description: "Create or overwrite a file. Requires readWrite root. Use createParents to auto-create intermediate directories.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Destination file path"},
                "content": {"type": "string", "description": "File content to write"},
                "createParents": {"type": "boolean", "description": "Create parent directories if they don't exist"}
            }),
            vec!["path", "content"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, _config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath {
            path: std::path::PathBuf::from(""),
        })?;
    let content = params["content"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath {
            path: std::path::PathBuf::from(""),
        })?;
    let create_parents = params["createParents"].as_bool().unwrap_or(false);

    let requested = Path::new(path_str);

    let resolved = sandbox.resolve_create_write(requested)?;

    // Build destination: canonical_existing_parent + non-existent tail
    // The parent validation already confirmed the nearest existing parent is inside
    // a readWrite root. Build the full path from the absolute requested path.
    let absolute = crate::path::resolve_absolute(requested, sandbox.workspace_root())?;

    // Ensure the destination itself doesn't have .. traversal
    let normalized = crate::path::normalize_path(&absolute);
    if crate::path::contains_traversal(&absolute) {
        return Err(FsError::OutsideAllowedRoots {
            path: requested.to_path_buf(),
        });
    }

    // Verify destination is within the canonical parent's allowed root
    sandbox.assert_child_is_allowed(
        &resolved.canonical_existing_parent,
        crate::sandbox::AccessKind::Write,
    )?;

    if create_parents {
        if let Some(parent) = normalized.parent() {
            std::fs::create_dir_all(parent)?;
        }
    } else if let Some(parent) = normalized.parent() {
        if !parent.exists() {
            return Err(FsError::PathNotFound {
                path: parent.to_path_buf(),
            });
        }
    }

    std::fs::write(&normalized, content)?;

    let bytes_written = content.len() as u64;
    serde_json::to_value(WriteFileOutput {
        path: normalized.display().to_string(),
        bytes_written,
    })
    .map_err(|e| FsError::SerializationError {
        message: e.to_string(),
    })
}
