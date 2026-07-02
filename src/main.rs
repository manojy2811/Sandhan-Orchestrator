mod protocol;
mod sandbox;
mod context;
mod orchestrator;
mod spaces;
mod observability;
mod plugins;
mod llm;
mod hybrid_reasoning;
mod rbac;
mod mcp;
mod a2a;

use protocol::{AcpRequest, AcpResponse};
use sandbox::Sandbox;
use context::{ContextManager, ContextCache};
use orchestrator::SubagentOrchestrator;
use spaces::SpacesManager;
use observability::ObservabilityTracker;
use plugins::PluginManager;
use llm::{LlmRouter, RoutingPolicy};
use hybrid_reasoning::HybridReasoningEngine;
use rbac::SecurityManager;

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
    
    // Observability & Telemetry Tracker
    let observability = ObservabilityTracker::new();

    // Plugin Manager, LLM Router, Hybrid reasoning, Security managers
    let plugin_manager = PluginManager::new();
    let llm_router = LlmRouter::new();
    let hybrid_reasoner = HybridReasoningEngine::new();
    let security_manager = SecurityManager::new();
    let mcp_registry = mcp::McpRegistry::new();
    let a2a_manager = a2a::A2aManager::new();

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
            "plugin_install" => {
                let name = request.params.get("name").and_then(|v| v.as_str());
                let desc = request.params.get("description").and_then(|v| v.as_str()).unwrap_or("marketplace plugin");
                let cmds_val = request.params.get("commands").and_then(|v| v.as_array());
                let deps_val = request.params.get("dependencies").and_then(|v| v.as_array());

                if let Some(n) = name {
                    let cmds: Vec<String> = cmds_val
                        .map(|arr| arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                        .unwrap_or_default();
                        
                    let deps: Vec<String> = deps_val
                        .map(|arr| arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                        .unwrap_or_default();

                    match plugin_manager.install_plugin(n, desc, cmds, deps) {
                        Ok(p) => {
                            let resp = AcpResponse::success(request.id.clone(), json!(p));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32005, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameter 'name'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "plugin_uninstall" => {
                let name = request.params.get("name").and_then(|v| v.as_str());
                if let Some(n) = name {
                    match plugin_manager.uninstall_plugin(n) {
                        Ok(_) => {
                            let resp = AcpResponse::success(request.id.clone(), json!({ "status": "success" }));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32005, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameter 'name'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "plugin_list" => {
                let list = plugin_manager.list_installed();
                let resp = AcpResponse::success(request.id.clone(), json!({ "plugins": list }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "plugin_execute" => {
                let name = request.params.get("name").and_then(|v| v.as_str());
                let cmd = request.params.get("command").and_then(|v| v.as_str());

                if let (Some(n), Some(c)) = (name, cmd) {
                    match plugin_manager.run_plugin_command(n, c) {
                        Ok(out) => {
                            let resp = AcpResponse::success(request.id.clone(), json!({ "output": out }));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32004, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                } else {
                    let resp = AcpResponse::error(request.id.clone(), -32602, "Missing parameters 'name' or 'command'");
                    println!("{}", serde_json::to_string(&resp).unwrap());
                }
            }
            "llm_route" => {
                let prompt = request.params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                let policy_str = request.params.get("policy").and_then(|v| v.as_str()).unwrap_or("cost");
                
                let policy = match policy_str {
                    "latency" => RoutingPolicy::LatencyAware,
                    "task" => RoutingPolicy::TaskAware,
                    _ => RoutingPolicy::CostAware,
                };

                match llm_router.route(prompt, policy) {
                    Ok(out) => {
                        let resp = AcpResponse::success(request.id.clone(), json!({ "response": out }));
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                    Err(e) => {
                        let resp = AcpResponse::error(request.id.clone(), -32006, &e);
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                }
            }
            "hybrid_reason" => {
                let query = request.params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                
                // If it can be resolved symbolically, do it!
                if let Some(sym_res) = hybrid_reasoner.evaluate_hybrid(query) {
                    let resp = AcpResponse::success(request.id.clone(), json!({ "source": "symbolic", "result": sym_res }));
                    println!("{}", serde_json::to_string(&resp).unwrap());
                } else {
                    // Fallback to LLM router
                    match llm_router.route(query, RoutingPolicy::CostAware) {
                        Ok(llm_res) => {
                            let resp = AcpResponse::success(request.id.clone(), json!({ "source": "llm_router", "result": llm_res }));
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = AcpResponse::error(request.id.clone(), -32007, &e);
                            println!("{}", serde_json::to_string(&resp).unwrap());
                        }
                    }
                }
            }
            "rbac_authorize" => {
                let username = request.params.get("username").and_then(|v| v.as_str()).unwrap_or("unknown");
                let action = request.params.get("action").and_then(|v| v.as_str()).unwrap_or("read");

                match security_manager.authorize_action(username, action) {
                    Ok(profile) => {
                        observability.record_audit(&format!("{:?}", profile.role), action, "Authorized");
                        let alert = security_manager.mock_jira_slack_alert(action, "Authorized");
                        let resp = AcpResponse::success(request.id.clone(), json!({ "profile": profile, "alert": alert }));
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                    Err(e) => {
                        observability.record_audit("unauthorized", action, "Denied");
                        let resp = AcpResponse::error(request.id.clone(), -32008, &e);
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                }
            }
            "audit_get" => {
                let logs = observability.get_audit_logs();
                let resp = AcpResponse::success(request.id.clone(), json!({ "audit_logs": logs }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "mcp_register_server" => {
                let server_id = request.params.get("server_id").and_then(|v| v.as_str()).unwrap_or("default");
                let url = request.params.get("endpoint_url").and_then(|v| v.as_str()).unwrap_or("");
                let scope = request.params.get("scope").and_then(|v| v.as_str()).unwrap_or("workspace");

                let config = mcp::McpServerConfig {
                    server_id: server_id.to_string(),
                    endpoint_url: url.to_string(),
                    scope: scope.to_string(),
                };
                mcp_registry.register_server(config);
                observability.record_audit("admin", "mcp_register_server", "Success");
                let resp = AcpResponse::success(request.id.clone(), json!({ "status": "registered" }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "mcp_list_servers" => {
                let list = mcp_registry.list_servers();
                let resp = AcpResponse::success(request.id.clone(), json!({ "servers": list }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "mcp_discover_tools" => {
                let list = mcp_registry.discover_tools();
                let resp = AcpResponse::success(request.id.clone(), json!({ "tools": list }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "mcp_invoke_tool" => {
                let name = request.params.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
                let args = request.params.get("arguments").cloned().unwrap_or(json!({}));

                match mcp_registry.validate_and_invoke_tool(name, args) {
                    Ok(res) => {
                        observability.record_execution(5, false);
                        observability.record_audit("user", &format!("mcp_invoke_tool:{}", name), "Success");
                        let resp = AcpResponse::success(request.id.clone(), res);
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                    Err(e) => {
                        observability.record_execution(0, true);
                        observability.record_audit("user", &format!("mcp_invoke_tool:{}", name), "Failure");
                        let resp = AcpResponse::error(request.id.clone(), -32009, &e);
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                }
            }
            "a2a_register_agent" => {
                let name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let caps_val = request.params.get("capabilities").and_then(|v| v.as_array());
                let input_schema = request.params.get("input_schema").cloned().unwrap_or(json!({}));
                let output_schema = request.params.get("output_schema").cloned().unwrap_or(json!({}));

                let caps: Vec<String> = caps_val
                    .map(|arr| arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                    .unwrap_or_default();

                let card = a2a::AgentCard {
                    name: name.to_string(),
                    capabilities: caps,
                    input_schema,
                    output_schema,
                };
                a2a_manager.register_remote_agent(card.clone());
                observability.record_audit("admin", &format!("a2a_register_agent:{}", name), "Success");
                let resp = AcpResponse::success(request.id.clone(), json!(card));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "a2a_list_agents" => {
                let list = a2a_manager.list_agent_cards();
                let resp = AcpResponse::success(request.id.clone(), json!({ "agents": list }));
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
            "a2a_delegate_task" => {
                let task_id = request.params.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                let delegator = request.params.get("delegator").and_then(|v| v.as_str()).unwrap_or("");
                let executor = request.params.get("executor").and_then(|v| v.as_str()).unwrap_or("");
                let payload = request.params.get("payload").cloned().unwrap_or(json!({}));

                // Check authorization first via RBAC
                match security_manager.authorize_action(delegator, "a2a_delegate") {
                    Ok(_) => {
                        match a2a_manager.delegate_task(task_id, delegator, executor, payload) {
                            Ok(task) => {
                                observability.record_execution(15, false);
                                observability.record_audit(delegator, &format!("a2a_delegate:{}", task_id), "Success");
                                let resp = AcpResponse::success(request.id.clone(), json!(task));
                                println!("{}", serde_json::to_string(&resp).unwrap());
                            }
                            Err(e) => {
                                let resp = AcpResponse::error(request.id.clone(), -32010, &e);
                                println!("{}", serde_json::to_string(&resp).unwrap());
                            }
                        }
                    }
                    Err(e) => {
                        observability.record_audit(delegator, "a2a_delegate_denied", "Failure");
                        let resp = AcpResponse::error(request.id.clone(), -32008, &e);
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                }
            }
            "a2a_update_task" => {
                let task_id = request.params.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                let status = request.params.get("status").and_then(|v| v.as_str()).unwrap_or("");
                let artifact = request.params.get("artifact").cloned();

                match a2a_manager.update_task_lifecycle(task_id, status, artifact) {
                    Ok(task) => {
                        let resp = AcpResponse::success(request.id.clone(), json!(task));
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
                    Err(e) => {
                        let resp = AcpResponse::error(request.id.clone(), -32011, &e);
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    }
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
