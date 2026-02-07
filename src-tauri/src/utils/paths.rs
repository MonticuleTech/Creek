use std::path::PathBuf;

/// Get application data directory
/// Get application data directory
pub fn get_app_data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("top.monticule.creek")
        .join("creek")
}

/// Get recordings directory
pub fn get_recordings_dir() -> PathBuf {
    get_app_data_dir().join("recordings")
}

/// Get state database path
pub fn get_state_db_path() -> PathBuf {
    get_app_data_dir().join("state.db")
}

pub fn get_recording_doc_path(recording_id: &str) -> PathBuf {
    get_recordings_dir().join(recording_id).join(format!("{}.md", recording_id))
}
