use crate::state::Provider;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRequest>,
}

#[derive(Debug, Clone)]
pub enum TurnMessage {
    User {
        text: String,
        screenshot_jpeg: Option<Vec<u8>>,
    },
    Assistant {
        text: String,
        tool_calls: Vec<ToolCallRequest>,
    },
    ToolResult {
        call_id: String,
        tool_name: String,
        result: String,
    },
}

pub fn device_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "tap",
            "description": "Tap on the Android screen at pixel coordinates (x, y).",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": { "type": "number", "description": "Horizontal pixel coordinate" },
                    "y": { "type": "number", "description": "Vertical pixel coordinate" }
                },
                "required": ["x", "y"]
            }
        }),
        json!({
            "name": "double_tap",
            "description": "Double-tap on the Android screen at pixel coordinates (x, y).",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": { "type": "number", "description": "Horizontal pixel coordinate" },
                    "y": { "type": "number", "description": "Vertical pixel coordinate" }
                },
                "required": ["x", "y"]
            }
        }),
        json!({
            "name": "long_press",
            "description": "Long-press on the Android screen at pixel coordinates (x, y).",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": { "type": "number", "description": "Horizontal pixel coordinate" },
                    "y": { "type": "number", "description": "Vertical pixel coordinate" },
                    "duration_ms": { "type": "integer", "description": "Duration in milliseconds (default 700)" }
                },
                "required": ["x", "y"]
            }
        }),
        json!({
            "name": "swipe",
            "description": "Swipe or scroll from (start_x, start_y) to (end_x, end_y) over duration_ms milliseconds.",
            "parameters": {
                "type": "object",
                "properties": {
                    "start_x": { "type": "number", "description": "Start X coordinate" },
                    "start_y": { "type": "number", "description": "Start Y coordinate" },
                    "end_x": { "type": "number", "description": "End X coordinate" },
                    "end_y": { "type": "number", "description": "End Y coordinate" },
                    "duration_ms": { "type": "integer", "description": "Swipe duration in ms (default 350)" }
                },
                "required": ["start_x", "start_y", "end_x", "end_y"]
            }
        }),
        json!({
            "name": "type_text",
            "description": "Type text into the currently focused editable input field (or optionally click a node by text/id first).",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to enter" },
                    "clear_first": { "type": "boolean", "description": "Replace existing text (default true)" }
                },
                "required": ["text"]
            }
        }),
        json!({
            "name": "click_node",
            "description": "Find an accessibility node by visible text, content description, or view ID and click it.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Text, content description, or resource ID substring to match" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "global_action",
            "description": "Perform an Android global system action / hardware key press.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["back", "home", "recents", "notifications", "quick_settings", "power_dialog", "lock_screen", "take_screenshot"],
                        "description": "Global action to execute"
                    }
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "launch_app",
            "description": "Launch an installed Android application by package name (e.g. 'com.android.settings') or app name.",
            "parameters": {
                "type": "object",
                "properties": {
                    "package_or_name": { "type": "string", "description": "Package name or app label" }
                },
                "required": ["package_or_name"]
            }
        }),
        json!({
            "name": "get_ui_tree",
            "description": "Fetch the current Android Accessibility UI hierarchy with element bounds, text, and clickable states.",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "wait",
            "description": "Wait for a given number of milliseconds (e.g. while an app or webpage loads).",
            "parameters": {
                "type": "object",
                "properties": {
                    "ms": { "type": "integer", "description": "Milliseconds to wait (max 10000)" }
                },
                "required": ["ms"]
            }
        }),
        json!({
            "name": "capture_screenshot",
            "description": "Capture the current Android screen as an image to share with the user, optionally cropping to a region in device screen pixel coordinates. Returns a URL and Markdown image tag to include in your reply.",
            "parameters": {
                "type": "object",
                "properties": {
                    "caption": { "type": "string", "description": "Short alt text / caption for the image" },
                    "crop": {
                        "type": "object",
                        "description": "Optional crop rectangle in device screen pixel coordinates (same coordinate space as UI tree bounds)",
                        "properties": {
                            "x": { "type": "integer", "description": "Left X pixel coordinate" },
                            "y": { "type": "integer", "description": "Top Y pixel coordinate" },
                            "width": { "type": "integer", "description": "Crop width in pixels" },
                            "height": { "type": "integer", "description": "Crop height in pixels" }
                        },
                        "required": ["x", "y", "width", "height"]
                    }
                }
            }
        }),
        json!({
            "name": "crop_image",
            "description": "Crop a previously captured image (from capture_screenshot or crop_image) to a tighter region and return a new Markdown image tag to show the user.",
            "parameters": {
                "type": "object",
                "properties": {
                    "source_url": { "type": "string", "description": "The /api/media/<file>.jpg URL returned by capture_screenshot or crop_image" },
                    "x": { "type": "integer", "description": "Left X coordinate in pixels of the source image" },
                    "y": { "type": "integer", "description": "Top Y coordinate in pixels of the source image" },
                    "width": { "type": "integer", "description": "Crop width in pixels" },
                    "height": { "type": "integer", "description": "Crop height in pixels" },
                    "caption": { "type": "string", "description": "Short alt text / caption for the cropped image" }
                },
                "required": ["source_url", "x", "y", "width", "height"]
            }
        }),
    ]
}

pub async fn fetch_provider_models(
    client: &reqwest::Client,
    provider: &Provider,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err(format!(
            "No API key configured for {}.",
            provider.display_name()
        ));
    }

    match provider {
        Provider::Claude => {
            let resp = client
                .get("https://api.anthropic.com/v1/models?limit=100")
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Claude models: {e}"))?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
            if !status.is_success() {
                return Err(format!("Claude models API ({status}): {text}"));
            }
            let data: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Invalid JSON: {e}"))?;
            let models: Vec<String> = data["data"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            Ok(models)
        }
        Provider::Gemini => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models?pageSize=100&key={key}"
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Gemini models: {e}"))?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
            if !status.is_success() {
                return Err(format!("Gemini models API ({status}): {text}"));
            }
            let data: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Invalid JSON: {e}"))?;
            let mut models: Vec<String> = data["models"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter(|m| {
                            m["supportedGenerationMethods"]
                                .as_array()
                                .map(|methods| {
                                    methods
                                        .iter()
                                        .any(|v| v.as_str() == Some("generateContent"))
                                })
                                .unwrap_or(true)
                        })
                        .filter_map(|m| {
                            m["name"]
                                .as_str()
                                .map(|s| s.strip_prefix("models/").unwrap_or(s).to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();
            models.sort();
            models.reverse();
            Ok(models)
        }
        Provider::Grok => {
            let resp = client
                .get("https://api.x.ai/v1/models")
                .bearer_auth(key)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Grok models: {e}"))?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
            if !status.is_success() {
                return Err(format!("Grok models API ({status}): {text}"));
            }
            let data: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Invalid JSON: {e}"))?;
            let mut models: Vec<String> = data["data"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            models.sort();
            models.reverse();
            Ok(models)
        }
        Provider::OpenAI => {
            let resp = client
                .get("https://api.openai.com/v1/models")
                .bearer_auth(key)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch OpenAI models: {e}"))?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
            if !status.is_success() {
                return Err(format!("OpenAI models API ({status}): {text}"));
            }
            let data: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Invalid JSON: {e}"))?;
            let all_models: Vec<String> = data["data"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let mut chat_models: Vec<String> = all_models
                .iter()
                .filter(|id| {
                    let s = id.as_str();
                    (s.starts_with("gpt-")
                        || s.starts_with("o1")
                        || s.starts_with("o3")
                        || s.starts_with("o4")
                        || s.starts_with("chatgpt-"))
                        && !s.contains("instruct")
                        && !s.contains("realtime")
                        && !s.contains("audio")
                        && !s.contains("transcribe")
                        && !s.contains("tts")
                })
                .cloned()
                .collect();
            if chat_models.is_empty() {
                chat_models = all_models;
            }
            chat_models.sort();
            chat_models.reverse();
            Ok(chat_models)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GeminiGroundingOptions {
    pub google_search: bool,
    pub google_maps: bool,
}

pub async fn call_llm(
    client: &reqwest::Client,
    provider: &Provider,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    messages: &[TurnMessage],
    gemini_grounding: GeminiGroundingOptions,
) -> Result<LlmResponse, String> {
    if api_key.trim().is_empty() {
        return Err(format!(
            "No API key configured for {}. Please add your API key in Settings.",
            provider.display_name()
        ));
    }

    match provider {
        Provider::OpenAI => {
            call_openai_compatible(
                client,
                "https://api.openai.com/v1/chat/completions",
                api_key,
                model,
                system_prompt,
                messages,
            )
            .await
        }
        Provider::Grok => {
            call_openai_compatible(
                client,
                "https://api.x.ai/v1/chat/completions",
                api_key,
                model,
                system_prompt,
                messages,
            )
            .await
        }
        Provider::Claude => {
            call_claude(client, api_key, model, system_prompt, messages).await
        }
        Provider::Gemini => {
            call_gemini(
                client,
                api_key,
                model,
                system_prompt,
                messages,
                gemini_grounding,
            )
            .await
        }
    }
}

fn format_api_error(provider_name: &str, status: reqwest::StatusCode, body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        let msg = v["error"]["message"]
            .as_str()
            .or_else(|| v["message"].as_str())
            .or_else(|| v["detail"].as_str());
        if let Some(m) = msg {
            return format!("{provider_name} ({}): {}", status.as_u16(), m.trim());
        }
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        format!("{provider_name} ({status})")
    } else {
        format!("{provider_name} ({}): {trimmed}", status.as_u16())
    }
}

async fn call_openai_compatible(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    messages: &[TurnMessage],
) -> Result<LlmResponse, String> {
    let mut api_messages = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    for msg in messages {
        match msg {
            TurnMessage::User {
                text,
                screenshot_jpeg,
            } => {
                if let Some(jpeg) = screenshot_jpeg {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
                    api_messages.push(json!({
                        "role": "user",
                        "content": [
                            { "type": "text", "text": text },
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:image/jpeg;base64,{b64}")
                                }
                            }
                        ]
                    }));
                } else {
                    api_messages.push(json!({
                        "role": "user",
                        "content": text
                    }));
                }
            }
            TurnMessage::Assistant { text, tool_calls } => {
                let mut obj = json!({
                    "role": "assistant",
                    "content": if text.is_empty() { Value::Null } else { Value::String(text.clone()) }
                });
                if !tool_calls.is_empty() {
                    let calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "{}".to_string())
                                }
                            })
                        })
                        .collect();
                    obj["tool_calls"] = Value::Array(calls);
                }
                api_messages.push(obj);
            }
            TurnMessage::ToolResult {
                call_id,
                result,
                ..
            } => {
                api_messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": result
                }));
            }
        }
    }

    let tools: Vec<Value> = device_tool_definitions()
        .into_iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": t
            })
        })
        .collect();

    let body = json!({
        "model": model,
        "messages": api_messages,
        "tools": tools,
        "tool_choice": "auto"
    });

    let resp = client
        .post(endpoint)
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    let status = resp.status();
    let resp_text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
    if !status.is_success() {
        return Err(format_api_error("API error", status, &resp_text));
    }

    let data: Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Invalid JSON response: {e}"))?;
    let message = &data["choices"][0]["message"];
    let text = message["content"].as_str().unwrap_or("").to_string();

    let mut tool_calls = Vec::new();
    if let Some(arr) = message["tool_calls"].as_array() {
        for tc in arr {
            let id = tc["id"].as_str().unwrap_or("call_0").to_string();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
            let arguments: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));
            if !name.is_empty() {
                tool_calls.push(ToolCallRequest {
                    id,
                    name,
                    arguments,
                });
            }
        }
    }

    Ok(LlmResponse { text, tool_calls })
}

async fn call_claude(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    messages: &[TurnMessage],
) -> Result<LlmResponse, String> {
    let mut api_messages: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            TurnMessage::User {
                text,
                screenshot_jpeg,
            } => {
                let mut content = vec![json!({ "type": "text", "text": text })];
                if let Some(jpeg) = screenshot_jpeg {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
                    content.push(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/jpeg",
                            "data": b64
                        }
                    }));
                }
                api_messages.push(json!({ "role": "user", "content": content }));
            }
            TurnMessage::Assistant { text, tool_calls } => {
                let mut content = Vec::new();
                if !text.is_empty() {
                    content.push(json!({ "type": "text", "text": text }));
                }
                for tc in tool_calls {
                    content.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": tc.arguments
                    }));
                }
                api_messages.push(json!({ "role": "assistant", "content": content }));
            }
            TurnMessage::ToolResult {
                call_id,
                result,
                ..
            } => {
                let block = json!({
                    "type": "tool_result",
                    "tool_use_id": call_id,
                    "content": result
                });
                if let Some(last) = api_messages.last_mut() {
                    if last["role"] == "user" {
                        if let Some(arr) = last["content"].as_array_mut() {
                            arr.push(block);
                            continue;
                        }
                    }
                }
                api_messages.push(json!({ "role": "user", "content": [block] }));
            }
        }
    }

    let tools: Vec<Value> = device_tool_definitions()
        .into_iter()
        .map(|t| {
            json!({
                "name": t["name"],
                "description": t["description"],
                "input_schema": t["parameters"]
            })
        })
        .collect();

    let body = json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "tools": tools,
        "messages": api_messages
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    let status = resp.status();
    let resp_text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
    if !status.is_success() {
        return Err(format_api_error("Claude API error", status, &resp_text));
    }

    let data: Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Invalid JSON response: {e}"))?;

    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(blocks) = data["content"].as_array() {
        for block in blocks {
            match block["type"].as_str().unwrap_or("") {
                "text" => {
                    if let Some(t) = block["text"].as_str() {
                        text_parts.push(t.to_string());
                    }
                }
                "tool_use" => {
                    let id = block["id"].as_str().unwrap_or("toolu_0").to_string();
                    let name = block["name"].as_str().unwrap_or("").to_string();
                    let arguments = block["input"].clone();
                    if !name.is_empty() {
                        tool_calls.push(ToolCallRequest {
                            id,
                            name,
                            arguments,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(LlmResponse {
        text: text_parts.join("\n"),
        tool_calls,
    })
}

async fn call_gemini(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    messages: &[TurnMessage],
    grounding: GeminiGroundingOptions,
) -> Result<LlmResponse, String> {
    let mut contents: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            TurnMessage::User {
                text,
                screenshot_jpeg,
            } => {
                let mut parts = vec![json!({ "text": text })];
                if let Some(jpeg) = screenshot_jpeg {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
                    parts.push(json!({
                        "inlineData": {
                            "mimeType": "image/jpeg",
                            "data": b64
                        }
                    }));
                }
                contents.push(json!({ "role": "user", "parts": parts }));
            }
            TurnMessage::Assistant { text, tool_calls } => {
                let mut parts = Vec::new();
                if !text.is_empty() {
                    parts.push(json!({ "text": text }));
                }
                for tc in tool_calls {
                    parts.push(json!({
                        "functionCall": {
                            "name": tc.name,
                            "args": tc.arguments
                        }
                    }));
                }
                contents.push(json!({ "role": "model", "parts": parts }));
            }
            TurnMessage::ToolResult {
                tool_name, result, ..
            } => {
                let part = json!({
                    "functionResponse": {
                        "name": tool_name,
                        "response": { "result": result }
                    }
                });
                if let Some(last) = contents.last_mut() {
                    if last["role"] == "user" {
                        if let Some(arr) = last["parts"].as_array_mut() {
                            arr.push(part);
                            continue;
                        }
                    }
                }
                contents.push(json!({ "role": "user", "parts": [part] }));
            }
        }
    }

    let function_declarations = device_tool_definitions();
    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model.trim(),
        api_key.trim()
    );

    // Build fallback plan in case a specific Gemini model rejects combined grounding tools
    let mut attempts: Vec<(bool, bool)> = vec![(grounding.google_search, grounding.google_maps)];
    if grounding.google_maps {
        attempts.push((grounding.google_search, false));
    }
    if grounding.google_search || grounding.google_maps {
        attempts.push((false, false));
    }

    let mut resp_text = String::new();
    let mut last_err = String::new();
    let mut succeeded = false;

    for (use_search, use_maps) in attempts {
        let mut tools = vec![json!({
            "functionDeclarations": function_declarations
        })];
        if use_search {
            tools.push(json!({ "googleSearch": {} }));
        }
        if use_maps {
            tools.push(json!({ "googleMaps": {} }));
        }

        let mut body = json!({
            "systemInstruction": {
                "parts": [{ "text": system_prompt }]
            },
            "contents": contents,
            "tools": tools
        });
        if use_search || use_maps {
            body["toolConfig"] = json!({
                "includeServerSideToolInvocations": true
            });
        }

        let resp = client
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
        if status.is_success() {
            resp_text = text;
            succeeded = true;
            break;
        }

        let is_tool_compat_err = status.as_u16() == 400
            && (text.contains("google_maps")
                || text.contains("googleMaps")
                || text.contains("google_search")
                || text.contains("googleSearch")
                || text.contains("includeServerSideToolInvocations")
                || text.contains("include_server_side_tool_invocations")
                || text.contains("Built-in tools")
                || text.contains("toolConfig")
                || text.contains("Tool use with function calling")
                || text.contains("unsupported")
                || text.contains("not supported"));

        last_err = format_api_error("Gemini API error", status, &text);
        if !is_tool_compat_err {
            return Err(last_err);
        }
        log::warn!(
            "Gemini grounding combination (search={use_search}, maps={use_maps}) rejected by model {}; retrying with fallback",
            model
        );
    }

    if !succeeded {
        return Err(last_err);
    }

    let data: Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Invalid JSON response: {e}"))?;

    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(parts) = data["candidates"][0]["content"]["parts"].as_array() {
        for (idx, part) in parts.iter().enumerate() {
            if let Some(t) = part["text"].as_str() {
                text_parts.push(t.to_string());
            }
            if let Some(fc) = part.get("functionCall") {
                let name = fc["name"].as_str().unwrap_or("").to_string();
                let arguments = fc.get("args").cloned().unwrap_or_else(|| json!({}));
                if !name.is_empty() {
                    tool_calls.push(ToolCallRequest {
                        id: format!("gemini_call_{idx}"),
                        name,
                        arguments,
                    });
                }
            }
        }
    }

    let mut final_text = text_parts.join("\n");

    // Append Google Search & Google Maps grounding citations if present
    if tool_calls.is_empty() {
        if let Some(chunks) = data["candidates"][0]["groundingMetadata"]["groundingChunks"].as_array() {
            let mut sources: Vec<String> = Vec::new();
            for chunk in chunks {
                if let Some(web) = chunk.get("web") {
                    if let Some(uri) = web["uri"].as_str() {
                        let title = web["title"].as_str().unwrap_or(uri);
                        let entry = format!("- [{title}]({uri})");
                        if !sources.contains(&entry) {
                            sources.push(entry);
                        }
                    }
                } else if let Some(maps) = chunk.get("maps") {
                    if let Some(uri) = maps["uri"].as_str() {
                        let title = maps["title"].as_str().unwrap_or("Google Maps");
                        let entry = format!("- [{title} (Google Maps)]({uri})");
                        if !sources.contains(&entry) {
                            sources.push(entry);
                        }
                    }
                }
            }
            if !sources.is_empty() {
                if !final_text.is_empty() {
                    final_text.push_str("\n\n");
                }
                final_text.push_str("**Sources:**\n");
                final_text.push_str(&sources.join("\n"));
            }
        }
    }

    Ok(LlmResponse {
        text: final_text,
        tool_calls,
    })
}
