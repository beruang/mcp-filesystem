use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct TreeNode {
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Self>>,
}

#[derive(Serialize)]
pub struct DirectoryTreeOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub children: Vec<TreeNode>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "directory_tree".to_string(),
        description: "Return a recursive directory tree. Respects maxDepth and maxDirectoryEntries. Does not follow symlinked directories by default.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the root directory"},
                "maxDepth": {"type": "integer", "description": "Maximum recursion depth"}
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

    #[allow(clippy::cast_possible_truncation)]
    let max_depth = params["maxDepth"]
        .as_u64()
        .map_or(config.limits.max_tree_depth, |n| n as usize)
        .min(config.limits.max_tree_depth);

    let mut entry_count = 0usize;

    #[allow(clippy::items_after_statements, clippy::option_if_let_else)]
    fn build_tree(
        path: &Path,
        depth: usize,
        max_depth: usize,
        max_entries: usize,
        entry_count: &mut usize,
    ) -> Result<Vec<TreeNode>, FsError> {
        let mut children = Vec::new();

        if depth >= max_depth {
            return Ok(children);
        }

        let dir_iter = std::fs::read_dir(path)?;
        for entry in dir_iter {
            if *entry_count >= max_entries {
                break;
            }

            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type()?;

            *entry_count += 1;

            let (node_type, child_nodes) = if file_type.is_dir() && !file_type.is_symlink() {
                match build_tree(&entry.path(), depth + 1, max_depth, max_entries, entry_count) {
                    Ok(c) => ("directory".to_string(), Some(c)),
                    Err(_) => ("directory".to_string(), Some(Vec::new())),
                }
            } else if file_type.is_dir() && file_type.is_symlink() {
                ("symlink".to_string(), None)
            } else if file_type.is_file() {
                ("file".to_string(), None)
            } else if file_type.is_symlink() {
                ("symlink".to_string(), None)
            } else {
                ("other".to_string(), None)
            };

            children.push(TreeNode { name, node_type, children: child_nodes });
        }

        Ok(children)
    }

    let root_name = resolved
        .canonical
        .file_name()
        .map_or_else(|| ".".to_string(), |n| n.to_string_lossy().to_string());

    let children = build_tree(
        &resolved.canonical,
        0,
        max_depth,
        config.limits.max_directory_entries,
        &mut entry_count,
    )?;

    serde_json::to_value(DirectoryTreeOutput {
        name: root_name,
        node_type: "directory".to_string(),
        children,
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
        let dir = std::env::temp_dir().join(format!("dt-{}", std::process::id()));
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
        assert_eq!(definition().name, "directory_tree");
    }

    #[test]
    fn test_execute_not_a_directory() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("test.txt");
        fs::write(&file, "data").unwrap();
        let result = execute(&sandbox, &config, json!({"path": file.display().to_string()}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_code(), "not_a_directory");
    }

    #[test]
    fn test_execute_with_max_depth() {
        let (dir, sandbox, config) = setup();
        let deep = dir.join("a/b/c/d");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("file.txt"), "x").unwrap();
        let result =
            execute(&sandbox, &config, json!({"path": dir.display().to_string(), "maxDepth": 1}))
                .unwrap();
        let children = result["children"].as_array().unwrap();
        // At depth 1, we should see "a" but not descend into it
        assert!(!children.is_empty());
    }
}
