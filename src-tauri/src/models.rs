use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FileNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub default_model_id: Option<String>,
    #[serde(default)]
    pub default_embedding_model_id: Option<String>,
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: i32,
    #[serde(default = "default_editor_font_family")]
    pub editor_font_family: String,
    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            default_model_id: None,
            default_embedding_model_id: None,
            editor_font_size: default_editor_font_size(),
            editor_font_family: default_editor_font_family(),
            auto_save_interval: default_auto_save_interval(),
            proxy: None,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default, alias = "model")]
    pub model_name: String,
    #[serde(default, alias = "api_key")]
    pub api_key: String,
    #[serde(default, alias = "base_url")]
    pub base_url: String,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens", alias = "max_tokens")]
    pub max_tokens: i32,
    #[serde(default)]
    pub purposes: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ModelProfile {
    pub fn normalized_id(&self) -> String {
        if !self.id.is_empty() {
            return self.id.clone();
        }
        if !self.name.is_empty() {
            return self.name.clone();
        }
        if !self.model_name.is_empty() {
            return self.model_name.clone();
        }
        uuid::Uuid::new_v4().to_string()
    }

    pub fn display_model_name(&self) -> &str {
        if !self.model_name.is_empty() {
            self.model_name.as_str()
        } else if !self.name.is_empty() {
            self.name.as_str()
        } else {
            self.id.as_str()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectData {
    #[serde(default = "default_project_id")]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub novel_config: NovelConfig,
    #[serde(default)]
    pub character_states: String,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    #[serde(default = "default_timestamp")]
    pub updated_at: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for ProjectData {
    fn default() -> Self {
        let now = default_timestamp();
        Self {
            id: default_project_id(),
            name: String::new(),
            path: String::new(),
            novel_config: NovelConfig::default(),
            character_states: String::new(),
            created_at: now.clone(),
            updated_at: now,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelConfig {
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub sub_genre: String,
    #[serde(default)]
    pub target_audience: String,
    #[serde(default = "default_total_chapters")]
    pub total_chapters: i32,
    #[serde(default = "default_words_per_chapter")]
    pub words_per_chapter: i32,
    #[serde(default = "default_plot_structure")]
    pub plot_structure: String,
    #[serde(
        default = "default_narrative_pov",
        rename = "narrativePOV",
        alias = "narrativePov"
    )]
    pub narrative_pov: String,
    #[serde(default)]
    pub core_outline: String,
    #[serde(default)]
    pub world_setting: String,
    #[serde(default)]
    pub golden_finger: String,
    #[serde(default)]
    pub protagonist_profile: String,
    #[serde(default)]
    pub global_guidance: String,
    #[serde(default)]
    pub writing_style: Option<String>,
    #[serde(default)]
    pub reference_works: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for NovelConfig {
    fn default() -> Self {
        Self {
            genre: String::new(),
            sub_genre: String::new(),
            target_audience: String::new(),
            total_chapters: default_total_chapters(),
            words_per_chapter: default_words_per_chapter(),
            plot_structure: default_plot_structure(),
            narrative_pov: default_narrative_pov(),
            core_outline: String::new(),
            world_setting: String::new(),
            golden_finger: String::new(),
            protagonist_profile: String::new(),
            global_guidance: String::new(),
            writing_style: None,
            reference_works: None,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMRequest {
    #[serde(default, alias = "model_id")]
    pub model_id: String,
    #[serde(default)]
    pub model: Option<ModelProfile>,
    pub messages: Vec<LLMMessage>,
    pub temperature: Option<f32>,
    #[serde(alias = "max_tokens")]
    pub max_tokens: Option<i32>,
    pub stream: Option<bool>,
    #[serde(default)]
    pub response_format: Option<Value>,
    #[serde(default)]
    pub thinking: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMResponse {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    #[serde(alias = "prompt_tokens")]
    pub prompt_tokens: i32,
    #[serde(alias = "completion_tokens")]
    pub completion_tokens: i32,
    #[serde(alias = "total_tokens")]
    pub total_tokens: i32,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_editor_font_size() -> i32 {
    16
}

fn default_editor_font_family() -> String {
    "Sora".to_string()
}

fn default_auto_save_interval() -> i32 {
    30
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> i32 {
    4096
}

fn default_project_id() -> String {
    "main".to_string()
}

fn default_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_total_chapters() -> i32 {
    100
}

fn default_words_per_chapter() -> i32 {
    3000
}

fn default_plot_structure() -> String {
    "three_act".to_string()
}

fn default_narrative_pov() -> String {
    "third_limited".to_string()
}
