use crate::commands::arg;
use crate::models::GlobalConfig;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub async fn get_config(state: Arc<AppState>) -> Result<Value, String> {
    serde_json::to_value(read_config(&state)?).map_err(|error| error.to_string())
}

pub async fn set_config(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let patch: Value = arg(&args, 0, "config")?;
    let mut existing =
        serde_json::to_value(read_config(&state)?).map_err(|error| error.to_string())?;
    merge_value(&mut existing, patch);
    write_json(config_path(&state), &existing)?;

    if let Ok(config) = serde_json::from_value::<GlobalConfig>(existing) {
        *state.default_model_id.write().await = config.default_model_id;
        *state.default_embedding_model_id.write().await = config.default_embedding_model_id;
    }

    Ok(json!({ "success": true }))
}

pub async fn get_vela_home(state: Arc<AppState>) -> Result<Value, String> {
    Ok(json!(state.vela_home.to_string_lossy().to_string()))
}

pub fn read_config(state: &AppState) -> Result<GlobalConfig, String> {
    read_json(config_path(state), GlobalConfig::default())
}

pub fn write_config(state: &AppState, config: &GlobalConfig) -> Result<(), String> {
    write_json(config_path(state), config)
}

pub fn config_path(state: &AppState) -> PathBuf {
    state.vela_home.join("config.json")
}

pub fn read_json<T>(path: PathBuf, default: T) -> Result<T, String>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    if !path.exists() {
        write_json(path, &default)?;
        return Ok(default);
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read '{}': {error}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(default);
    }

    serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse '{}': {error}", path.display()))
}

pub fn write_json<T>(path: PathBuf, data: &T) -> Result<(), String>
where
    T: serde::Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create '{}': {error}", parent.display()))?;
    }

    let temp_path = path.with_extension("tmp");
    let content = serde_json::to_string_pretty(data).map_err(|error| error.to_string())?;
    std::fs::write(&temp_path, content)
        .map_err(|error| format!("Failed to write '{}': {error}", temp_path.display()))?;
    std::fs::rename(&temp_path, &path).map_err(|error| {
        format!(
            "Failed to replace '{}' with '{}': {error}",
            temp_path.display(),
            path.display()
        )
    })
}

fn merge_value(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target_map), Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                match target_map.get_mut(&key) {
                    Some(existing) => merge_value(existing, value),
                    None => {
                        target_map.insert(key, value);
                    }
                }
            }
        }
        (target_slot, patch_value) => *target_slot = patch_value,
    }
}
