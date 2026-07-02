use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub registered_commands: Vec<String>,
}

#[derive(Clone)]
pub struct PluginManager {
    installed: Arc<RwLock<HashMap<String, Plugin>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut manager = Self {
            installed: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Seed default core plugin
        let core = Plugin {
            name: "core-utils".to_string(),
            version: "1.0.0".to_string(),
            description: "Standard agent utility helper hooks.".to_string(),
            registered_commands: vec!["format".to_string(), "lint".to_string()],
        };
        manager.installed.write().unwrap().insert(core.name.clone(), core);
        
        manager
    }

    pub fn install_plugin(&self, name: &str, desc: &str, cmds: Vec<String>) -> Plugin {
        let plugin = Plugin {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: desc.to_string(),
            registered_commands: cmds,
        };

        let mut write = self.installed.write().unwrap();
        write.insert(name.to_string(), plugin.clone());
        plugin
    }

    pub fn list_installed(&self) -> Vec<Plugin> {
        let read = self.installed.read().unwrap();
        read.values().cloned().collect()
    }

    pub fn run_plugin_command(&self, plugin_name: &str, cmd: &str) -> Result<String, String> {
        let read = self.installed.read().unwrap();
        if let Some(plugin) = read.get(plugin_name) {
            if plugin.registered_commands.contains(&cmd.to_string()) {
                Ok(format!("Successfully executed hook [{}] of plugin '{}'.", cmd, plugin_name))
            } else {
                Err(format!("Plugin '{}' does not support command '{}'.", plugin_name, cmd))
            }
        } else {
            Err(format!("Plugin '{}' is not installed.", plugin_name))
        }
    }
}
