use crate::config::AppConfig;
use crate::tools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[must_use]
pub fn make_response(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
}

#[must_use]
pub fn make_error(
    id: Option<Value>,
    code: i32,
    message: String,
    data: Option<Value>,
) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError { code, message, data }),
    }
}

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn json_schema_object(properties: Value, required: Vec<&str>) -> Value {
    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}

fn build_tool_defs() -> Vec<ToolDef> {
    tools::tool_registry().into_iter().map(|(d, _)| d).collect()
}

/// Run the MCP stdio server.
#[allow(clippy::too_many_lines, clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub async fn run_stdio(app: Arc<AppConfig>) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let tool_defs = build_tool_defs();
    let tools: Vec<(ToolDef, tools::ToolFn)> = tools::tool_registry();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut out = stdout;

    info!("MCP filesystem server ready");

    while let Ok(Some(line)) = lines.next_line().await {
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let err = make_error(None, -32700, format!("Parse error: {e}"), None);
                let resp = serde_json::to_string(&err).unwrap();
                out.write_all(resp.as_bytes()).await?;
                out.write_all(b"\n").await?;
                out.flush().await?;
                continue;
            }
        };

        debug!(method = %request.method, id = ?request.id, "received request");

        let response = match request.method.as_str() {
            "initialize" => {
                let result = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "mcp-filesystem-rs",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                make_response(request.id, result)
            }

            "notifications/initialized" => {
                continue;
            }

            "tools/list" => {
                let result = serde_json::json!({
                    "tools": tool_defs
                });
                make_response(request.id, result)
            }

            "tools/call" => {
                let tool_name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let tool_args = request.params.get("arguments").cloned().unwrap_or(Value::Null);

                let handler = tools.iter().find(|(d, _)| d.name == tool_name).map(|(_, h)| *h);

                match handler {
                    Some(h) => {
                        let config_ref = Arc::clone(&app);
                        let result = tokio::task::spawn_blocking(move || {
                            h(&config_ref.sandbox, &config_ref, tool_args)
                        })
                        .await;

                        match result {
                            Ok(Ok(data)) => {
                                let text = serde_json::to_string(&data).unwrap();
                                let tool_result = ToolResult {
                                    content: vec![ToolContent {
                                        content_type: "text".to_string(),
                                        text,
                                    }],
                                    is_error: None,
                                };
                                make_response(
                                    request.id,
                                    serde_json::to_value(tool_result).unwrap(),
                                )
                            }
                            Ok(Err(fs_err)) => {
                                let err_resp = fs_err.to_error_response();
                                let text = serde_json::to_string(&err_resp).unwrap();
                                warn!(
                                    code = %fs_err.error_code(),
                                    path = ?err_resp.path,
                                    "tool error"
                                );
                                let tool_result = ToolResult {
                                    content: vec![ToolContent {
                                        content_type: "text".to_string(),
                                        text,
                                    }],
                                    is_error: Some(true),
                                };
                                make_response(
                                    request.id,
                                    serde_json::to_value(tool_result).unwrap(),
                                )
                            }
                            Err(join_err) => make_error(
                                request.id,
                                -32603,
                                format!("Internal error: {join_err}"),
                                None,
                            ),
                        }
                    }
                    None => {
                        make_error(request.id, -32602, format!("Unknown tool: {tool_name}"), None)
                    }
                }
            }

            "ping" => make_response(request.id, serde_json::json!({})),

            _ => make_error(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
                None,
            ),
        };

        let resp = serde_json::to_string(&response).unwrap();
        out.write_all(resp.as_bytes()).await?;
        out.write_all(b"\n").await?;
        out.flush().await?;
    }

    Ok(())
}
