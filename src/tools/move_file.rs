use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct MoveFileOutput {
    pub source: String,
    pub destination: String,
    pub moved: bool,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "move_file".to_string(),
        description: "Move or rename a file or directory. Source must be writable; destination must be under a readWrite root.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "source": {"type": "string", "description": "Source path (must exist)"},
                "destination": {"type": "string", "description": "Destination path"}
            }),
            vec!["source", "destination"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, _config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let src_str = params["source"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let dst_str = params["destination"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;

    let source = Path::new(src_str);
    let destination = Path::new(dst_str);

    // Source must exist and be writable
    let src_resolved = sandbox.resolve_existing_write(source)?;

    // Destination must be creatable under a readWrite root
    let _dst_resolved = sandbox.resolve_create_write(destination)?;

    // Resolve destination absolute path
    let dst_absolute = crate::path::resolve_absolute(destination, sandbox.workspace_root())?;
    let dst_normalized = crate::path::normalize_path(&dst_absolute);

    if crate::path::contains_traversal(&dst_absolute) {
        return Err(FsError::OutsideAllowedRoots { path: destination.to_path_buf() });
    }

    std::fs::rename(&src_resolved.canonical, &dst_normalized)?;

    serde_json::to_value(MoveFileOutput {
        source: src_resolved.requested.display().to_string(),
        destination: dst_normalized.display().to_string(),
        moved: true,
    })
    .map_err(|e| FsError::SerializationError { message: e.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, Behavior, Limits};
    use crate::sandbox::{AllowedRoot, RootMode, Sandbox};
    use serde_json::json;
    use std::fs;

    fn setup() -> (std::path::PathBuf, Sandbox, AppConfig) {
        let dir = std::env::temp_dir().join(format!("mv-{}", std::process::id()));
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
        assert_eq!(definition().name, "move_file");
    }

    #[test]
    fn test_execute_source_not_found() {
        let (dir, sandbox, config) = setup();
        let result = execute(
            &sandbox,
            &config,
            json!({
                "source": dir.join("nope.txt").display().to_string(),
                "destination": dir.join("dest.txt").display().to_string(),
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_invalid_source() {
        let (_dir, sandbox, config) = setup();
        let result = execute(&sandbox, &config, json!({"source": "", "destination": "/tmp/x"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_invalid_destination() {
        let (dir, sandbox, config) = setup();
        let src = dir.join("src.txt");
        fs::write(&src, "x").unwrap();
        let result = execute(
            &sandbox,
            &config,
            json!({"source": src.display().to_string(), "destination": ""}),
        );
        assert!(result.is_err());
    }
}
