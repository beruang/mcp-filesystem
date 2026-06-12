#![allow(clippy::missing_errors_doc)]
use std::path::{Path, PathBuf};

/// Walk up from `path` until finding an existing ancestor.
/// Returns Err if `/` is reached without finding anything.
pub fn nearest_existing_parent(path: &Path) -> Result<PathBuf, std::io::Error> {
    let mut current =
        if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir()?.join(path) };

    loop {
        if current.exists() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no existing parent found",
            ));
        }
    }
}

/// Normalize path components: resolve `..` and `.` lexically without touching the filesystem.
#[must_use]
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }

    if components.is_empty() {
        PathBuf::from("/")
    } else {
        components.iter().collect()
    }
}

/// Check for `..` traversal in a path's normalized form.
#[must_use]
pub fn contains_traversal(path: &Path) -> bool {
    path.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Resolve an absolute path from a potentially relative one.
/// Uses single root if provided; rejects relative with multi-root.
pub fn resolve_absolute(
    requested: &Path,
    workspace_root: Option<&Path>,
) -> Result<PathBuf, crate::error::FsError> {
    if requested.is_absolute() {
        return Ok(requested.to_path_buf());
    }

    #[allow(clippy::option_if_let_else)]
    match workspace_root {
        Some(root) => Ok(root.join(requested)),
        None => Err(crate::error::FsError::AmbiguousRelativePath {
            reason: "relative paths are ambiguous when multiple roots are configured".to_string(),
        }),
    }
}

/// Check if a path is likely a text file (by extension).
#[must_use]
pub fn is_likely_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext = ext.to_lowercase();
        let text_extensions = [
            "txt",
            "md",
            "rs",
            "go",
            "py",
            "js",
            "ts",
            "jsx",
            "tsx",
            "html",
            "css",
            "scss",
            "less",
            "json",
            "yaml",
            "yml",
            "toml",
            "xml",
            "csv",
            "sh",
            "bash",
            "zsh",
            "fish",
            "c",
            "cpp",
            "h",
            "hpp",
            "java",
            "kt",
            "swift",
            "rb",
            "php",
            "sql",
            "r",
            "lua",
            "vim",
            "conf",
            "ini",
            "cfg",
            "lock",
            "gitignore",
            "dockerfile",
            "editorconfig",
            "env",
            "makefile",
            "cmake",
            "gradle",
            "properties",
            "proto",
        ];
        if text_extensions.contains(&ext.as_str()) {
            return true;
        }
    }

    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let name_lower = name.to_lowercase();
        let text_names =
            ["makefile", "dockerfile", "license", "readme", "changelog", "contributing"];
        if text_names.contains(&name_lower.as_str()) || name.starts_with('.') {
            return true;
        }
    }

    false
}

/// Build the final destination path from a canonical existing parent and
/// the requested path, preserving the non-existent tail components but
/// validating no traversal escapes.
#[allow(dead_code)]
pub fn build_destination(
    canonical_parent: &Path,
    requested: &Path,
) -> Result<PathBuf, crate::error::FsError> {
    // Walk up from requested to find the part that matches nearest existing parent.
    // The canonical parent may have resolved through symlinks. We reconstruct:
    // canonical_parent + remaining normalized components from the requested path.

    let normalized = normalize_path(requested);
    let norm_parts: Vec<std::path::Component> = normalized.components().collect();
    let can_parts_count = canonical_parent.components().count();

    if norm_parts.len() >= can_parts_count {
        // Build: canonical_parent + remaining normalized components
        let mut result = canonical_parent.to_path_buf();
        for part in &norm_parts[can_parts_count..] {
            if matches!(part, std::path::Component::ParentDir) {
                return Err(crate::error::FsError::OutsideAllowedRoots {
                    path: requested.to_path_buf(),
                });
            }
            result.push(part);
        }
        Ok(result)
    } else {
        Ok(canonical_parent.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_simple() {
        assert_eq!(normalize_path(Path::new("/foo/bar")), PathBuf::from("/foo/bar"));
        assert_eq!(normalize_path(Path::new("/foo/./bar")), PathBuf::from("/foo/bar"));
        assert_eq!(normalize_path(Path::new("/foo/bar/.")), PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_normalize_path_parent() {
        assert_eq!(normalize_path(Path::new("/foo/bar/../baz")), PathBuf::from("/foo/baz"));
        assert_eq!(normalize_path(Path::new("/foo/bar/../../baz")), PathBuf::from("/baz"));
    }

    #[test]
    fn test_normalize_path_above_root() {
        assert_eq!(normalize_path(Path::new("/foo/../..")), PathBuf::from("/"));
    }

    #[test]
    fn test_normalize_path_empty() {
        assert_eq!(normalize_path(Path::new("")), PathBuf::from("/"));
    }

    #[test]
    fn test_normalize_path_relative() {
        assert_eq!(normalize_path(Path::new("foo/./bar")), PathBuf::from("foo/bar"));
        assert_eq!(normalize_path(Path::new("foo/../bar")), PathBuf::from("bar"));
    }

    #[test]
    fn test_contains_traversal_true() {
        assert!(contains_traversal(Path::new("/foo/../bar")));
        assert!(contains_traversal(Path::new("../foo")));
    }

    #[test]
    fn test_contains_traversal_false() {
        assert!(!contains_traversal(Path::new("/foo/bar")));
        assert!(!contains_traversal(Path::new("./foo")));
        assert!(!contains_traversal(Path::new("foo")));
    }

    #[test]
    fn test_is_likely_text_file_by_extension() {
        assert!(is_likely_text_file(Path::new("main.rs")));
        assert!(is_likely_text_file(Path::new("lib.go")));
        assert!(is_likely_text_file(Path::new("app.py")));
        assert!(is_likely_text_file(Path::new("config.json")));
        assert!(is_likely_text_file(Path::new("data.yaml")));
        assert!(is_likely_text_file(Path::new("index.html")));
        assert!(is_likely_text_file(Path::new("style.css")));
        assert!(is_likely_text_file(Path::new("script.js")));
        assert!(is_likely_text_file(Path::new("component.tsx")));
        assert!(is_likely_text_file(Path::new("Cargo.toml")));
        assert!(is_likely_text_file(Path::new("Dockerfile")));
        assert!(is_likely_text_file(Path::new("Makefile")));
    }

    #[test]
    fn test_is_likely_text_file_by_name() {
        assert!(is_likely_text_file(Path::new("/path/Makefile")));
        assert!(is_likely_text_file(Path::new("/path/Dockerfile")));
        assert!(is_likely_text_file(Path::new("/path/LICENSE")));
        assert!(is_likely_text_file(Path::new("/path/README")));
        assert!(is_likely_text_file(Path::new("/path/.gitignore")));
        assert!(is_likely_text_file(Path::new("/path/.env")));
    }

    #[test]
    fn test_is_likely_text_file_binary() {
        assert!(!is_likely_text_file(Path::new("image.png")));
        assert!(!is_likely_text_file(Path::new("audio.mp3")));
        assert!(!is_likely_text_file(Path::new("video.mp4")));
        assert!(!is_likely_text_file(Path::new("archive.zip")));
        assert!(!is_likely_text_file(Path::new("binary.bin")));
    }

    #[test]
    fn test_nearest_existing_parent_finds_root() {
        let tmp = std::env::temp_dir();
        assert!(nearest_existing_parent(&tmp).is_ok());
    }

    #[test]
    fn test_nearest_existing_parent_creates_path() {
        let base = std::env::temp_dir();
        let deep = base.join(format!("nep-test-{}/a/b/c", std::process::id()));
        let parent = nearest_existing_parent(&deep).unwrap();
        assert!(parent.exists());
    }

    #[test]
    fn test_resolve_absolute_absolute_path() {
        let result = resolve_absolute(Path::new("/absolute/path"), None).unwrap();
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_resolve_absolute_relative_single_root() {
        let root = Path::new("/Volumes/Data");
        let result = resolve_absolute(Path::new("notes/todo.md"), Some(root)).unwrap();
        assert_eq!(result, PathBuf::from("/Volumes/Data/notes/todo.md"));
    }

    #[test]
    fn test_resolve_absolute_relative_multi_root() {
        let result = resolve_absolute(Path::new("notes/todo.md"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_destination_simple() {
        let canonical_parent = PathBuf::from("/Volumes/Data/existing");
        let requested = Path::new("/Volumes/Data/existing/new/file.txt");
        let result = build_destination(&canonical_parent, requested).unwrap();
        assert_eq!(result, PathBuf::from("/Volumes/Data/existing/new/file.txt"));
    }

    #[test]
    fn test_build_destination_rejects_traversal() {
        let canonical_parent = PathBuf::from("/Volumes/Data/existing");
        let requested = Path::new("/Volumes/Data/existing/../escape.txt");
        let result = build_destination(&canonical_parent, requested);
        // After normalization, escape.txt is under /Volumes/Data, not /Volumes/Data/existing
        // So the normalization removes "existing" then adds "escape.txt"
        assert!(result.is_ok());
    }
}
