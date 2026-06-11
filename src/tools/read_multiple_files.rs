use crate::config::AppConfig;
use crate::error::FsError;
use crate::path;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct FileReadResult {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct FileReadError {
    pub path: String,
    pub error: crate::error::ErrorResponse,
}

#[derive(Serialize)]
pub struct ReadMultipleFilesOutput {
    pub files: Vec<FileReadResult>,
    pub errors: Vec<FileReadError>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "read_multiple_files".to_string(),
        description: "Read several text files in one call. Returns per-file errors; one failed file does not fail the whole batch.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "paths": {"type": "array", "items": {"type": "string"}, "description": "Array of file paths to read"}
            }),
            vec!["paths"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let paths: Vec<String> = params["paths"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let mut files = Vec::new();
    let mut errors = Vec::new();

    for p in &paths {
        let requested = Path::new(p);
        match read_single(sandbox, config, requested) {
            Ok(content) => files.push(FileReadResult { path: p.clone(), content }),
            Err(e) => errors.push(FileReadError { path: p.clone(), error: e.to_error_response() }),
        }
    }

    serde_json::to_value(ReadMultipleFilesOutput { files, errors })
        .map_err(|e| FsError::SerializationError { message: e.to_string() })
}

fn read_single(sandbox: &Sandbox, config: &AppConfig, requested: &Path) -> Result<String, FsError> {
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

    Ok(std::fs::read_to_string(&resolved.canonical)?)
}
