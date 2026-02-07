use crate::modules::workspace_manager::{Workspace, WorkspaceManager};
use tauri::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn create_workspace(
    name: String,
    workspace_manager: State<'_, Arc<RwLock<WorkspaceManager>>>,
) -> Result<Workspace, String> {
    let manager = workspace_manager.read().await;
    manager.create_workspace(name)
}

#[tauri::command]
pub async fn list_workspaces(
    workspace_manager: State<'_, Arc<RwLock<WorkspaceManager>>>,
) -> Result<Vec<Workspace>, String> {
    let manager = workspace_manager.read().await;
    manager.list_workspaces()
}

#[tauri::command]
pub async fn rename_workspace(
    id: String,
    new_name: String,
    workspace_manager: State<'_, Arc<RwLock<WorkspaceManager>>>,
) -> Result<(), String> {
    let manager = workspace_manager.read().await;
    manager.rename_workspace(id, new_name)
}

#[tauri::command]
pub async fn delete_workspace(
    id: String,
    workspace_manager: State<'_, Arc<RwLock<WorkspaceManager>>>,
) -> Result<(), String> {
    let manager = workspace_manager.read().await;
    manager.delete_workspace(id)
}

#[tauri::command]
pub async fn get_current_workspace(
    workspace_manager: State<'_, Arc<RwLock<WorkspaceManager>>>,
) -> Result<Option<Workspace>, String> {
    let manager = workspace_manager.read().await;
    manager.get_current_workspace()
}

#[tauri::command]
pub async fn set_current_workspace(
    id: String,
    workspace_manager: State<'_, Arc<RwLock<WorkspaceManager>>>,
) -> Result<(), String> {
    let manager = workspace_manager.read().await;
    manager.set_current_workspace(id)
}
