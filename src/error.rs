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
    #[must_use]
    pub const fn error_code(&self) -> &str {
        match self {
            Self::InvalidPath { .. } => "invalid_path",
            Self::PathNotFound { .. } => "path_not_found",
            Self::OutsideAllowedRoots { .. } => "outside_allowed_roots",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::ReadOnlyRoot { .. } => "read_only_root",
            Self::NotAFile { .. } => "not_a_file",
            Self::NotADirectory { .. } => "not_a_directory",
            Self::FileTooLarge { .. } => "file_too_large",
            Self::BinaryFileNotSupported { .. } => "binary_file_not_supported",
            Self::TooManyResults { .. } => "too_many_results",
            Self::AmbiguousRelativePath { .. } => "ambiguous_relative_path",
            Self::EditPatternNotFound { .. } => "edit_pattern_not_found",
            Self::EditPatternAmbiguous { .. } => "edit_pattern_ambiguous",
            Self::IoError { .. } => "io_error",
            Self::SerializationError { .. } => "serialization_error",
            Self::UnsupportedOperation { .. } => "unsupported_operation",
        }
    }

    #[must_use]
    pub fn to_error_response(&self) -> ErrorResponse {
        let (path, pattern, count) = match self {
            Self::OutsideAllowedRoots { path }
            | Self::PathNotFound { path }
            | Self::InvalidPath { path }
            | Self::PermissionDenied { path }
            | Self::ReadOnlyRoot { path }
            | Self::NotAFile { path }
            | Self::NotADirectory { path }
            | Self::FileTooLarge { path, .. }
            | Self::BinaryFileNotSupported { path } => {
                (Some(path.display().to_string()), None, None)
            }
            Self::TooManyResults { .. }
            | Self::AmbiguousRelativePath { .. }
            | Self::IoError { .. }
            | Self::SerializationError { .. }
            | Self::UnsupportedOperation { .. } => (None, None, None),
            Self::EditPatternNotFound { path, pattern } => {
                (Some(path.display().to_string()), Some(pattern.clone()), None)
            }
            Self::EditPatternAmbiguous { path, count } => {
                (Some(path.display().to_string()), None, Some(*count))
            }
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
            std::io::ErrorKind::NotFound => Self::PathNotFound { path: PathBuf::new() },
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied { path: PathBuf::new() },
            _ => Self::IoError { message: e.to_string() },
        }
    }
}
