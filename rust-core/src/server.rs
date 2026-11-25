use crate::agent::{execute_device_tool, run_agent_turn};
use crate::auth;
use crate::bridge::DeviceBridge;
use crate::llm::fetch_provider_models;
use crate::state::{now_ms, AppPlaybook, AppState, Provider, Session};
use crate::telegram::spawn_telegram_worker;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
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
const I18N_JS: &str = include_str!("../web/i18n.js");
const I18N_EN_JSON: &str = include_str!("../web/i18n/en.json");
const I18N_DE_JSON: &str = include_str!("../web/i18n/de.json");
const I18N_ZH_CN_JSON: &str = include_str!("../web/i18n/zh-CN.json");
const APP_JS: &str = include_str!("../web/app.js");
const MANIFEST_JSON: &str = include_str!("../web/manifest.json");
const SW_JS: &str = include_str!("../web/sw.js");
const LOGO_PNG: &[u8] = include_bytes!("../web/logo.png");
const FAVICON_PNG: &[u8] = include_bytes!("../web/favicon.png");
const ICON_192_PNG: &[u8] = include_bytes!("../web/icon-192.png");
const ICON_512_PNG: &[u8] = include_bytes!("../web/icon-512.png");
const APPLE_TOUCH_ICON_PNG: &[u8] = include_bytes!("../web/apple-touch-icon.png");

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
    let tg_handle = spawn_telegram_worker(state.clone(), bridge.clone());
    let ctx = ServerCtx { state, bridge };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let protected_routes = Router::new()
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
        .route("/api/media/:filename", get(api_get_media))
        .route("/api/app-memory", get(api_list_app_memory))
        .route(
            "/api/app-memory/:package",
            axum::routing::put(api_upsert_app_memory).delete(api_delete_app_memory),
        )
        .route("/api/auth/credentials", post(api_auth_set_credentials))
        .route("/api/auth/disable", post(api_auth_disable))
        .route("/api/auth/mfa/setup", post(api_auth_mfa_setup))
        .route("/api/auth/mfa/enable", post(api_auth_mfa_enable))
        .route("/api/auth/mfa/disable", post(api_auth_mfa_disable))
        .route("/ws/video", get(ws_video))
        .route("/ws/audio", get(ws_audio))
        .route("/ws/events", get(ws_events))
        .layer(middleware::from_fn_with_state(ctx.clone(), require_auth));

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/i18n.js", get(serve_i18n_js))
        .route("/i18n/:file", get(serve_i18n_json))
        .route("/app.js", get(serve_js))
        .route("/manifest.json", get(serve_manifest))
        .route("/sw.js", get(serve_sw))
        .route("/logo.png", get(serve_logo))
        .route("/favicon.png", get(serve_favicon))
        .route("/icon-192.png", get(serve_icon_192))
        .route("/icon-512.png", get(serve_icon_512))
        .route("/apple-touch-icon.png", get(serve_apple_touch_icon))
        .route("/api/health", get(api_health))
        .route("/api/auth/status", get(api_auth_status))
        .route("/api/auth/login", post(api_auth_login))
        .route("/api/auth/logout", post(api_auth_logout))
        .route(
            "/api/internal/backup",
            get(api_internal_backup).post(api_internal_restore),
        )
        .merge(protected_routes)
        .layer(cors)
        .with_state(ctx);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind port {port}: {e}"))?;

    log::info!("Agentic Android Rust webserver listening on http://{addr}");

    let res = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = (&mut shutdown_rx).await;
            log::info!("Shutting down Agentic Android webserver");
        })
        .await
        .map_err(|e| format!("Server error: {e}"));
    tg_handle.abort();
    res
}

async fn serve_index(State(ctx): State<ServerCtx>) -> Html<String> {
    let status = ctx.bridge.get_status().await;
    let html = if let Some(name) = status["device_name"].as_str().filter(|s| !s.trim().is_empty()) {
        let escaped = name
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        INDEX_HTML.replace(
            "<title>Agentic Android</title>",
            &format!("<title>{escaped} (Agentic Android)</title>"),
        )
    } else {
        INDEX_HTML.to_string()
    };
    Html(html)
}

async fn serve_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
        .into_response()
}

async fn serve_i18n_js() -> Response {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        I18N_JS,
    )
        .into_response()
}

async fn serve_i18n_json(Path(file): Path<String>) -> Response {
    let body = match file.as_str() {
        "en.json" => I18N_EN_JSON,
        "de.json" => I18N_DE_JSON,
        "zh-CN.json" => I18N_ZH_CN_JSON,
        _ => return (StatusCode::NOT_FOUND, "Not found").into_response(),
    };
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
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

async fn serve_logo() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], LOGO_PNG).into_response()
}

async fn serve_favicon() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], FAVICON_PNG).into_response()
}

async fn serve_manifest() -> Response {
    (
        [(header::CONTENT_TYPE, "application/manifest+json; charset=utf-8")],
        MANIFEST_JSON,
    )
        .into_response()
}

async fn serve_sw() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        SW_JS,
    )
        .into_response()
}

async fn serve_icon_192() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], ICON_192_PNG).into_response()
}

async fn serve_icon_512() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], ICON_512_PNG).into_response()
}

async fn serve_apple_touch_icon() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], APPLE_TOUCH_ICON_PNG).into_response()
}

const SESSION_COOKIE_NAME: &str = "agentic_session";
const SESSION_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix("agentic_session=") {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

async fn is_request_authenticated(state: &Arc<AppState>, headers: &HeaderMap) -> bool {
    let auth_enabled = state.config.read().await.auth.enabled;
    if !auth_enabled {
        return true;
    }
    let Some(token) = extract_session_cookie(headers) else {
        return false;
    };
    let now = now_ms();
    let tokens = state.auth_tokens.read().await;
    tokens
        .get(&token)
        .map(|&exp| exp > now)
        .unwrap_or(false)
}

async fn require_auth(
    State(ctx): State<ServerCtx>,
    req: Request,
    next: Next,
) -> Response {
    if is_request_authenticated(&ctx.state, req.headers()).await {
        return next.run(req).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "ok": false, "error": "Authentication required" })),
    )
        .into_response()
}

fn make_session_cookie(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800"
    ))
    .unwrap()
}

fn make_clear_cookie() -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE_NAME}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
    ))
    .unwrap()
}

async fn api_auth_status(
    State(ctx): State<ServerCtx>,
    headers: HeaderMap,
) -> Json<Value> {
    let auth_cfg = ctx.state.config.read().await.auth.clone();
    let authenticated = is_request_authenticated(&ctx.state, &headers).await;
    Json(json!({
        "ok": true,
        "auth_enabled": auth_cfg.enabled,
        "mfa_enabled": auth_cfg.enabled && auth_cfg.mfa_enabled,
        "authenticated": authenticated,
        "username": if authenticated && auth_cfg.enabled { auth_cfg.username } else { String::new() }
    }))
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
    #[serde(default)]
    totp_code: String,
}

async fn api_auth_login(
    State(ctx): State<ServerCtx>,
    Json(req): Json<LoginRequest>,
) -> Response {
    let auth_cfg = ctx.state.config.read().await.auth.clone();
    if !auth_cfg.enabled {
        return Json(json!({ "ok": true, "authenticated": true })).into_response();
    }

    if req.username.trim() != auth_cfg.username
        || !auth::verify_password(&req.password, &auth_cfg.password_salt, &auth_cfg.password_hash)
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "error": "Invalid username or password." })),
        )
            .into_response();
    }

    if auth_cfg.mfa_enabled {
        if req.totp_code.trim().is_empty() {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "ok": false,
                    "mfa_required": true,
                    "error": "Enter the 6-digit code from your authenticator app."
                })),
            )
                .into_response();
        }
        if !auth::verify_totp(&auth_cfg.totp_secret, &req.totp_code) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "ok": false,
                    "mfa_required": true,
                    "error": "Invalid 2FA verification code."
                })),
            )
                .into_response();
        }
    }

    let token = auth::generate_session_token();
    let expires_at = now_ms().saturating_add(SESSION_TTL_MS);
    {
        let mut tokens = ctx.state.auth_tokens.write().await;
        tokens.retain(|_, &mut exp| exp > now_ms());
        tokens.insert(token.clone(), expires_at);
    }
    ctx.state.save_auth_tokens().await;

    let mut resp = Json(json!({ "ok": true, "authenticated": true })).into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, make_session_cookie(&token));
    resp
}

async fn api_auth_logout(
    State(ctx): State<ServerCtx>,
    headers: HeaderMap,
) -> Response {
    if let Some(token) = extract_session_cookie(&headers) {
        ctx.state.auth_tokens.write().await.remove(&token);
        ctx.state.save_auth_tokens().await;
    }
    let mut resp = Json(json!({ "ok": true })).into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, make_clear_cookie());
    resp
}

#[derive(Deserialize)]
struct SetCredentialsRequest {
    username: String,
    password: String,
}

async fn api_auth_set_credentials(
    State(ctx): State<ServerCtx>,
    Json(req): Json<SetCredentialsRequest>,
) -> Response {
    let username = req.username.trim().to_string();
    if username.is_empty() || req.password.len() < 4 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "Username is required and password must be at least 4 characters." })),
        )
            .into_response();
    }

    let (salt, hash) = auth::hash_password(&req.password);
    {
        let mut cfg = ctx.state.config.write().await;
        cfg.auth.enabled = true;
        cfg.auth.username = username;
        cfg.auth.password_salt = salt;
        cfg.auth.password_hash = hash;
    }
    ctx.state.save_config().await;

    let token = auth::generate_session_token();
    let expires_at = now_ms().saturating_add(SESSION_TTL_MS);
    {
        let mut tokens = ctx.state.auth_tokens.write().await;
        tokens.clear();
        tokens.insert(token.clone(), expires_at);
    }
    ctx.state.save_auth_tokens().await;

    let mut resp = Json(json!({ "ok": true, "auth_enabled": true })).into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, make_session_cookie(&token));
    resp
}

async fn api_auth_disable(State(ctx): State<ServerCtx>) -> Response {
    {
        let mut cfg = ctx.state.config.write().await;
        cfg.auth.enabled = false;
        cfg.auth.mfa_enabled = false;
        cfg.auth.totp_secret.clear();
        cfg.auth.password_salt.clear();
        cfg.auth.password_hash.clear();
    }
    ctx.state.auth_tokens.write().await.clear();
    ctx.state.save_auth_tokens().await;
    *ctx.state.pending_totp_secret.write().await = None;
    ctx.state.save_config().await;

    let mut resp = Json(json!({ "ok": true, "auth_enabled": false })).into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, make_clear_cookie());
    resp
}

async fn api_auth_mfa_setup(State(ctx): State<ServerCtx>) -> Response {
    let auth_cfg = ctx.state.config.read().await.auth.clone();
    if !auth_cfg.enabled {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "Enable username & password login before setting up 2FA." })),
        )
            .into_response();
    }

    let secret = auth::generate_totp_secret();
    let uri = auth::build_otpauth_uri(&auth_cfg.username, &secret);
    let qr_svg = match auth::render_qr_svg(&uri) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": e })),
            )
                .into_response();
        }
    };

    *ctx.state.pending_totp_secret.write().await = Some(secret.clone());

    Json(json!({
        "ok": true,
        "secret": secret,
        "otpauth_uri": uri,
        "qr_svg": qr_svg
    }))
    .into_response()
}

#[derive(Deserialize)]
struct MfaVerifyRequest {
    code: String,
}

async fn api_auth_mfa_enable(
    State(ctx): State<ServerCtx>,
    Json(req): Json<MfaVerifyRequest>,
) -> Response {
    let pending = ctx.state.pending_totp_secret.read().await.clone();
    let Some(secret) = pending else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "No pending 2FA setup. Click 'Set Up 2FA' first." })),
        )
            .into_response();
    };

    if !auth::verify_totp(&secret, &req.code) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "Invalid 6-digit code. Check your authenticator app and try again." })),
        )
            .into_response();
    }

    {
        let mut cfg = ctx.state.config.write().await;
        cfg.auth.mfa_enabled = true;
        cfg.auth.totp_secret = secret;
    }
    *ctx.state.pending_totp_secret.write().await = None;
    ctx.state.save_config().await;

    Json(json!({ "ok": true, "mfa_enabled": true })).into_response()
}

async fn api_auth_mfa_disable(State(ctx): State<ServerCtx>) -> Json<Value> {
    {
        let mut cfg = ctx.state.config.write().await;
        cfg.auth.mfa_enabled = false;
        cfg.auth.totp_secret.clear();
    }
    *ctx.state.pending_totp_secret.write().await = None;
    ctx.state.save_config().await;
    Json(json!({ "ok": true, "mfa_enabled": false }))
}

fn verify_backup_secret(state: &Arc<AppState>, headers: &HeaderMap) -> bool {
    let Some(ref expected) = state.backup_secret else {
        return false;
    };
    let Some(provided) = headers.get("x-backup-secret").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    !expected.is_empty() && provided == expected
}

async fn api_internal_backup(
    State(ctx): State<ServerCtx>,
    headers: HeaderMap,
) -> Response {
    if !verify_backup_secret(&ctx.state, &headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let config = ctx.state.config.read().await.clone();
    let sessions = ctx.state.sessions.read().await.clone();
    let auth_tokens = ctx.state.auth_tokens.read().await.clone();
    let app_playbooks = ctx.state.app_playbooks.read().await.clone();
    Json(json!({
        "config": config,
        "sessions": sessions,
        "auth_tokens": auth_tokens,
        "app_playbooks": app_playbooks
    }))
    .into_response()
}

#[derive(Deserialize)]
struct RestoreBackupPayload {
    config: Option<crate::state::AppConfig>,
    sessions: Option<Vec<Session>>,
    auth_tokens: Option<std::collections::HashMap<String, u64>>,
    app_playbooks: Option<std::collections::HashMap<String, AppPlaybook>>,
}

async fn api_internal_restore(
    State(ctx): State<ServerCtx>,
    headers: HeaderMap,
    Json(payload): Json<RestoreBackupPayload>,
) -> Response {
    if !verify_backup_secret(&ctx.state, &headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    // Only restore if current state appears freshly initialized (no auth & no API keys)
    {
        let cur = ctx.state.config.read().await;
        let already_configured = cur.auth.enabled
            || !cur.api_keys.claude.is_empty()
            || !cur.api_keys.gemini.is_empty()
            || !cur.api_keys.grok.is_empty()
            || !cur.api_keys.openai.is_empty();
        if already_configured {
            return Json(json!({ "ok": true, "restored": false, "reason": "already_configured" }))
                .into_response();
        }
    }

    if let Some(cfg) = payload.config {
        *ctx.state.config.write().await = cfg;
        ctx.state.save_config().await;
    }
    if let Some(sessions) = payload.sessions {
        if !sessions.is_empty() {
            *ctx.state.sessions.write().await = sessions;
            ctx.state.save_sessions().await;
        }
    }
    if let Some(tokens) = payload.auth_tokens {
        let now = now_ms();
        let valid: std::collections::HashMap<String, u64> =
            tokens.into_iter().filter(|(_, exp)| *exp > now).collect();
        *ctx.state.auth_tokens.write().await = valid;
        ctx.state.save_auth_tokens().await;
    }
    if let Some(playbooks) = payload.app_playbooks {
        *ctx.state.app_playbooks.write().await = playbooks;
        ctx.state.save_app_playbooks().await;
    }

    log::info!("Restored Agentic Android state from host backup");
    Json(json!({ "ok": true, "restored": true })).into_response()
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
    gemini_google_search: bool,
    gemini_google_maps: bool,
    notify_on_complete: bool,
    system_prompt: String,
    telegram: Value,
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
    let tg_status = ctx.state.telegram_status.read().await.clone();
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
        gemini_google_search: cfg.gemini_google_search,
        gemini_google_maps: cfg.gemini_google_maps,
        notify_on_complete: cfg.notify_on_complete,
        system_prompt: cfg.system_prompt,
        telegram: json!({
            "enabled": cfg.telegram.enabled,
            "token_configured": !cfg.telegram.bot_token.trim().is_empty(),
            "token_masked": mask_key(&cfg.telegram.bot_token),
            "allowed_chat_ids": cfg.telegram.allowed_chat_ids,
            "status": tg_status,
        }),
    })
}

#[derive(Deserialize)]
struct TelegramConfigPatch {
    enabled: Option<bool>,
    bot_token: Option<String>,
    allowed_chat_ids: Option<Vec<i64>>,
}

#[derive(Deserialize)]
struct UpdateConfigRequest {
    api_keys: Option<ApiKeysPatch>,
    default_provider: Option<Provider>,
    default_model: Option<String>,
    max_agent_steps: Option<usize>,
    step_delay_ms: Option<u64>,
    include_screenshot: Option<bool>,
    gemini_google_search: Option<bool>,
    gemini_google_maps: Option<bool>,
    notify_on_complete: Option<bool>,
    system_prompt: Option<String>,
    telegram: Option<TelegramConfigPatch>,
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
            cfg.max_agent_steps = steps.clamp(1, 1024);
        }
        if let Some(delay) = req.step_delay_ms {
            cfg.step_delay_ms = delay.min(5000);
        }
        if let Some(inc) = req.include_screenshot {
            cfg.include_screenshot = inc;
        }
        if let Some(gs) = req.gemini_google_search {
            cfg.gemini_google_search = gs;
        }
        if let Some(gm) = req.gemini_google_maps {
            cfg.gemini_google_maps = gm;
        }
        if let Some(notify) = req.notify_on_complete {
            cfg.notify_on_complete = notify;
        }
        if let Some(prompt) = req.system_prompt {
            cfg.system_prompt = prompt;
        }
        if let Some(tg) = req.telegram {
            if let Some(en) = tg.enabled {
                cfg.telegram.enabled = en;
            }
            if let Some(tok) = tg.bot_token {
                cfg.telegram.bot_token = tok.trim().to_string();
            }
            if let Some(ids) = tg.allowed_chat_ids {
                cfg.telegram.allowed_chat_ids = ids;
            }
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
    let provider = req.provider.unwrap_or_else(|| cfg.default_provider.clone());
    let model = req
        .model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| {
            if provider == cfg.default_provider && !cfg.default_model.trim().is_empty() {
                cfg.default_model.clone()
            } else {
                provider.default_model().to_string()
            }
        });
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

async fn api_list_app_memory(State(ctx): State<ServerCtx>) -> Json<Vec<AppPlaybook>> {
    let mut items: Vec<AppPlaybook> = ctx
        .state
        .app_playbooks
        .read()
        .await
        .values()
        .cloned()
        .collect();
    items.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms));
    Json(items)
}

#[derive(Deserialize)]
struct UpsertAppMemoryRequest {
    #[serde(default)]
    app_label: String,
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    app_version: String,
    #[serde(default)]
    notes: Vec<String>,
}

async fn api_upsert_app_memory(
    State(ctx): State<ServerCtx>,
    Path(package): Path<String>,
    Json(req): Json<UpsertAppMemoryRequest>,
) -> Result<Json<AppPlaybook>, StatusCode> {
    let pkg = package.trim().to_string();
    if pkg.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let clean_purpose = req.purpose.trim().to_string();
    let clean_notes: Vec<String> = req
        .notes
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(25)
        .collect();

    let saved = {
        let mut map = ctx.state.app_playbooks.write().await;
        if clean_notes.is_empty() && clean_purpose.is_empty() {
            map.remove(&pkg);
            AppPlaybook {
                package_name: pkg.clone(),
                app_label: req.app_label.trim().to_string(),
                purpose: String::new(),
                app_version: req.app_version.trim().to_string(),
                notes: Vec::new(),
                updated_at_ms: now_ms(),
            }
        } else {
            let entry = AppPlaybook {
                package_name: pkg.clone(),
                app_label: req.app_label.trim().to_string(),
                purpose: clean_purpose,
                app_version: req.app_version.trim().to_string(),
                notes: clean_notes,
                updated_at_ms: now_ms(),
            };
            map.insert(pkg, entry.clone());
            entry
        }
    };
    ctx.state.save_app_playbooks().await;
    Ok(Json(saved))
}

async fn api_delete_app_memory(
    State(ctx): State<ServerCtx>,
    Path(package): Path<String>,
) -> Json<Value> {
    ctx.state.app_playbooks.write().await.remove(package.trim());
    ctx.state.save_app_playbooks().await;
    Json(json!({ "ok": true }))
}

async fn api_get_media(
    State(ctx): State<ServerCtx>,
    Path(filename): Path<String>,
) -> Response {
    let valid = !filename.is_empty()
        && filename
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !filename.contains("..");
    if !valid {
        return (StatusCode::BAD_REQUEST, "Invalid media filename").into_response();
    }

    let path = ctx.state.config_dir.join("media").join(&filename);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let content_type = if filename.ends_with(".png") {
                "image/png"
            } else {
                "image/jpeg"
            };
            (
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CACHE_CONTROL, "public, max-age=31536000"),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Media file not found").into_response(),
    }
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
