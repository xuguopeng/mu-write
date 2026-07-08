use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use tauri::AppHandle;

pub mod config;
pub mod db;
pub mod fs;
pub mod import;
pub mod kb;
pub mod llm;
pub mod mcp;
pub mod project;

pub async fn dispatch(
    app: AppHandle,
    state: Arc<AppState>,
    channel: String,
    args: Vec<Value>,
) -> Result<Value, String> {
    match channel.as_str() {
        "config:get" => config::get_config(state).await,
        "config:set" => config::set_config(state, args).await,
        "config:get-vela-home" => config::get_vela_home(state).await,
        "llm:list-models" => llm::list_models(state).await,
        "llm:save-model" => llm::save_model(state, args).await,
        "llm:delete-model" => llm::delete_model(state, args).await,
        "llm:set-default-model" => llm::set_default_model(state, args).await,
        "llm:get-default-model" => llm::get_default_model(state).await,
        "llm:set-default-embedding-model" => llm::set_default_embedding_model(state, args).await,
        "llm:get-default-embedding-model" => llm::get_default_embedding_model(state).await,
        "llm:test-connection" => llm::test_connection(state, args).await,
        "llm:generate" => llm::generate(state, args).await,
        "llm:generate-stream" => llm::generate_stream(app, state, args).await,
        "llm:cancel" => llm::cancel(state, args).await,
        "fs:read-file" => fs::read_file(args).await,
        "fs:write-file" => fs::write_file(args).await,
        "fs:list-dir" => fs::list_dir(args).await,
        "fs:mkdir" => fs::mkdir(args).await,
        "fs:check-exists" => fs::check_exists(args).await,
        "fs:read-json" => fs::read_json(args).await,
        "fs:write-json" => fs::write_json(args).await,
        "project:create" => project::create_project(state, args).await,
        "project:open" => project::open_project(state, args).await,
        "project:save" => project::save_project(state, args).await,
        "project:update-config" => project::update_config(state, args).await,
        "project:recent-list" => project::recent_list(state).await,
        "dialog:select-folder" => project::select_folder(app).await,
        channel if channel.starts_with("db:") => db::dispatch_db(state, channel, args).await,
        channel if channel.starts_with("kb:") => kb::dispatch_kb(state, channel, args).await,
        channel
            if channel.starts_with("import:")
                || channel.starts_with("dialog:select-novel")
                || channel == "dialog:select-files"
                || channel == "dialog:select-import-folder" =>
        {
            import::dispatch_import(app, channel, args).await
        }
        channel if channel.starts_with("mcp:") => mcp::dispatch_mcp(state, channel, args).await,
        _ => Err(format!("Unsupported IPC channel: {channel}")),
    }
}

pub async fn dispatch_cli(
    state: Arc<AppState>,
    channel: String,
    args: Vec<Value>,
) -> Result<Value, String> {
    match channel.as_str() {
        "config:get" => config::get_config(state).await,
        "config:set" => config::set_config(state, args).await,
        "config:get-vela-home" => config::get_vela_home(state).await,
        "llm:list-models" => llm::list_models(state).await,
        "llm:save-model" => llm::save_model(state, args).await,
        "llm:delete-model" => llm::delete_model(state, args).await,
        "llm:set-default-model" => llm::set_default_model(state, args).await,
        "llm:get-default-model" => llm::get_default_model(state).await,
        "llm:set-default-embedding-model" => llm::set_default_embedding_model(state, args).await,
        "llm:get-default-embedding-model" => llm::get_default_embedding_model(state).await,
        "llm:test-connection" => llm::test_connection(state, args).await,
        "llm:generate" => llm::generate(state, args).await,
        "llm:generate-stream" => Err(
            "llm:generate-stream requires the desktop app event loop; use llm:generate from CLI"
                .to_string(),
        ),
        "llm:cancel" => llm::cancel(state, args).await,
        "fs:read-file" => fs::read_file(args).await,
        "fs:write-file" => fs::write_file(args).await,
        "fs:list-dir" => fs::list_dir(args).await,
        "fs:mkdir" => fs::mkdir(args).await,
        "fs:check-exists" => fs::check_exists(args).await,
        "fs:read-json" => fs::read_json(args).await,
        "fs:write-json" => fs::write_json(args).await,
        "project:create" => project::create_project(state, args).await,
        "project:open" => project::open_project(state, args).await,
        "project:save" => project::save_project(state, args).await,
        "project:update-config" => project::update_config(state, args).await,
        "project:recent-list" => project::recent_list(state).await,
        "dialog:select-folder" => {
            Err("dialog:select-folder requires the desktop app UI".to_string())
        }
        channel if channel.starts_with("db:") => db::dispatch_db(state, channel, args).await,
        channel if channel.starts_with("kb:") => kb::dispatch_kb(state, channel, args).await,
        channel
            if channel.starts_with("import:")
                || channel.starts_with("dialog:select-novel")
                || channel == "dialog:select-files"
                || channel == "dialog:select-import-folder" =>
        {
            Err(format!("{channel} requires the desktop app UI"))
        }
        channel if channel.starts_with("mcp:") => mcp::dispatch_mcp(state, channel, args).await,
        _ => Err(format!("Unsupported CLI channel: {channel}")),
    }
}

pub fn arg<T>(args: &[Value], index: usize, name: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let value = args
        .get(index)
        .ok_or_else(|| format!("Missing argument: {name}"))?;
    serde_json::from_value(value.clone())
        .map_err(|error| format!("Invalid argument {name}: {error}"))
}

pub fn optional_arg<T>(args: &[Value], index: usize) -> Result<Option<T>, String>
where
    T: serde::de::DeserializeOwned,
{
    match args.get(index) {
        Some(value) if !value.is_null() => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|error| format!("Invalid argument at {index}: {error}")),
        _ => Ok(None),
    }
}
