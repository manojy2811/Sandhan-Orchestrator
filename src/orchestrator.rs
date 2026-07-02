use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subagent {
    pub id: String,
    pub name: String,
    pub task: String,
    pub status: String,
    pub context_history: Vec<String>,
}

#[derive(Clone)]
pub struct SubagentOrchestrator {
    agents: Arc<RwLock<HashMap<String, Subagent>>>,
}

impl SubagentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn spawn_subagent(&self, name: &str, task: &str, shared_context: Vec<String>) -> String {
        let id = Uuid::new_v4().to_string();
        let agent = Subagent {
            id: id.clone(),
            name: name.to_string(),
            task: task.to_string(),
            status: "idle".to_string(),
            context_history: shared_context,
        };

        let mut write = self.agents.write().unwrap();
        write.insert(id.clone(), agent);
        id
    }

    pub fn list_subagents(&self) -> Vec<Subagent> {
        let read = self.agents.read().unwrap();
        read.values().cloned().collect()
    }

    pub fn execute_subagent_task(&self, id: &str, subtask: &str) -> Result<String, String> {
        let mut write = self.agents.write().unwrap();
        if let Some(agent) = write.get_mut(id) {
            agent.status = "busy".to_string();
            
            // Simulating execution on shared context logs
            let log_msg = format!("Subagent [{}] executed task: '{}'", agent.name, subtask);
            agent.context_history.push(log_msg.clone());
            agent.status = "idle".to_string();
            
            Ok(format!("Task execution completed successfully. Output: '{}'", log_msg))
        } else {
            Err(format!("Subagent with ID '{}' not found.", id))
        }
    }
}
