use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value, // JSON schema for arguments validation
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub server_id: String,
    pub endpoint_url: String,
    pub scope: String, // "workspace" or "agent:<agent_id>"
}

#[derive(Clone)]
pub struct McpRegistry {
    servers: Arc<RwLock<HashMap<String, McpServerConfig>>>,
    tools: Arc<RwLock<HashMap<String, McpTool>>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
        };

        // Seed a default mock math tools server
        let mock_tool = McpTool {
            name: "math_add".to_string(),
            description: "Adds two integers together".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "a": { "type": "integer" },
                    "b": { "type": "integer" }
                },
                "required": ["a", "b"]
            }),
        };
        registry.tools.write().unwrap().insert(mock_tool.name.clone(), mock_tool);

        registry
    }

    pub fn register_server(&self, config: McpServerConfig) {
        let mut write = self.servers.write().unwrap();
        write.insert(config.server_id.clone(), config);
    }

    pub fn list_servers(&self) -> Vec<McpServerConfig> {
        let read = self.servers.read().unwrap();
        read.values().cloned().collect()
    }

    pub fn discover_tools(&self) -> Vec<McpTool> {
        let read = self.tools.read().unwrap();
        read.values().cloned().collect()
    }

    pub fn validate_and_invoke_tool(&self, tool_name: &str, arguments: Value) -> Result<Value, String> {
        let read = self.tools.read().unwrap();
        if let Some(tool) = read.get(tool_name) {
            // Strict Schema validation (mock logic checking required parameters)
            if let Some(required) = tool.input_schema.get("required").and_then(|r| r.as_array()) {
                for req_key in required {
                    let req_str = req_key.as_str().unwrap_or("");
                    if arguments.get(req_str).is_none() {
                        return Err(format!("Schema validation failure: Missing required parameter '{}'", req_str));
                    }
                }
            }

            // Mock implementation of tool call execution
            if tool_name == "math_add" {
                let a = arguments.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                let b = arguments.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(json!({ "result": a + b }))
            } else {
                Ok(json!({ "status": "executed", "tool": tool_name }))
            }
        } else {
            Err(format!("MCP Tool '{}' not found in registry.", tool_name))
        }
    }
}
