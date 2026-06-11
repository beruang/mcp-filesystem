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
