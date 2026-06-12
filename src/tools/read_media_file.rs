use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct ReadMediaFileOutput {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "read_media_file".to_string(),
        description: "Read a media/binary file (image, audio, etc.). Returns base64-encoded data with inferred MIME type.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the media file"}
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

    let bytes = std::fs::read(&resolved.canonical)?;

    let mime_type = mime_guess::from_path(&resolved.canonical).first_or_octet_stream().to_string();

    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);

    serde_json::to_value(ReadMediaFileOutput { mime_type, data })
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
        let dir = std::env::temp_dir().join(format!("rmf-{}", std::process::id()));
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
        assert_eq!(definition().name, "read_media_file");
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

    #[test]
    fn test_execute_jpeg_mime() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("photo.jpg");
        fs::write(&file, [0xFF, 0xD8, 0xFF, 0xE0]).unwrap();
        let result =
            execute(&sandbox, &config, json!({"path": file.display().to_string()})).unwrap();
        assert_eq!(result["mimeType"], "image/jpeg");
        assert!(!result["data"].as_str().unwrap().is_empty());
    }
}
