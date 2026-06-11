use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Path is outside allowed directories: {path}")]
    OutsideAllowedRoots { path: PathBuf },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Read-only root: write denied for {path}")]
    ReadOnlyRoot { path: PathBuf },

    #[error("Not a file: {path}")]
    NotAFile { path: PathBuf },

    #[error("Not a directory: {path}")]
    NotADirectory { path: PathBuf },

    #[error("File too large: {path} ({size} bytes, max {max})")]
    FileTooLarge { path: PathBuf, size: u64, max: u64 },

    #[error("Binary file not supported: {path}")]
    BinaryFileNotSupported { path: PathBuf },

    #[error("Too many results ({count}, max {max})")]
    TooManyResults { count: usize, max: usize },

    #[error("Ambiguous relative path: {reason}")]
    AmbiguousRelativePath { reason: String },

    #[error("Edit pattern not found")]
    EditPatternNotFound { path: PathBuf, pattern: String },

    #[error("Edit pattern ambiguous: found {count} occurrences")]
    EditPatternAmbiguous { path: PathBuf, count: usize },

    #[error("IO error: {message}")]
    IoError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Unsupported operation: {message}")]
    UnsupportedOperation { message: String },
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

impl FsError {
    pub fn error_code(&self) -> &str {
        match self {
            FsError::InvalidPath { .. } => "invalid_path",
            FsError::PathNotFound { .. } => "path_not_found",
            FsError::OutsideAllowedRoots { .. } => "outside_allowed_roots",
            FsError::PermissionDenied { .. } => "permission_denied",
            FsError::ReadOnlyRoot { .. } => "read_only_root",
            FsError::NotAFile { .. } => "not_a_file",
            FsError::NotADirectory { .. } => "not_a_directory",
            FsError::FileTooLarge { .. } => "file_too_large",
            FsError::BinaryFileNotSupported { .. } => "binary_file_not_supported",
            FsError::TooManyResults { .. } => "too_many_results",
            FsError::AmbiguousRelativePath { .. } => "ambiguous_relative_path",
            FsError::EditPatternNotFound { .. } => "edit_pattern_not_found",
            FsError::EditPatternAmbiguous { .. } => "edit_pattern_ambiguous",
            FsError::IoError { .. } => "io_error",
            FsError::SerializationError { .. } => "serialization_error",
            FsError::UnsupportedOperation { .. } => "unsupported_operation",
        }
    }

    pub fn to_error_response(&self) -> ErrorResponse {
        let (path, pattern, count) = match self {
            FsError::OutsideAllowedRoots { path } => (Some(path.display().to_string()), None, None),
            FsError::PathNotFound { path } => (Some(path.display().to_string()), None, None),
            FsError::InvalidPath { path } => (Some(path.display().to_string()), None, None),
            FsError::PermissionDenied { path } => (Some(path.display().to_string()), None, None),
            FsError::ReadOnlyRoot { path } => (Some(path.display().to_string()), None, None),
            FsError::NotAFile { path } => (Some(path.display().to_string()), None, None),
            FsError::NotADirectory { path } => (Some(path.display().to_string()), None, None),
            FsError::FileTooLarge { path, .. } => (Some(path.display().to_string()), None, None),
            FsError::BinaryFileNotSupported { path } => {
                (Some(path.display().to_string()), None, None)
            }
            FsError::TooManyResults { .. } => (None, None, None),
            FsError::AmbiguousRelativePath { .. } => (None, None, None),
            FsError::EditPatternNotFound { path, pattern } => (
                Some(path.display().to_string()),
                Some(pattern.clone()),
                None,
            ),
            FsError::EditPatternAmbiguous { path, count } => {
                (Some(path.display().to_string()), None, Some(*count))
            }
            FsError::IoError { .. }
            | FsError::SerializationError { .. }
            | FsError::UnsupportedOperation { .. } => (None, None, None),
        };

        ErrorResponse {
            code: self.error_code().to_string(),
            message: self.to_string(),
            path,
            pattern,
            count,
        }
    }
}

impl From<std::io::Error> for FsError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => FsError::PathNotFound {
                path: PathBuf::new(),
            },
            std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: PathBuf::new(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        }
    }
}
