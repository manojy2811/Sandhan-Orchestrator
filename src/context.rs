use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub token_count: usize,
}

impl Message {
    pub fn new(role: &str, content: &str) -> Self {
        // Simple token count approximation (char count / 4)
        let tokens = (content.len() / 4).max(1);
        Self {
            role: role.to_string(),
            content: content.to_string(),
            token_count: tokens,
        }
    }
}

pub struct ContextManager {
    pub history: Vec<Message>,
    pub max_tokens: usize,
    pub current_tokens: usize,
}

impl ContextManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            history: Vec::new(),
            max_tokens,
            current_tokens: 0,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let msg = Message::new(role, content);
        self.current_tokens += msg.token_count;
        self.history.push(msg);

        // Perform adaptive sliding window truncation if exceeding max tokens limit
        while self.current_tokens > self.max_tokens && !self.history.is_empty() {
            let removed = self.history.remove(0);
            self.current_tokens -= removed.token_count;
        }
    }

    pub fn get_messages(&self) -> &Vec<Message> {
        &self.history
    }
}

#[derive(Clone)]
pub struct ContextCache {
    store: Arc<RwLock<HashMap<String, String>>>,
}

impl ContextCache {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let read = self.store.read().unwrap();
        read.get(key).cloned()
    }

    pub fn insert(&self, key: String, val: String) {
        let mut write = self.store.write().unwrap();
        write.insert(key, val);
    }
}
