use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use tauri::State;
use crate::state::AppState;
use crate::modules::pipeline::PipelineCommand;

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub content: String,
    pub has_git: bool,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub name: String,
}

pub fn get_recording_metadata(recording_path: &Path) -> Option<RecordingMetadata> {
    let metadata_path = recording_path.join("metadata.json");
    if metadata_path.exists() {
        if let Ok(content) = fs::read_to_string(&metadata_path) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

pub fn save_recording_metadata(recording_path: &Path, name: &str) -> std::io::Result<()> {
    let metadata_path = recording_path.join("metadata.json");
    let metadata = RecordingMetadata {
        name: name.to_string(),
    };
    let content = serde_json::to_string_pretty(&metadata)?;
    fs::write(metadata_path, content)
}

fn get_all_recording_names(recordings_dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(recordings_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                 // Skip hidden
                 if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                     if name.starts_with('.') { continue; }
                 } else { continue; }

                 let metadata = get_recording_metadata(&path);
                 if let Some(m) = metadata {
                     names.push(m.name);
                 } else {
                     // Fallback to dirname if no metadata
                     if let Some(n) = path.file_name().and_then(|s| s.to_str()) {
                        names.push(n.to_string());
                     }
                 }
            }
        }
    }
    names
}

fn ensure_unique_name(base_name: &str, existing_names: &[String]) -> String {
    let mut name = base_name.to_string();
    let mut i = 1;
    while existing_names.contains(&name) {
        name = format!("{} ({})", base_name, i);
        i += 1;
    }
    name
}

/// List all recordings in current workspace
#[tauri::command]
pub async fn list_recordings(
    workspace_manager: State<'_, std::sync::Arc<tokio::sync::RwLock<crate::modules::workspace_manager::WorkspaceManager>>>,
) -> Result<Vec<RecordingInfo>, String> {
    let manager = workspace_manager.read().await;
    let workspace = manager.get_current_workspace()?
        .ok_or_else(|| "No workspace selected".to_string())?;
    
    let recordings_dir = workspace.path.join("recordings");
    
    if !recordings_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut recordings = Vec::new();
    
    let entries = fs::read_dir(&recordings_dir)
        .map_err(|e| format!("Failed to read recordings directory: {:?}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {:?}", e))?;
        let path = entry.path();
        
        if path.is_dir() {
            let recording_id = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            // Skip hidden directories (like .git or system files)
            if recording_id.starts_with('.') {
                continue;
            }

            let doc_path = path.join(format!("{}.md", &recording_id));
            let git_path = path.join(".git");
            
            // Determine display name: metadata > id
            let metadata = get_recording_metadata(&path);
            let display_name = metadata.map(|m| m.name).unwrap_or_else(|| recording_id.clone());
            
            if doc_path.exists() {
                let content = fs::read_to_string(&doc_path)
                    .unwrap_or_default();
                
                let created_at = fs::metadata(&path)
                    .and_then(|m| m.created())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                recordings.push(RecordingInfo {
                    id: recording_id,
                    name: display_name,
                    path: path.to_string_lossy().to_string(),
                    content,
                    has_git: git_path.exists(),
                    created_at,
                });
            }
        }
    }
    
    recordings.sort_by(|a, b| {
        let path_a = Path::new(&a.path);
        let path_b = Path::new(&b.path);
        let meta_a = fs::metadata(path_a).ok();
        let meta_b = fs::metadata(path_b).ok();
        
        let time_a = meta_a.and_then(|m| m.created().ok()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let time_b = meta_b.and_then(|m| m.created().ok()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        
        // Descending order (newest first)
        time_b.cmp(&time_a)
    });
    
    Ok(recordings)
}

/// Get recording content by ID
#[tauri::command]
pub async fn get_recording(
    recording_id: String,
    workspace_manager: State<'_, std::sync::Arc<tokio::sync::RwLock<crate::modules::workspace_manager::WorkspaceManager>>>,
) -> Result<String, String> {
    let manager = workspace_manager.read().await;
    let workspace = manager.get_current_workspace()?
        .ok_or_else(|| "No workspace selected".to_string())?;
    
    let doc_path = workspace.path.join("recordings").join(&recording_id).join(format!("{}.md", &recording_id));
    
    if !doc_path.exists() {
        return Err(format!("Recording not found: {}", recording_id));
    }
    
    fs::read_to_string(&doc_path)
        .map_err(|e| format!("Failed to read recording: {:?}", e))
}

/// Update recording content by ID
#[tauri::command]
pub async fn update_recording(
    recording_id: String,
    content: String,
    workspace_manager: State<'_, std::sync::Arc<tokio::sync::RwLock<crate::modules::workspace_manager::WorkspaceManager>>>,
) -> Result<(), String> {
    let manager = workspace_manager.read().await;
    let workspace = manager.get_current_workspace()?
        .ok_or_else(|| "No workspace selected".to_string())?;
    
    let recording_path = workspace.path.join("recordings").join(&recording_id);
    let doc_path = recording_path.join(format!("{}.md", &recording_id));
    
    if !recording_path.exists() {
        return Err(format!("Recording not found: {}", recording_id));
    }
    
    fs::write(&doc_path, content)
        .map_err(|e| format!("Failed to update recording: {:?}", e))
}

/// Delete recording by ID (files + RAG + metadata)
#[tauri::command]
pub async fn delete_recording(
    state: State<'_, AppState>,
    recording_id: String,
    workspace_manager: State<'_, std::sync::Arc<tokio::sync::RwLock<crate::modules::workspace_manager::WorkspaceManager>>>,
) -> Result<(), String> {
    let manager = workspace_manager.read().await;
    let workspace = manager.get_current_workspace()?
        .ok_or_else(|| "No workspace selected".to_string())?;
    
    let recording_path = workspace.path.join("recordings").join(&recording_id);
    
    // 1. Notify Pipeline to cleanup RAG and State (async)
    if let Err(e) = state.pipeline_tx.try_send(PipelineCommand::DeleteRecording { recording_id: recording_id.clone() }) {
        eprintln!("Failed to send DeleteRecording command to pipeline: {:?}", e);
    }

    if !recording_path.exists() {
        return Err(format!("Recording not found: {}", recording_id));
    }
    
    // 2. Delete Filesystem entries (includes metadata.json and everything inside)
    fs::remove_dir_all(&recording_path)
        .map_err(|e| format!("Failed to delete recording: {:?}", e))
}

/// Create a new recording with custom name (mapped via metadata)
/// Returns full RecordingInfo so frontend can update unique name immediately
#[tauri::command]
pub async fn create_recording(
    name: String,
    workspace_manager: State<'_, std::sync::Arc<tokio::sync::RwLock<crate::modules::workspace_manager::WorkspaceManager>>>,
) -> Result<RecordingInfo, String> {
    let manager = workspace_manager.read().await;
    let workspace = manager.get_current_workspace()?
        .ok_or_else(|| "No workspace selected".to_string())?;
    
    let recordings_dir = workspace.path.join("recordings");
    
    // Ensure recordings directory exists
    fs::create_dir_all(&recordings_dir)
        .map_err(|e| format!("Failed to create recordings directory: {:?}", e))?;
    
    // Generate UUID-based ID
    let recording_id = uuid::Uuid::new_v4().to_string();
    
    let recording_path = recordings_dir.join(&recording_id);
    fs::create_dir_all(&recording_path)
        .map_err(|e| format!("Failed to create recording directory: {:?}", e))?;
    
    // Determine proper Display Name
    let mut final_name = name.clone();
    
    if final_name.trim().is_empty() {
        // Default name logic
        final_name = "New Recording".to_string();
    }
    
    // Ensure uniqueness
    let existing_names = get_all_recording_names(&recordings_dir);
    final_name = ensure_unique_name(&final_name, &existing_names);

    // Save metadata
    save_recording_metadata(&recording_path, &final_name)
        .map_err(|e| {
            let _ = fs::remove_dir_all(&recording_path);
            format!("Failed to save recording metadata: {:?}", e)
        })?;
    
    // Create empty markdown file with ID as filename
    let doc_path = recording_path.join(format!("{}.md", &recording_id));
    fs::write(&doc_path, "")
        .map_err(|e| {
             let _ = fs::remove_dir_all(&recording_path);
             format!("Failed to create document: {:?}", e)
        })?;
    
    Ok(RecordingInfo {
        id: recording_id,
        name: final_name,
        path: recording_path.to_string_lossy().to_string(),
        content: "".to_string(),
        has_git: false,
        created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
    })
}

/// Rename recording (updates display name in metadata only)
#[tauri::command]
pub async fn rename_recording(
    old_id: String,
    new_id: String,
    workspace_manager: State<'_, std::sync::Arc<tokio::sync::RwLock<crate::modules::workspace_manager::WorkspaceManager>>>,
) -> Result<(), String> {
    let manager = workspace_manager.read().await;
    let workspace = manager.get_current_workspace()?
        .ok_or_else(|| "No workspace selected".to_string())?;
    
    let recording_path = workspace.path.join("recordings").join(&old_id);
    
    if !recording_path.exists() {
        return Err(format!("Recording not found: {}", old_id));
    }

    // "new_id" here effectively refs to the new *Display Name* the user wants.
    let desired_name = new_id;
    
    // Check uniqueness (exclude current name effectively, because we look at OTHER files, 
    // but here we just get all names. If I rename "A" to "A", it stays "A". 
    // If I rename "A" to "B", and "B" exists, it becomes "B (1)".
    
    // We need to be careful: get_all_recording_names reads metadata.
    // The current recording has metadata with OLD name.
    // So existing_names will contain "Old Name".
    // We want to check against existing names EXCEPT the current one 
    // (unless we are just changing case or something, but the requirement implies strict uniqueness).
    // Actually, simple "ensure_unique" against ALL names is safest. 
    // If I rename "A" to "A", existing list has "A", so it becomes "A (1)". 
    // Wait, that's bad. 
    // I should filter out the *current* recording ID from the check? 
    // `get_all_recording_names` returns Names, not IDs.
    // I need a version that returns (ID, Name).
    
    let recordings_dir = workspace.path.join("recordings");
    let mut existing_names = Vec::new();
     if let Ok(entries) = fs::read_dir(&recordings_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                 let rec_id = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                 if rec_id == old_id || rec_id.starts_with('.') { continue; } // Skip self and hidden

                 let metadata = get_recording_metadata(&path);
                 if let Some(m) = metadata {
                     existing_names.push(m.name);
                 } else {
                     existing_names.push(rec_id.to_string());
                 }
            }
        }
    }
    
    let final_name = ensure_unique_name(&desired_name, &existing_names);

    // Update metadata
    save_recording_metadata(&recording_path, &final_name)
        .map_err(|e| format!("Failed to update recording name: {:?}", e))
}
