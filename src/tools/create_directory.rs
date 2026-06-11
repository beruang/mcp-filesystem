use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct CreateDirectoryOutput {
    pub path: String,
    pub created: bool,
}

pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "create_directory".to_string(),
        description: "Create a directory recursively. Requires readWrite root. Validates the nearest existing parent before creation.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the directory to create"}
            }),
            vec!["path"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, _config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath {
            path: std::path::PathBuf::from(""),
        })?;
    let requested = Path::new(path_str);

    let _resolved = sandbox.resolve_create_write(requested)?;

    let absolute = crate::path::resolve_absolute(requested, sandbox.workspace_root())?;
    let normalized = crate::path::normalize_path(&absolute);

    if crate::path::contains_traversal(&absolute) {
        return Err(FsError::OutsideAllowedRoots {
            path: requested.to_path_buf(),
        });
    }

    std::fs::create_dir_all(&normalized)?;

    serde_json::to_value(CreateDirectoryOutput {
        path: normalized.display().to_string(),
        created: true,
    })
    .map_err(|e| FsError::SerializationError {
        message: e.to_string(),
    })
}
