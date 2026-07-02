mod protocol;
mod sandbox;
mod context;
mod orchestrator;
mod spaces;
mod observability;

use protocol::{AcpRequest, AcpResponse};
use sandbox::Sandbox;
use context::{ContextManager, ContextCache};
use orchestrator::SubagentOrchestrator;
use spaces::SpacesManager;
use observability::ObservabilityTracker;
use std::io::{self, BufRead};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let sandbox = match Sandbox::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize sandbox environment: {}", e);
            std::process::exit(1);
        }
    };

    let context_manager = Arc::new(Mutex::new(ContextManager::new(4096)));
    let cache = ContextCache::new();
    let orchestrator = SubagentOrchestrator::new();
    
    let spaces_base = PathBuf::from(sandbox.get_workspace_path()).join("spaces");
    let spaces_manager = SpacesManager::new(spaces_base);

    // Instantiate Observability tracker
    let observability = ObservabilityTracker::new();

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
                            observability.record_execution(10, false);
                            let resp = AcpResponse::success(request.id.clone(), res);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            observability.record_execution(0, true);
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
                    
                    let tokens = manager.current_tokens;
                    observability.record_execution(tokens, false);

                    let resp = AcpResponse::success(request.id.clone(), json!({ "status": "success", "tokens": tokens }));
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
            "subagent_spawn" => {
                let name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or("worker");
                let task = request.params.get("task").and_then(|v| v.as_str()).unwrap_or("assist");
                
                let history_snapshot: Vec<String> = {
                    let manager = context_manager.lock().unwrap();
                    manager.get_messages().iter().map(|m| format!("{}: {}", m.role, m.content)).collect()
                };

                let agent_id = orchestrator.spawn_subagent(name, task, history_snapshot);
                let resp = AcpResponse::success(request.id.clone(), json!({ "subagent_id": agent_id }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "subagent_list" => {
                let list = orchestrator.list_subagents();
                let resp = AcpResponse::success(request.id.clone(), json!({ "subagents": list }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "subagent_execute" => {
                let agent_id = request.params.get("subagent_id").and_then(|v| v.as_str());
                let task = request.params.get("task").and_then(|v| v.as_str());

                if let (Some(id), Some(t)) = (agent_id, task) {
                    match orchestrator.execute_subagent_task(id, t) {
                        Ok(output) => {
                            let resp = AcpResponse::success(request.id.clone(), json!({ "output": output }));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32002, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameters 'subagent_id' or 'task'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "space_create" => {
                let name = request.params.get("space_name").and_then(|v| v.as_str());
                if let Some(n) = name {
                    match spaces_manager.create_space(n) {
                        Ok(sp) => {
                            let resp = AcpResponse::success(request.id.clone(), json!(sp));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32003, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameter 'space_name'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "space_checkout" => {
                let name = request.params.get("space_name").and_then(|v| v.as_str());
                let branch = request.params.get("branch").and_then(|v| v.as_str());

                if let (Some(n), Some(b)) = (name, branch) {
                    match spaces_manager.checkout_branch(n, b) {
                        Ok(sp) => {
                            let resp = AcpResponse::success(request.id.clone(), json!(sp));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32003, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameters 'space_name' or 'branch'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "space_pr" => {
                let name = request.params.get("space_name").and_then(|v| v.as_str());
                let pr_id_val = request.params.get("pr_id").and_then(|v| v.as_u64());

                if let (Some(n), Some(pr_id)) = (name, pr_id_val) {
                    match spaces_manager.link_pull_request(n, pr_id as usize) {
                        Ok(sp) => {
                            let resp = AcpResponse::success(request.id.clone(), json!(sp));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32003, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameters 'space_name' or 'pr_id'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "telemetry_get" => {
                let stats = observability.get_telemetry();
                let resp = AcpResponse::success(request.id.clone(), json!(stats));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "reasoning_add" => {
                let node = request.params.get("node").and_then(|v| v.as_str()).unwrap_or("unknown");
                let decision = request.params.get("decision").and_then(|v| v.as_str());
                let justification = request.params.get("justification").and_then(|v| v.as_str());

                if let (Some(dec), Some(just)) = (decision, justification) {
                    observability.add_reasoning_step(node, dec, just);
                    let resp = AcpResponse::success(request.id.clone(), json!({ "status": "success" }));
                    println!("{}", serde_json::to_string(&resp).unwrap());
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameters 'decision' or 'justification'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "reasoning_get" => {
                let traces = observability.get_reasoning_traces();
                let resp = AcpResponse::success(request.id.clone(), json!({ "traces": traces }));
                println!("{}", serde_json::to_string(&resp).unwrap());
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
