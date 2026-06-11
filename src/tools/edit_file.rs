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
