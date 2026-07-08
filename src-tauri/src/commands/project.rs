use crate::commands::arg;
use crate::commands::config::{read_json, write_json};
use crate::models::{NovelConfig, ProjectData};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

const DIR_VELA_INTERNAL: &str = ".vela";
const DIR_PROMPTS: &str = ".vela/prompts";
const PROJECT_FILE: &str = ".vela/project.json";
const RECENT_PROJECTS_FILE: &str = "recent-projects.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub name: String,
    pub path: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectConfig {
    name: String,
    path: String,
    #[serde(default)]
    genre: String,
    #[serde(default)]
    target_audience: String,
}

pub async fn create_project(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let config: CreateProjectConfig = arg(&args, 0, "config")?;
    let project_id = uuid::Uuid::new_v4().to_string();
    let project_dir = Path::new(&config.path).join(&config.name);

    std::fs::create_dir_all(project_dir.join(DIR_VELA_INTERNAL))
        .map_err(|error| format!("Failed to create project directory: {error}"))?;
    std::fs::create_dir_all(project_dir.join(DIR_PROMPTS))
        .map_err(|error| format!("Failed to create prompts directory: {error}"))?;

    let now = now_string();
    let project = ProjectData {
        id: project_id.clone(),
        name: config.name,
        path: project_dir.to_string_lossy().to_string(),
        novel_config: NovelConfig {
            genre: config.genre,
            target_audience: config.target_audience,
            ..NovelConfig::default()
        },
        character_states: String::new(),
        created_at: now.clone(),
        updated_at: now.clone(),
        extra: Default::default(),
    };

    write_project_file(&project)?;
    set_current_project(&state, &project.path).await;
    add_recent_project(
        &state,
        RecentProject {
            name: project.name.clone(),
            path: project.path.clone(),
            updated_at: now,
        },
    )?;

    Ok(json!({ "success": true, "projectId": project_id, "projectPath": project.path }))
}

pub async fn open_project(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let project_path: String = arg(&args, 0, "projectPath")?;
    let project_dir = PathBuf::from(&project_path);
    if !project_dir.exists() {
        return Ok(json!({ "success": false, "project": null, "error": "目录不存在" }));
    }

    std::fs::create_dir_all(project_dir.join(DIR_VELA_INTERNAL))
        .map_err(|error| format!("Failed to create project metadata directory: {error}"))?;
    std::fs::create_dir_all(project_dir.join(DIR_PROMPTS))
        .map_err(|error| format!("Failed to create prompts directory: {error}"))?;

    let project_file = project_dir.join(PROJECT_FILE);
    let mut project = if project_file.exists() {
        read_json(project_file, ProjectData::default())?
    } else {
        ProjectData::default()
    };

    let now = now_string();
    if project.id.is_empty() {
        project.id = uuid::Uuid::new_v4().to_string();
    }
    if project.name.is_empty() {
        project.name = project_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());
    }
    project.path = project_path;
    if project.created_at.is_empty() {
        project.created_at = now.clone();
    }
    project.updated_at = now.clone();

    write_project_file(&project)?;
    set_current_project(&state, &project.path).await;
    add_recent_project(
        &state,
        RecentProject {
            name: project.name.clone(),
            path: project.path.clone(),
            updated_at: now,
        },
    )?;

    Ok(json!({ "success": true, "project": project }))
}

pub async fn save_project(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let _project_id: String = arg(&args, 0, "projectId")?;
    let mut data: ProjectData = arg(&args, 1, "data")?;
    if data.path.is_empty() {
        return Ok(json!({ "success": false, "error": "缺少项目路径" }));
    }

    let project_file = Path::new(&data.path).join(PROJECT_FILE);
    let existing = if project_file.exists() {
        read_json(project_file, ProjectData::default()).ok()
    } else {
        None
    };

    if data.id.is_empty() {
        data.id = existing
            .as_ref()
            .map(|project| project.id.clone())
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    }
    if data.created_at.is_empty() {
        data.created_at = existing
            .as_ref()
            .map(|project| project.created_at.clone())
            .filter(|created_at| !created_at.is_empty())
            .unwrap_or_else(now_string);
    }
    data.updated_at = now_string();

    write_project_file(&data)?;
    set_current_project(&state, &data.path).await;
    add_recent_project(
        &state,
        RecentProject {
            name: data.name.clone(),
            path: data.path.clone(),
            updated_at: data.updated_at.clone(),
        },
    )?;

    Ok(json!({ "success": true }))
}

pub async fn update_config(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    save_project(state, args).await
}

pub async fn recent_list(state: Arc<AppState>) -> Result<Value, String> {
    serde_json::to_value(load_recent_projects(&state)?).map_err(|error| error.to_string())
}

pub async fn select_folder(app: AppHandle) -> Result<Value, String> {
    let folder = app.dialog().file().blocking_pick_folder();
    let Some(folder) = folder else {
        return Ok(Value::Null);
    };

    let path = folder
        .into_path()
        .map_err(|error| format!("Invalid folder path: {error}"))?;

    Ok(json!(path.to_string_lossy().to_string()))
}

pub fn get_app_data_dir(app: AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|error| error.to_string())
}

pub fn ensure_project_dir(project_path: &str) -> Result<(), String> {
    let project_dir = Path::new(project_path);
    std::fs::create_dir_all(project_dir.join(DIR_VELA_INTERNAL))
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(project_dir.join(DIR_PROMPTS)).map_err(|error| error.to_string())
}

fn recent_projects_path(state: &AppState) -> PathBuf {
    state.vela_home.join(RECENT_PROJECTS_FILE)
}

fn load_recent_projects(state: &AppState) -> Result<Vec<RecentProject>, String> {
    read_json(recent_projects_path(state), Vec::<RecentProject>::new())
}

fn add_recent_project(state: &AppState, project: RecentProject) -> Result<(), String> {
    let mut list = load_recent_projects(state).unwrap_or_default();
    list.retain(|item| item.path != project.path);
    list.insert(0, project);
    list.truncate(20);
    write_json(recent_projects_path(state), &list)
}

fn write_project_file(project: &ProjectData) -> Result<(), String> {
    let path = Path::new(&project.path).join(PROJECT_FILE);
    write_json(path, project)
}

async fn set_current_project(state: &AppState, project_path: &str) {
    let path = PathBuf::from(project_path);
    *state.current_project_path.write().await = Some(path.clone());
    *state.current_db_path.write().await = Some(path.join(DIR_VELA_INTERNAL).join("project.db"));
}

fn now_string() -> String {
    chrono::Utc::now().to_rfc3339()
}
