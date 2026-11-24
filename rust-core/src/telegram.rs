use crate::agent::run_agent_turn;
use crate::bridge::DeviceBridge;
use crate::state::{AgentEvent, AppState, Session};
use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub fn spawn_telegram_worker(state: Arc<AppState>, bridge: DeviceBridge) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut offset: i64 = 0;
        let mut active_token = String::new();

        loop {
            let tg_cfg = state.config.read().await.telegram.clone();
            let token = tg_cfg.bot_token.trim().to_string();

            if !tg_cfg.enabled || token.is_empty() {
                if !active_token.is_empty() {
                    active_token.clear();
                    offset = 0;
                }
                *state.telegram_status.write().await = "Disabled".to_string();
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            if token != active_token {
                active_token = token.clone();
                offset = 0;
                match fetch_bot_username(&state.http_client, &token).await {
                    Ok(username) => {
                        *state.telegram_status.write().await = format!("Connected (@{username})");
                        log::info!("Telegram bot connected as @{username}");
                    }
                    Err(e) => {
                        *state.telegram_status.write().await = format!("Error: {e}");
                        log::warn!("Telegram getMe failed: {e}");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }

            let url = format!(
                "https://api.telegram.org/bot{token}/getUpdates?timeout=25&offset={offset}"
            );
            let resp = match state.http_client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    *state.telegram_status.write().await = format!("Reconnecting ({e})");
                    sleep(Duration::from_secs(4)).await;
                    continue;
                }
            };

            let status = resp.status();
            let body = match resp.text().await {
                Ok(t) => t,
                Err(_) => {
                    sleep(Duration::from_secs(3)).await;
                    continue;
                }
            };

            if !status.is_success() {
                *state.telegram_status.write().await = format!("API error ({status})");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let parsed: Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            if let Some(updates) = parsed["result"].as_array() {
                for update in updates {
                    if let Some(uid) = update["update_id"].as_i64() {
                        offset = offset.max(uid + 1);
                    }
                    let msg = &update["message"];
                    if msg.is_null() {
                        continue;
                    }
                    let Some(chat_id) = msg["chat"]["id"].as_i64() else {
                        continue;
                    };
                    let Some(text) = msg["text"].as_str() else {
                        continue;
                    };
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }

                    let state_c = state.clone();
                    let bridge_c = bridge.clone();
                    let token_c = token.clone();
                    tokio::spawn(async move {
                        handle_incoming_message(state_c, bridge_c, token_c, chat_id, text).await;
                    });
                }
            }
        }
    })
}

async fn fetch_bot_username(client: &reqwest::Client, token: &str) -> Result<String, String> {
    let url = format!("https://api.telegram.org/bot{token}/getMe");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Invalid JSON: {e}"))?;
    if v["ok"].as_bool() != Some(true) {
        return Err(v["description"].as_str().unwrap_or("Invalid token").to_string());
    }
    Ok(v["result"]["username"]
        .as_str()
        .unwrap_or("bot")
        .to_string())
}

async fn handle_incoming_message(
    state: Arc<AppState>,
    bridge: DeviceBridge,
    token: String,
    chat_id: i64,
    text: String,
) {
    let cfg = state.config.read().await.clone();
    let authorized = cfg.telegram.allowed_chat_ids.contains(&chat_id);

    if text == "/id" || text == "/start" {
        let auth_note = if authorized {
            "Status: Authorized"
        } else {
            "Status: Not authorized yet. Add this Chat ID in Agentic Android Settings -> Telegram."
        };
        let help = format!(
            "Agentic Android Telegram Bot\n\nYour Chat ID: {chat_id}\n{auth_note}\n\nCommands:\n/status - Device & model status\n/screenshot - Capture current screen\n/new - Start a new session\n/stop - Stop running task\n\nSend any message to control your Android device."
        );
        let _ = send_text(&state.http_client, &token, chat_id, &help).await;
        return;
    }

    if !authorized {
        let _ = send_text(
            &state.http_client,
            &token,
            chat_id,
            &format!(
                "Unauthorized Chat ID ({chat_id}). Add {chat_id} to Allowed Chat IDs in Agentic Android Settings -> Telegram."
            ),
        )
        .await;
        return;
    }

    if text == "/status" {
        let dev = bridge.get_status().await;
        let name = dev["device_name"].as_str().unwrap_or("Android Device");
        let acc = if dev["accessibility_enabled"].as_bool().unwrap_or(false) {
            "Active"
        } else {
            "Disabled"
        };
        let reply = format!(
            "Device: {name}\nAccessibility: {acc}\nDefault Provider: {}\nDefault Model: {}",
            cfg.default_provider.display_name(),
            cfg.default_model
        );
        let _ = send_text(&state.http_client, &token, chat_id, &reply).await;
        return;
    }

    if text == "/screenshot" {
        let _ = bridge.request_screenshot().await;
        sleep(Duration::from_millis(300)).await;
        if let Some(frame) = state.latest_frame.read().await.clone() {
            let _ = send_photo(
                &state.http_client,
                &token,
                chat_id,
                frame.jpeg.to_vec(),
                "Current Android screen",
            )
            .await;
        } else {
            let _ = send_text(
                &state.http_client,
                &token,
                chat_id,
                "No screenshot frame available yet.",
            )
            .await;
        }
        return;
    }

    if text == "/new" {
        let session_id = create_telegram_session(&state, chat_id).await;
        let _ = state.event_tx.send(AgentEvent::SessionUpdated { session_id });
        let _ = send_text(
            &state.http_client,
            &token,
            chat_id,
            "Started a new Agentic Android session.",
        )
        .await;
        return;
    }

    if text == "/stop" {
        let session_id = state.telegram_sessions.lock().await.get(&chat_id).cloned();
        if let Some(sid) = session_id {
            let active = state.active_runs.lock().await;
            if let Some(tx) = active.get(&sid) {
                let _ = tx.send(true);
                let _ = send_text(&state.http_client, &token, chat_id, "Stopping current task...").await;
                return;
            }
        }
        let _ = send_text(&state.http_client, &token, chat_id, "No active task is running.").await;
        return;
    }

    // Regular prompt -> run agent turn
    let _ = send_typing(&state.http_client, &token, chat_id).await;
    let session_id = get_or_create_telegram_session(&state, chat_id).await;

    run_agent_turn(state.clone(), bridge, session_id.clone(), text).await;

    // Fetch latest assistant/error message from the session
    let last_msg = {
        let sessions = state.sessions.read().await;
        sessions
            .iter()
            .find(|s| s.id == session_id)
            .and_then(|s| s.messages.last().cloned())
    };

    if let Some(msg) = last_msg {
        let (clean_text, media_files) = extract_media_markdown(&msg.content);
        if !clean_text.is_empty() {
            let _ = send_text(&state.http_client, &token, chat_id, &clean_text).await;
        }
        for (caption, filename) in media_files {
            let path = state.config_dir.join("media").join(&filename);
            if let Ok(bytes) = tokio::fs::read(&path).await {
                let _ = send_photo(&state.http_client, &token, chat_id, bytes, &caption).await;
            }
        }
    }
}

async fn get_or_create_telegram_session(state: &Arc<AppState>, chat_id: i64) -> String {
    if let Some(existing_id) = state.telegram_sessions.lock().await.get(&chat_id).cloned() {
        let exists = state
            .sessions
            .read()
            .await
            .iter()
            .any(|s| s.id == existing_id);
        if exists {
            return existing_id;
        }
    }
    create_telegram_session(state, chat_id).await
}

async fn create_telegram_session(state: &Arc<AppState>, chat_id: i64) -> String {
    let cfg = state.config.read().await.clone();
    let model = if !cfg.default_model.trim().is_empty() {
        cfg.default_model.clone()
    } else {
        cfg.default_provider.default_model().to_string()
    };
    let session = Session::new_telegram(chat_id, cfg.default_provider, model);
    let sid = session.id.clone();
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(0, session);
    }
    state.save_sessions().await;
    state
        .telegram_sessions
        .lock()
        .await
        .insert(chat_id, sid.clone());
    let _ = state
        .event_tx
        .send(AgentEvent::SessionUpdated { session_id: sid.clone() });
    sid
}

fn extract_media_markdown(text: &str) -> (String, Vec<(String, String)>) {
    let mut cleaned = String::new();
    let mut media = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("![") {
        cleaned.push_str(&rest[..start]);
        let after_bang = &rest[start + 2..];
        if let Some(mid) = after_bang.find("](/api/media/") {
            let caption = &after_bang[..mid];
            let after_mid = &after_bang[mid + "](/api/media/".len()..];
            if let Some(end) = after_mid.find(')') {
                let filename = &after_mid[..end];
                if !filename.contains('/') && !filename.contains("..") {
                    media.push((caption.to_string(), filename.to_string()));
                }
                rest = &after_mid[end + 1..];
                continue;
            }
        }
        cleaned.push_str("![");
        rest = after_bang;
    }
    cleaned.push_str(rest);
    (cleaned.trim().to_string(), media)
}

async fn send_typing(client: &reqwest::Client, token: &str, chat_id: i64) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{token}/sendChatAction");
    let _ = client
        .post(&url)
        .json(&json!({ "chat_id": chat_id, "action": "typing" }))
        .send()
        .await;
    Ok(())
}

async fn send_text(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    text: &str,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    // Telegram message limit is 4096 chars; split safely
    let chars: Vec<char> = text.chars().collect();
    for chunk in chars.chunks(3900) {
        let part: String = chunk.iter().collect();
        client
            .post(&url)
            .json(&json!({
                "chat_id": chat_id,
                "text": part
            }))
            .send()
            .await
            .map_err(|e| format!("sendMessage error: {e}"))?;
    }
    Ok(())
}

async fn send_photo(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    jpeg_bytes: Vec<u8>,
    caption: &str,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{token}/sendPhoto");
    let photo_part = Part::bytes(jpeg_bytes)
        .file_name("screenshot.jpg")
        .mime_str("image/jpeg")
        .map_err(|e| format!("MIME error: {e}"))?;
    let mut form = Form::new()
        .text("chat_id", chat_id.to_string())
        .part("photo", photo_part);
    if !caption.trim().is_empty() {
        let short_cap: String = caption.chars().take(900).collect();
        form = form.text("caption", short_cap);
    }
    client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("sendPhoto error: {e}"))?;
    Ok(())
}
