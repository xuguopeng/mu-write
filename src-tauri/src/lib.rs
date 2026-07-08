pub mod commands;
pub mod models;
pub mod state;

use crate::models::{LLMRequest, LLMResponse, ModelProfile};
use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tracing::info;

#[tauri::command]
async fn vela_invoke(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    channel: String,
    args: Vec<Value>,
) -> Result<Value, String> {
    commands::dispatch(app, state.inner().clone(), channel, args).await
}

#[tauri::command]
async fn llm_generate(
    state: State<'_, Arc<AppState>>,
    request: LLMRequest,
) -> Result<LLMResponse, String> {
    commands::llm::generate_response(state.inner().clone(), request).await
}

#[tauri::command]
async fn llm_list_models(state: State<'_, Arc<AppState>>) -> Result<Vec<ModelProfile>, String> {
    commands::llm::load_models_into_state(state.inner()).await?;
    let models = state.model_configs.read().await;
    Ok(models.values().cloned().collect())
}

#[tauri::command]
async fn llm_save_model(
    state: State<'_, Arc<AppState>>,
    model: ModelProfile,
) -> Result<(), String> {
    commands::llm::save_model_direct(state.inner(), model).await
}

#[tauri::command]
async fn llm_delete_model(state: State<'_, Arc<AppState>>, model_id: String) -> Result<(), String> {
    commands::llm::delete_model_direct(state.inner(), model_id).await
}

#[tauri::command]
async fn llm_test_connection(
    state: State<'_, Arc<AppState>>,
    model: ModelProfile,
) -> Result<bool, String> {
    let messages = vec![models::LLMMessage {
        role: "user".to_string(),
        content: "Say hello and nothing else.".to_string(),
    }];

    let response =
        commands::llm::call_llm_provider(&state.http_client, &model, &messages, 0.7, 10, false)
            .await
            .map_err(|error| error.to_string())?;

    Ok(response.success)
}

#[tauri::command]
fn get_app_data_dir(app: AppHandle) -> Result<String, String> {
    commands::project::get_app_data_dir(app)
}

#[tauri::command]
fn ensure_project_dir(project_path: &str) -> Result<(), String> {
    commands::project::ensure_project_dir(project_path)
}

#[tauri::command]
fn init_database() -> Vec<String> {
    commands::db::init_database()
}

#[tauri::command]
fn export_file(
    app: AppHandle,
    content: String,
    filename: String,
    format: String,
) -> Result<String, String> {
    let ext = match format.as_str() {
        "txt" => "txt",
        "md" => "md",
        _ => {
            return Err(format!(
                "Unsupported export format: {format}. Use 'txt' or 'md'."
            ))
        }
    };

    let default_filename = if filename.ends_with(&format!(".{ext}")) {
        filename
    } else {
        format!("{filename}.{ext}")
    };

    let file_path = app
        .dialog()
        .file()
        .set_file_name(&default_filename)
        .add_filter(&format!("{ext} files"), &[ext])
        .blocking_save_file()
        .ok_or_else(|| "Save dialog cancelled".to_string())?;

    let path_buf = file_path
        .into_path()
        .map_err(|error| format!("Invalid file path: {error}"))?;
    let path_str = path_buf.to_string_lossy().to_string();

    commands::fs::write_text(&path_str, &content)?;
    Ok(path_str)
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    commands::fs::read_text(&path)
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    commands::fs::write_text(&path, &content)
}

pub fn run() {
    tracing_subscriber::fmt().with_env_filter("info").init();
    info!("Starting AI Novel Generator...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .manage(Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            vela_invoke,
            llm_generate,
            llm_list_models,
            llm_save_model,
            llm_delete_model,
            llm_test_connection,
            get_app_data_dir,
            ensure_project_dir,
            init_database,
            export_file,
            read_text_file,
            write_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
