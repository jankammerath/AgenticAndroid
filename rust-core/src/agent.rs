use crate::bridge::DeviceBridge;
use crate::llm::{call_llm, TurnMessage};
use crate::state::{now_ms, AgentEvent, AppState, ChatMessage, ToolTrace};
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
            role: "assistant".to_string(),
            content: format!("Error: {err}"),
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
    let max_steps = config.max_agent_steps.clamp(1, 50);

    // Build initial conversation context
    let mut turn_messages: Vec<TurnMessage> = Vec::new();
    let hist_len = history.len();
    for (idx, msg) in history.iter().enumerate() {
        let is_last = idx + 1 == hist_len;
        if msg.role == "user" {
            if is_last {
                let ui_context = build_device_context(&state, &bridge, config.include_screenshot).await;
                let enriched = format!("{}\n\n[Current Android Device State]\n{}", msg.content, ui_context.0);
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

            let result_str = execute_device_tool(&bridge, &call.name, &call.arguments).await;

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

async fn build_device_context(
    state: &Arc<AppState>,
    bridge: &DeviceBridge,
    include_screenshot: bool,
) -> (String, Option<Vec<u8>>) {
    let ui_tree = bridge.get_ui_tree().await;
    let mut ui_str = serde_json::to_string(&ui_tree).unwrap_or_else(|_| "{}".to_string());
    if ui_str.len() > 14000 {
        ui_str.truncate(14000);
        ui_str.push_str("...(truncated)");
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

pub async fn execute_device_tool(bridge: &DeviceBridge, name: &str, args: &Value) -> String {
    if name == "wait" {
        let ms = args["ms"].as_u64().unwrap_or(500).min(10_000);
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        return json!({ "ok": true, "waited_ms": ms }).to_string();
    }

    let mut payload = args.as_object().cloned().unwrap_or_default();
    payload.insert("action".to_string(), Value::String(name.to_string()));

    match bridge.dispatch_async(Value::Object(payload)).await {
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
