use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFolderConfig {
    pub watch_path: String,
    pub action_list_id: i64,
    pub output_path: String,
    pub file_pattern: String,
}

pub fn start_hot_folder_watcher(_config: &HotFolderConfig) -> Result<(), String> {
    Err("Hot folder watcher requires the 'notify' crate (not installed in this build).".to_string())
}

pub fn stop_hot_folder_watcher() -> Result<(), String> {
    Ok(())
}
