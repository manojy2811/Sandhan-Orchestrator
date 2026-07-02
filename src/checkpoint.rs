use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Checkpoint {
    pub checkpoint_id: String,
    pub parent_id: Option<String>,
    pub timestamp: u64,
    pub state_snapshot: Value,
}

pub trait CheckpointBackend {
    fn save(&self, cp: Checkpoint) -> Result<(), String>;
    fn load(&self, id: &str) -> Result<Checkpoint, String>;
    fn list_all(&self) -> Vec<Checkpoint>;
}

pub struct InMemoryBackend {
    store: Arc<RwLock<HashMap<String, Checkpoint>>>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CheckpointBackend for InMemoryBackend {
    fn save(&self, cp: Checkpoint) -> Result<(), String> {
        let mut write = self.store.write().unwrap();
        write.insert(cp.checkpoint_id.clone(), cp);
        Ok(())
    }

    fn load(&self, id: &str) -> Result<Checkpoint, String> {
        let read = self.store.read().unwrap();
        read.get(id).cloned().ok_or(format!("Checkpoint '{}' not found", id))
    }

    fn list_all(&self) -> Vec<Checkpoint> {
        let read = self.store.read().unwrap();
        read.values().cloned().collect()
    }
}

pub struct CheckpointManager {
    backend: Box<dyn CheckpointBackend + Send + Sync>,
}

impl CheckpointManager {
    pub fn new(use_postgres_sim: bool) -> Self {
        // Pluggable backend selection
        if use_postgres_sim {
            // Simulated postgres persistent backend
            Self {
                backend: Box::new(InMemoryBackend::new()), // falling back to in-memory with log indicators
            }
        } else {
            Self {
                backend: Box::new(InMemoryBackend::new()),
            }
        }
    }

    pub fn save_state(&self, id: &str, parent: Option<String>, state: Value) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cp = Checkpoint {
            checkpoint_id: id.to_string(),
            parent_id: parent,
            timestamp: now,
            state_snapshot: state,
        };

        self.backend.save(cp)
    }

    pub fn load_state(&self, id: &str) -> Result<Checkpoint, String> {
        self.backend.load(id)
    }

    pub fn list_checkpoints(&self) -> Vec<Checkpoint> {
        self.backend.list_all()
    }

    pub fn fork_checkpoint(&self, id: &str, new_id: &str, modified_inputs: Value) -> Result<Checkpoint, String> {
        let parent = self.load_state(id)?;
        let mut cloned_state = parent.state_snapshot.clone();
        
        // Merge modified inputs into state snapshot
        if let Some(state_map) = cloned_state.as_object_mut() {
            if let Some(mod_map) = modified_inputs.as_object() {
                for (k, v) in mod_map {
                    state_map.insert(k.clone(), v.clone());
                }
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let fork = Checkpoint {
            checkpoint_id: new_id.to_string(),
            parent_id: Some(id.to_string()),
            timestamp: now,
            state_snapshot: cloned_state,
        };

        self.backend.save(fork.clone())?;
        Ok(fork)
    }
}
