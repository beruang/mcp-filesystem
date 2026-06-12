#![allow(clippy::too_many_lines, clippy::missing_errors_doc, clippy::missing_panics_doc)]
use crate::error::FsError;
use crate::path::normalize_path;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMode {
    ReadOnly,
    ReadWrite,
}

impl RootMode {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ro" | "readonly" | "read_only" | "read-only" => Some(Self::ReadOnly),
            "rw" | "readwrite" | "read_write" | "read-write" => Some(Self::ReadWrite),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Debug, Clone)]
pub struct AllowedRoot {
    pub original: PathBuf,
    pub canonical: PathBuf,
    pub mode: RootMode,
}

impl AllowedRoot {
    #[must_use]
    pub fn depth(&self) -> usize {
        self.canonical.components().count()
    }

    pub fn ensure_access(&self, required: AccessKind) -> Result<(), FsError> {
        match (self.mode, required) {
            (RootMode::ReadOnly, AccessKind::Write) => {
                Err(FsError::ReadOnlyRoot { path: self.original.clone() })
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPath {
    pub requested: PathBuf,
    pub canonical: PathBuf,
    #[allow(dead_code)]
    pub root: AllowedRoot,
}

#[derive(Debug, Clone)]
pub struct ResolvedCreatePath {
    #[allow(dead_code)]
    pub requested: PathBuf,
    pub canonical_existing_parent: PathBuf,
    #[allow(dead_code)]
    pub root: AllowedRoot,
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    roots: Vec<AllowedRoot>,
    workspace_root: Option<PathBuf>,
}

impl Sandbox {
    #[must_use]
    pub const fn new(roots: Vec<AllowedRoot>, workspace_root: Option<PathBuf>) -> Self {
        Self { roots, workspace_root }
    }

    #[must_use]
    pub const fn has_single_root(&self) -> bool {
        self.roots.len() == 1
    }

    #[must_use]
    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    #[must_use]
    pub fn list_allowed_directories(&self) -> Vec<String> {
        self.roots.iter().map(|r| r.original.display().to_string()).collect()
    }

    pub fn resolve_existing_read(&self, requested: &Path) -> Result<ResolvedPath, FsError> {
        let absolute = crate::path::resolve_absolute(requested, self.workspace_root())?;
        let canonical = std::fs::canonicalize(&absolute).map_err(|e| FsError::IoError {
            message: format!("cannot resolve path '{}': {e}", absolute.display()),
        })?;

        let root = self
            .find_best_matching_root(&canonical)
            .ok_or_else(|| FsError::OutsideAllowedRoots { path: requested.to_path_buf() })?;

        root.ensure_access(AccessKind::Read)?;

        Ok(ResolvedPath { requested: requested.to_path_buf(), canonical, root })
    }

    pub fn resolve_existing_write(&self, requested: &Path) -> Result<ResolvedPath, FsError> {
        let absolute = crate::path::resolve_absolute(requested, self.workspace_root())?;
        let canonical = std::fs::canonicalize(&absolute).map_err(|e| FsError::IoError {
            message: format!("cannot resolve path '{}': {e}", absolute.display()),
        })?;

        let root = self
            .find_best_matching_root(&canonical)
            .ok_or_else(|| FsError::OutsideAllowedRoots { path: requested.to_path_buf() })?;

        root.ensure_access(AccessKind::Write)?;

        Ok(ResolvedPath { requested: requested.to_path_buf(), canonical, root })
    }

    #[allow(clippy::too_many_lines)]
    pub fn resolve_create_write(&self, requested: &Path) -> Result<ResolvedCreatePath, FsError> {
        let absolute = crate::path::resolve_absolute(requested, self.workspace_root())?;

        if absolute.as_os_str().is_empty() {
            return Err(FsError::InvalidPath { path: requested.to_path_buf() });
        }

        // Get the parent directory
        let parent = absolute
            .parent()
            .ok_or_else(|| FsError::InvalidPath { path: requested.to_path_buf() })?;

        // Find nearest existing parent
        let existing_parent =
            crate::path::nearest_existing_parent(parent).map_err(|e| FsError::IoError {
                message: format!("cannot find existing parent for '{}': {e}", requested.display()),
            })?;

        // Canonicalize the existing parent (resolves symlinks)
        let canonical_parent =
            std::fs::canonicalize(&existing_parent).map_err(|e| FsError::IoError {
                message: format!("cannot resolve parent path '{}': {e}", existing_parent.display()),
            })?;

        let root = self
            .find_best_matching_root(&canonical_parent)
            .ok_or_else(|| FsError::OutsideAllowedRoots { path: requested.to_path_buf() })?;

        root.ensure_access(AccessKind::Write)?;

        // Build the destination path: canonical_parent + non-existent tail
        let normalized = normalize_path(&absolute);
        let norm_components: Vec<_> = normalized.components().collect();
        let existing_count = existing_parent.components().count();

        // If normalized path is shorter than the existing parent, the path
        // traversed above the parent (via ..), which means escape attempt.
        if norm_components.len() < existing_count {
            return Err(FsError::OutsideAllowedRoots { path: requested.to_path_buf() });
        }

        // The canonical parent may differ from the existing parent (symlinks).
        // We build: canonical_parent + components beyond existing_parent's depth
        let tail: Vec<_> = norm_components[existing_count.min(norm_components.len())..].to_vec();

        let mut destination = canonical_parent.clone();
        for component in &tail {
            match component {
                std::path::Component::ParentDir => {
                    return Err(FsError::OutsideAllowedRoots { path: requested.to_path_buf() });
                }
                std::path::Component::Normal(os_str) => {
                    destination.push(os_str);
                }
                c => {
                    destination.push(c);
                }
            }
        }

        Ok(ResolvedCreatePath {
            requested: requested.to_path_buf(),
            canonical_existing_parent: canonical_parent,
            root,
        })
    }

    pub fn assert_child_is_allowed(
        &self,
        canonical_path: &Path,
        access: AccessKind,
    ) -> Result<AllowedRoot, FsError> {
        let root = self
            .find_best_matching_root(canonical_path)
            .ok_or_else(|| FsError::OutsideAllowedRoots { path: canonical_path.to_path_buf() })?;
        root.ensure_access(access)?;
        Ok(root)
    }

    fn find_best_matching_root(&self, canonical_path: &Path) -> Option<AllowedRoot> {
        let mut matching: Vec<&AllowedRoot> = self
            .roots
            .iter()
            .filter(|root| {
                canonical_path == root.canonical || canonical_path.starts_with(&root.canonical)
            })
            .collect();

        // Sort by depth descending — most specific first
        matching.sort_by_key(|b| std::cmp::Reverse(b.depth()));

        matching.first().copied().cloned()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    use super::*;
    use std::fs;

    fn temp_root() -> (PathBuf, AllowedRoot) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("sb-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        let canonical = fs::canonicalize(&dir).unwrap();
        let root = AllowedRoot { original: dir.clone(), canonical, mode: RootMode::ReadWrite };
        (dir, root)
    }

    #[test]
    fn test_root_mode_ensure_access_read_ok() {
        let root = AllowedRoot {
            original: PathBuf::from("/x"),
            canonical: PathBuf::from("/x"),
            mode: RootMode::ReadOnly,
        };
        assert!(root.ensure_access(AccessKind::Read).is_ok());
        let root = AllowedRoot {
            original: PathBuf::from("/x"),
            canonical: PathBuf::from("/x"),
            mode: RootMode::ReadWrite,
        };
        assert!(root.ensure_access(AccessKind::Read).is_ok());
    }

    #[test]
    fn test_root_mode_ensure_access_write_rejected() {
        let root = AllowedRoot {
            original: PathBuf::from("/x"),
            canonical: PathBuf::from("/x"),
            mode: RootMode::ReadOnly,
        };
        assert!(root.ensure_access(AccessKind::Write).is_err());
    }

    #[test]
    fn test_root_mode_ensure_access_write_ok() {
        let root = AllowedRoot {
            original: PathBuf::from("/x"),
            canonical: PathBuf::from("/x"),
            mode: RootMode::ReadWrite,
        };
        assert!(root.ensure_access(AccessKind::Write).is_ok());
    }

    #[test]
    fn test_root_depth() {
        let root = AllowedRoot {
            original: PathBuf::from("/x"),
            canonical: PathBuf::from("/a/b/c"),
            mode: RootMode::ReadWrite,
        };
        assert_eq!(root.depth(), 4);
    }

    #[test]
    fn test_sandbox_new_empty() {
        let sb = Sandbox::new(vec![], None);
        assert!(sb.workspace_root().is_none());
        assert!(!sb.has_single_root());
        assert!(sb.list_allowed_directories().is_empty());
    }

    #[test]
    fn test_sandbox_new_single_root() {
        let (dir, root) = temp_root();
        let sb = Sandbox::new(vec![root], Some(dir));
        assert!(sb.has_single_root());
        assert!(sb.workspace_root().is_some());
        assert_eq!(sb.list_allowed_directories().len(), 1);
    }

    #[test]
    fn test_find_best_matching_root_exact() {
        let (dir, root) = temp_root();
        let root_canonical = root.canonical.clone();
        let sb = Sandbox::new(vec![root], Some(dir));
        let found = sb.find_best_matching_root(&root_canonical);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_best_matching_root_child() {
        let (dir, root) = temp_root();
        let child = dir.join("child");
        fs::create_dir_all(&child).unwrap();
        let child_canonical = fs::canonicalize(&child).unwrap();
        let sb = Sandbox::new(vec![root], Some(dir));
        let found = sb.find_best_matching_root(&child_canonical);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_best_matching_root_outside() {
        let (dir, root) = temp_root();
        let sb = Sandbox::new(vec![root], Some(dir));
        let found = sb.find_best_matching_root(Path::new("/etc"));
        assert!(found.is_none());
    }

    #[test]
    fn test_find_best_matching_root_most_specific() {
        let (broad_dir, broad_root) = temp_root();
        let narrow_dir = broad_dir.join("narrow");
        fs::create_dir_all(&narrow_dir).unwrap();
        let narrow_canonical = fs::canonicalize(&narrow_dir).unwrap();
        let narrow_root = AllowedRoot {
            original: narrow_dir.clone(),
            canonical: narrow_canonical,
            mode: RootMode::ReadWrite,
        };

        let sb = Sandbox::new(
            vec![broad_root, AllowedRoot { mode: RootMode::ReadOnly, ..narrow_root }],
            None,
        );

        let file = narrow_dir.join("file.txt");
        fs::write(&file, "data").unwrap();
        let found = sb.find_best_matching_root(&fs::canonicalize(&file).unwrap()).unwrap();
        // Most specific = narrow
        assert_eq!(found.mode, RootMode::ReadOnly);
    }

    #[test]
    fn test_assert_child_is_allowed_ok() {
        let (_dir, root) = temp_root();
        let root_canonical = root.canonical.clone();
        let sb = Sandbox::new(vec![root], None);
        let result = sb.assert_child_is_allowed(&root_canonical, AccessKind::Read);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_child_is_allowed_outside() {
        let (_dir, root) = temp_root();
        let sb = Sandbox::new(vec![root], None);
        let result = sb.assert_child_is_allowed(Path::new("/etc"), AccessKind::Read);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_create_write_empty_path() {
        let (dir, root) = temp_root();
        let sb = Sandbox::new(vec![root], Some(dir));
        let result = sb.resolve_create_write(Path::new(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_create_write_no_parent() {
        let (_dir, root) = temp_root();
        // A path with no parent (just "/") would be the root itself
        let sb = Sandbox::new(vec![root], None);
        let result = sb.resolve_create_write(Path::new("/"));
        assert!(result.is_err()); // root "/" has no parent
    }

    #[test]
    fn test_resolve_existing_read_not_found() {
        let (dir, root) = temp_root();
        let sb = Sandbox::new(vec![root], Some(dir.clone()));
        let nonexistent = dir.join("nonexistent.txt");
        let result = sb.resolve_existing_read(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_root_multi_root() {
        let (_dir1, root1) = temp_root();
        let dir2 = std::env::temp_dir().join(format!("sb-test-multi-{}", std::process::id()));
        fs::create_dir_all(&dir2).unwrap();
        let canonical2 = fs::canonicalize(&dir2).unwrap();
        let root2 = AllowedRoot { original: dir2, canonical: canonical2, mode: RootMode::ReadOnly };
        let sb = Sandbox::new(vec![root1, root2], None);
        assert!(sb.workspace_root().is_none());
    }
}
