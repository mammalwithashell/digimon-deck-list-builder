//! JSON-RPC 2.0 framing for the exam MCP's stdio transport.
//!
//! Deliberately mirrors `code/digimon-engine-mcp/src/protocol.rs`. Two stdio
//! MCP servers in one repo that disagree about error codes or the initialize
//! handshake is a maintenance trap, so this is the same dialect rather than a
//! second one.

use serde::{Deserialize, Serialize};

/// A parsed JSON-RPC request. `id` is absent for notifications, which must
/// never be answered.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC reserved codes used here. `PARSE_ERROR` matches the code
/// `digimon-engine-mcp/src/main.rs` uses for an unparseable request line --
/// kept a named constant here (rather than a bare literal at the call site)
/// so the two servers' dialects are compared code-to-code, not code-to-magic-number.
pub const PARSE_ERROR: i64 = -32700;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

impl JsonRpcResponse {
    pub fn result(id: Option<serde_json::Value>, result: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i64, message: &str) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_tools_call_request() {
        let raw = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call",
                      "params":{"name":"exam_status","arguments":{"card":"EX12-004"}}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).expect("parse");
        assert_eq!(req.method, "tools/call");
        assert_eq!(req.id, Some(json!(7)));
    }

    #[test]
    fn a_notification_has_no_id() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).expect("parse");
        assert!(req.id.is_none(), "a notification must not be answered");
    }

    #[test]
    fn result_and_error_both_carry_the_request_id() {
        let ok = JsonRpcResponse::result(Some(json!(1)), json!({"ok": true}));
        let text = serde_json::to_string(&ok).unwrap();
        assert!(text.contains("\"id\":1"));
        assert!(text.contains("\"result\""));

        let err = JsonRpcResponse::error(Some(json!(2)), -32601, "no such method");
        let text = serde_json::to_string(&err).unwrap();
        assert!(text.contains("\"id\":2"));
        assert!(text.contains("-32601"));
        assert!(!text.contains("\"result\""), "an error response carries no result");
    }
}
