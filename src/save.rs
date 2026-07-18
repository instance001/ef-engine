use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::EngineState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineSaveEnvelope {
    pub save_version: u32,
    pub host_id: String,
    #[serde(default)]
    pub notes: Vec<String>,
    pub state: EngineState,
}

pub fn save_state_to_path(
    path: impl AsRef<Path>,
    payload: &EngineSaveEnvelope,
) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create save folder: {}.", err))?;
    }
    let json = serde_json::to_string_pretty(payload)
        .map_err(|err| format!("Could not serialize save payload: {}.", err))?;
    fs::write(path, json).map_err(|err| format!("Could not write save file: {}.", err))
}

pub fn load_state_from_path(path: impl AsRef<Path>) -> Result<EngineSaveEnvelope, String> {
    let json = fs::read_to_string(path.as_ref())
        .map_err(|err| format!("Could not read save file: {}.", err))?;
    serde_json::from_str(&json).map_err(|err| format!("Could not parse save file: {}.", err))
}
