use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize)]
pub struct FileInfoOutput {
    pub path: String,
    #[serde(rename = "type")]
    pub file_type: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<String>,
    pub readonly: bool,
}

const fn _format_time(_t: std::time::SystemTime) -> Option<String> {
    None
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "get_file_info".to_string(),
        description:
            "Return metadata for a file or directory: type, size, timestamps, readonly flag."
                .to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({
                "path": {"type": "string", "description": "Path to the file or directory"}
            }),
            vec!["path"],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, _config: &AppConfig, params: Value) -> Result<Value, FsError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| FsError::InvalidPath { path: std::path::PathBuf::from("") })?;
    let requested = Path::new(path_str);

    let resolved = sandbox.resolve_existing_read(requested)?;
    let metadata = std::fs::metadata(&resolved.canonical)?;

    let file_type = if metadata.is_dir() {
        "directory"
    } else if metadata.is_symlink() {
        "symlink"
    } else {
        "file"
    };

    let readonly = metadata.permissions().readonly();

    let format_ts = |t: std::io::Result<std::time::SystemTime>| -> Option<String> {
        t.ok().map(|time| {
            let dur = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let secs = dur.as_secs();
            #[allow(clippy::cast_possible_wrap)]
            chrono_lite::from_unix(secs as i64)
        })
    };

    serde_json::to_value(FileInfoOutput {
        path: resolved.requested.display().to_string(),
        file_type: file_type.to_string(),
        size: metadata.len(),
        created: format_ts(metadata.created()),
        modified: format_ts(metadata.modified()),
        accessed: format_ts(metadata.accessed()),
        readonly,
    })
    .map_err(|e| FsError::SerializationError { message: e.to_string() })
}

mod chrono_lite {
    pub fn from_unix(secs: i64) -> String {
        let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        let mut remaining = secs;
        let mut year = 1970i64;
        loop {
            let days_in_year = if is_leap(year) { 366 } else { 365 };
            let secs_in_year = days_in_year * 86400;
            if remaining < secs_in_year {
                break;
            }
            remaining -= secs_in_year;
            year += 1;
        }

        let mut month = 0usize;
        for (i, &days) in days_in_month.iter().enumerate() {
            let days_in_this_month = if i == 1 && is_leap(year) { 29 } else { i64::from(days) };
            let secs_in_month = days_in_this_month * 86400;
            if remaining < secs_in_month {
                month = i;
                break;
            }
            remaining -= secs_in_month;
        }

        let day = (remaining / 86400) + 1;
        remaining %= 86400;
        let hour = remaining / 3600;
        remaining %= 3600;
        let min = remaining / 60;
        let sec = remaining % 60;

        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month + 1, day, hour, min, sec)
    }

    const fn is_leap(y: i64) -> bool {
        (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
    }
}
