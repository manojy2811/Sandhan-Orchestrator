use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryStats {
    pub execution_count: usize,
    pub error_count: usize,
    pub total_tokens_processed: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReasoningStep {
    pub timestamp: u64,
    pub node_name: String,
    pub decision: String,
    pub justification: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditRecord {
    pub timestamp: u64,
    pub user_role: String,
    pub action: String,
    pub status: String,
}

#[derive(Clone)]
pub struct ObservabilityTracker {
    stats: Arc<RwLock<TelemetryStats>>,
    traces: Arc<RwLock<Vec<ReasoningStep>>>,
    audit_logs: Arc<RwLock<Vec<AuditRecord>>>,
}

impl ObservabilityTracker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(TelemetryStats {
                execution_count: 0,
                error_count: 0,
                total_tokens_processed: 0,
            })),
            traces: Arc::new(RwLock::new(Vec::new())),
            audit_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn record_execution(&self, tokens: usize, is_error: bool) {
        let mut write = self.stats.write().unwrap();
        write.execution_count += 1;
        write.total_tokens_processed += tokens;
        if is_error {
            write.error_count += 1;
        }
    }

    pub fn get_telemetry(&self) -> TelemetryStats {
        let read = self.stats.read().unwrap();
        read.clone()
    }

    pub fn add_reasoning_step(&self, node: &str, decision: &str, justification: &str) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let step = ReasoningStep {
            timestamp: now,
            node_name: node.to_string(),
            decision: decision.to_string(),
            justification: justification.to_string(),
        };

        let mut write = self.traces.write().unwrap();
        write.push(step);
    }

    pub fn get_reasoning_traces(&self) -> Vec<ReasoningStep> {
        let read = self.traces.read().unwrap();
        read.clone()
    }

    pub fn record_audit(&self, user_role: &str, action: &str, status: &str) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let record = AuditRecord {
            timestamp: now,
            user_role: user_role.to_string(),
            action: action.to_string(),
            status: status.to_string(),
        };

        let mut write = self.audit_logs.write().unwrap();
        write.push(record);
    }

    pub fn get_audit_logs(&self) -> Vec<AuditRecord> {
        let read = self.audit_logs.read().unwrap();
        read.clone()
    }
}
