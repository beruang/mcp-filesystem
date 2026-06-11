use crate::config::AppConfig;
use crate::error::FsError;
use crate::sandbox::Sandbox;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct ListAllowedDirectoriesOutput {
    pub directories: Vec<String>,
}

#[must_use]
pub fn definition() -> crate::server::ToolDef {
    crate::server::ToolDef {
        name: "list_allowed_directories".to_string(),
        description: "Return the configured allowed root directories. Does not recursively enumerate descendants.".to_string(),
        input_schema: crate::server::json_schema_object(
            serde_json::json!({}),
            vec![],
        ),
    }
}

pub fn execute(sandbox: &Sandbox, _config: &AppConfig, _params: Value) -> Result<Value, FsError> {
    let dirs = sandbox.list_allowed_directories();
    let output = ListAllowedDirectoriesOutput { directories: dirs };
    serde_json::to_value(output).map_err(|e| FsError::SerializationError { message: e.to_string() })
}
