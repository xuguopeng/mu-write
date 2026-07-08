use crate::commands::arg;
use crate::models::FileNode;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tracing::info;

pub async fn read_file(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "filePath")?;
    match read_text(&path) {
        Ok(content) => Ok(json!({ "success": true, "content": content })),
        Err(error) => Ok(json!({ "success": false, "content": "", "error": error })),
    }
}

pub async fn write_file(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "filePath")?;
    let content: String = arg(&args, 1, "content")?;
    match write_text(&path, &content) {
        Ok(()) => Ok(json!({ "success": true })),
        Err(error) => Ok(json!({ "success": false, "error": error })),
    }
}

pub async fn list_dir(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "dirPath")?;
    if !Path::new(&path).exists() {
        return Ok(json!([]));
    }

    let nodes = read_dir_recursive(Path::new(&path)).unwrap_or_default();
    serde_json::to_value(nodes).map_err(|error| error.to_string())
}

pub async fn mkdir(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "dirPath")?;
    match fs::create_dir_all(&path) {
        Ok(()) => Ok(json!({ "success": true })),
        Err(error) => Ok(
            json!({ "success": false, "error": format!("Failed to create directory '{path}': {error}") }),
        ),
    }
}

pub async fn check_exists(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "filePath")?;
    Ok(json!(Path::new(&path).exists()))
}

pub async fn read_json(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "filePath")?;
    match read_text(&path).and_then(|content| {
        serde_json::from_str::<Value>(&content)
            .map_err(|error| format!("Failed to parse JSON '{path}': {error}"))
    }) {
        Ok(data) => Ok(json!({ "success": true, "data": data })),
        Err(error) => Ok(json!({ "success": false, "data": null, "error": error })),
    }
}

pub async fn write_json(args: Vec<Value>) -> Result<Value, String> {
    let path: String = arg(&args, 0, "filePath")?;
    let data: Value = arg(&args, 1, "data")?;
    match write_json_value(&path, &data) {
        Ok(()) => Ok(json!({ "success": true })),
        Err(error) => Ok(json!({ "success": false, "error": error })),
    }
}

pub fn write_json_value(path: &str, data: &Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(data).map_err(|error| error.to_string())?;
    write_text(path, &content)
}

pub fn write_text(path: &str, content: &str) -> Result<(), String> {
    let path_ref = Path::new(path);
    if let Some(parent) = path_ref.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("Failed to create directory '{}': {error}", parent.display())
        })?;
    }

    let temp_path = path_ref.with_extension(format!(
        "{}.tmp",
        path_ref
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("vela")
    ));

    fs::write(&temp_path, content)
        .map_err(|error| format!("Failed to write '{}': {error}", temp_path.display()))?;
    fs::rename(&temp_path, path_ref).map_err(|error| {
        format!(
            "Failed to replace '{}' with '{}': {error}",
            temp_path.display(),
            path_ref.display()
        )
    })?;
    info!("File written to: {}", path);
    Ok(())
}

pub fn read_text(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("Failed to read file '{path}': {error}"))
}

fn read_dir_recursive(dir_path: &Path) -> Result<Vec<FileNode>, String> {
    let mut entries = fs::read_dir(dir_path)
        .map_err(|error| format!("Failed to read directory '{}': {error}", dir_path.display()))?
        .filter_map(Result::ok)
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        b_is_dir.cmp(&a_is_dir).then_with(|| {
            a.file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy())
        })
    });

    entries
        .into_iter()
        .map(|entry| {
            let path = entry.path();
            let is_dir = path.is_dir();
            let children = if is_dir {
                read_dir_recursive(&path).unwrap_or_default()
            } else {
                Vec::new()
            };

            Ok(FileNode {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                is_dir,
                children,
            })
        })
        .collect()
}
