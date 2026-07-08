use ai_novel_generator_lib::commands;
use ai_novel_generator_lib::state::AppState;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
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
        "project" => project_command(args).await,
        "chapter" => chapter_command(args).await,
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "Unknown command: {other}\nRun `plotforge-cli help` for usage."
        )),
    }
}

async fn project_command(mut args: Vec<String>) -> Result<(), String> {
    let subcommand = args
        .first()
        .cloned()
        .ok_or_else(|| "project requires a subcommand: recent, status, new".to_string())?;
    args.remove(0);

    match subcommand.as_str() {
        "recent" => {
            let state = Arc::new(AppState::default());
            let result = invoke(state, "project:recent-list", vec![]).await?;
            print_json(result)
        }
        "status" => project_status_command().await,
        "new" => project_new_command(args).await,
        other => Err(format!("Unknown project subcommand: {other}")),
    }
}

async fn project_new_command(args: Vec<String>) -> Result<(), String> {
    let name = args
        .first()
        .cloned()
        .ok_or_else(|| "project new requires a project name".to_string())?;
    let options = parse_options(&args[1..]);
    let dir = option_required(&options, "dir")?;
    let genre = option_value(&options, "genre").unwrap_or_default();
    let target_audience = option_value(&options, "audience").unwrap_or_default();

    let state = Arc::new(AppState::default());
    let result = invoke(
        state,
        "project:create",
        vec![json!({
            "name": name,
            "path": dir,
            "genre": genre,
            "targetAudience": target_audience,
        })],
    )
    .await?;
    print_json(result)
}

async fn project_status_command() -> Result<(), String> {
    let state = Arc::new(AppState::default());
    let recent = invoke(state.clone(), "project:recent-list", vec![]).await?;
    let Some(projects) = recent.as_array() else {
        return print_json(recent);
    };

    let mut rows = Vec::new();
    for project in projects {
        let name = project["name"].as_str().unwrap_or("").to_string();
        let path = project["path"].as_str().unwrap_or("").to_string();
        if path.is_empty() {
            continue;
        }

        if let Err(error) = set_cli_project_path(state.clone(), &path).await {
            rows.push(json!({
                "name": name,
                "path": project["path"],
                "error": error,
            }));
            continue;
        }

        let blueprints = invoke(state.clone(), "db:blueprint-get-all", vec![]).await?;
        let blueprint_to_chapter = max_chapter_number(&blueprints);
        let finalized_to_chapter =
            invoke(state.clone(), "db:draft-get-max-finalized-chapter", vec![]).await?;
        let (draft_to_chapter, draft_count) =
            draft_progress_for_blueprints(state.clone(), &blueprints).await?;

        rows.push(json!({
            "name": name,
            "path": project["path"],
            "blueprintToChapter": blueprint_to_chapter,
            "draftToChapter": draft_to_chapter,
            "finalizedToChapter": finalized_to_chapter.as_i64().unwrap_or(0),
            "draftCount": draft_count,
        }));
    }

    print_json(json!(rows))
}

async fn chapter_command(mut args: Vec<String>) -> Result<(), String> {
    let subcommand = args.first().cloned().ok_or_else(|| {
        "chapter requires a subcommand: blueprint, draft, write, next".to_string()
    })?;
    args.remove(0);

    match subcommand.as_str() {
        "blueprint" => chapter_blueprint_command(args).await,
        "draft" => chapter_draft_command(args).await,
        "write" => chapter_write_command(args, false).await,
        "next" => chapter_write_command(args, true).await,
        other => Err(format!("Unknown chapter subcommand: {other}")),
    }
}

async fn chapter_blueprint_command(args: Vec<String>) -> Result<(), String> {
    let options = parse_options(&args);
    let project = option_required(&options, "project")?;
    let number = option_i64(&options, "number")?;
    let title = option_value(&options, "title").unwrap_or_else(|| format!("第{number}章"));
    let role = option_value(&options, "role").unwrap_or_default();
    let purpose = option_value(&options, "purpose").unwrap_or_default();
    let key_events = option_value(&options, "events").unwrap_or_default();
    let characters = option_value(&options, "characters")
        .map(|text| {
            text.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(Value::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let state = Arc::new(AppState::default());
    set_cli_project_path(state.clone(), &project).await?;
    let result = invoke(
        state,
        "db:blueprint-upsert",
        vec![json!({
            "chapterNumber": number,
            "title": title,
            "role": role,
            "purpose": purpose,
            "keyEvents": key_events,
            "characters": characters,
            "suspenseHook": option_value(&options, "hook").unwrap_or_default(),
            "userGuidance": option_value(&options, "guidance").unwrap_or_default(),
            "notes": option_value(&options, "notes").unwrap_or_default(),
            "notesUpdatedAt": "",
        })],
    )
    .await?;
    print_json(result)
}

async fn chapter_draft_command(args: Vec<String>) -> Result<(), String> {
    let options = parse_options(&args);
    let project = option_required(&options, "project")?;
    let number = option_i64(&options, "number")?;
    let content = option_required(&options, "content")?;
    let source = option_value(&options, "source").unwrap_or_else(|| "write".to_string());

    let state = Arc::new(AppState::default());
    set_cli_project_path(state.clone(), &project).await?;
    let version = invoke(state.clone(), "db:draft-next-version", vec![json!(number)])
        .await?
        .as_i64()
        .unwrap_or(1);
    let result = create_draft(state, number, version, source, content).await?;
    print_json(result)
}

async fn chapter_write_command(args: Vec<String>, auto_next: bool) -> Result<(), String> {
    let options = parse_options(&args);
    let project = option_required(&options, "project")?;
    let state = Arc::new(AppState::default());
    set_cli_project_path(state.clone(), &project).await?;
    let project_data = read_project_data(&project)?;

    let blueprints = invoke(state.clone(), "db:blueprint-get-all", vec![]).await?;
    let chapter_number = if auto_next {
        next_chapter_number(state.clone(), &blueprints).await?
    } else {
        option_i64(&options, "number")?
    };

    let blueprint = invoke(
        state.clone(),
        "db:blueprint-get",
        vec![json!(chapter_number)],
    )
    .await?;
    if blueprint.is_null() {
        return Err(format!("未找到第 {chapter_number} 章蓝图，请先创建蓝图"));
    }

    let version = invoke(
        state.clone(),
        "db:draft-next-version",
        vec![json!(chapter_number)],
    )
    .await?
    .as_i64()
    .unwrap_or(1);
    let prompt = build_chapter_prompt(
        state.clone(),
        &project_data,
        &blueprint,
        chapter_number,
        option_value(&options, "guidance"),
    )
    .await?;

    if options.contains_key("dry-run") {
        return print_json(json!({
            "success": true,
            "dryRun": true,
            "chapterNumber": chapter_number,
            "version": version,
            "blueprint": blueprint,
            "prompt": prompt,
        }));
    }

    eprintln!("Writing chapter {chapter_number} with AI...");
    let llm_response = invoke(
        state.clone(),
        "llm:generate",
        vec![json!({
            "modelId": option_value(&options, "model").unwrap_or_default(),
            "messages": [
                {
                    "role": "system",
                    "content": "你是专业中文网文作者。只输出章节正文，不要解释，不要 Markdown 标题，不要复述要求。"
                },
                { "role": "user", "content": prompt }
            ],
            "temperature": option_f64(&options, "temperature").unwrap_or(0.75),
            "maxTokens": option_i64(&options, "max-tokens").unwrap_or(16000)
        })],
    )
    .await?;

    if llm_response.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(llm_response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("AI 生成失败")
            .to_string());
    }

    let content = strip_thinking_tags(
        llm_response
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    if content.trim().is_empty() {
        return Err("AI 返回内容为空".to_string());
    }

    let draft = create_draft(
        state,
        chapter_number,
        version,
        "write".to_string(),
        content.clone(),
    )
    .await?;

    print_json(json!({
        "success": true,
        "chapterNumber": chapter_number,
        "version": version,
        "wordCount": content.chars().count(),
        "draft": draft,
        "usage": llm_response.get("usage").cloned().unwrap_or(Value::Null),
    }))
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

async fn invoke(state: Arc<AppState>, channel: &str, args: Vec<Value>) -> Result<Value, String> {
    commands::dispatch_cli(state, channel.to_string(), args).await
}

async fn set_cli_project_path(state: Arc<AppState>, project: &str) -> Result<(), String> {
    let path = PathBuf::from(project);
    if !path.exists() {
        return Err("目录不存在".to_string());
    }
    *state.current_project_path.write().await = Some(path.clone());
    *state.current_db_path.write().await = Some(path.join(".vela").join("project.db"));
    Ok(())
}

fn read_project_data(project: &str) -> Result<Value, String> {
    let project_path = Path::new(project);
    let project_file = project_path.join(".vela").join("project.json");
    if !project_file.exists() {
        let name = project_path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());
        return Ok(json!({
            "name": name,
            "path": project,
            "novelConfig": {},
        }));
    }
    let text = std::fs::read_to_string(&project_file)
        .map_err(|error| format!("Failed to read '{}': {error}", project_file.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("Invalid project json '{}': {error}", project_file.display()))
}

async fn draft_progress_for_blueprints(
    state: Arc<AppState>,
    blueprints: &Value,
) -> Result<(i64, i64), String> {
    let mut draft_to_chapter = 0;
    let mut draft_count = 0;
    let Some(items) = blueprints.as_array() else {
        return Ok((0, 0));
    };

    for blueprint in items {
        let chapter_number = blueprint
            .get("chapterNumber")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if chapter_number <= 0 {
            continue;
        }
        let drafts = invoke(state.clone(), "db:draft-list", vec![json!(chapter_number)]).await?;
        if let Some(list) = drafts.as_array() {
            if !list.is_empty() {
                draft_to_chapter = draft_to_chapter.max(chapter_number);
                draft_count += list.len() as i64;
            }
        }
    }

    Ok((draft_to_chapter, draft_count))
}

async fn next_chapter_number(state: Arc<AppState>, blueprints: &Value) -> Result<i64, String> {
    let (draft_to_chapter, _) = draft_progress_for_blueprints(state.clone(), blueprints).await?;
    let finalized_to_chapter = invoke(state, "db:draft-get-max-finalized-chapter", vec![])
        .await?
        .as_i64()
        .unwrap_or(0);
    let next = draft_to_chapter.max(finalized_to_chapter) + 1;
    if next <= 0 {
        return Ok(1);
    }
    Ok(next)
}

fn max_chapter_number(blueprints: &Value) -> i64 {
    blueprints
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("chapterNumber").and_then(Value::as_i64))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

async fn build_chapter_prompt(
    state: Arc<AppState>,
    project_data: &Value,
    blueprint: &Value,
    chapter_number: i64,
    extra_guidance: Option<String>,
) -> Result<String, String> {
    let core = invoke(state.clone(), "db:project-core-get", vec![])
        .await
        .unwrap_or(Value::Null);
    let characters = invoke(state.clone(), "db:character-get-all", vec![])
        .await
        .unwrap_or(json!([]));
    let previous = if chapter_number > 1 {
        latest_previous_content(state.clone(), chapter_number - 1).await?
    } else {
        String::new()
    };
    let future = future_blueprints(state, chapter_number).await?;

    let novel_config = project_data.get("novelConfig").unwrap_or(&Value::Null);
    let words_per_chapter = novel_config
        .get("wordsPerChapter")
        .and_then(Value::as_i64)
        .or_else(|| core.get("wordsPerChapter").and_then(Value::as_i64))
        .unwrap_or(3000);

    let mut parts = Vec::new();
    parts.push(format!(
        "请创作第 {chapter_number} 章正文，目标字数约 {words_per_chapter} 字。"
    ));
    parts.push("要求：中文网文正文风格；有场景、有动作、有情绪推进；不要输出分析、标题解释、JSON 或 Markdown。".to_string());

    let project_name = project_data
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| core.get("projectName").and_then(Value::as_str))
        .unwrap_or("");
    if !project_name.is_empty() {
        parts.push(format!("项目名：{project_name}"));
    }

    let genre = novel_config
        .get("genre")
        .and_then(Value::as_str)
        .or_else(|| core.get("genre").and_then(Value::as_str))
        .unwrap_or("");
    let audience = novel_config
        .get("targetAudience")
        .and_then(Value::as_str)
        .or_else(|| core.get("targetAudience").and_then(Value::as_str))
        .unwrap_or("");
    if !genre.is_empty() || !audience.is_empty() {
        parts.push(format!("类型/读者：{genre} / {audience}"));
    }

    for (label, key) in [
        ("核心梗概", "premise"),
        ("世界观", "worldbuilding"),
        ("人物架构", "charactersArch"),
        ("全书梗概", "synopsis"),
        ("全局写作要求", "globalGuidance"),
    ] {
        if let Some(text) = core.get(key).and_then(Value::as_str) {
            if !text.trim().is_empty() {
                parts.push(format!("【{label}】\n{}", trim_to_chars(text, 4000)));
            }
        }
    }

    parts.push(format!("【本章蓝图】\n{}", render_blueprint(blueprint)));

    if let Some(text) = extra_guidance {
        if !text.trim().is_empty() {
            parts.push(format!("【本次额外指令】\n{}", text.trim()));
        }
    }

    let character_text = render_characters(&characters);
    if !character_text.is_empty() {
        parts.push(format!("【角色状态】\n{character_text}"));
    }

    if !previous.trim().is_empty() {
        parts.push(format!(
            "【上一章结尾】\n{}",
            trim_to_chars(&previous, 1600)
        ));
    }

    if !future.trim().is_empty() {
        parts.push(format!("【后续蓝图参考】\n{future}"));
    }

    parts.push("现在开始写正文。正文必须直接进入故事。".to_string());
    Ok(parts.join("\n\n---\n\n"))
}

async fn latest_previous_content(
    state: Arc<AppState>,
    chapter_number: i64,
) -> Result<String, String> {
    let meta = invoke(
        state.clone(),
        "db:draft-get-finalized",
        vec![json!(chapter_number)],
    )
    .await?;
    let meta = if meta.is_null() {
        invoke(
            state.clone(),
            "db:draft-get-latest",
            vec![json!(chapter_number)],
        )
        .await?
    } else {
        meta
    };
    let Some(id) = meta.get("id").and_then(Value::as_i64) else {
        return Ok(String::new());
    };
    let full = invoke(state, "db:draft-get-full", vec![json!(id)]).await?;
    Ok(full
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string())
}

async fn future_blueprints(state: Arc<AppState>, chapter_number: i64) -> Result<String, String> {
    let blueprints = invoke(state, "db:blueprint-get-all", vec![]).await?;
    let mut lines = Vec::new();
    if let Some(items) = blueprints.as_array() {
        for item in items {
            let n = item
                .get("chapterNumber")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if n > chapter_number && n <= chapter_number + 5 {
                let title = item.get("title").and_then(Value::as_str).unwrap_or("");
                let events = item.get("keyEvents").and_then(Value::as_str).unwrap_or("");
                lines.push(format!("第{n}章 {title}: {events}"));
            }
        }
    }
    Ok(lines.join("\n"))
}

fn render_blueprint(blueprint: &Value) -> String {
    let characters = blueprint
        .get("characters")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("、")
        })
        .unwrap_or_default();
    [
        ("标题", "title"),
        ("剧情作用", "role"),
        ("本章目的", "purpose"),
        ("关键事件", "keyEvents"),
        ("悬念钩子", "suspenseHook"),
        ("写作指导", "userGuidance"),
        ("章节要点", "notes"),
    ]
    .iter()
    .filter_map(|(label, key)| {
        blueprint
            .get(*key)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(|text| format!("{label}: {}", text.trim()))
    })
    .chain((!characters.is_empty()).then(|| format!("出场人物: {characters}")))
    .collect::<Vec<_>>()
    .join("\n")
}

fn render_characters(characters: &Value) -> String {
    let Some(items) = characters.as_array() else {
        return String::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let name = item.get("name").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                return None;
            }
            let role = item.get("role").and_then(Value::as_str).unwrap_or("");
            let notes = item.get("notes").and_then(Value::as_str).unwrap_or("");
            let state = item.get("currentState").unwrap_or(&Value::Null);
            let recent = state
                .get("recentEvents")
                .and_then(Value::as_str)
                .unwrap_or("");
            Some(format!(
                "{name}({role})：{}{}",
                trim_to_chars(notes, 240),
                if recent.is_empty() {
                    String::new()
                } else {
                    format!(" 最近：{}", trim_to_chars(recent, 160))
                }
            ))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn create_draft(
    state: Arc<AppState>,
    chapter_number: i64,
    version: i64,
    source: String,
    content: String,
) -> Result<Value, String> {
    invoke(
        state,
        "db:draft-create",
        vec![json!({
            "chapterNumber": chapter_number,
            "version": version,
            "source": source,
            "content": content,
            "wordCount": content.chars().count(),
        })],
    )
    .await
}

fn parse_options(args: &[String]) -> HashMap<String, String> {
    let mut options = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        let item = &args[i];
        if let Some(key) = item.strip_prefix("--") {
            if let Some((key, value)) = key.split_once('=') {
                options.insert(key.to_string(), value.to_string());
                i += 1;
            } else if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                options.insert(key.to_string(), args[i + 1].clone());
                i += 2;
            } else {
                options.insert(key.to_string(), "true".to_string());
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    options
}

fn option_value(options: &HashMap<String, String>, key: &str) -> Option<String> {
    options.get(key).cloned()
}

fn option_required(options: &HashMap<String, String>, key: &str) -> Result<String, String> {
    option_value(options, key).ok_or_else(|| format!("Missing required option --{key}"))
}

fn option_i64(options: &HashMap<String, String>, key: &str) -> Result<i64, String> {
    option_required(options, key)?
        .parse::<i64>()
        .map_err(|error| format!("Invalid --{key}: {error}"))
}

fn option_f64(options: &HashMap<String, String>, key: &str) -> Option<f64> {
    options.get(key).and_then(|value| value.parse::<f64>().ok())
}

fn trim_to_chars(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn strip_thinking_tags(text: &str) -> String {
    let mut output = String::new();
    let mut rest = text;
    loop {
        let Some(start) = rest.find("<think>") else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let after_start = &rest[start + "<think>".len()..];
        let Some(end) = after_start.find("</think>") else {
            break;
        };
        rest = &after_start[end + "</think>".len()..];
    }
    output.trim().to_string()
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
plotforge-cli project recent\n  \
plotforge-cli project status\n  \
plotforge-cli project new <name> --dir <directory> [--genre <genre>] [--audience <audience>]\n  \
plotforge-cli chapter blueprint --project <path> --number <n> --title <title> [--events <text>]\n  \
plotforge-cli chapter draft --project <path> --number <n> --content <text>\n  \
plotforge-cli chapter write --project <path> --number <n> [--guidance <text>] [--model <model-id>] [--dry-run]\n  \
plotforge-cli chapter next --project <path> [--guidance <text>] [--model <model-id>] [--dry-run]\n  \
plotforge-cli call [--project <path>] <channel> '[json-array-args]'\n  \
plotforge-cli deeplink 'plotforge://call?channel=<channel>&args=<json-array>&project=<path>'\n\n\
Examples:\n  \
plotforge-cli project status\n  \
plotforge-cli chapter next --project /path/to/novel\n  \
plotforge-cli call config:get\n  \
plotforge-cli call project:recent-list\n  \
plotforge-cli call --project /path/to/novel db:project-core-get\n  \
plotforge-cli deeplink 'plotforge://call?channel=config:get'\n"
    );
}
