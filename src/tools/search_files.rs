use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct SearchFilesOutput {
    pub matches: Vec<String>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "search_files".to_string(),
        description: "Search for files by glob pattern under an allowed root. Supports exclude patterns and respects maxSearchResults. Does not traverse symlinked directories by default.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Root directory to search under"},
                "pattern": {"type": "string", "description": "Glob pattern to match (e.g., *.rs, src/**)"},
                "excludePatterns": {"type": "array", "items": {"type": "string"}, "description": "Glob patterns to exclude"}
            }),
            vec!["path", "pattern"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let pattern = params["pattern"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let requested = Path::new(path_str);

    let resolved = sandbox.resolve_existing_read(requested)?;

    if !resolved.canonical.is_dir() {
        return Err(FsError::NotADirectory { path: requested.to_path_buf() });
    }

    let exclude_patterns: Vec<String> = params["excludePatterns"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let mut builder = ignore::WalkBuilder::new(&resolved.canonical);
    builder.follow_links(false).hidden(true).standard_filters(false);

    // Build exclude glob patterns
    let exclude_globs: Vec<glob::Pattern> =
        exclude_patterns.iter().filter_map(|p| glob::Pattern::new(p).ok()).collect();

    builder.filter_entry(move |entry| {
        let path_str = entry.path().to_string_lossy();
        !exclude_globs.iter().any(|pat| pat.matches(&path_str))
    });

    let mut matches: Vec<String> = Vec::new();
    let glob_pattern = glob::Pattern::new(pattern)
        .map_err(|e| FsError::InvalidPath { path: std::path::PathBuf::from(e.to_string()) })?;

    for result in builder.build() {
        if matches.len() >= config.limits.max_search_results {
            return Err(FsError::TooManyResults {
                count: matches.len(),
                max: config.limits.max_search_results,
            });
        }

        let entry = result.map_err(|e| FsError::IoError { message: e.to_string() })?;

        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let path = entry.path();
            if glob_pattern.matches(&path.to_string_lossy()) {
                matches.push(path.display().to_string());
            }
        }
    }

    serde_json::to_value(SearchFilesOutput { matches })
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
        let dir = std::env::temp_dir().join(format!("sf-{}", std::process::id()));
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
        assert_eq!(definition().name, "search_files");
    }

    #[test]
    fn test_execute_not_a_directory() {
        let (dir, sandbox, config) = setup();
        let file = dir.join("test.rs");
        fs::write(&file, "// test").unwrap();
        let result = execute(
            &sandbox,
            &config,
            json!({"path": file.display().to_string(), "pattern": "*.rs"}),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_code(), "not_a_directory");
    }

    #[test]
    fn test_execute_with_excludes() {
        let (dir, sandbox, config) = setup();
        fs::write(dir.join("a.rs"), "// a").unwrap();
        fs::create_dir_all(dir.join("target")).unwrap();
        fs::write(dir.join("target/b.rs"), "// b").unwrap();
        let result = execute(
            &sandbox,
            &config,
            json!({
                "path": dir.display().to_string(),
                "pattern": "*.rs",
                "excludePatterns": ["**/*.txt"]
            }),
        )
        .unwrap();
        let matches = result["matches"].as_array().unwrap();
        assert!(!matches.is_empty(), "got {matches:?}");
    }

    #[test]
    fn test_execute_no_pattern() {
        let (dir, sandbox, config) = setup();
        let result = execute(&sandbox, &config, json!({"path": dir.display().to_string()}));
        assert!(result.is_err());
    }
}
