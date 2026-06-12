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

#[must_use]
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
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let requested = Path::new(path_str);

    let _resolved = sandbox.resolve_create_write(requested)?;

    let absolute = crate::path::resolve_absolute(requested, sandbox.workspace_root())?;
    let normalized = crate::path::normalize_path(&absolute);

    if crate::path::contains_traversal(&absolute) {
        return Err(FsError::OutsideAllowedRoots { path: requested.to_path_buf() });
    }

    std::fs::create_dir_all(&normalized)?;

    serde_json::to_value(CreateDirectoryOutput {
        path: normalized.display().to_string(),
        created: true,
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
        let dir = std::env::temp_dir().join(format!("cd-{}", std::process::id()));
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
        assert_eq!(definition().name, "create_directory");
    }

    #[test]
    fn test_execute_new_subdirectory() {
        let (dir, sandbox, config) = setup();
        let new_dir = dir.join("new_subdir");
        let result =
            execute(&sandbox, &config, json!({"path": new_dir.display().to_string()})).unwrap();
        assert_eq!(result["created"], true);
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_execute_outside_root() {
        let (_dir, sandbox, config) = setup();
        let outside = std::env::temp_dir().join(format!("outside-cd-{}", std::process::id()));
        let result = execute(&sandbox, &config, json!({"path": outside.display().to_string()}));
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_invalid_path() {
        let (_dir, sandbox, config) = setup();
        let result = execute(&sandbox, &config, json!({"path": ""}));
        assert!(result.is_err());
    }
}
