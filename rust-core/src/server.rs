use crate::agent::{execute_device_tool, run_agent_turn};
use crate::bridge::DeviceBridge;
use crate::llm::fetch_provider_models;
use crate::state::{AppState, Provider, Session};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

const INDEX_HTML: &str = include_str!("../web/index.html");
const STYLE_CSS: &str = include_str!("../web/style.css");
const APP_JS: &str = include_str!("../web/app.js");

#[derive(Clone)]
pub struct ServerCtx {
    pub state: Arc<AppState>,
    pub bridge: DeviceBridge,
}

pub async fn start_http_server(
    state: Arc<AppState>,
    bridge: DeviceBridge,
    port: u16,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    let ctx = ServerCtx { state, bridge };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/api/health", get(api_health))
        .route("/api/status", get(api_status))
        .route("/api/config", get(api_get_config).put(api_update_config))
        .route("/api/providers", get(api_providers))
        .route("/api/providers/:provider/models", get(api_provider_models))
        .route("/api/sessions", get(api_list_sessions).post(api_create_session))
        .route(
            "/api/sessions/:id",
            get(api_get_session)
                .put(api_update_session)
                .delete(api_delete_session),
        )
        .route("/api/sessions/:id/message", post(api_send_message))
        .route("/api/sessions/:id/stop", post(api_stop_session))
        .route("/api/device/action", post(api_device_action))
        .route("/api/device/ui-tree", get(api_ui_tree))
        .route("/api/device/screenshot", get(api_screenshot))
        .route("/ws/video", get(ws_video))
        .route("/ws/audio", get(ws_audio))
        .route("/ws/events", get(ws_events))
        .layer(cors)
        .with_state(ctx);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind port {port}: {e}"))?;

    log::info!("Agentic Android Rust webserver listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = (&mut shutdown_rx).await;
            log::info!("Shutting down Agentic Android webserver");
        })
        .await
        .map_err(|e| format!("Server error: {e}"))
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
        .into_response()
}

async fn serve_js() -> Response {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS,
    )
        .into_response()
}

async fn api_health() -> Json<Value> {
    Json(json!({ "ok": true, "service": "agentic-android-rust" }))
}

async fn api_status(State(ctx): State<ServerCtx>) -> Json<Value> {
    let device_status = ctx.bridge.get_status().await;
    let frame_info = ctx.state.latest_frame.read().await.as_ref().map(|f| {
        json!({
            "width": f.width,
            "height": f.height,
            "timestamp_ms": f.timestamp_ms,
        })
    });
    let cfg = ctx.state.config.read().await;
    Json(json!({
        "ok": true,
        "port": ctx.state.port,
        "device": device_status,
        "latest_frame": frame_info,
        "configured_providers": {
            "claude": !cfg.api_keys.claude.trim().is_empty(),
            "gemini": !cfg.api_keys.gemini.trim().is_empty(),
            "grok": !cfg.api_keys.grok.trim().is_empty(),
            "openai": !cfg.api_keys.openai.trim().is_empty(),
        }
    }))
}

async fn api_providers() -> Json<Value> {
    Json(json!({
        "providers": [
            {
                "id": "claude",
                "name": "Claude (Anthropic)",
                "models": ["claude-sonnet-4-20250514", "claude-opus-4-20250514", "claude-3-7-sonnet-latest", "claude-3-5-haiku-latest"]
            },
            {
                "id": "gemini",
                "name": "Gemini (Google)",
                "models": ["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"]
            },
            {
                "id": "grok",
                "name": "Grok (xAI)",
                "models": ["grok-3", "grok-3-mini", "grok-2-vision-1212"]
            },
            {
                "id": "openai",
                "name": "OpenAI",
                "models": ["gpt-4o", "gpt-4o-mini", "gpt-4.1", "o4-mini"]
            }
        ]
    }))
}

async fn api_provider_models(
    State(ctx): State<ServerCtx>,
    Path(provider_str): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let provider = match provider_str.to_lowercase().as_str() {
        "claude" => Provider::Claude,
        "gemini" => Provider::Gemini,
        "grok" => Provider::Grok,
        "openai" => Provider::OpenAI,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let api_key = {
        let cfg = ctx.state.config.read().await;
        cfg.api_keys.get(&provider).to_string()
    };

    match fetch_provider_models(&ctx.state.http_client, &provider, &api_key).await {
        Ok(models) if !models.is_empty() => Ok(Json(json!({
            "ok": true,
            "source": "api",
            "provider": provider,
            "models": models
        }))),
        Ok(_) => Ok(Json(json!({
            "ok": false,
            "source": "fallback",
            "provider": provider,
            "error": "Provider returned an empty model list.",
            "models": [provider.default_model()]
        }))),
        Err(err) => Ok(Json(json!({
            "ok": false,
            "source": "fallback",
            "provider": provider,
            "error": err,
            "models": [provider.default_model()]
        }))),
    }
}

#[derive(Serialize)]
struct SafeConfigResponse {
    api_keys_configured: Value,
    api_keys_masked: Value,
    default_provider: Provider,
    default_model: String,
    max_agent_steps: usize,
    step_delay_ms: u64,
    include_screenshot: bool,
    system_prompt: String,
}

fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}••••{suffix}")
}

async fn api_get_config(State(ctx): State<ServerCtx>) -> Json<SafeConfigResponse> {
    let cfg = ctx.state.config.read().await.clone();
    Json(SafeConfigResponse {
        api_keys_configured: json!({
            "claude": !cfg.api_keys.claude.trim().is_empty(),
            "gemini": !cfg.api_keys.gemini.trim().is_empty(),
            "grok": !cfg.api_keys.grok.trim().is_empty(),
            "openai": !cfg.api_keys.openai.trim().is_empty(),
        }),
        api_keys_masked: json!({
            "claude": mask_key(&cfg.api_keys.claude),
            "gemini": mask_key(&cfg.api_keys.gemini),
            "grok": mask_key(&cfg.api_keys.grok),
            "openai": mask_key(&cfg.api_keys.openai),
        }),
        default_provider: cfg.default_provider,
        default_model: cfg.default_model,
        max_agent_steps: cfg.max_agent_steps,
        step_delay_ms: cfg.step_delay_ms,
        include_screenshot: cfg.include_screenshot,
        system_prompt: cfg.system_prompt,
    })
}

#[derive(Deserialize)]
struct UpdateConfigRequest {
    api_keys: Option<ApiKeysPatch>,
    default_provider: Option<Provider>,
    default_model: Option<String>,
    max_agent_steps: Option<usize>,
    step_delay_ms: Option<u64>,
    include_screenshot: Option<bool>,
    system_prompt: Option<String>,
}

#[derive(Deserialize)]
struct ApiKeysPatch {
    claude: Option<String>,
    gemini: Option<String>,
    grok: Option<String>,
    openai: Option<String>,
}

async fn api_update_config(
    State(ctx): State<ServerCtx>,
    Json(req): Json<UpdateConfigRequest>,
) -> Json<Value> {
    {
        let mut cfg = ctx.state.config.write().await;
        if let Some(keys) = req.api_keys {
            if let Some(v) = keys.claude {
                cfg.api_keys.claude = v.trim().to_string();
            }
            if let Some(v) = keys.gemini {
                cfg.api_keys.gemini = v.trim().to_string();
            }
            if let Some(v) = keys.grok {
                cfg.api_keys.grok = v.trim().to_string();
            }
            if let Some(v) = keys.openai {
                cfg.api_keys.openai = v.trim().to_string();
            }
        }
        if let Some(p) = req.default_provider {
            cfg.default_provider = p;
        }
        if let Some(m) = req.default_model {
            if !m.trim().is_empty() {
                cfg.default_model = m.trim().to_string();
            }
        }
        if let Some(steps) = req.max_agent_steps {
            cfg.max_agent_steps = steps.clamp(1, 50);
        }
        if let Some(delay) = req.step_delay_ms {
            cfg.step_delay_ms = delay.min(5000);
        }
        if let Some(inc) = req.include_screenshot {
            cfg.include_screenshot = inc;
        }
        if let Some(prompt) = req.system_prompt {
            cfg.system_prompt = prompt;
        }
    }
    ctx.state.save_config().await;
    Json(json!({ "ok": true }))
}

async fn api_list_sessions(State(ctx): State<ServerCtx>) -> Json<Vec<Session>> {
    let sessions = ctx.state.sessions.read().await.clone();
    Json(sessions)
}

#[derive(Deserialize)]
struct CreateSessionRequest {
    title: Option<String>,
    provider: Option<Provider>,
    model: Option<String>,
}

async fn api_create_session(
    State(ctx): State<ServerCtx>,
    Json(req): Json<CreateSessionRequest>,
) -> Json<Session> {
    let cfg = ctx.state.config.read().await.clone();
    let provider = req.provider.unwrap_or(cfg.default_provider);
    let model = req
        .model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| provider.default_model().to_string());
    let title = req
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "New Session".to_string());

    let session = Session::new(title, provider, model);
    {
        let mut sessions = ctx.state.sessions.write().await;
        sessions.insert(0, session.clone());
    }
    ctx.state.save_sessions().await;
    Json(session)
}

async fn api_get_session(
    State(ctx): State<ServerCtx>,
    Path(id): Path<String>,
) -> Result<Json<Session>, StatusCode> {
    let sessions = ctx.state.sessions.read().await;
    sessions
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct UpdateSessionRequest {
    title: Option<String>,
    provider: Option<Provider>,
    model: Option<String>,
}

async fn api_update_session(
    State(ctx): State<ServerCtx>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<Json<Session>, StatusCode> {
    let updated = {
        let mut sessions = ctx.state.sessions.write().await;
        let session = sessions.iter_mut().find(|s| s.id == id).ok_or(StatusCode::NOT_FOUND)?;
        if let Some(title) = req.title {
            session.title = title;
        }
        if let Some(provider) = req.provider {
            session.provider = provider;
        }
        if let Some(model) = req.model {
            if !model.trim().is_empty() {
                session.model = model.trim().to_string();
            }
        }
        session.clone()
    };
    ctx.state.save_sessions().await;
    Ok(Json(updated))
}

async fn api_delete_session(
    State(ctx): State<ServerCtx>,
    Path(id): Path<String>,
) -> Json<Value> {
    {
        let mut sessions = ctx.state.sessions.write().await;
        sessions.retain(|s| s.id != id);
        if sessions.is_empty() {
            let cfg = ctx.state.config.read().await.clone();
            sessions.push(Session::new(
                "New Session".to_string(),
                cfg.default_provider,
                cfg.default_model,
            ));
        }
    }
    ctx.state.save_sessions().await;
    Json(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct SendMessageRequest {
    content: String,
    provider: Option<Provider>,
    model: Option<String>,
}

async fn api_send_message(
    State(ctx): State<ServerCtx>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<Value>, StatusCode> {
    if req.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    {
        let mut sessions = ctx.state.sessions.write().await;
        let session = sessions.iter_mut().find(|s| s.id == id).ok_or(StatusCode::NOT_FOUND)?;
        if let Some(p) = req.provider {
            session.provider = p;
        }
        if let Some(m) = req.model {
            if !m.trim().is_empty() {
                session.model = m.trim().to_string();
            }
        }
    }

    let state = ctx.state.clone();
    let bridge = ctx.bridge.clone();
    let content = req.content.trim().to_string();
    tokio::spawn(async move {
        run_agent_turn(state, bridge, id, content).await;
    });

    Ok(Json(json!({ "ok": true, "started": true })))
}

async fn api_stop_session(
    State(ctx): State<ServerCtx>,
    Path(id): Path<String>,
) -> Json<Value> {
    let active = ctx.state.active_runs.lock().await;
    if let Some(tx) = active.get(&id) {
        let _ = tx.send(true);
        return Json(json!({ "ok": true, "stopped": true }));
    }
    Json(json!({ "ok": true, "stopped": false }))
}

#[derive(Deserialize)]
struct DeviceActionRequest {
    action: String,
    #[serde(default)]
    params: Value,
}

async fn api_device_action(
    State(ctx): State<ServerCtx>,
    Json(req): Json<DeviceActionRequest>,
) -> Json<Value> {
    let res = execute_device_tool(&ctx.bridge, &req.action, &req.params).await;
    let parsed: Value = serde_json::from_str(&res).unwrap_or_else(|_| json!({ "raw": res }));
    Json(parsed)
}

async fn api_ui_tree(State(ctx): State<ServerCtx>) -> Json<Value> {
    Json(ctx.bridge.get_ui_tree().await)
}

async fn api_screenshot(State(ctx): State<ServerCtx>) -> Response {
    let latest = ctx.state.latest_frame.read().await.clone();
    if let Some(frame) = latest {
        return (
            [(header::CONTENT_TYPE, "image/jpeg")],
            frame.jpeg.to_vec(),
        )
            .into_response();
    }
    let _ = ctx.bridge.request_screenshot().await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    if let Some(frame) = ctx.state.latest_frame.read().await.clone() {
        return (
            [(header::CONTENT_TYPE, "image/jpeg")],
            frame.jpeg.to_vec(),
        )
            .into_response();
    }
    (StatusCode::NOT_FOUND, "No screenshot frame available yet").into_response()
}

async fn ws_video(ws: WebSocketUpgrade, State(ctx): State<ServerCtx>) -> Response {
    ws.on_upgrade(move |socket| handle_ws_video(socket, ctx))
}

async fn handle_ws_video(mut socket: WebSocket, ctx: ServerCtx) {
    // Send latest frame immediately if available
    if let Some(frame) = ctx.state.latest_frame.read().await.clone() {
        let meta = json!({ "width": frame.width, "height": frame.height }).to_string();
        let _ = socket.send(Message::Text(meta)).await;
        let _ = socket.send(Message::Binary(frame.jpeg.to_vec())).await;
    }

    let mut rx = ctx.state.video_tx.subscribe();
    let mut last_dims = (0u32, 0u32);

    loop {
        tokio::select! {
            frame_res = rx.recv() => {
                match frame_res {
                    Ok(frame) => {
                        if (frame.width, frame.height) != last_dims {
                            last_dims = (frame.width, frame.height);
                            let meta = json!({ "width": frame.width, "height": frame.height }).to_string();
                            if socket.send(Message::Text(meta)).await.is_err() {
                                break;
                            }
                        }
                        if socket.send(Message::Binary(frame.jpeg.to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

async fn ws_audio(ws: WebSocketUpgrade, State(ctx): State<ServerCtx>) -> Response {
    ws.on_upgrade(move |socket| handle_ws_audio(socket, ctx))
}

async fn handle_ws_audio(mut socket: WebSocket, ctx: ServerCtx) {
    let mut rx = ctx.state.audio_tx.subscribe();
    let mut last_cfg = (0u32, 0u16);

    loop {
        tokio::select! {
            chunk_res = rx.recv() => {
                match chunk_res {
                    Ok(chunk) => {
                        if (chunk.sample_rate, chunk.channels) != last_cfg {
                            last_cfg = (chunk.sample_rate, chunk.channels);
                            let meta = json!({
                                "sample_rate": chunk.sample_rate,
                                "channels": chunk.channels,
                                "format": "pcm_s16le"
                            }).to_string();
                            if socket.send(Message::Text(meta)).await.is_err() {
                                break;
                            }
                        }
                        if socket.send(Message::Binary(chunk.pcm.to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

async fn ws_events(ws: WebSocketUpgrade, State(ctx): State<ServerCtx>) -> Response {
    ws.on_upgrade(move |socket| handle_ws_events(socket, ctx))
}

async fn handle_ws_events(socket: WebSocket, ctx: ServerCtx) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = ctx.state.event_tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(evt) = rx.recv().await {
            if let Ok(txt) = serde_json::to_string(&evt) {
                if sender.send(Message::Text(txt)).await.is_err() {
                    break;
                }
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if matches!(msg, Message::Close(_)) {
            break;
        }
    }

    send_task.abort();
}
