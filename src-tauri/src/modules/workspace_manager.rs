use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspaces: Vec<Workspace>,
    pub current_workspace_id: Option<String>,
}

pub struct WorkspaceManager {
    app_handle: AppHandle,
}

impl WorkspaceManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    fn get_config_dir(&self) -> Result<PathBuf, String> {
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        let creek_dir = app_data_dir.join("creek");
        if !creek_dir.exists() {
            fs::create_dir_all(&creek_dir)
                .map_err(|e| format!("Failed to create creek directory: {}", e))?;
        }

        Ok(creek_dir)
    }

    fn get_config_path(&self) -> Result<PathBuf, String> {
        Ok(self.get_config_dir()?.join("workspaces.json"))
    }

    fn load_config(&self) -> Result<WorkspaceConfig, String> {
        let config_path = self.get_config_path()?;

        if !config_path.exists() {
            return Ok(WorkspaceConfig {
                workspaces: Vec::new(),
                current_workspace_id: None,
            });
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
    }

    fn save_config(&self, config: &WorkspaceConfig) -> Result<(), String> {
        let config_path = self.get_config_path()?;
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, content).map_err(|e| format!("Failed to write config: {}", e))
    }

    pub fn create_workspace(&self, name: String) -> Result<Workspace, String> {
        let mut config = self.load_config()?;

        let id = uuid::Uuid::new_v4().to_string();
        let workspace_dir = self.get_config_dir()?.join("workspaces").join(&id);

        if workspace_dir.exists() {
            return Err("Workspace directory already exists".to_string());
        }

        fs::create_dir_all(&workspace_dir)
            .map_err(|e| format!("Failed to create workspace directory: {}", e))?;

        fs::create_dir_all(workspace_dir.join("recordings"))
            .map_err(|e| format!("Failed to create recordings directory: {}", e))?;

        fs::create_dir_all(workspace_dir.join("uploads"))
            .map_err(|e| format!("Failed to create uploads directory: {}", e))?;

        let workspace = Workspace {
            id: id.clone(),
            name: if name.is_empty() {
                "New Workspace".to_string()
            } else {
                name
            },
            created_at: Utc::now().timestamp_millis(),
            path: workspace_dir,
        };

        config.workspaces.push(workspace.clone());

        if config.current_workspace_id.is_none() {
            config.current_workspace_id = Some(id);
        }

        self.save_config(&config)?;

        Ok(workspace)
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, String> {
        let config = self.load_config()?;
        Ok(config.workspaces)
    }

    pub fn rename_workspace(&self, id: String, new_name: String) -> Result<(), String> {
        let mut config = self.load_config()?;

        let workspace = config
            .workspaces
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?;

        workspace.name = new_name;

        self.save_config(&config)
    }

    pub fn delete_workspace(&self, id: String) -> Result<(), String> {
        let mut config = self.load_config()?;

        let workspace = config
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?;

        // If we're deleting the current workspace, clear the current workspace ID
        if let Some(current_id) = &config.current_workspace_id {
            if current_id == &id {
                config.current_workspace_id = None;
            }
        }

        if workspace.path.exists() {
            fs::remove_dir_all(&workspace.path)
                .map_err(|e| format!("Failed to remove workspace directory: {}", e))?;
        }

        config.workspaces.retain(|w| w.id != id);

        self.save_config(&config)
    }

    pub fn get_current_workspace(&self) -> Result<Option<Workspace>, String> {
        let config = self.load_config()?;

        if let Some(id) = config.current_workspace_id {
            let workspace = config.workspaces.iter().find(|w| w.id == id).cloned();
            Ok(workspace)
        } else {
            Ok(None)
        }
    }

    pub fn set_current_workspace(&self, id: String) -> Result<(), String> {
        let mut config = self.load_config()?;

        if !config.workspaces.iter().any(|w| w.id == id) {
            return Err("Workspace not found".to_string());
        }

        config.current_workspace_id = Some(id);

        self.save_config(&config)
    }

    pub fn initialize_default_workspace(&self) -> Result<(), String> {
        // Just ensure the config directory exists - don't auto-create workspaces
        let _ = self.get_config_dir()?;
        Ok(())
    }

    pub fn get_workspace_uploads_path(&self, workspace_id: String) -> Result<String, String> {
        let config = self.load_config()?;
        let workspace = config
            .workspaces
            .iter()
            .find(|w| w.id == workspace_id)
            .ok_or_else(|| "Workspace not found".to_string())?;

        if !workspace.path.join("uploads").exists() {
            std::fs::create_dir_all(workspace.path.join("uploads"))
                .map_err(|e| format!("Failed to create uploads directory: {}", e))?;
        }

        Ok(format!("creek/workspaces/{}/uploads", workspace_id))
    }
}
