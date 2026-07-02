use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentCard {
    pub name: String,
    pub capabilities: Vec<String>,
    pub input_schema: Value,
    pub output_schema: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum A2aRole {
    Client,
    Remote,
    Mediator,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskEnvelope {
    pub task_id: String,
    pub delegator_id: String,
    pub executor_id: String,
    pub payload: Value,
    pub status: String, // "submitted", "processing", "completed", "failed"
    pub artifact: Option<Value>,
}

#[derive(Clone)]
pub struct A2aManager {
    cards: Arc<RwLock<HashMap<String, AgentCard>>>,
    tasks: Arc<RwLock<HashMap<String, TaskEnvelope>>>,
}

impl A2aManager {
    pub fn new() -> Self {
        let mut manager = Self {
            cards: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        };

        // Seed self agent card
        let self_card = AgentCard {
            name: "AegisCore".to_string(),
            capabilities: vec!["code_refactoring".to_string(), "compile_check".to_string()],
            input_schema: json!({ "type": "object", "properties": { "source_code": { "type": "string" } } }),
            output_schema: json!({ "type": "object", "properties": { "success": { "type": "boolean" } } }),
        };
        manager.cards.write().unwrap().insert(self_card.name.clone(), self_card);

        manager
    }

    pub fn register_remote_agent(&self, card: AgentCard) {
        let mut write = self.cards.write().unwrap();
        write.insert(card.name.clone(), card);
    }

    pub fn list_agent_cards(&self) -> Vec<AgentCard> {
        let read = self.cards.read().unwrap();
        read.values().cloned().collect()
    }

    pub fn delegate_task(
        &self, 
        task_id: &str, 
        delegator: &str, 
        executor: &str, 
        payload: Value
    ) -> Result<TaskEnvelope, String> {
        let cards_read = self.cards.read().unwrap();
        if !cards_read.contains_key(executor) {
            return Err(format!("A2A Delegation Error: Target executor agent '{}' is not registered.", executor));
        }

        let task = TaskEnvelope {
            task_id: task_id.to_string(),
            delegator_id: delegator.to_string(),
            executor_id: executor.to_string(),
            payload,
            status: "submitted".to_string(),
            artifact: None,
        };

        let mut write = self.tasks.write().unwrap();
        write.insert(task_id.to_string(), task.clone());
        Ok(task)
    }

    pub fn update_task_lifecycle(&self, task_id: &str, status: &str, artifact: Option<Value>) -> Result<TaskEnvelope, String> {
        let mut write = self.tasks.write().unwrap();
        if let Some(task) = write.get_mut(task_id) {
            task.status = status.to_string();
            if artifact.is_some() {
                task.artifact = artifact;
            }
            Ok(task.clone())
        } else {
            Err(format!("Task '{}' not found.", task_id))
        }
    }
}
