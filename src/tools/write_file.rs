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

#[must_use]
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
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let content = params["content"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
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
        return Err(FsError::OutsideAllowedRoots { path: requested.to_path_buf() });
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
            return Err(FsError::PathNotFound { path: parent.to_path_buf() });
        }
    }

    std::fs::write(&normalized, content)?;

    let bytes_written = content.len() as u64;
    serde_json::to_value(WriteFileOutput { path: normalized.display().to_string(), bytes_written })
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
        let dir = std::env::temp_dir().join(format!("wf-{}", std::process::id()));
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
        assert_eq!(definition().name, "write_file");
    }

    #[test]
    fn test_execute_no_content() {
        let (dir, sandbox, config) = setup();
        let result =
            execute(&sandbox, &config, json!({"path": dir.join("x.txt").display().to_string()}));
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_overwrite() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("overwrite.txt");
        fs::write(&file, "old").unwrap();
        let result = execute(
            &sandbox,
            &config,
            json!({"path": file.display().to_string(), "content": "new"}),
        )
        .unwrap();
        assert_eq!(result["bytesWritten"], 3);
        assert_eq!(fs::read_to_string(&file).unwrap(), "new");
    }

    #[test]
    fn test_execute_no_create_parents_missing() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("missing_parent/file.txt");
        let result = execute(
            &sandbox,
            &config,
            json!({"path": file.display().to_string(), "content": "x", "createParents": false}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_outside_root() {
        let (_dir, sandbox, config) = setup();
        let outside = std::env::temp_dir().join(format!("outside-wf-{}", std::process::id()));
        let result = execute(
            &sandbox,
            &config,
            json!({"path": outside.join("x.txt").display().to_string(), "content": "x"}),
        );
        assert!(result.is_err());
    }
}
