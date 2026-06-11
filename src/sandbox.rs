use crate::error::FsError;
use crate::path::normalize_path;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMode {
    ReadOnly,
    ReadWrite,
}

impl RootMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ro" | "readonly" | "read_only" | "read-only" => Some(RootMode::ReadOnly),
            "rw" | "readwrite" | "read_write" | "read-write" => Some(RootMode::ReadWrite),
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
    pub fn depth(&self) -> usize {
        self.canonical.components().count()
    }

    pub fn ensure_access(&self, required: AccessKind) -> Result<(), FsError> {
        match (self.mode, required) {
            (RootMode::ReadOnly, AccessKind::Write) => Err(FsError::ReadOnlyRoot {
                path: self.original.clone(),
            }),
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
    pub fn new(roots: Vec<AllowedRoot>, workspace_root: Option<PathBuf>) -> Self {
        Sandbox {
            roots,
            workspace_root,
        }
    }

    pub fn has_single_root(&self) -> bool {
        self.roots.len() == 1
    }

    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    pub fn list_allowed_directories(&self) -> Vec<String> {
        self.roots
            .iter()
            .map(|r| r.original.display().to_string())
            .collect()
    }

    pub fn resolve_existing_read(&self, requested: &Path) -> Result<ResolvedPath, FsError> {
        let absolute = crate::path::resolve_absolute(requested, self.workspace_root())?;
        let canonical = std::fs::canonicalize(&absolute).map_err(|e| FsError::IoError {
            message: format!("cannot resolve path '{}': {e}", absolute.display()),
        })?;

        let root = self.find_best_matching_root(&canonical).ok_or_else(|| {
            FsError::OutsideAllowedRoots {
                path: requested.to_path_buf(),
            }
        })?;

        root.ensure_access(AccessKind::Read)?;

        Ok(ResolvedPath {
            requested: requested.to_path_buf(),
            canonical,
            root,
        })
    }

    pub fn resolve_existing_write(&self, requested: &Path) -> Result<ResolvedPath, FsError> {
        let absolute = crate::path::resolve_absolute(requested, self.workspace_root())?;
        let canonical = std::fs::canonicalize(&absolute).map_err(|e| FsError::IoError {
            message: format!("cannot resolve path '{}': {e}", absolute.display()),
        })?;

        let root = self.find_best_matching_root(&canonical).ok_or_else(|| {
            FsError::OutsideAllowedRoots {
                path: requested.to_path_buf(),
            }
        })?;

        root.ensure_access(AccessKind::Write)?;

        Ok(ResolvedPath {
            requested: requested.to_path_buf(),
            canonical,
            root,
        })
    }

    pub fn resolve_create_write(&self, requested: &Path) -> Result<ResolvedCreatePath, FsError> {
        let absolute = crate::path::resolve_absolute(requested, self.workspace_root())?;

        if absolute.as_os_str().is_empty() {
            return Err(FsError::InvalidPath {
                path: requested.to_path_buf(),
            });
        }

        // Get the parent directory
        let parent = absolute.parent().ok_or_else(|| FsError::InvalidPath {
            path: requested.to_path_buf(),
        })?;

        // Find nearest existing parent
        let existing_parent =
            crate::path::nearest_existing_parent(parent).map_err(|e| FsError::IoError {
                message: format!(
                    "cannot find existing parent for '{}': {e}",
                    requested.display()
                ),
            })?;

        // Canonicalize the existing parent (resolves symlinks)
        let canonical_parent =
            std::fs::canonicalize(&existing_parent).map_err(|e| FsError::IoError {
                message: format!(
                    "cannot resolve parent path '{}': {e}",
                    existing_parent.display()
                ),
            })?;

        let root = self
            .find_best_matching_root(&canonical_parent)
            .ok_or_else(|| FsError::OutsideAllowedRoots {
                path: requested.to_path_buf(),
            })?;

        root.ensure_access(AccessKind::Write)?;

        // Build the destination path: canonical_parent + non-existent tail
        let normalized = normalize_path(&absolute);
        let norm_components: Vec<_> = normalized.components().collect();
        let existing_count = existing_parent.components().count();

        // If normalized path is shorter than the existing parent, the path
        // traversed above the parent (via ..), which means escape attempt.
        if norm_components.len() < existing_count {
            return Err(FsError::OutsideAllowedRoots {
                path: requested.to_path_buf(),
            });
        }

        // The canonical parent may differ from the existing parent (symlinks).
        // We build: canonical_parent + components beyond existing_parent's depth
        let tail: Vec<_> = norm_components[existing_count.min(norm_components.len())..].to_vec();

        let mut destination = canonical_parent.clone();
        for component in &tail {
            match component {
                std::path::Component::ParentDir => {
                    return Err(FsError::OutsideAllowedRoots {
                        path: requested.to_path_buf(),
                    });
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
            .ok_or_else(|| FsError::OutsideAllowedRoots {
                path: canonical_path.to_path_buf(),
            })?;
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

        matching.first().cloned().cloned()
    }
}
