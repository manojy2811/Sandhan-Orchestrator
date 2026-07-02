use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AcpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AcpResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<AcpError>,
    pub id: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AcpNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

impl AcpResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(AcpError {
                code,
                message: message.to_string(),
                data: None,
            }),
            id,
        }
    }
}
