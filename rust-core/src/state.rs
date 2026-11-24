use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Gemini,
    Grok,
    OpenAI,
}

impl Provider {
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Claude => "claude-sonnet-4-20250514",
            Provider::Gemini => "gemini-2.5-flash",
            Provider::Grok => "grok-3",
            Provider::OpenAI => "gpt-4o",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::Claude => "Claude (Anthropic)",
            Provider::Gemini => "Gemini (Google)",
            Provider::Grok => "Grok (xAI)",
            Provider::OpenAI => "OpenAI",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeys {
    #[serde(default)]
    pub claude: String,
    #[serde(default)]
    pub gemini: String,
    #[serde(default)]
    pub grok: String,
    #[serde(default)]
    pub openai: String,
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            claude: String::new(),
            gemini: String::new(),
            grok: String::new(),
            openai: String::new(),
        }
    }
}

impl ApiKeys {
    pub fn get(&self, provider: &Provider) -> &str {
        match provider {
            Provider::Claude => &self.claude,
            Provider::Gemini => &self.gemini,
            Provider::Grok => &self.grok,
            Provider::OpenAI => &self.openai,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password_salt: String,
    #[serde(default)]
    pub password_hash: String,
    #[serde(default)]
    pub mfa_enabled: bool,
    #[serde(default)]
    pub totp_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub allowed_chat_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub api_keys: ApiKeys,
    #[serde(default = "default_provider")]
    pub default_provider: Provider,
    #[serde(default = "default_model_str")]
    pub default_model: String,
    #[serde(default = "default_max_steps")]
    pub max_agent_steps: usize,
    #[serde(default = "default_step_delay_ms")]
    pub step_delay_ms: u64,
    #[serde(default = "default_include_screenshot")]
    pub include_screenshot: bool,
    #[serde(default = "default_true")]
    pub gemini_google_search: bool,
    #[serde(default = "default_true")]
    pub gemini_google_maps: bool,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
}

fn default_provider() -> Provider {
    Provider::Gemini
}
fn default_model_str() -> String {
    "gemini-2.5-flash".to_string()
}
fn default_max_steps() -> usize {
    15
}
fn default_step_delay_ms() -> u64 {
    600
}
fn default_include_screenshot() -> bool {
    true
}
fn default_true() -> bool {
    true
}
fn default_system_prompt() -> String {
    "You are Agentic Android, an autonomous AI assistant controlling an Android device via Accessibility Services. \
You can see the device's current screen (screenshot and UI accessibility hierarchy) and perform human-like interactions: \
tap, double_tap, long_press, swipe, type_text, click_node, global_action (back, home, recents, notifications, quick_settings, power_dialog, lock_screen, take_screenshot), \
launch_app, wait, get_ui_tree, capture_screenshot, and crop_image. \
Always verify coordinates from the UI tree or screenshot before tapping. \
When the user asks for a screenshot, picture, QR code, receipt, chart, or visual excerpt from the device, call `capture_screenshot` (optionally passing `crop: {x, y, width, height}` in device screen pixel coordinates, or refine an existing image with `crop_image`), and include the returned Markdown image syntax `![caption](/api/media/...)` directly in your final response so the user sees the image inline. \
Be concise and report clearly when the task is complete."
        .to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_keys: ApiKeys::default(),
            default_provider: default_provider(),
            default_model: default_model_str(),
            max_agent_steps: default_max_steps(),
            step_delay_ms: default_step_delay_ms(),
            include_screenshot: default_include_screenshot(),
            gemini_google_search: true,
            gemini_google_maps: true,
            system_prompt: default_system_prompt(),
            auth: AuthConfig::default(),
            telegram: TelegramConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTrace {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    #[serde(default)]
    pub tool_traces: Vec<ToolTrace>,
    pub timestamp_ms: u64,
}

fn default_session_source() -> String {
    "web".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub provider: Provider,
    pub model: String,
    #[serde(default = "default_session_source")]
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_chat_id: Option<i64>,
    pub messages: Vec<ChatMessage>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl Session {
    pub fn new(title: String, provider: Provider, model: String) -> Self {
        let now = now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            provider,
            model,
            source: "web".to_string(),
            telegram_chat_id: None,
            messages: Vec::new(),
            created_at_ms: now,
            updated_at_ms: now,
        }
    }

    pub fn new_telegram(chat_id: i64, provider: Provider, model: String) -> Self {
        let now = now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            title: format!("Telegram ({chat_id})"),
            provider,
            model,
            source: "telegram".to_string(),
            telegram_chat_id: Some(chat_id),
            messages: Vec::new(),
            created_at_ms: now,
            updated_at_ms: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub jpeg: Bytes,
    pub width: u32,
    pub height: u32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub pcm: Bytes,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    SessionUpdated {
        session_id: String,
    },
    AgentStatus {
        session_id: String,
        running: bool,
        step: usize,
        status: String,
    },
    ToolCall {
        session_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        result: String,
    },
    AssistantMessage {
        session_id: String,
        message: ChatMessage,
    },
    Error {
        session_id: String,
        message: String,
    },
}

pub struct AppState {
    pub config_dir: PathBuf,
    pub port: u16,
    pub config: RwLock<AppConfig>,
    pub sessions: RwLock<Vec<Session>>,
    pub latest_frame: RwLock<Option<VideoFrame>>,
    pub video_tx: broadcast::Sender<VideoFrame>,
    pub audio_tx: broadcast::Sender<AudioChunk>,
    pub event_tx: broadcast::Sender<AgentEvent>,
    pub active_runs: Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,
    pub auth_tokens: RwLock<HashMap<String, u64>>, // token -> expires_at_ms
    pub pending_totp_secret: RwLock<Option<String>>,
    pub telegram_status: RwLock<String>,
    pub telegram_sessions: Mutex<HashMap<i64, String>>, // chat_id -> session_id
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(config_dir: PathBuf, port: u16) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&config_dir);
        let config = load_json::<AppConfig>(&config_dir.join("config.json")).unwrap_or_default();
        let mut sessions =
            load_json::<Vec<Session>>(&config_dir.join("sessions.json")).unwrap_or_default();

        if sessions.is_empty() {
            sessions.push(Session::new(
                "New Session".to_string(),
                config.default_provider.clone(),
                config.default_model.clone(),
            ));
        }

        let (video_tx, _) = broadcast::channel(8);
        let (audio_tx, _) = broadcast::channel(32);
        let (event_tx, _) = broadcast::channel(64);

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .expect("Failed to build reqwest client");

        let mut auth_tokens =
            load_json::<HashMap<String, u64>>(&config_dir.join("auth_tokens.json"))
                .unwrap_or_default();
        let now = now_ms();
        auth_tokens.retain(|_, &mut exp| exp > now);

        let mut telegram_sessions = HashMap::new();
        for s in &sessions {
            if let Some(chat_id) = s.telegram_chat_id {
                telegram_sessions.entry(chat_id).or_insert_with(|| s.id.clone());
            }
        }

        Arc::new(Self {
            config_dir,
            port,
            config: RwLock::new(config),
            sessions: RwLock::new(sessions),
            latest_frame: RwLock::new(None),
            video_tx,
            audio_tx,
            event_tx,
            active_runs: Mutex::new(HashMap::new()),
            auth_tokens: RwLock::new(auth_tokens),
            pending_totp_secret: RwLock::new(None),
            telegram_status: RwLock::new("Disabled".to_string()),
            telegram_sessions: Mutex::new(telegram_sessions),
            http_client,
        })
    }

    pub async fn save_config(&self) {
        let cfg = self.config.read().await.clone();
        let path = self.config_dir.join("config.json");
        let _ = save_json(&path, &cfg);
    }

    pub async fn save_sessions(&self) {
        let sessions = self.sessions.read().await.clone();
        let path = self.config_dir.join("sessions.json");
        let _ = save_json(&path, &sessions);
    }

    pub async fn save_auth_tokens(&self) {
        let now = now_ms();
        let tokens = {
            let mut map = self.auth_tokens.write().await;
            map.retain(|_, &mut exp| exp > now);
            map.clone()
        };
        let path = self.config_dir.join("auth_tokens.json");
        let _ = save_json(&path, &tokens);
    }

    pub async fn push_video_frame(&self, jpeg: Vec<u8>, width: u32, height: u32) {
        let frame = VideoFrame {
            jpeg: Bytes::from(jpeg),
            width,
            height,
            timestamp_ms: now_ms(),
        };
        *self.latest_frame.write().await = Some(frame.clone());
        let _ = self.video_tx.send(frame);
    }

    pub fn push_audio_pcm(&self, pcm: Vec<u8>, sample_rate: u32, channels: u16) {
        let chunk = AudioChunk {
            pcm: Bytes::from(pcm),
            sample_rate,
            channels,
        };
        let _ = self.audio_tx.send(chunk);
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn load_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_json<T: Serialize>(path: &PathBuf, value: &T) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(value)?;
    std::fs::write(path, data)
}
