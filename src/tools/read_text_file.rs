use crate::config::AppConfig;
use crate::error::FsError;
use crate::path;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct ReadTextFileOutput {
    pub content: String,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "read_text_file".to_string(),
        description: "Read a text file within an allowed root. Rejects binary files, directories, and paths outside allowed roots.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the text file to read"},
                "head": {"type": "integer", "description": "Return only the first N lines"},
                "tail": {"type": "integer", "description": "Return only the last N lines"}
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

    if !resolved.canonical.is_file() {
        return Err(FsError::NotAFile { path: requested.to_path_buf() });
    }

    let metadata = std::fs::metadata(&resolved.canonical)?;
    let size = metadata.len();
    if size > config.limits.max_read_bytes {
        return Err(FsError::FileTooLarge {
            path: requested.to_path_buf(),
            size,
            max: config.limits.max_read_bytes,
        });
    }

    if !path::is_likely_text_file(&resolved.canonical) {
        return Err(FsError::BinaryFileNotSupported { path: requested.to_path_buf() });
    }

    let content = std::fs::read_to_string(&resolved.canonical)?;

    // Handle head/tail
    #[allow(clippy::cast_possible_truncation)]
    let head = params["head"].as_u64().map(|n| n as usize);
    #[allow(clippy::cast_possible_truncation)]
    let tail = params["tail"].as_u64().map(|n| n as usize);

    let output_content = match (head, tail) {
        (Some(n), _) => content.lines().take(n).collect::<Vec<_>>().join("\n"),
        (_, Some(n)) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = if n >= lines.len() { 0 } else { lines.len() - n };
            lines[start..].join("\n")
        }
        (None, None) => content,
    };

    serde_json::to_value(ReadTextFileOutput { content: output_content })
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
        let dir = std::env::temp_dir().join(format!("rtf-{}", std::process::id()));
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
        assert_eq!(def.name, "read_text_file");
    }

    #[test]
    fn test_execute_tail() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("tail.txt");
        fs::write(&file, "line1\nline2\nline3\nline4\nline5\n").unwrap();
        let result =
            execute(&sandbox, &config, json!({"path": file.display().to_string(), "tail": 2}))
                .unwrap();
        let lines: Vec<&str> = result["content"].as_str().unwrap().lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line4");
    }

    #[test]
    fn test_execute_binary_rejected() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("data.bin");
        fs::write(&file, [0x00, 0x01, 0x02, 0x03]).unwrap();
        let result = execute(&sandbox, &config, json!({"path": file.display().to_string()}));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_code(), "binary_file_not_supported");
    }

    #[test]
    fn test_execute_not_a_file() {
        let (dir, sandbox, config) = setup();
        let result = execute(&sandbox, &config, json!({"path": dir.display().to_string()}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_code(), "not_a_file");
    }

    #[test]
    fn test_execute_invalid_path() {
        let (_dir, sandbox, config) = setup();
        let result = execute(&sandbox, &config, json!({"path": ""}));
        assert!(result.is_err());
    }
}
