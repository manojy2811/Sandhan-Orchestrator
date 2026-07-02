mod protocol;
mod sandbox;
mod context;

use protocol::{AcpRequest, AcpResponse};
use sandbox::Sandbox;
use context::{ContextManager, ContextCache};
use std::io::{self, BufRead};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let sandbox = match Sandbox::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize sandbox environment: {}", e);
            std::process::exit(1);
        }
    };

    // Instantiate Context Manager and Thread-safe cache
    let context_manager = Arc::new(Mutex::new(ContextManager::new(4096)));
    let cache = ContextCache::new();

    let stdin = io::stdin();
    let reader = stdin.lock();

    // Stdin / Stdout message processing loop
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        // Parse ACP request JSON
        let request: AcpRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = AcpResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    &format!("JSON parsing error: {}", e),
                );
                println!("{}", serde_json::to_string(&err_resp).unwrap());
                continue;
            }
        };

        // Router method dispatcher
        match request.method.as_str() {
            "execute" => {
                let cmd = request.params.get("command").and_then(|v| v.as_str());
                let args_val = request.params.get("args").and_then(|v| v.as_array());

                if let Some(c) = cmd {
                    let args: Vec<String> = args_val
                        .map(|arr| {
                            arr.iter()
                                .map(|v| v.as_str().unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default();

                    match sandbox.execute_command(c, args).await {
                        Ok(res) => {
                            let resp = AcpResponse::success(request.id.clone(), res);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32000, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(
                        request.id.clone(),
                        -32602,
                        "Missing mandatory parameter 'command'",
                    );
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "get_workspace" => {
                let path = sandbox.get_workspace_path();
                let resp = AcpResponse::success(request.id.clone(), json!({ "workspace_path": path }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "context_add" => {
                let role = request.params.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                let content = request.params.get("content").and_then(|v| v.as_str());

                if let Some(txt) = content {
                    let mut manager = context_manager.lock().unwrap();
                    manager.add_message(role, txt);
                    let resp = AcpResponse::success(request.id.clone(), json!({ "status": "success", "tokens": manager.current_tokens }));
                    println!("{}", serde_json::to_string(&resp).unwrap());
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameter 'content'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "context_get" => {
                let manager = context_manager.lock().unwrap();
                let msgs = manager.get_messages();
                let resp = AcpResponse::success(request.id.clone(), json!({ "messages": msgs, "tokens": manager.current_tokens }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "cache_get" => {
                let key = request.params.get("key").and_then(|v| v.as_str());
                if let Some(k) = key {
                    let val = cache.get(k);
                    let resp = AcpResponse::success(request.id.clone(), json!({ "value": val }));
                    println!("{}", serde_json::to_string(&resp).unwrap());
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameter 'key'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "cache_set" => {
                let key = request.params.get("key").and_then(|v| v.as_str());
                let value = request.params.get("value").and_then(|v| v.as_str());

                if let (Some(k), Some(v)) = (key, value) {
                    cache.insert(k.to_string(), v.to_string());
                    let resp = AcpResponse::success(request.id.clone(), json!({ "status": "success" }));
                    println!("{}", serde_json::to_string(&resp).unwrap());
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing 'key' or 'value' parameter");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            _ => {
                let resp = AcpResponse::error(
                    request.id.clone(),
                    -32601,
                    &format!("Method '{}' not found", request.method),
                );
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
        }
    }
}
