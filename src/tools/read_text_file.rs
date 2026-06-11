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
