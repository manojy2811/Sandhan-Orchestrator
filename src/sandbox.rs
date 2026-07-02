use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tempfile::TempDir;
use serde_json::{json, Value};

pub struct Sandbox {
    workspace_dir: TempDir,
}

impl Sandbox {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp = tempfile::tempdir()?;
        Ok(Self { workspace_dir: temp })
    }

    pub fn get_workspace_path(&self) -> String {
        self.workspace_dir.path().to_string_lossy().to_string()
    }

    pub async fn execute_command(&self, cmd: &str, args: Vec<String>) -> Result<Value, String> {
        // Enforce safe command routing (no arbitrary system tools)
        let allowed_commands = ["git", "echo", "python", "node", "rustc", "cargo"];
        if !allowed_commands.contains(&cmd) {
            return Err(format!("Command '{}' is blocked by sandbox policy.", cmd));
        }

        // Configure subprocess execution
        let mut child = Command::new(cmd)
            .args(&args)
            .current_dir(self.workspace_dir.path())
            .env_clear() // Strip host environment variables for isolation
            .env("PATH", std::env::var("PATH").unwrap_or_default()) // retain PATH for locating binaries
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        // Enforce execution timeout constraint (5 seconds)
        match timeout(Duration::from_secs(5), child.wait()).await {
            Ok(Ok(status)) => {
                let output = child
                    .wait_with_output()
                    .await
                    .map_err(|e| format!("Failed to capture process output: {}", e))?;

                let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(json!({
                    "exit_code": status.code().unwrap_or(-1),
                    "stdout": stdout_str,
                    "stderr": stderr_str,
                    "success": status.success()
                }))
            }
            Ok(Err(e)) => Err(format!("Subprocess execution error: {}", e)),
            Err(_) => {
                // Terminate timed out subprocess
                let _ = child.kill().await;
                Err("Process execution timed out after 5 seconds.".to_string())
            }
        }
    }
}
