//! JSON-RPC 2.0 framing for the MCP stdio transport.
//!
//! Hand-rolled rather than pulling in a crate — the wire shape is small
//! and we want fine-grained control over notification-vs-response and
//! error encoding. See the [JSON-RPC 2.0 spec][rpc] and the [MCP
//! transport spec][mcp].
//!
//! [rpc]: https://www.jsonrpc.org/specification
//! [mcp]: https://modelcontextprotocol.io/specification

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    /// Absent for notifications. We accept any JSON value (string or
    /// number) since both are common in the wild.
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Internal flag — when true, `main` suppresses wire emission. Used
    /// for notification methods that must not produce a response.
    #[serde(skip)]
    pub notification_ack: bool,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn result(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            // Distinguish "no id provided" (notification) from "id was
            // null" (a legal JSON-RPC id). We treat both as
            // notification-shape for simplicity — MCP clients always
            // attach IDs to requests they expect to be answered.
            notification_ack: id.is_none(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, err: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            notification_ack: id.is_none(),
            id,
            result: None,
            error: Some(err),
        }
    }

    pub fn is_notification_ack(&self) -> bool {
        self.notification_ack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn result_serializes_without_error_field() {
        let r = JsonRpcResponse::result(Some(json!(1)), json!({"ok": true}));
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"result\""));
        assert!(!s.contains("\"error\""));
    }

    #[test]
    fn error_serializes_without_result_field() {
        let r = JsonRpcResponse::error(
            Some(json!(1)),
            JsonRpcError {
                code: -32601,
                message: "x".into(),
                data: None,
            },
        );
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"error\""));
        assert!(!s.contains("\"result\""));
    }

    #[test]
    fn notification_ack_for_missing_id() {
        let r = JsonRpcResponse::result(None, Value::Null);
        assert!(r.is_notification_ack());
    }
}
