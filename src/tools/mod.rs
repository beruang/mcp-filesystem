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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_has_all_tools() {
        let tools = tool_registry();
        let names: Vec<&str> = tools.iter().map(|(d, _)| d.name.as_str()).collect();
        assert!(names.contains(&"list_allowed_directories"));
        assert!(names.contains(&"read_text_file"));
        assert!(names.contains(&"read_media_file"));
        assert!(names.contains(&"read_multiple_files"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"create_directory"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"list_directory_with_sizes"));
        assert!(names.contains(&"directory_tree"));
        assert!(names.contains(&"move_file"));
        assert!(names.contains(&"search_files"));
        assert!(names.contains(&"get_file_info"));
    }

    #[test]
    fn test_tool_registry_exact_count() {
        assert_eq!(tool_registry().len(), 13);
    }

    #[test]
    fn test_tool_registry_definitions_not_empty() {
        for (def, _) in tool_registry() {
            assert!(!def.name.is_empty(), "tool name empty");
            assert!(!def.description.is_empty(), "tool {} description empty", def.name);
        }
    }
}
