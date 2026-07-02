use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ==========================================
// MILESTONE 5: POLICY ENGINE & APPROVAL GATES
// ==========================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicyRule {
    pub rule_id: String,
    pub action_type: String,
    pub cost_threshold: f64,
    pub data_sensitivity: String, // "low", "medium", "high"
    pub requires_human_approval: bool,
}

pub struct PolicyEngine {
    rules: Arc<RwLock<HashMap<String, PolicyRule>>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert("destructive_delete".to_string(), PolicyRule {
            rule_id: "POL_01".to_string(),
            action_type: "delete_plugin".to_string(),
            cost_threshold: 0.0,
            data_sensitivity: "high".to_string(),
            requires_human_approval: true,
        });
        Self { rules: Arc::new(RwLock::new(rules)) }
    }

    pub fn evaluate_action(&self, action: &str, cost: f64, sensitivity: &str) -> Result<bool, String> {
        let read = self.rules.read().unwrap();
        for rule in read.values() {
            if rule.action_type == action {
                if rule.requires_human_approval || cost > rule.cost_threshold || sensitivity == "high" {
                    return Ok(false); // Needs human intervention
                }
            }
        }
        Ok(true) // Auto-approved
    }
}

// ==========================================
// MILESTONE 6: PERSISTENT MEMORY LAYER
// ==========================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub vector_embedding: Vec<f32>,
}

pub struct MemoryLayer {
    // Pluggable backend store (simulating in-memory vector indexing)
    vector_store: Arc<RwLock<Vec<MemoryEntry>>>,
}

impl MemoryLayer {
    pub fn new() -> Self {
        Self {
            vector_store: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_memory(&self, content: &str) {
        let mut write = self.vector_store.write().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        // Mock embedding calculation
        let vector_embedding = vec![0.1, 0.5, 0.9];
        write.push(MemoryEntry { id, content: content.to_string(), vector_embedding });
    }

    pub fn query_semantic_memories(&self, query: &str) -> Vec<String> {
        let read = self.vector_store.read().unwrap();
        // Mock cosine-similarity semantic match
        read.iter()
            .filter(|m| m.content.contains(query))
            .map(|m| m.content.clone())
            .collect()
    }
}

// ==========================================
// MILESTONE 7: EXPLICIT MODEL TIERING
// ==========================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModelTier {
    Triage,
    Reasoning,
    Frontier,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelTierConfig {
    pub agent_role: String,
    pub assigned_tier: ModelTier,
    pub model_name: String,
}

pub struct ModelTierManager {
    configs: Arc<RwLock<HashMap<String, ModelTierConfig>>>,
}

impl ModelTierManager {
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        configs.insert("triage".to_string(), ModelTierConfig {
            agent_role: "triage".to_string(),
            assigned_tier: ModelTier::Triage,
            model_name: "gemini-2.5-flash".to_string(),
        });
        configs.insert("reasoner".to_string(), ModelTierConfig {
            agent_role: "reasoner".to_string(),
            assigned_tier: ModelTier::Reasoning,
            model_name: "gemini-2.5-pro".to_string(),
        });
        Self { configs: Arc::new(RwLock::new(configs)) }
    }

    pub fn get_model_for_role(&self, role: &str) -> Option<ModelTierConfig> {
        let read = self.configs.read().unwrap();
        read.get(role).cloned()
    }
}

// ==========================================
// MILESTONE 8: COST/TOKEN ATTRIBUTION
// ==========================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostRecord {
    pub agent_id: String,
    pub tokens: usize,
    pub usd_cost: f64,
}

pub struct CostAttributionTracker {
    records: Arc<RwLock<Vec<CostRecord>>>,
}

impl CostAttributionTracker {
    pub fn new() -> Self {
        Self { records: Arc::new(RwLock::new(Vec::new())) }
    }

    pub fn track_spend(&self, agent: &str, tokens: usize, cost_per_token: f64) -> Result<(), String> {
        let mut write = self.records.write().unwrap();
        let usd_cost = tokens as f64 * cost_per_token;
        write.push(CostRecord { agent_id: agent.to_string(), tokens, usd_cost });

        // Loop detection: trigger alert if single agent call count > 50 in brief timeline
        let agent_calls = write.iter().filter(|r| r.agent_id == agent).count();
        if agent_calls > 50 {
            return Err(format!("Loop Warning: Agent '{}' has exceeded anomalous spend call threshold of 50 invocations.", agent));
        }
        Ok(())
    }

    pub fn get_attribution_breakdown(&self) -> HashMap<String, f64> {
        let read = self.records.read().unwrap();
        let mut breakdown = HashMap::new();
        for r in read.iter() {
            let entry = breakdown.entry(r.agent_id.clone()).or_insert(0.0);
            *entry += r.usd_cost;
        }
        breakdown
    }
}

// ==========================================
// MILESTONE 9: STRUCTURED AGENT HANDOFF CONTRACTS
// ==========================================
pub struct HandoffValidator;

impl HandoffValidator {
    pub fn validate_handoff_contract(payload: &Value, required_schema: &Value) -> Result<(), String> {
        // Simple schema key validation
        if let Some(req_keys) = required_schema.get("required").and_then(|r| r.as_array()) {
            for key_val in req_keys {
                let key = key_val.as_str().unwrap_or("");
                if payload.get(key).is_none() {
                    return Err(format!("Handoff Contract Validation Failed: Missing required key '{}'", key));
                }
            }
        }
        Ok(())
    }
}

// ==========================================
// MILESTONE 11: SANDBOX SIMULATION MODE
// ==========================================
pub struct SandboxSimulationManager {
    is_simulation: Arc<RwLock<bool>>,
}

impl SandboxSimulationManager {
    pub fn new() -> Self {
        Self { is_simulation: Arc::new(RwLock::new(false)) }
    }

    pub fn set_simulation_mode(&self, enable: bool) {
        let mut write = self.is_simulation.write().unwrap();
        *write = enable;
    }

    pub fn execute_simulated_tool(&self, tool: &str) -> Value {
        json!({
            "status": "simulation_success",
            "tool": tool,
            "simulated": true,
            "mocked_data": format!("Synthetic result for tool '{}'", tool)
        })
    }

    pub fn is_sim_enabled(&self) -> bool {
        *self.is_simulation.read().unwrap()
    }
}

// ==========================================
// MILESTONE 12: DATA RESIDENCY & MULTI-REGION
// ==========================================
pub struct ResidencyManager {
    pinned_region: Arc<RwLock<String>>,
}

impl ResidencyManager {
    pub fn new() -> Self {
        Self { pinned_region: Arc::new(RwLock::new("us-east-1".to_string())) }
    }

    pub fn pin_region(&self, region: &str) {
        let mut write = self.pinned_region.write().unwrap();
        *write = region.to_string();
    }

    pub fn get_pinned_region(&self) -> String {
        self.pinned_region.read().unwrap().clone()
    }
}
