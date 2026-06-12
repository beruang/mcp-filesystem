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

#[cfg(test)]
mod tests {
    use super::*;

    fn mkpath(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn test_error_code_all_variants() {
        assert_eq!(FsError::InvalidPath { path: mkpath("x") }.error_code(), "invalid_path");
        assert_eq!(FsError::PathNotFound { path: mkpath("x") }.error_code(), "path_not_found");
        assert_eq!(
            FsError::OutsideAllowedRoots { path: mkpath("x") }.error_code(),
            "outside_allowed_roots"
        );
        assert_eq!(
            FsError::PermissionDenied { path: mkpath("x") }.error_code(),
            "permission_denied"
        );
        assert_eq!(FsError::ReadOnlyRoot { path: mkpath("x") }.error_code(), "read_only_root");
        assert_eq!(FsError::NotAFile { path: mkpath("x") }.error_code(), "not_a_file");
        assert_eq!(FsError::NotADirectory { path: mkpath("x") }.error_code(), "not_a_directory");
        assert_eq!(
            FsError::FileTooLarge { path: mkpath("x"), size: 1, max: 2 }.error_code(),
            "file_too_large"
        );
        assert_eq!(
            FsError::BinaryFileNotSupported { path: mkpath("x") }.error_code(),
            "binary_file_not_supported"
        );
        assert_eq!(FsError::TooManyResults { count: 1, max: 2 }.error_code(), "too_many_results");
        assert_eq!(
            FsError::AmbiguousRelativePath { reason: "r".into() }.error_code(),
            "ambiguous_relative_path"
        );
        assert_eq!(
            FsError::EditPatternNotFound { path: mkpath("x"), pattern: "p".into() }.error_code(),
            "edit_pattern_not_found"
        );
        assert_eq!(
            FsError::EditPatternAmbiguous { path: mkpath("x"), count: 3 }.error_code(),
            "edit_pattern_ambiguous"
        );
        assert_eq!(FsError::IoError { message: "m".into() }.error_code(), "io_error");
        assert_eq!(
            FsError::SerializationError { message: "m".into() }.error_code(),
            "serialization_error"
        );
        assert_eq!(
            FsError::UnsupportedOperation { message: "m".into() }.error_code(),
            "unsupported_operation"
        );
    }

    #[test]
    fn test_to_error_response_has_path() {
        let e = FsError::OutsideAllowedRoots { path: mkpath("/etc/passwd") };
        let r = e.to_error_response();
        assert_eq!(r.code, "outside_allowed_roots");
        assert_eq!(r.path, Some("/etc/passwd".into()));
        assert!(r.pattern.is_none());
        assert!(r.count.is_none());
    }

    #[test]
    fn test_to_error_response_edit_pattern_not_found() {
        let e = FsError::EditPatternNotFound { path: mkpath("f.txt"), pattern: "hello".into() };
        let r = e.to_error_response();
        assert_eq!(r.code, "edit_pattern_not_found");
        assert_eq!(r.pattern, Some("hello".into()));
    }

    #[test]
    fn test_to_error_response_edit_pattern_ambiguous() {
        let e = FsError::EditPatternAmbiguous { path: mkpath("f.txt"), count: 5 };
        let r = e.to_error_response();
        assert_eq!(r.code, "edit_pattern_ambiguous");
        assert_eq!(r.count, Some(5));
    }

    #[test]
    fn test_to_error_response_no_path() {
        let e = FsError::TooManyResults { count: 10, max: 5 };
        let r = e.to_error_response();
        assert_eq!(r.code, "too_many_results");
        assert!(r.path.is_none());
    }

    #[test]
    fn test_to_error_response_io_error() {
        let e = FsError::IoError { message: "disk full".into() };
        let r = e.to_error_response();
        assert_eq!(r.code, "io_error");
        assert!(r.path.is_none());
        assert!(r.pattern.is_none());
        assert!(r.count.is_none());
    }

    #[test]
    fn test_display_formatting() {
        let e = FsError::OutsideAllowedRoots { path: mkpath("/etc") };
        assert!(e.to_string().contains("/etc"));
        assert!(e.to_string().contains("outside allowed"));
    }

    #[test]
    fn test_from_io_error_not_found() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::PathNotFound { .. }));
    }

    #[test]
    fn test_from_io_error_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::PermissionDenied { .. }));
    }

    #[test]
    fn test_from_io_error_other() {
        let io_err = std::io::Error::new(std::io::ErrorKind::AlreadyExists, "exists");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::IoError { .. }));
    }
}
