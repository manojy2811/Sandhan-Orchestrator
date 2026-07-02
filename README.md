# Sandhan Orchestrator

An ACP-compatible Rust agentic runtime framework acting as an orchestrator with multi-LLM routing, hybrid reasoning, compliance auditing, plugin dependency resolution, and Role-Based Access Control (RBAC).

## Features

- **ACP Message Dispatcher**: Handles JSON-RPC 2.0 messages over standard I/O (stdin/stdout).
- **Execution Sandbox**: Runs subprocesses in an isolated temp environment with command policy controls and execution timeouts.
- **Adaptive Memory Context**: Employs character-to-token sliding window history trimmers and thread-safe caches.
- **Subagent Orchestration**: Spawns worker agents with context propagation.
- **Workspace Isolation**: Creates separate checkout "Spaces" mapped to Git branches and Pull Requests.
- **Multi-LLM Router**: Features cost, latency, and task-aware routing policies with automatic fallback retry chains.
- **Hybrid Reasoning**: Deterministically solves dependency graphs topologically to optimize LLM token consumption.
- **Compliance Observability**: Logs audit records mapping actions to user roles and captures decision logs.
- **Marketplace Registry**: Solves version compatibility checks during plugin installation or uninstallation.
- **Enterprise Security**: Enforces Role-Based Access Control (`Admin`, `Operator`, `User`) and mock Jira/Slack alerting.

## Getting Started

### Prerequisites

- [Rust Toolchain](https://rustup.rs/) (Cargo)

### Build and Run

To build the executable:
```bash
cargo build --release
```

To run the agent and interact with it via the ACP protocol:
```bash
./target/release/acp-agent-wrapper
```

### ACP Protocol Examples

Send JSON-RPC requests via standard input (stdin) to interact:

#### 1. Execute Sandboxed Command
```json
{"jsonrpc":"2.0","method":"execute","params":{"command":"echo","args":["Hello","World"]},"id":1}
```

#### 2. Get Sandbox Workspace Path
```json
{"jsonrpc":"2.0","method":"get_workspace","params":{},"id":2}
```

#### 3. Route Query to LLMs (Task-Aware Policy)
```json
{"jsonrpc":"2.0","method":"llm_route","params":{"prompt":"refactor main.rs","policy":"task"},"id":3}
```

#### 4. Topologically Resolve Task Dependencies
```json
{"jsonrpc":"2.0","method":"hybrid_reason","params":{"query":"cargo test"},"id":4}
```

#### 5. Authenticate Action via RBAC
```json
{"jsonrpc":"2.0","method":"rbac_authorize","params":{"username":"admin","action":"execute"},"id":5}
```

#### 6. Register MCP Server
```json
{"jsonrpc":"2.0","method":"mcp_register_server","params":{"server_id":"math-srv","endpoint_url":"http://localhost:8081","scope":"workspace"},"id":6}
```

#### 7. Delegate A2A Inter-Agent Task
```json
{"jsonrpc":"2.0","method":"a2a_delegate_task","params":{"task_id":"t_01","delegator":"admin","executor":"AegisCore","payload":{"source_code":"println!();"}},"id":7}
```

#### 8. Save State Checkpoint (Time-Travel)
```json
{"jsonrpc":"2.0","method":"checkpoint_save","params":{"checkpoint_id":"cp_01","state":{"stage":"init","tokens_left":200}},"id":8}
```

#### 9. Run Evaluation Harness Suite
```json
{"jsonrpc":"2.0","method":"eval_run","params":{"scenarios":[{"scenario_name":"t1","query":"test compile","expected_output":"success","expected_tools":[]}]},"id":9}
```

#### 10. Query Semantic Episodic Memory
```json
{"jsonrpc":"2.0","method":"memory_query","params":{"query":"refactor"},"id":10}
```

## Cloud Deployment

A pre-configured Helm chart is located in `deploy/helm-chart/`. To install the agent on your Kubernetes cluster:
```bash
helm install sandhan-orchestrator ./deploy/helm-chart
```
