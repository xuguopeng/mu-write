use ai_novel_generator_lib::commands;
use ai_novel_generator_lib::state::AppState;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::process;
use std::sync::Arc;

const CHANNELS: &[&str] = &[
    "config:get",
    "config:set",
    "config:get-vela-home",
    "project:create",
    "project:open",
    "project:save",
    "project:update-config",
    "project:recent-list",
    "fs:read-file",
    "fs:write-file",
    "fs:list-dir",
    "fs:mkdir",
    "fs:check-exists",
    "fs:read-json",
    "fs:write-json",
    "db:*",
    "kb:*",
    "llm:list-models",
    "llm:generate",
    "llm:test-connection",
    "mcp:*",
];

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args.remove(0).as_str() {
        "list" => print_json(json!({ "channels": CHANNELS })),
        "call" => call_command(args).await,
        "deeplink" => deeplink_command(args).await,
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "Unknown command: {other}\nRun `plotforge-cli help` for usage."
        )),
    }
}

async fn call_command(mut args: Vec<String>) -> Result<(), String> {
    let mut project_path: Option<String> = None;
    if args.first().map(String::as_str) == Some("--project") {
        args.remove(0);
        project_path = Some(
            args.first()
                .cloned()
                .ok_or_else(|| "--project requires a path".to_string())?,
        );
        args.remove(0);
    }

    let channel = args
        .first()
        .cloned()
        .ok_or_else(|| "call requires a channel name".to_string())?;
    args.remove(0);

    let call_args = parse_args_json(args.first().map(String::as_str))?;
    let state = Arc::new(AppState::default());

    if let Some(path) = project_path {
        let opened =
            commands::dispatch_cli(state.clone(), "project:open".to_string(), vec![json!(path)])
                .await?;
        if opened.get("success").and_then(Value::as_bool) == Some(false) {
            return Err(format!("Failed to open project before call: {opened}"));
        }
    }

    let result = commands::dispatch_cli(state, channel, call_args).await?;
    print_json(result)
}

async fn deeplink_command(args: Vec<String>) -> Result<(), String> {
    let uri = args
        .first()
        .ok_or_else(|| "deeplink requires a plotforge:// URI".to_string())?;
    let request = parse_deeplink(uri)?;
    let mut call_args = vec![request.channel];
    call_args.push(serde_json::to_string(&request.args).map_err(|error| error.to_string())?);
    if let Some(project) = request.project {
        call_args.insert(0, project);
        call_args.insert(0, "--project".to_string());
    }
    call_command(call_args).await
}

fn parse_args_json(raw: Option<&str>) -> Result<Vec<Value>, String> {
    match raw {
        None => Ok(Vec::new()),
        Some(text) if text.trim().is_empty() => Ok(Vec::new()),
        Some(text) => {
            let value: Value = serde_json::from_str(text)
                .map_err(|error| format!("Invalid JSON args array: {error}"))?;
            match value {
                Value::Array(items) => Ok(items),
                other => Ok(vec![other]),
            }
        }
    }
}

struct DeepLinkRequest {
    channel: String,
    args: Vec<Value>,
    project: Option<String>,
}

fn parse_deeplink(uri: &str) -> Result<DeepLinkRequest, String> {
    if !uri.starts_with("plotforge://call") {
        return Err("Deep link must start with plotforge://call".to_string());
    }
    let query = uri.split_once('?').map(|(_, query)| query).unwrap_or("");
    let params = parse_query(query)?;
    let channel = params
        .get("channel")
        .cloned()
        .ok_or_else(|| "Deep link missing channel parameter".to_string())?;
    let args = parse_args_json(params.get("args").map(String::as_str))?;
    let project = params.get("project").cloned();
    Ok(DeepLinkRequest {
        channel,
        args,
        project,
    })
}

fn parse_query(query: &str) -> Result<HashMap<String, String>, String> {
    let mut params = HashMap::new();
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(percent_decode(key)?, percent_decode(value)?);
    }
    Ok(params)
}

fn percent_decode(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                    .map_err(|error| format!("Invalid percent encoding: {error}"))?;
                let value = u8::from_str_radix(hex, 16)
                    .map_err(|error| format!("Invalid percent encoding %{hex}: {error}"))?;
                out.push(value);
                i += 3;
            }
            b'%' => return Err("Invalid trailing percent encoding".to_string()),
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|error| format!("Invalid UTF-8 in decoded query: {error}"))
}

fn print_json(value: Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    println!("{text}");
    Ok(())
}

fn print_help() {
    println!(
        "PlotForge CLI\n\n\
Usage:\n  \
plotforge-cli list\n  \
plotforge-cli call [--project <path>] <channel> '[json-array-args]'\n  \
plotforge-cli deeplink 'plotforge://call?channel=<channel>&args=<json-array>&project=<path>'\n\n\
Examples:\n  \
plotforge-cli call config:get\n  \
plotforge-cli call project:recent-list\n  \
plotforge-cli call --project /path/to/novel db:project-core-get\n  \
plotforge-cli deeplink 'plotforge://call?channel=config:get'\n"
    );
}
