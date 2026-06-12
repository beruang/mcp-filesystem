use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct EditFileOutput {
    pub path: String,
    pub changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "edit_file".to_string(),
        description: "Apply pattern-based edits to a text file. Supports dryRun (default true), replaceAll, and returns unified diff. Rejects binary files.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the file to edit"},
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "oldText": {"type": "string", "description": "Text to find"},
                            "newText": {"type": "string", "description": "Replacement text"},
                            "replaceAll": {"type": "boolean", "description": "Replace all occurrences"}
                        },
                        "required": ["oldText", "newText"]
                    },
                    "description": "Array of edit operations"
                },
                "dryRun": {"type": "boolean", "description": "Preview changes without writing (default: true)"}
            }),
            vec!["path", "edits"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let requested = Path::new(path_str);

    let resolved = sandbox.resolve_existing_write(requested)?;

    if !resolved.canonical.is_file() {
        return Err(FsError::NotAFile { path: requested.to_path_buf() });
    }

    let metadata = std::fs::metadata(&resolved.canonical)?;
    let size = metadata.len();
    if size > config.limits.max_edit_bytes {
        return Err(FsError::FileTooLarge {
            path: requested.to_path_buf(),
            size,
            max: config.limits.max_edit_bytes,
        });
    }

    if !crate::path::is_likely_text_file(&resolved.canonical) {
        return Err(FsError::BinaryFileNotSupported { path: requested.to_path_buf() });
    }

    let dry_run = params["dryRun"].as_bool().unwrap_or(config.behavior.dry_run_edits_by_default);

    let edits = params["edits"].as_array().ok_or_else(|| FsError::InvalidPath {
        path: std::path::PathBuf::from("edits array required"),
    })?;

    let mut content = std::fs::read_to_string(&resolved.canonical)?;
    let original = content.clone();

    for edit in edits {
        let old_text = edit["oldText"].as_str().ok_or_else(|| FsError::InvalidPath {
            path: std::path::PathBuf::from("oldText required"),
        })?;
        let new_text = edit["newText"].as_str().ok_or_else(|| FsError::InvalidPath {
            path: std::path::PathBuf::from("newText required"),
        })?;
        let replace_all = edit["replaceAll"].as_bool().unwrap_or(false);

        let occurrences: Vec<_> = content.match_indices(old_text).collect();

        if occurrences.is_empty() {
            return Err(FsError::EditPatternNotFound {
                path: requested.to_path_buf(),
                pattern: old_text.to_string(),
            });
        }

        if occurrences.len() > 1 && !replace_all {
            return Err(FsError::EditPatternAmbiguous {
                path: requested.to_path_buf(),
                count: occurrences.len(),
            });
        }

        if replace_all {
            content = content.replace(old_text, new_text);
        } else {
            // Replace only one occurrence (first match)
            if let Some((idx, _)) = occurrences.first() {
                let mut new_content = String::with_capacity(content.len());
                new_content.push_str(&content[..*idx]);
                new_content.push_str(new_text);
                new_content.push_str(&content[idx + old_text.len()..]);
                content = new_content;
            }
        }
    }

    let changed = content != original;

    let diff = if changed { Some(generate_diff(&original, &content)) } else { None };

    if !dry_run && changed {
        std::fs::write(&resolved.canonical, &content)?;
    }

    let display_path = resolved.requested.display().to_string();
    serde_json::to_value(EditFileOutput { path: display_path, changed, diff })
        .map_err(|e| FsError::SerializationError { message: e.to_string() })
}

fn generate_diff(original: &str, modified: &str) -> String {
    use similar::TextDiff;
    let diff = TextDiff::from_lines(original, modified);
    diff.unified_diff().context_radius(3).to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    use super::*;
    use crate::config::{AppConfig, Behavior, Limits};
    use crate::sandbox::{AllowedRoot, RootMode, Sandbox};
    use serde_json::json;
    use std::fs;

    fn setup() -> (std::path::PathBuf, Sandbox, AppConfig) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("ef-{}-{}", std::process::id(), n));
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
        assert_eq!(definition().name, "edit_file");
    }

    #[test]
    fn test_execute_multiple_edits() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("multi.txt");
        fs::write(&file, "hello world\n").unwrap();
        let result = execute(
            &sandbox,
            &config,
            json!({
                "path": file.display().to_string(),
                "edits": [
                    {"oldText": "hello", "newText": "hi", "replaceAll": false},
                    {"oldText": "world", "newText": "there", "replaceAll": false}
                ],
                "dryRun": true
            }),
        )
        .unwrap();
        assert!(result["changed"].as_bool().unwrap());
        let diff = result["diff"].as_str().unwrap();
        assert!(diff.contains("hello") && diff.contains("hi"));
    }

    #[test]
    fn test_execute_no_changes() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("same.txt");
        fs::write(&file, "unchanged\n").unwrap();
        let result = execute(
            &sandbox,
            &config,
            json!({
                "path": file.display().to_string(),
                "edits": [{"oldText": "unchanged", "newText": "unchanged", "replaceAll": false}],
                "dryRun": true
            }),
        )
        .unwrap();
        assert!(!result["changed"].as_bool().unwrap());
    }

    #[test]
    fn test_execute_invalid_path() {
        let (_dir, sandbox, config) = setup();
        let result = execute(&sandbox, &config, json!({"path": "", "edits": []}));
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_no_edits() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("noedits.txt");
        fs::write(&file, "content\n").unwrap();
        let _result = execute(
            &sandbox,
            &config,
            json!({
                "path": file.display().to_string(),
                "edits": [{"oldText": "content", "newText": "updated", "replaceAll": false}],
                "dryRun": false
            }),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "updated\n");
    }

    #[test]
    fn test_execute_readonly_root() {
        let dir = std::env::temp_dir().join(format!("ef-ro-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ro.txt");
        fs::write(&file, "data").unwrap();
        let canonical = fs::canonicalize(&dir).unwrap();
        let sandbox = Sandbox::new(
            vec![AllowedRoot { original: dir, canonical, mode: RootMode::ReadOnly }],
            None,
        );
        let config =
            AppConfig { sandbox, limits: Limits::default(), behavior: Behavior::default() };
        let result = execute(
            &config.sandbox,
            &config,
            json!({
                "path": file.display().to_string(),
                "edits": [{"oldText": "data", "newText": "changed", "replaceAll": false}],
                "dryRun": true
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_not_a_file() {
        let (dir, sandbox, config) = setup();
        let result =
            execute(&sandbox, &config, json!({"path": dir.display().to_string(), "edits": []}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_code(), "not_a_file");
    }

    #[test]
    fn test_execute_large_file() {
        let (dir, _sandbox, config) = setup();
        let file = dir.join("large.txt");
        let content = "x".repeat(1000);
        fs::write(&file, &content).unwrap();
        let limits = Limits { max_edit_bytes: 10, ..Limits::default() };
        let config_small =
            AppConfig { sandbox: config.sandbox, limits, behavior: Behavior::default() };
        let result = execute(
            &config_small.sandbox,
            &config_small,
            json!({
                "path": file.display().to_string(),
                "edits": [{"oldText": "x", "newText": "y", "replaceAll": true}],
                "dryRun": true
            }),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_code(), "file_too_large");
    }
}
