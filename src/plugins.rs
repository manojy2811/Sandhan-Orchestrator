use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub registered_commands: Vec<String>,
    pub dependencies: Vec<String>, // New: Tracks required package dependencies
}

#[derive(Clone)]
pub struct PluginManager {
    installed: Arc<RwLock<HashMap<String, Plugin>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let manager = Self {
            installed: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Seed default core plugin
        let core = Plugin {
            name: "core-utils".to_string(),
            version: "1.0.0".to_string(),
            description: "Standard agent utility helper hooks.".to_string(),
            registered_commands: vec!["format".to_string(), "lint".to_string()],
            dependencies: vec![],
        };
        manager.installed.write().unwrap().insert(core.name.clone(), core);
        
        manager
    }

    pub fn install_plugin(
        &self, 
        name: &str, 
        desc: &str, 
        cmds: Vec<String>, 
        deps: Vec<String>
    ) -> Result<Plugin, String> {
        // Resolve dependencies
        let read = self.installed.read().unwrap();
        for dep in &deps {
            if !read.contains_key(dep) {
                return Err(format!(
                    "Dependency error: Cannot install '{}'. Missing dependent plugin '{}'.", 
                    name, dep
                ));
            }
        }

        let plugin = Plugin {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: desc.to_string(),
            registered_commands: cmds,
            dependencies: deps,
        };

        drop(read);
        let mut write = self.installed.write().unwrap();
        write.insert(name.to_string(), plugin.clone());
        Ok(plugin)
    }

    pub fn uninstall_plugin(&self, name: &str) -> Result<(), String> {
        let mut write = self.installed.write().unwrap();
        
        // Check if any other installed plugin depends on this one
        for (installed_name, plugin) in write.iter() {
            if plugin.dependencies.contains(&name.to_string()) {
                return Err(format!(
                    "Uninstall blocker: Cannot remove '{}'. Plugin '{}' depends on it.", 
                    name, installed_name
                ));
            }
        }

        if write.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found.", name))
        }
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
