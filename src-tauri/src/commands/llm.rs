use crate::commands::config::{read_config, read_json, write_config, write_json};
use crate::commands::{arg, optional_arg};
use crate::models::{LLMMessage, LLMRequest, LLMResponse, ModelProfile, TokenUsage};
use crate::state::AppState;
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::info;

pub async fn generate(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let request: LLMRequest = arg(&args, 0, "request")?;
    let response = generate_response(state, request).await?;
    serde_json::to_value(response).map_err(|error| error.to_string())
}

pub async fn generate_stream(
    app: AppHandle,
    state: Arc<AppState>,
    args: Vec<Value>,
) -> Result<Value, String> {
    let request_id: String = arg(&args, 0, "requestId")?;
    let request: LLMRequest = arg(&args, 1, "request")?;

    {
        let mut cancellations = state.stream_cancellations.write().await;
        cancellations.remove(&request_id);
    }

    let stream_state = state.clone();
    let stream_app = app.clone();
    let stream_request_id = request_id.clone();

    tokio::spawn(async move {
        let result = stream_response(
            stream_app.clone(),
            stream_state.clone(),
            stream_request_id.clone(),
            request,
        )
        .await;
        if let Err(error) = result {
            let _ = stream_app.emit(
                "llm:stream-error",
                json!({ "requestId": stream_request_id, "error": error }),
            );
        }
        let mut cancellations = stream_state.stream_cancellations.write().await;
        cancellations.remove(&stream_request_id);
    });

    Ok(json!({ "requestId": request_id, "started": true }))
}

pub async fn cancel(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let request_id: String = arg(&args, 0, "requestId")?;
    let mut cancellations = state.stream_cancellations.write().await;
    cancellations.insert(request_id);
    Ok(json!({ "success": true }))
}

pub async fn list_models(state: Arc<AppState>) -> Result<Value, String> {
    load_models_into_state(&state).await?;
    let models = state.model_configs.read().await;
    serde_json::to_value(models.values().cloned().collect::<Vec<_>>())
        .map_err(|error| error.to_string())
}

pub async fn save_model(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let model: ModelProfile = arg(&args, 0, "model")?;
    save_model_direct(&state, model).await?;
    Ok(json!({ "success": true }))
}

pub async fn delete_model(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let model_id: String = arg(&args, 0, "modelId")?;
    load_models_into_state(&state).await?;

    let mut models = state.model_configs.write().await;
    models.remove(&model_id);
    persist_models(&state, models.values().cloned().collect())?;
    drop(models);

    let mut config = read_config(&state)?;
    if config.default_model_id.as_deref() == Some(&model_id) {
        config.default_model_id = None;
        *state.default_model_id.write().await = None;
    }
    if config.default_embedding_model_id.as_deref() == Some(&model_id) {
        config.default_embedding_model_id = None;
        *state.default_embedding_model_id.write().await = None;
    }
    write_config(&state, &config)?;

    info!("Model deleted: {}", model_id);
    Ok(json!({ "success": true }))
}

pub async fn set_default_model(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let model_id: Option<String> = optional_arg(&args, 0)?;
    let mut config = read_config(&state)?;
    config.default_model_id = model_id.clone();
    write_config(&state, &config)?;
    *state.default_model_id.write().await = model_id;
    Ok(json!({ "success": true }))
}

pub async fn get_default_model(state: Arc<AppState>) -> Result<Value, String> {
    let config = read_config(&state)?;
    *state.default_model_id.write().await = config.default_model_id.clone();
    Ok(json!(config.default_model_id))
}

pub async fn set_default_embedding_model(
    state: Arc<AppState>,
    args: Vec<Value>,
) -> Result<Value, String> {
    let model_id: Option<String> = optional_arg(&args, 0)?;
    let mut config = read_config(&state)?;
    config.default_embedding_model_id = model_id.clone();
    write_config(&state, &config)?;
    *state.default_embedding_model_id.write().await = model_id;
    Ok(json!({ "success": true }))
}

pub async fn get_default_embedding_model(state: Arc<AppState>) -> Result<Value, String> {
    let config = read_config(&state)?;
    *state.default_embedding_model_id.write().await = config.default_embedding_model_id.clone();
    Ok(json!(config.default_embedding_model_id))
}

pub async fn test_connection(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let model = match optional_arg::<ModelProfile>(&args, 0)? {
        Some(model) => model,
        None => {
            let model_id: String = arg(&args, 0, "modelId")?;
            get_model_config(&state, &model_id).await?
        }
    };

    let messages = vec![LLMMessage {
        role: "user".to_string(),
        content: "Say hello and nothing else.".to_string(),
    }];

    match call_llm_provider(&state.http_client, &model, &messages, 0.7, 10, false).await {
        Ok(response) => Ok(json!({ "success": response.success, "error": response.error })),
        Err(error) => Ok(json!({ "success": false, "error": error.to_string() })),
    }
}

pub async fn generate_response(
    state: Arc<AppState>,
    request: LLMRequest,
) -> Result<LLMResponse, String> {
    let model = resolve_model(&state, &request).await?;
    let temperature = request.temperature.unwrap_or(model.temperature);
    let max_tokens = request.max_tokens.unwrap_or(model.max_tokens);

    call_llm_provider(
        &state.http_client,
        &model,
        &request.messages,
        temperature,
        max_tokens,
        false,
    )
    .await
    .map_err(|error| error.to_string())
}

pub async fn save_model_direct(
    state: &Arc<AppState>,
    mut model: ModelProfile,
) -> Result<(), String> {
    load_models_into_state(state).await?;
    let model_id = model.normalized_id();
    model.id = model_id.clone();

    let mut models = state.model_configs.write().await;
    models.insert(model_id.clone(), model);
    persist_models(state, models.values().cloned().collect())?;

    info!("Model saved: {}", model_id);
    Ok(())
}

pub async fn delete_model_direct(state: &Arc<AppState>, model_id: String) -> Result<(), String> {
    delete_model(state.clone(), vec![json!(model_id)])
        .await
        .map(|_| ())
}

pub async fn load_models_into_state(state: &Arc<AppState>) -> Result<(), String> {
    let models: Vec<ModelProfile> = read_json(models_path(state), Vec::<ModelProfile>::new())?;
    let mut map = state.model_configs.write().await;
    map.clear();
    for mut model in models {
        let model_id = model.normalized_id();
        model.id = model_id.clone();
        map.insert(model_id, model);
    }
    Ok(())
}

pub fn models_path(state: &AppState) -> PathBuf {
    state.vela_home.join("models.json")
}

fn persist_models(state: &AppState, models: Vec<ModelProfile>) -> Result<(), String> {
    write_json(models_path(state), &models)
}

async fn resolve_model(
    state: &Arc<AppState>,
    request: &LLMRequest,
) -> Result<ModelProfile, String> {
    if let Some(model) = request.model.clone() {
        return Ok(model);
    }

    let model_id = if request.model_id.is_empty() {
        let config = read_config(state)?;
        config
            .default_model_id
            .ok_or_else(|| "No default model configured".to_string())?
    } else {
        request.model_id.clone()
    };

    get_model_config(state, &model_id).await
}

async fn get_model_config(state: &Arc<AppState>, model_id: &str) -> Result<ModelProfile, String> {
    load_models_into_state(state).await?;
    let models = state.model_configs.read().await;
    models
        .get(model_id)
        .cloned()
        .ok_or_else(|| format!("Model not found: {model_id}"))
}

fn is_anthropic(model: &ModelProfile) -> bool {
    model.protocol.as_deref() == Some("anthropic")
}

/// Extract system messages into a top-level system string; return non-system messages.
fn split_system_messages(messages: &[LLMMessage]) -> (String, Vec<&LLMMessage>) {
    let system: String = messages
        .iter()
        .filter(|m| m.role == "system")
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let rest: Vec<&LLMMessage> = messages.iter().filter(|m| m.role != "system").collect();
    (system, rest)
}

async fn stream_response(
    app: AppHandle,
    state: Arc<AppState>,
    request_id: String,
    request: LLMRequest,
) -> Result<(), String> {
    let model = resolve_model(&state, &request).await?;
    let temperature = request.temperature.unwrap_or(model.temperature);
    let max_tokens = request.max_tokens.unwrap_or(model.max_tokens);
    let mut full_text = String::new();

    if is_anthropic(&model) {
        // ── Anthropic Messages API (streaming) ──
        let endpoint = format!("{}/v1/messages", model.base_url.trim_end_matches('/'));
        let (system_prompt, rest) = split_system_messages(&request.messages);
        let mut body = json!({
            "model": model.display_model_name(),
            "max_tokens": max_tokens,
            "stream": true,
            "temperature": temperature,
        });
        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        let messages_value: Vec<Value> = rest
            .iter()
            .map(|m| json!({"role": m.role, "content": m.content}))
            .collect();
        body["messages"] = json!(messages_value);

        let response = state
            .http_client
            .post(&endpoint)
            .header("x-api-key", &model.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {error_text}"));
        }

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if state
                .stream_cancellations
                .read()
                .await
                .contains(&request_id)
            {
                return Ok(());
            }

            let chunk = chunk.map_err(|error| error.to_string())?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }

                let data = line.trim_start_matches("data:").trim();
                let parsed: Value = match serde_json::from_str(data) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                let event_type = parsed["type"].as_str().unwrap_or("");

                if event_type == "message_stop" {
                    let _ = app.emit(
                        "llm:stream-done",
                        json!({ "requestId": request_id, "fullText": full_text }),
                    );
                    return Ok(());
                }

                if event_type == "content_block_delta" {
                    let delta = parsed["delta"]["text"].as_str().unwrap_or("");
                    if !delta.is_empty() {
                        full_text.push_str(delta);
                        let _ = app.emit(
                            "llm:stream-chunk",
                            json!({ "requestId": request_id, "chunk": delta }),
                        );
                    }
                }
            }
        }

        let _ = app.emit(
            "llm:stream-done",
            json!({ "requestId": request_id, "fullText": full_text }),
        );
        Ok(())
    } else {
        // ── OpenAI-compatible (streaming) ──
        let endpoint = format!("{}/chat/completions", model.base_url.trim_end_matches('/'));
        let body = json!({
            "model": model.display_model_name(),
            "messages": request.messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": true,
            "response_format": request.response_format,
        });

        let response = state
            .http_client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {error_text}"));
        }

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if state
                .stream_cancellations
                .read()
                .await
                .contains(&request_id)
            {
                return Ok(());
            }

            let chunk = chunk.map_err(|error| error.to_string())?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }

                let data = line.trim_start_matches("data:").trim();
                if data == "[DONE]" {
                    let _ = app.emit(
                        "llm:stream-done",
                        json!({ "requestId": request_id, "fullText": full_text }),
                    );
                    return Ok(());
                }

                let parsed: Value = match serde_json::from_str(data) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let delta = parsed["choices"]
                    .get(0)
                    .and_then(|choice| choice["delta"]["content"].as_str())
                    .unwrap_or("");
                if delta.is_empty() {
                    continue;
                }

                full_text.push_str(delta);
                let _ = app.emit(
                    "llm:stream-chunk",
                    json!({ "requestId": request_id, "chunk": delta }),
                );
            }
        }

        let _ = app.emit(
            "llm:stream-done",
            json!({ "requestId": request_id, "fullText": full_text }),
        );
        Ok(())
    }
}

pub async fn call_llm_provider(
    client: &Client,
    model: &ModelProfile,
    messages: &[LLMMessage],
    temperature: f32,
    max_tokens: i32,
    stream: bool,
) -> Result<LLMResponse> {
    if is_anthropic(model) {
        // ── Anthropic Messages API (non-streaming) ──
        let endpoint = format!("{}/v1/messages", model.base_url.trim_end_matches('/'));
        let (system_prompt, rest) = split_system_messages(messages);
        let mut body = json!({
            "model": model.display_model_name(),
            "max_tokens": max_tokens,
            "stream": stream,
            "temperature": temperature,
        });
        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        let messages_value: Vec<Value> = rest
            .iter()
            .map(|m| json!({"role": m.role, "content": m.content}))
            .collect();
        body["messages"] = json!(messages_value);

        let response = client
            .post(&endpoint)
            .header("x-api-key", &model.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Ok(LLMResponse {
                success: false,
                content: String::new(),
                error: Some(format!("API error: {error_text}")),
                usage: None,
            });
        }

        let parsed: Value = response.json().await?;
        let content = parsed["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|block| block["type"] == "text")
                    .and_then(|block| block["text"].as_str())
            })
            .unwrap_or("")
            .to_string();

        let usage = parsed.get("usage").map(|u| TokenUsage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            total_tokens: {
                let inp = u.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let out = u.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                (inp + out) as i32
            },
        });

        Ok(LLMResponse {
            success: !content.is_empty(),
            content,
            error: None,
            usage,
        })
    } else {
        // ── OpenAI-compatible (non-streaming) ──
        let endpoint = format!("{}/chat/completions", model.base_url.trim_end_matches('/'));
        let body = json!({
            "model": model.display_model_name(),
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": stream,
        });

        let response = client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Ok(LLMResponse {
                success: false,
                content: String::new(),
                error: Some(format!("API error: {error_text}")),
                usage: None,
            });
        }

        let parsed: Value = response.json().await?;
        let content = parsed["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        let usage = parsed.get("usage").map(|usage| TokenUsage {
            prompt_tokens: usage
                .get("prompt_tokens")
                .and_then(|value| value.as_i64())
                .unwrap_or(0) as i32,
            completion_tokens: usage
                .get("completion_tokens")
                .and_then(|value| value.as_i64())
                .unwrap_or(0) as i32,
            total_tokens: usage
                .get("total_tokens")
                .and_then(|value| value.as_i64())
                .unwrap_or(0) as i32,
        });

        Ok(LLMResponse {
            success: !content.is_empty(),
            content,
            error: None,
            usage,
        })
    }
}
