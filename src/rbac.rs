use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum UserRole {
    User,
    Operator,
    Admin,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub username: String,
    pub role: UserRole,
    pub preferences: HashMap<String, String>,
}

#[derive(Clone)]
pub struct SecurityManager {
    profiles: Arc<RwLock<HashMap<String, UserProfile>>>,
}

impl SecurityManager {
    pub fn new() -> Self {
        let mut manager = Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Seed default admin profile
        let mut prefs = HashMap::new();
        prefs.insert("theme".to_string(), "dark".to_string());
        let admin = UserProfile {
            username: "admin".to_string(),
            role: UserRole::Admin,
            preferences: prefs,
        };
        manager.profiles.write().unwrap().insert(admin.username.clone(), admin);

        manager
    }

    pub fn authorize_action(&self, username: &str, action: &str) -> Result<UserProfile, String> {
        let read = self.profiles.read().unwrap();
        if let Some(profile) = read.get(username) {
            match profile.role {
                UserRole::Admin => Ok(profile.clone()), // Admins can perform any action
                UserRole::Operator => {
                    if action == "delete_plugin" {
                        Err(format!("Access Denied: Role 'Operator' is not permitted to perform '{}'.", action))
                    } else {
                        Ok(profile.clone())
                    }
                }
                UserRole::User => {
                    if action == "execute" || action == "delete_plugin" {
                        Err(format!("Access Denied: Role 'User' is not permitted to perform '{}'.", action))
                    } else {
                        Ok(profile.clone())
                    }
                }
            }
        } else {
            Err(format!("Authentication Failure: User '{}' not found.", username))
        }
    }

    pub fn mock_jira_slack_alert(&self, action: &str, status: &str) -> String {
        format!("[Mock Notification] Alert dispatched to #jira-feed & Slack channel. Action: '{}', Status: '{}'", action, status)
    }
}
