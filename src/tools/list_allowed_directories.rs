use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct ListAllowedDirectoriesOutput {
    pub directories: Vec<String>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "list_allowed_directories".to_string(),
        description: "Return the configured allowed root directories. Does not recursively enumerate descendants.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({}),
            vec![],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, _config: &AppConfig, _params: Value) -> Result<Value, FsError> {
    let dirs = sandbox.list_allowed_directories();
    let output = ListAllowedDirectoriesOutput { directories: dirs };
    serde_json::to_value(output).map_err(|e| FsError::SerializationError { message: e.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, Behavior, Limits};
    use crate::sandbox::{AllowedRoot, Sandbox};
    use serde_json::json;

    #[test]
    fn test_definition_has_name() {
        let def = definition();
        assert_eq!(def.name, "list_allowed_directories");
        assert!(!def.description.is_empty());
    }

    #[test]
    fn test_execute_returns_directories() {
        let dir = std::env::temp_dir();
        let canonical = std::fs::canonicalize(&dir).unwrap();
        let sandbox = Sandbox::new(
            vec![AllowedRoot {
                original: dir.clone(),
                canonical,
                mode: crate::sandbox::RootMode::ReadWrite,
            }],
            Some(dir),
        );
        let config = AppConfig {
            sandbox: sandbox.clone(),
            limits: Limits::default(),
            behavior: Behavior::default(),
        };
        let result = execute(&sandbox, &config, json!({})).unwrap();
        let dirs: Vec<String> = serde_json::from_value(result["directories"].clone()).unwrap();
        assert_eq!(dirs.len(), 1);
    }

    #[test]
    fn test_execute_multiple_roots() {
        let dir1 = std::env::temp_dir().join(format!("ld1-{}", std::process::id()));
        let dir2 = std::env::temp_dir().join(format!("ld2-{}", std::process::id()));
        std::fs::create_dir_all(&dir1).unwrap();
        std::fs::create_dir_all(&dir2).unwrap();
        let c1 = std::fs::canonicalize(&dir1).unwrap();
        let c2 = std::fs::canonicalize(&dir2).unwrap();
        let sandbox = Sandbox::new(
            vec![
                AllowedRoot {
                    original: dir1,
                    canonical: c1,
                    mode: crate::sandbox::RootMode::ReadWrite,
                },
                AllowedRoot {
                    original: dir2,
                    canonical: c2,
                    mode: crate::sandbox::RootMode::ReadOnly,
                },
            ],
            None,
        );
        let config = AppConfig {
            sandbox: sandbox.clone(),
            limits: Limits::default(),
            behavior: Behavior::default(),
        };
        let result = execute(&sandbox, &config, json!({})).unwrap();
        let directories: Vec<String> =
            serde_json::from_value(result["directories"].clone()).unwrap();
        assert_eq!(directories.len(), 2);
    }
}
