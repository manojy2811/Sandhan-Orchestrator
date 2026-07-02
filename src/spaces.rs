use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Space {
    pub name: String,
    pub path: String,
    pub branch: String,
    pub pr_url: Option<String>,
}

#[derive(Clone)]
pub struct SpacesManager {
    base_dir: PathBuf,
    spaces: Arc<RwLock<HashMap<String, Space>>>,
}

impl SpacesManager {
    pub fn new(base_dir: PathBuf) -> Self {
        // Ensure the base directory for spaces exists
        let _ = fs::create_dir_all(&base_dir);
        Self {
            base_dir,
            spaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_space(&self, name: &str) -> Result<Space, String> {
        let space_path = self.base_dir.join(name);
        fs::create_dir_all(&space_path)
            .map_err(|e| format!("Failed to create space directory: {}", e))?;

        let space = Space {
            name: name.to_string(),
            path: space_path.to_string_lossy().to_string(),
            branch: "main".to_string(),
            pr_url: None,
        };

        let mut write = self.spaces.write().unwrap();
        write.insert(name.to_string(), space.clone());
        Ok(space)
    }

    pub fn checkout_branch(&self, name: &str, branch: &str) -> Result<Space, String> {
        let mut write = self.spaces.write().unwrap();
        if let Some(space) = write.get_mut(name) {
            space.branch = branch.to_string();
            Ok(space.clone())
        } else {
            Err(format!("Space '{}' does not exist.", name))
        }
    }

    pub fn link_pull_request(&self, name: &str, pr_id: usize) -> Result<Space, String> {
        let mut write = self.spaces.write().unwrap();
        if let Some(space) = write.get_mut(name) {
            let pr_url = format!("https://github.com/org/repo/pull/{}", pr_id);
            space.pr_url = Some(pr_url);
            Ok(space.clone())
        } else {
            Err(format!("Space '{}' does not exist.", name))
        }
    }
}
