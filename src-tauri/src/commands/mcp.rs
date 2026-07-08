use crate::commands::arg;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

const MCP_TIMEOUT_SECONDS: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpServerConfig {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    transport: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeMcpConfig {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: HashMap<String, ClaudeMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeMcpServer {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    url: Option<String>,
}

pub async fn dispatch_mcp(
    state: Arc<AppState>,
    channel: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    match channel {
        "mcp:get-config-path" => get_config_path(state).await,
        "mcp:load-config" => load_config(state, args).await,
        "mcp:connect" => connect(state, args).await,
        "mcp:disconnect" => disconnect(state, args).await,
        "mcp:disconnect-all" => disconnect_all(state).await,
        "mcp:list-tools" => list_tools(state).await,
        "mcp:list-resources" => list_resources(state).await,
        "mcp:call-tool" => call_tool(state, args).await,
        "mcp:get-servers-status" => get_servers_status(state).await,
        _ => Err(format!("Unsupported IPC channel: {channel}")),
    }
}

async fn get_config_path(state: Arc<AppState>) -> Result<Value, String> {
    Ok(json!(default_config_path(&state)
        .to_string_lossy()
        .to_string()))
}

async fn load_config(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let config_path = args
        .first()
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&state));

    if !config_path.exists() {
        return Ok(json!({ "success": false, "configs": [] }));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|error| format!("读取 MCP 配置失败 '{}': {error}", config_path.display()))?;
    let raw: ClaudeMcpConfig = serde_json::from_str(&content)
        .map_err(|error| format!("解析 MCP 配置失败 '{}': {error}", config_path.display()))?;

    let configs = raw
        .mcp_servers
        .into_iter()
        .map(|(id, server)| {
            let transport = if server.url.is_some() { "sse" } else { "stdio" };
            json!({
                "id": id,
                "name": id,
                "transport": transport,
                "command": server.command,
                "args": server.args,
                "env": server.env,
                "url": server.url,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({ "success": true, "configs": configs }))
}

async fn connect(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let mut config: McpServerConfig = arg(&args, 0, "config")?;
    if config.name.trim().is_empty() {
        config.name = config.id.clone();
    }
    let transport = config
        .transport
        .clone()
        .unwrap_or_else(|| if config.url.is_some() { "sse" } else { "stdio" }.to_string());

    state
        .mcp_status
        .write()
        .await
        .insert(config.id.clone(), "connecting".to_string());

    if transport == "sse" {
        state
            .mcp_status
            .write()
            .await
            .insert(config.id.clone(), "error:SSE 传输暂未实现".to_string());
        return Ok(json!({ "success": false, "error": "SSE 传输暂未实现" }));
    }

    match discover_stdio(&config).await {
        Ok((tools, resources)) => {
            let config_value = serde_json::to_value(&config).map_err(|error| error.to_string())?;
            state
                .mcp_configs
                .write()
                .await
                .insert(config.id.clone(), config_value);
            state
                .mcp_status
                .write()
                .await
                .insert(config.id.clone(), "connected".to_string());

            {
                let mut all_tools = state.mcp_tools.write().await;
                all_tools.retain(|tool| {
                    tool.get("serverId").and_then(Value::as_str) != Some(&config.id)
                });
                all_tools.extend(tools);
            }
            {
                let mut all_resources = state.mcp_resources.write().await;
                all_resources
                    .retain(|res| res.get("serverId").and_then(Value::as_str) != Some(&config.id));
                all_resources.extend(resources);
            }

            Ok(json!({ "success": true }))
        }
        Err(error) => {
            state
                .mcp_status
                .write()
                .await
                .insert(config.id.clone(), format!("error:{error}"));
            Ok(json!({ "success": false, "error": error }))
        }
    }
}

async fn disconnect(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let server_id: String = arg(&args, 0, "serverId")?;
    state.mcp_configs.write().await.remove(&server_id);
    state.mcp_status.write().await.remove(&server_id);
    state
        .mcp_tools
        .write()
        .await
        .retain(|tool| tool.get("serverId").and_then(Value::as_str) != Some(&server_id));
    state
        .mcp_resources
        .write()
        .await
        .retain(|res| res.get("serverId").and_then(Value::as_str) != Some(&server_id));
    Ok(json!({ "success": true }))
}

async fn disconnect_all(state: Arc<AppState>) -> Result<Value, String> {
    state.mcp_configs.write().await.clear();
    state.mcp_status.write().await.clear();
    state.mcp_tools.write().await.clear();
    state.mcp_resources.write().await.clear();
    Ok(json!({ "success": true }))
}

async fn list_tools(state: Arc<AppState>) -> Result<Value, String> {
    Ok(Value::Array(state.mcp_tools.read().await.clone()))
}

async fn list_resources(state: Arc<AppState>) -> Result<Value, String> {
    Ok(Value::Array(state.mcp_resources.read().await.clone()))
}

async fn call_tool(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let server_id: String = arg(&args, 0, "serverId")?;
    let tool_name: String = arg(&args, 1, "toolName")?;
    let tool_args: Value = args.get(2).cloned().unwrap_or_else(|| json!({}));

    let config_value = state
        .mcp_configs
        .read()
        .await
        .get(&server_id)
        .cloned()
        .ok_or_else(|| format!("服务器 {server_id} 未连接"))?;
    let config: McpServerConfig =
        serde_json::from_value(config_value).map_err(|error| error.to_string())?;

    match call_stdio_tool(&config, &tool_name, tool_args).await {
        Ok(content) => Ok(json!({ "success": true, "content": content })),
        Err(error) => Ok(json!({ "success": false, "content": "", "error": error })),
    }
}

async fn get_servers_status(state: Arc<AppState>) -> Result<Value, String> {
    let configs = state.mcp_configs.read().await;
    let statuses = state.mcp_status.read().await;
    let tools = state.mcp_tools.read().await;

    let mut server_ids = statuses.keys().cloned().collect::<Vec<_>>();
    for id in configs.keys() {
        if !server_ids.contains(id) {
            server_ids.push(id.clone());
        }
    }

    let result = server_ids
        .into_iter()
        .map(|id| {
            let raw_status = statuses
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "disconnected".to_string());
            let (status, error) = raw_status
                .strip_prefix("error:")
                .map(|err| ("error", Some(err.to_string())))
                .unwrap_or((raw_status.as_str(), None));
            let name = configs
                .get(&id)
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            let tool_count = tools
                .iter()
                .filter(|tool| tool.get("serverId").and_then(Value::as_str) == Some(&id))
                .count();
            json!({
                "id": id,
                "name": name,
                "status": status,
                "toolCount": tool_count,
                "error": error,
            })
        })
        .collect::<Vec<_>>();

    Ok(Value::Array(result))
}

async fn discover_stdio(config: &McpServerConfig) -> Result<(Vec<Value>, Vec<Value>), String> {
    let mut session = StdioSession::start(config).await?;
    session.initialize().await?;

    let tools = session
        .request("tools/list", json!({}))
        .await
        .ok()
        .and_then(|value| value.get("tools").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .map(|mut tool| {
            if let Value::Object(ref mut map) = tool {
                map.insert("serverId".to_string(), json!(config.id));
            }
            tool
        })
        .collect::<Vec<_>>();

    let resources = session
        .request("resources/list", json!({}))
        .await
        .ok()
        .and_then(|value| value.get("resources").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .map(|mut resource| {
            if let Value::Object(ref mut map) = resource {
                map.insert("serverId".to_string(), json!(config.id));
            }
            resource
        })
        .collect::<Vec<_>>();

    session.shutdown().await;
    Ok((tools, resources))
}

async fn call_stdio_tool(
    config: &McpServerConfig,
    tool_name: &str,
    tool_args: Value,
) -> Result<String, String> {
    let mut session = StdioSession::start(config).await?;
    session.initialize().await?;
    let result = session
        .request(
            "tools/call",
            json!({
                "name": tool_name,
                "arguments": tool_args,
            }),
        )
        .await?;
    session.shutdown().await;
    Ok(format_tool_content(result))
}

fn format_tool_content(result: Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    if item.get("type").and_then(Value::as_str) == Some("text") {
                        item.get("text").and_then(Value::as_str).map(str::to_string)
                    } else {
                        Some(item.to_string())
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| result.to_string())
}

fn default_config_path(state: &AppState) -> PathBuf {
    state.vela_home.join("mcp_config.json")
}

struct StdioSession {
    child: Child,
    next_id: i64,
}

impl StdioSession {
    async fn start(config: &McpServerConfig) -> Result<Self, String> {
        let command = config
            .command
            .as_deref()
            .ok_or_else(|| "stdio MCP 配置缺少 command".to_string())?;
        let mut cmd = Command::new(command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|error| format!("启动 MCP 进程失败 '{}': {error}", command))?;
        Ok(Self { child, next_id: 1 })
    }

    async fn initialize(&mut self) -> Result<(), String> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "vela-tauri", "version": "1.0.0" }
            }),
        )
        .await?;
        self.notify("notifications/initialized", json!({})).await
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(message).await?;

        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| "MCP 进程 stdout 不可用".to_string())?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        loop {
            line.clear();
            let read = timeout(
                Duration::from_secs(MCP_TIMEOUT_SECONDS),
                reader.read_line(&mut line),
            )
            .await
            .map_err(|_| format!("MCP 请求超时: {method}"))?
            .map_err(|error| format!("读取 MCP 响应失败: {error}"))?;
            if read == 0 {
                return Err(format!("MCP 进程已退出: {method}"));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }
            let response: Value = match serde_json::from_str(trimmed) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if response.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("MCP error");
                return Err(message.to_string());
            }
            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(message).await
    }

    async fn write_message(&mut self, message: Value) -> Result<(), String> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| "MCP 进程 stdin 不可用".to_string())?;
        stdin
            .write_all(format!("{}\n", message).as_bytes())
            .await
            .map_err(|error| format!("写入 MCP 请求失败: {error}"))?;
        stdin
            .flush()
            .await
            .map_err(|error| format!("刷新 MCP 请求失败: {error}"))
    }

    async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
    }
}
