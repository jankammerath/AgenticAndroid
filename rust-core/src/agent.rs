use crate::bridge::DeviceBridge;
use crate::llm::{call_llm, GeminiGroundingOptions, TurnMessage};
use crate::state::{now_ms, AgentEvent, AppPlaybook, AppState, ChatMessage, ToolTrace};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::watch;
use uuid::Uuid;

pub async fn run_agent_turn(
    state: Arc<AppState>,
    bridge: DeviceBridge,
    session_id: String,
    user_text: String,
) {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    {
        let mut active = state.active_runs.lock().await;
        if let Some(prev) = active.insert(session_id.clone(), cancel_tx) {
            let _ = prev.send(true);
        }
    }

    let result = execute_agent_loop(
        state.clone(),
        bridge,
        session_id.clone(),
        user_text,
        cancel_rx,
    )
    .await;

    {
        let mut active = state.active_runs.lock().await;
        active.remove(&session_id);
    }

    let _ = state.event_tx.send(AgentEvent::AgentStatus {
        session_id: session_id.clone(),
        running: false,
        step: 0,
        status: "Idle".to_string(),
    });

    if let Err(err) = result {
        let _ = state.event_tx.send(AgentEvent::Error {
            session_id: session_id.clone(),
            message: err.clone(),
        });
        let err_msg = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "error".to_string(),
            content: err,
            tool_traces: Vec::new(),
            timestamp_ms: now_ms(),
        };
        append_session_message(&state, &session_id, err_msg.clone()).await;
        let _ = state.event_tx.send(AgentEvent::AssistantMessage {
            session_id,
            message: err_msg,
        });
    }
}

async fn execute_agent_loop(
    state: Arc<AppState>,
    bridge: DeviceBridge,
    session_id: String,
    user_text: String,
    cancel_rx: watch::Receiver<bool>,
) -> Result<(), String> {
    let user_msg = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: user_text.clone(),
        tool_traces: Vec::new(),
        timestamp_ms: now_ms(),
    };
    append_session_message(&state, &session_id, user_msg).await;
    let _ = state.event_tx.send(AgentEvent::SessionUpdated {
        session_id: session_id.clone(),
    });

    let (provider, model, history) = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .iter()
            .find(|s| s.id == session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        (
            session.provider.clone(),
            session.model.clone(),
            session.messages.clone(),
        )
    };

    let config = state.config.read().await.clone();
    let api_key = config.api_keys.get(&provider).to_string();
    let max_steps = config.max_agent_steps.clamp(1, 1024);

    // Build initial conversation context
    let mut turn_messages: Vec<TurnMessage> = Vec::new();
    let hist_len = history.len();
    for (idx, msg) in history.iter().enumerate() {
        let is_last = idx + 1 == hist_len;
        if msg.role == "user" {
            if is_last {
                let ui_context = build_device_context(&state, &bridge, config.include_screenshot).await;
                let app_directory = build_app_directory(&state).await;
                let enriched = format!(
                    "{}{}\n\n[Current Android Device State]\n{}",
                    msg.content, app_directory, ui_context.0
                );
                turn_messages.push(TurnMessage::User {
                    text: enriched,
                    screenshot_jpeg: ui_context.1,
                });
            } else {
                turn_messages.push(TurnMessage::User {
                    text: msg.content.clone(),
                    screenshot_jpeg: None,
                });
            }
        } else if msg.role == "assistant" {
            turn_messages.push(TurnMessage::Assistant {
                text: msg.content.clone(),
                tool_calls: Vec::new(),
            });
        }
    }

    let mut collected_traces: Vec<ToolTrace> = Vec::new();
    let mut final_text = String::new();

    for step in 1..=max_steps {
        if *cancel_rx.borrow() {
            final_text = "Stopped by user.".to_string();
            break;
        }

        let _ = state.event_tx.send(AgentEvent::AgentStatus {
            session_id: session_id.clone(),
            running: true,
            step,
            status: format!("Step {step}/{max_steps}: Thinking ({model})..."),
        });

        let llm_resp = call_llm(
            &state.http_client,
            &provider,
            &api_key,
            &model,
            &config.system_prompt,
            &turn_messages,
            GeminiGroundingOptions {
                google_search: config.gemini_google_search,
                google_maps: config.gemini_google_maps,
            },
        )
        .await?;

        if *cancel_rx.borrow() {
            final_text = "Stopped by user.".to_string();
            break;
        }

        if llm_resp.tool_calls.is_empty() {
            final_text = llm_resp.text;
            break;
        }

        turn_messages.push(TurnMessage::Assistant {
            text: llm_resp.text.clone(),
            tool_calls: llm_resp.tool_calls.clone(),
        });

        for call in llm_resp.tool_calls {
            if *cancel_rx.borrow() {
                break;
            }

            let _ = state.event_tx.send(AgentEvent::AgentStatus {
                session_id: session_id.clone(),
                running: true,
                step,
                status: format!("Step {step}: Executing {}...", call.name),
            });

            let result_str = execute_agent_tool(&state, &bridge, &call.name, &call.arguments).await;

            let trace = ToolTrace {
                tool_name: call.name.clone(),
                arguments: call.arguments.clone(),
                result: result_str.clone(),
            };
            collected_traces.push(trace);

            let _ = state.event_tx.send(AgentEvent::ToolCall {
                session_id: session_id.clone(),
                tool_name: call.name.clone(),
                arguments: call.arguments.clone(),
                result: result_str.clone(),
            });

            turn_messages.push(TurnMessage::ToolResult {
                call_id: call.id,
                tool_name: call.name,
                result: result_str,
            });

            if config.step_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(config.step_delay_ms)).await;
            }
        }

        // Refresh UI state and screenshot for the next step
        let (ui_summary, screenshot) =
            build_device_context(&state, &bridge, config.include_screenshot).await;
        turn_messages.push(TurnMessage::User {
            text: format!("[Updated Android Device State after actions]\n{ui_summary}"),
            screenshot_jpeg: screenshot,
        });

        if step == max_steps {
            final_text = format!(
                "Reached maximum agent steps ({max_steps}). Last response: {}",
                llm_resp.text
            );
        }
    }

    let assistant_msg = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: if final_text.is_empty() {
            "Done.".to_string()
        } else {
            final_text
        },
        tool_traces: collected_traces,
        timestamp_ms: now_ms(),
    };

    append_session_message(&state, &session_id, assistant_msg.clone()).await;
    let _ = state.event_tx.send(AgentEvent::AssistantMessage {
        session_id: session_id.clone(),
        message: assistant_msg,
    });
    let _ = state.event_tx.send(AgentEvent::SessionUpdated { session_id });

    Ok(())
}

async fn build_app_directory(state: &Arc<AppState>) -> String {
    let map = state.app_playbooks.read().await;
    let mut entries: Vec<&AppPlaybook> = map
        .values()
        .filter(|pb| !pb.purpose.trim().is_empty())
        .collect();
    if entries.is_empty() {
        return String::new();
    }
    entries.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms));
    let mut out = String::from("\n\n[Known Apps & What To Use Them For]\n");
    for pb in entries.into_iter().take(40) {
        let name = if !pb.app_label.trim().is_empty() {
            format!("{} ({})", pb.app_label.trim(), pb.package_name)
        } else {
            pb.package_name.clone()
        };
        out.push_str(&format!("- {}: {}\n", name, pb.purpose.trim()));
    }
    out
}

async fn build_device_context(
    state: &Arc<AppState>,
    bridge: &DeviceBridge,
    include_screenshot: bool,
) -> (String, Option<Vec<u8>>) {
    let ui_tree = bridge.get_ui_tree().await;
    let pkg = ui_tree["package"].as_str().unwrap_or("").trim().to_string();
    let current_ver = ui_tree["app_version"].as_str().unwrap_or("").trim().to_string();

    let mut ui_str = serde_json::to_string(&ui_tree).unwrap_or_else(|_| "{}".to_string());
    if ui_str.len() > 14000 {
        ui_str.truncate(14000);
        ui_str.push_str("...(truncated)");
    }

    // Just-in-time injection of saved playbook for the active foreground app
    if !pkg.is_empty() {
        let playbook_opt = state.app_playbooks.read().await.get(&pkg).cloned();
        if let Some(pb) = playbook_opt {
            if !pb.notes.is_empty() {
                let mut block = String::from("\n\n[Saved App Playbook for ");
                block.push_str(&pb.package_name);
                if !pb.app_label.is_empty() {
                    block.push_str(&format!(" ({})", pb.app_label));
                }
                block.push_str("]\n");
                if !pb.app_version.is_empty()
                    && !current_ver.is_empty()
                    && pb.app_version != current_ver
                {
                    block.push_str(&format!(
                        "(Note: recorded on v{}, current app is v{} — verify before assuming exact selectors)\n",
                        pb.app_version, current_ver
                    ));
                }
                for n in pb.notes.iter().take(20) {
                    block.push_str("- ");
                    block.push_str(n);
                    block.push('\n');
                }
                ui_str.push_str(&block);
            }
        }
    }

    let screenshot = if include_screenshot {
        let current = state.latest_frame.read().await.clone();
        if let Some(frame) = current {
            Some(frame.jpeg.to_vec())
        } else {
            let _ = bridge.request_screenshot().await;
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            state
                .latest_frame
                .read()
                .await
                .as_ref()
                .map(|f| f.jpeg.to_vec())
        }
    } else {
        None
    };

    (ui_str, screenshot)
}

pub async fn execute_agent_tool(
    state: &Arc<AppState>,
    bridge: &DeviceBridge,
    name: &str,
    args: &Value,
) -> String {
    match name {
        "capture_screenshot" => handle_capture_screenshot(state, bridge, args).await,
        "crop_image" => handle_crop_image(state, args).await,
        "save_app_note" => handle_save_app_note(state, args).await,
        "delete_app_note" => handle_delete_app_note(state, args).await,
        "get_app_notes" => handle_get_app_notes(state, args).await,
        _ => execute_device_tool(bridge, name, args).await,
    }
}

async fn handle_save_app_note(state: &Arc<AppState>, args: &Value) -> String {
    let pkg = args["package_name"].as_str().unwrap_or("").trim().to_string();
    let purpose = args["purpose"].as_str().unwrap_or("").trim().to_string();
    let note = args["note"].as_str().unwrap_or("").trim().to_string();
    if pkg.is_empty() || (purpose.is_empty() && note.is_empty()) {
        return json!({
            "ok": false,
            "error": "package_name and at least one of purpose or note are required"
        })
        .to_string();
    }
    let label = args["app_label"].as_str().unwrap_or("").trim().to_string();
    let version = args["app_version"].as_str().unwrap_or("").trim().to_string();

    let count = {
        let mut map = state.app_playbooks.write().await;
        let entry = map.entry(pkg.clone()).or_insert_with(|| AppPlaybook {
            package_name: pkg.clone(),
            app_label: label.clone(),
            purpose: purpose.clone(),
            app_version: version.clone(),
            notes: Vec::new(),
            updated_at_ms: now_ms(),
        });
        if !label.is_empty() {
            entry.app_label = label;
        }
        if !purpose.is_empty() {
            entry.purpose = purpose;
        }
        if !version.is_empty() {
            entry.app_version = version;
        }
        if !note.is_empty() {
            let lower = note.to_lowercase();
            if !entry.notes.iter().any(|existing| existing.to_lowercase() == lower) {
                if entry.notes.len() >= 25 {
                    entry.notes.remove(0);
                }
                entry.notes.push(note.clone());
            }
        }
        entry.updated_at_ms = now_ms();
        entry.notes.len()
    };
    state.save_app_playbooks().await;
    json!({ "ok": true, "package_name": pkg, "total_notes": count }).to_string()
}

async fn handle_delete_app_note(state: &Arc<AppState>, args: &Value) -> String {
    let pkg = args["package_name"].as_str().unwrap_or("").trim().to_string();
    let needle = args["match_text"].as_str().unwrap_or("").trim().to_lowercase();
    if pkg.is_empty() || needle.is_empty() {
        return json!({ "ok": false, "error": "package_name and match_text are required" })
            .to_string();
    }

    let removed = {
        let mut map = state.app_playbooks.write().await;
        if let Some(entry) = map.get_mut(&pkg) {
            let before = entry.notes.len();
            entry.notes.retain(|n| !n.to_lowercase().contains(&needle));
            entry.updated_at_ms = now_ms();
            let diff = before - entry.notes.len();
            if entry.notes.is_empty() {
                map.remove(&pkg);
            }
            diff
        } else {
            0
        }
    };
    state.save_app_playbooks().await;
    json!({ "ok": true, "package_name": pkg, "removed": removed }).to_string()
}

async fn handle_get_app_notes(state: &Arc<AppState>, args: &Value) -> String {
    let pkg = args["package_name"].as_str().unwrap_or("").trim();
    if pkg.is_empty() {
        return json!({ "ok": false, "error": "package_name is required" }).to_string();
    }
    let map = state.app_playbooks.read().await;
    match map.get(pkg) {
        Some(pb) => serde_json::to_string(pb).unwrap_or_else(|_| "{}".to_string()),
        None => json!({ "ok": true, "package_name": pkg, "notes": [] }).to_string(),
    }
}

async fn handle_capture_screenshot(
    state: &Arc<AppState>,
    bridge: &DeviceBridge,
    args: &Value,
) -> String {
    // Request a fresh screenshot if fallback mode is active
    let _ = bridge.request_screenshot().await;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let frame = match state.latest_frame.read().await.clone() {
        Some(f) => f,
        None => {
            return json!({
                "ok": false,
                "error": "No screenshot frame available from device."
            })
            .to_string();
        }
    };

    let caption = args["caption"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Screenshot")
        .to_string();
    let crop_opt = args.get("crop").and_then(|c| c.as_object()).cloned();
    let media_dir = state.config_dir.join("media");

    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        std::fs::create_dir_all(&media_dir).map_err(|e| format!("Failed to create media dir: {e}"))?;
        let img = image::load_from_memory(&frame.jpeg)
            .map_err(|e| format!("Failed to decode screenshot JPEG: {e}"))?;
        let img_w = img.width();
        let img_h = img.height();

        let final_img = if let Some(crop) = crop_opt {
            let dev_w = frame.width.max(1) as f64;
            let dev_h = frame.height.max(1) as f64;
            let scale_x = img_w as f64 / dev_w;
            let scale_y = img_h as f64 / dev_h;

            let rx = crop.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0);
            let ry = crop.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0);
            let rw = crop.get("width").and_then(|v| v.as_f64()).unwrap_or(dev_w).max(1.0);
            let rh = crop.get("height").and_then(|v| v.as_f64()).unwrap_or(dev_h).max(1.0);

            let cx = ((rx * scale_x).round() as u32).min(img_w.saturating_sub(1));
            let cy = ((ry * scale_y).round() as u32).min(img_h.saturating_sub(1));
            let cw = ((rw * scale_x).round() as u32).max(1).min(img_w.saturating_sub(cx));
            let ch = ((rh * scale_y).round() as u32).max(1).min(img_h.saturating_sub(cy));

            img.crop_imm(cx, cy, cw, ch)
        } else {
            img
        };

        let out_w = final_img.width();
        let out_h = final_img.height();
        let filename = format!("{}.jpg", Uuid::new_v4());
        let out_path = media_dir.join(&filename);
        final_img
            .to_rgb8()
            .save_with_format(&out_path, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to save JPEG: {e}"))?;

        let url = format!("/api/media/{filename}");
        let safe_caption = caption.replace(['[', ']'], "");
        Ok(json!({
            "ok": true,
            "url": url,
            "markdown": format!("![{safe_caption}]({url})"),
            "width": out_w,
            "height": out_h
        }))
    })
    .await;

    match res {
        Ok(Ok(val)) => val.to_string(),
        Ok(Err(e)) => json!({ "ok": false, "error": e }).to_string(),
        Err(e) => json!({ "ok": false, "error": format!("Image task failed: {e}") }).to_string(),
    }
}

async fn handle_crop_image(state: &Arc<AppState>, args: &Value) -> String {
    let source_url = args["source_url"].as_str().unwrap_or("").trim();
    let filename = source_url.strip_prefix("/api/media/").unwrap_or(source_url);
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return json!({
            "ok": false,
            "error": "Invalid source_url. Pass a /api/media/<filename>.jpg URL from capture_screenshot."
        })
        .to_string();
    }

    let x = args["x"].as_i64().unwrap_or(0).max(0) as u32;
    let y = args["y"].as_i64().unwrap_or(0).max(0) as u32;
    let w = args["width"].as_i64().unwrap_or(0).max(1) as u32;
    let h = args["height"].as_i64().unwrap_or(0).max(1) as u32;
    let caption = args["caption"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Cropped image")
        .to_string();

    let media_dir = state.config_dir.join("media");
    let src_path = media_dir.join(filename);

    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let img = image::open(&src_path)
            .map_err(|e| format!("Failed to open source image: {e}"))?;
        let img_w = img.width();
        let img_h = img.height();

        let cx = x.min(img_w.saturating_sub(1));
        let cy = y.min(img_h.saturating_sub(1));
        let cw = w.max(1).min(img_w.saturating_sub(cx));
        let ch = h.max(1).min(img_h.saturating_sub(cy));

        let cropped = img.crop_imm(cx, cy, cw, ch);
        let out_filename = format!("{}.jpg", Uuid::new_v4());
        let out_path = media_dir.join(&out_filename);
        cropped
            .to_rgb8()
            .save_with_format(&out_path, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to save cropped JPEG: {e}"))?;

        let url = format!("/api/media/{out_filename}");
        let safe_caption = caption.replace(['[', ']'], "");
        Ok(json!({
            "ok": true,
            "url": url,
            "markdown": format!("![{safe_caption}]({url})"),
            "width": cropped.width(),
            "height": cropped.height()
        }))
    })
    .await;

    match res {
        Ok(Ok(val)) => val.to_string(),
        Ok(Err(e)) => json!({ "ok": false, "error": e }).to_string(),
        Err(e) => json!({ "ok": false, "error": format!("Crop task failed: {e}") }).to_string(),
    }
}

pub async fn execute_device_tool(bridge: &DeviceBridge, name: &str, args: &Value) -> String {
    if name == "wait" {
        let ms = args["ms"].as_u64().unwrap_or(500).min(10_000);
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        return json!({ "ok": true, "waited_ms": ms }).to_string();
    }

    let payload = json!({
        "tool": name,
        "params": args
    });

    match bridge.dispatch_async(payload).await {
        Ok(val) => val.to_string(),
        Err(err) => json!({ "ok": false, "error": err }).to_string(),
    }
}

async fn append_session_message(state: &Arc<AppState>, session_id: &str, msg: ChatMessage) {
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
            if session.messages.is_empty() && msg.role == "user" {
                let title: String = msg.content.chars().take(36).collect();
                if !title.is_empty() {
                    session.title = title;
                }
            }
            session.messages.push(msg);
            session.updated_at_ms = now_ms();
        }
    }
    state.save_sessions().await;
}
