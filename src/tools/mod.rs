#![allow(clippy::needless_pass_by_value, clippy::missing_errors_doc)]
pub mod create_directory;
pub mod directory_tree;
pub mod edit_file;
pub mod get_file_info;
pub mod list_allowed_directories;
pub mod list_directory;
pub mod list_directory_with_sizes;
pub mod move_file;
pub mod read_media_file;
pub mod read_multiple_files;
pub mod read_text_file;
pub mod search_files;
pub mod write_file;

use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde_json::Value;

pub type ToolFn = fn(&Sandbox, &AppConfig, Value) -> Result<Value, FsError>;

#[must_use]
pub fn tool_registry() -> Vec<(crate::server::ToolDef, ToolFn)> {
    vec![
        (list_allowed_directories::definition(), list_allowed_directories::execute as ToolFn),
        (read_text_file::definition(), read_text_file::execute as ToolFn),
        (read_media_file::definition(), read_media_file::execute as ToolFn),
        (read_multiple_files::definition(), read_multiple_files::execute as ToolFn),
        (write_file::definition(), write_file::execute as ToolFn),
        (edit_file::definition(), edit_file::execute as ToolFn),
        (create_directory::definition(), create_directory::execute as ToolFn),
        (list_directory::definition(), list_directory::execute as ToolFn),
        (list_directory_with_sizes::definition(), list_directory_with_sizes::execute as ToolFn),
        (directory_tree::definition(), directory_tree::execute as ToolFn),
        (move_file::definition(), move_file::execute as ToolFn),
        (search_files::definition(), search_files::execute as ToolFn),
        (get_file_info::definition(), get_file_info::execute as ToolFn),
    ]
}
