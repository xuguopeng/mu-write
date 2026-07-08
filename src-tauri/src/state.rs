use crate::models::ModelProfile;
use reqwest::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::sync::RwLock;

pub struct AppState {
    pub vela_home: PathBuf,
    pub current_project_path: RwLock<Option<PathBuf>>,
    pub current_db_path: RwLock<Option<PathBuf>>,
    pub http_client: Client,
    pub model_configs: RwLock<HashMap<String, ModelProfile>>,
    pub default_model_id: RwLock<Option<String>>,
    pub default_embedding_model_id: RwLock<Option<String>>,
    pub stream_cancellations: RwLock<HashSet<String>>,
    pub mcp_status: RwLock<HashMap<String, String>>,
    pub mcp_configs: RwLock<HashMap<String, Value>>,
    pub mcp_tools: RwLock<Vec<Value>>,
    pub mcp_resources: RwLock<Vec<Value>>,
}

impl AppState {
    pub fn new() -> Self {
        let vela_home = dirs::home_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join(".vela");

        let _ = std::fs::create_dir_all(&vela_home);

        Self {
            vela_home,
            current_project_path: RwLock::new(None),
            current_db_path: RwLock::new(None),
            http_client: Client::new(),
            model_configs: RwLock::new(HashMap::new()),
            default_model_id: RwLock::new(None),
            default_embedding_model_id: RwLock::new(None),
            stream_cancellations: RwLock::new(HashSet::new()),
            mcp_status: RwLock::new(HashMap::new()),
            mcp_configs: RwLock::new(HashMap::new()),
            mcp_tools: RwLock::new(Vec::new()),
            mcp_resources: RwLock::new(Vec::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
