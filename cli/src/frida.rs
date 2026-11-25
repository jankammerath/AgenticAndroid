//! Frida instrumentation support for the managed Android emulator.
//!
//! When enabled, the supervisor:
//!   1. switches adbd to root (`adb root`, falling back to `su`),
//!   2. downloads (or uses a configured) `frida-server` matching the host `frida` CLI version
//!      and the emulator ABI, pushes it to `/data/local/tmp/frida-server` and starts it,
//!   3. launches one host-side `frida` CLI session per enabled script (spawn or attach mode),
//!      logging output to `<state_dir>/frida/logs/<id>.log`.
//!
//! Settings (enabled flag + script list incl. sources) are persisted in
//! `<state_dir>/frida/settings.json` and can be edited from the Web UI (Settings -> Emulator).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{adb_bin, ensure_port_forwarded, EffectiveConfig};

const DEVICE_SERVER_PATH: &str = "/data/local/tmp/frida-server";
const MAX_SCRIPTS: usize = 32;
const MAX_SCRIPT_BYTES: usize = 256 * 1024;
const ATTACH_RETRY_SECS: u64 = 30;
const SERVER_CHECK_SECS: u64 = 60;
const SERVER_RETRY_SECS: u64 = 120;

fn default_mode() -> String {
    "attach".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FridaScript {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// App package / process identifier (e.g. `com.example.app`)
    #[serde(default)]
    pub target: String,
    /// `spawn` (launch app with script, `frida -f`) or `attach` (attach to running app, `frida -N`)
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FridaSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub scripts: Vec<FridaScript>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RootMode {
    Adbd,
    Su,
}

struct Session {
    script_id: String,
    child: Child,
}

pub struct FridaManager {
    base_dir: PathBuf,
    settings_path: PathBuf,
    frida_bin: String,
    server_override: Option<PathBuf>,
    settings: Mutex<FridaSettings>,
    sessions: Mutex<Vec<Session>>,
    last_exit: Mutex<HashMap<String, String>>,
    last_attempt: Mutex<HashMap<String, Instant>>,
    last_server_check: Mutex<Option<Instant>>,
    root_mode: Mutex<Option<RootMode>>,
    host_version: Mutex<Option<Option<String>>>,
    status: Mutex<String>,
    server_running: AtomicBool,
    busy: AtomicBool,
    op_lock: Mutex<()>,
}

impl FridaManager {
    pub fn new(cfg: &EffectiveConfig) -> Self {
        let base_dir = cfg.state_dir.join("frida");
        let settings_path = base_dir.join("settings.json");
        let settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<FridaSettings>(&raw).ok())
            .unwrap_or_else(|| FridaSettings {
                enabled: cfg.frida_enabled,
                scripts: Vec::new(),
            });
        Self {
            base_dir,
            settings_path,
            frida_bin: cfg
                .frida_bin
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(detect_frida_bin),
            server_override: cfg.frida_server.clone(),
            settings: Mutex::new(settings),
            sessions: Mutex::new(Vec::new()),
            last_exit: Mutex::new(HashMap::new()),
            last_attempt: Mutex::new(HashMap::new()),
            last_server_check: Mutex::new(None),
            root_mode: Mutex::new(None),
            host_version: Mutex::new(None),
            status: Mutex::new(String::new()),
            server_running: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            op_lock: Mutex::new(()),
        }
    }

    pub fn settings(&self) -> FridaSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.settings.lock().unwrap().enabled
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }

    /// Mark the manager busy; returns false if another Frida operation is already running.
    pub fn try_begin(&self) -> bool {
        !self.busy.swap(true, Ordering::SeqCst)
    }

    pub fn end(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }

    fn set_status(&self, msg: impl Into<String>) {
        let msg = msg.into();
        if !msg.is_empty() {
            println!("    [frida] {msg}");
        }
        *self.status.lock().unwrap() = msg;
    }

    /// Validate, normalise, and persist new settings. Returns the stored settings.
    pub fn save_settings(&self, mut new: FridaSettings) -> Result<FridaSettings, String> {
        if new.scripts.len() > MAX_SCRIPTS {
            return Err(format!("At most {MAX_SCRIPTS} Frida scripts are supported."));
        }
        let mut seen = std::collections::HashSet::new();
        for (idx, s) in new.scripts.iter_mut().enumerate() {
            s.name = s.name.trim().chars().take(80).collect();
            s.target = s.target.trim().to_string();
            s.mode = match s.mode.trim() {
                "spawn" => "spawn".to_string(),
                _ => "attach".to_string(),
            };
            if s.name.is_empty() {
                s.name = format!("Script {}", idx + 1);
            }
            if s.target.is_empty()
                || s.target.len() > 200
                || !s
                    .target
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':'))
            {
                return Err(format!(
                    "Script '{}': target must be an app package / process name (e.g. com.example.app).",
                    s.name
                ));
            }
            if s.source.trim().is_empty() {
                return Err(format!("Script '{}': source must not be empty.", s.name));
            }
            if s.source.len() > MAX_SCRIPT_BYTES {
                return Err(format!(
                    "Script '{}' exceeds {} KB.",
                    s.name,
                    MAX_SCRIPT_BYTES / 1024
                ));
            }
            let valid_id = !s.id.is_empty()
                && s.id.len() <= 64
                && s.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if !valid_id || seen.contains(&s.id) {
                s.id = generate_id(idx);
            }
            seen.insert(s.id.clone());
        }

        fs::create_dir_all(&self.base_dir)
            .map_err(|e| format!("Failed to create {}: {e}", self.base_dir.display()))?;
        let json = serde_json::to_string_pretty(&new)
            .map_err(|e| format!("Failed to serialise Frida settings: {e}"))?;
        let tmp = self.settings_path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| format!("Failed to write Frida settings: {e}"))?;
        set_private(&tmp);
        fs::rename(&tmp, &self.settings_path)
            .map_err(|e| format!("Failed to save Frida settings: {e}"))?;

        // Drop logs / scripts of removed entries
        let ids: std::collections::HashSet<String> =
            new.scripts.iter().map(|s| s.id.clone()).collect();
        for sub in ["scripts", "logs"] {
            if let Ok(entries) = fs::read_dir(self.base_dir.join(sub)) {
                for e in entries.flatten() {
                    let stem = e
                        .path()
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    if !ids.contains(&stem) {
                        let _ = fs::remove_file(e.path());
                    }
                }
            }
        }
        self.last_exit.lock().unwrap().retain(|k, _| ids.contains(k));

        *self.settings.lock().unwrap() = new.clone();
        Ok(new)
    }

    fn host_version(&self) -> Option<String> {
        let mut cached = self.host_version.lock().unwrap();
        if let Some(Some(ref v)) = *cached {
            return Some(v.clone());
        }
        // Not cached when missing, so installing frida-tools later is picked up without restart
        let v = Command::new(&self.frida_bin)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty());
        *cached = Some(v.clone());
        v
    }

    /// Bring Frida in line with the current settings: start server + sessions when enabled,
    /// tear everything down when disabled. Safe to call repeatedly.
    pub fn apply(&self, cfg: &EffectiveConfig) {
        let _guard = self.op_lock.lock().unwrap();
        self.stop_sessions_locked();
        if !self.is_enabled() {
            if self.server_running.swap(false, Ordering::SeqCst) {
                self.kill_server(cfg);
            }
            self.set_status("");
            return;
        }
        match self.prepare_device(cfg) {
            Ok(()) => {
                self.start_sessions_locked(cfg);
            }
            Err(e) => {
                eprintln!("WARNING: Frida setup failed: {e}");
                self.set_status(format!("Error: {e}"));
            }
        }
    }

    /// Restart all script sessions without touching frida-server.
    pub fn reload(&self, cfg: &EffectiveConfig) {
        let _guard = self.op_lock.lock().unwrap();
        self.stop_sessions_locked();
        if !self.is_enabled() {
            return;
        }
        if !self.server_running.load(Ordering::SeqCst) {
            if let Err(e) = self.prepare_device(cfg) {
                self.set_status(format!("Error: {e}"));
                return;
            }
        }
        self.start_sessions_locked(cfg);
    }

    /// Called before the emulator is stopped/restarted.
    pub fn on_emulator_stopping(&self) {
        let _guard = self.op_lock.lock().unwrap();
        self.stop_sessions_locked();
        self.server_running.store(false, Ordering::SeqCst);
        *self.root_mode.lock().unwrap() = None;
    }

    pub fn shutdown(&self) {
        let _guard = self.op_lock.lock().unwrap();
        self.stop_sessions_locked();
    }

    /// Periodic housekeeping from the supervisor loop: reap exited sessions, re-attach
    /// attach-mode scripts, and restart frida-server if it died.
    pub fn maintain(&self, cfg: &EffectiveConfig) {
        if !self.is_enabled() || self.is_busy() {
            return;
        }
        let Ok(_guard) = self.op_lock.try_lock() else {
            return;
        };
        self.reap_sessions();

        let check_server = {
            let mut last = self.last_server_check.lock().unwrap();
            let due = last.is_none_or(|t| t.elapsed() >= Duration::from_secs(SERVER_CHECK_SECS));
            if due {
                *last = Some(Instant::now());
            }
            due
        };
        if check_server || !self.server_running.load(Ordering::SeqCst) {
            let alive = self.server_running.load(Ordering::SeqCst)
                && self
                    .root_mode()
                    .map(|m| server_pid_alive(&adb_bin(&cfg.android_home), m))
                    .unwrap_or(false);
            if !alive {
                // Back off between server restart attempts (e.g. no root / frida missing)
                let mut attempts = self.last_attempt.lock().unwrap();
                let recent = attempts
                    .get("__server__")
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(SERVER_RETRY_SECS));
                if recent {
                    return;
                }
                attempts.insert("__server__".into(), Instant::now());
                drop(attempts);
                eprintln!("    [frida] frida-server not running; restarting...");
                self.stop_sessions_locked();
                match self.prepare_device(cfg) {
                    Ok(()) => self.start_sessions_locked(cfg),
                    Err(e) => self.set_status(format!("Error: {e}")),
                }
                return;
            }
        }

        // Re-attach attach-mode scripts whose session ended (e.g. app restarted)
        let running: Vec<String> = self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.script_id.clone())
            .collect();
        let scripts = self.settings().scripts;
        for script in scripts
            .iter()
            .filter(|s| s.enabled && s.mode == "attach" && !running.contains(&s.id))
        {
            let due = self
                .last_attempt
                .lock()
                .unwrap()
                .get(&script.id)
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(ATTACH_RETRY_SECS));
            if due && target_running(&adb_bin(&cfg.android_home), &script.target) {
                self.spawn_session(script);
            }
        }
    }

    pub fn status_json(&self, include_sources: bool) -> serde_json::Value {
        let settings = self.settings();
        let running: Vec<String> = {
            let mut sessions = self.sessions.lock().unwrap();
            let mut exits = self.last_exit.lock().unwrap();
            sessions.retain_mut(|s| match s.child.try_wait() {
                Ok(Some(st)) => {
                    exits.insert(s.script_id.clone(), describe_exit(st));
                    false
                }
                _ => true,
            });
            sessions.iter().map(|s| s.script_id.clone()).collect()
        };
        let exits = self.last_exit.lock().unwrap().clone();
        let scripts: Vec<serde_json::Value> = settings
            .scripts
            .iter()
            .map(|s| {
                let mut v = serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "target": s.target,
                    "mode": s.mode,
                    "enabled": s.enabled,
                    "running": running.contains(&s.id),
                    "last_exit": exits.get(&s.id),
                });
                if include_sources {
                    v["source"] = serde_json::Value::String(s.source.clone());
                }
                v
            })
            .collect();
        serde_json::json!({
            "enabled": settings.enabled,
            "busy": self.is_busy(),
            "server_running": self.server_running.load(Ordering::SeqCst),
            "status": self.status.lock().unwrap().clone(),
            "frida_bin": self.frida_bin,
            "host_version": self.host_version(),
            "scripts": scripts,
        })
    }

    pub fn log_tail(&self, id: &str, max_lines: usize) -> Option<String> {
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') || id.is_empty() {
            return None;
        }
        let raw = fs::read_to_string(self.base_dir.join("logs").join(format!("{id}.log"))).ok()?;
        let lines: Vec<&str> = raw.lines().collect();
        let start = lines.len().saturating_sub(max_lines);
        Some(lines[start..].join("\n"))
    }

    // ---- internals (call with op_lock held) ----

    fn root_mode(&self) -> Option<RootMode> {
        *self.root_mode.lock().unwrap()
    }

    fn prepare_device(&self, cfg: &EffectiveConfig) -> Result<(), String> {
        let adb = adb_bin(&cfg.android_home);

        let version = self.host_version().ok_or_else(|| {
            format!(
                "Frida CLI '{}' not found on the host. Install frida-tools (pip install frida-tools) or set frida_bin in config.toml.",
                self.frida_bin
            )
        })?;

        self.set_status("Enabling root access on emulator...");
        let mode = ensure_root(&adb)?;
        *self.root_mode.lock().unwrap() = Some(mode);
        // `adb root` restarts adbd, which drops existing port forwards
        let _ = ensure_port_forwarded(&cfg.android_home, cfg.port);

        let arch = device_frida_arch(&adb)?;
        let local_server = self.ensure_server_binary(&version, arch)?;

        let local_size = fs::metadata(&local_server).map(|m| m.len()).unwrap_or(0);
        let remote_size = shell_out(&adb, &format!("stat -c %s {DEVICE_SERVER_PATH} 2>/dev/null"))
            .parse::<u64>()
            .unwrap_or(0);
        if local_size == 0 || local_size != remote_size {
            self.set_status(format!("Pushing frida-server {version} ({arch}) to emulator..."));
            let _ = root_shell(&adb, mode, "pkill -f frida-server");
            let out = Command::new(&adb)
                .arg("push")
                .arg(&local_server)
                .arg(DEVICE_SERVER_PATH)
                .output()
                .map_err(|e| format!("adb push failed: {e}"))?;
            if !out.status.success() {
                return Err(format!(
                    "adb push frida-server failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ));
            }
        }
        let _ = root_shell(&adb, mode, &format!("chmod 755 {DEVICE_SERVER_PATH}"));

        if !server_pid_alive(&adb, mode) {
            self.set_status("Starting frida-server...");
            let cmd = format!("{DEVICE_SERVER_PATH} -D >/dev/null 2>&1 </dev/null &");
            // Daemonised start can keep the adb shell pipe open on some images; guard with a timeout
            let mut child = root_shell_command(&adb, mode, &cmd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to start frida-server: {e}"))?;
            for _ in 0..25 {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            }
            let _ = child.kill();
            let _ = child.wait();

            let mut alive = false;
            for _ in 0..10 {
                if server_pid_alive(&adb, mode) {
                    alive = true;
                    break;
                }
                thread::sleep(Duration::from_millis(500));
            }
            if !alive {
                return Err("frida-server did not start on the emulator (see `adb logcat`).".into());
            }
        }
        self.server_running.store(true, Ordering::SeqCst);
        *self.last_server_check.lock().unwrap() = Some(Instant::now());
        self.set_status(format!("frida-server {version} running"));
        Ok(())
    }

    fn ensure_server_binary(&self, version: &str, arch: &str) -> Result<PathBuf, String> {
        if let Some(ref p) = self.server_override {
            if p.is_file() {
                return Ok(p.clone());
            }
            return Err(format!("Configured frida_server not found: {}", p.display()));
        }
        let dir = self.base_dir.join("server");
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
        let name = format!("frida-server-{version}-android-{arch}");
        let bin = dir.join(&name);
        if bin.is_file() && fs::metadata(&bin).map(|m| m.len() > 0).unwrap_or(false) {
            return Ok(bin);
        }
        let xz_path = dir.join(format!("{name}.xz"));
        let url = format!("https://github.com/frida/frida/releases/download/{version}/{name}.xz");
        self.set_status(format!("Downloading {name}..."));
        let dl = Command::new("curl")
            .args(["-fsSL", "--max-time", "600", "-o"])
            .arg(&xz_path)
            .arg(&url)
            .output()
            .map_err(|e| format!("Failed to run curl: {e}"))?;
        if !dl.status.success() {
            let _ = fs::remove_file(&xz_path);
            return Err(format!(
                "Failed to download {url}: {}",
                String::from_utf8_lossy(&dl.stderr).trim()
            ));
        }
        let unxz = Command::new("xz")
            .args(["-d", "-f"])
            .arg(&xz_path)
            .output()
            .map_err(|e| format!("Failed to run xz (install xz-utils): {e}"))?;
        if !unxz.status.success() || !bin.is_file() {
            let _ = fs::remove_file(&xz_path);
            let _ = fs::remove_file(&bin);
            return Err(format!(
                "Failed to decompress {name}.xz: {}",
                String::from_utf8_lossy(&unxz.stderr).trim()
            ));
        }
        Ok(bin)
    }

    fn kill_server(&self, cfg: &EffectiveConfig) {
        let adb = adb_bin(&cfg.android_home);
        let mode = self.root_mode().unwrap_or(RootMode::Adbd);
        let _ = root_shell(&adb, mode, "pkill -f frida-server");
        println!("    [frida] frida-server stopped");
    }

    fn start_sessions_locked(&self, cfg: &EffectiveConfig) {
        let scripts = self.settings().scripts;
        let adb = adb_bin(&cfg.android_home);
        let mut started = 0;
        let mut total = 0;
        for script in scripts.iter().filter(|s| s.enabled) {
            total += 1;
            // Attach-mode scripts need a running target; maintain() attaches once it appears
            if script.mode == "attach" && !target_running(&adb, &script.target) {
                self.last_exit
                    .lock()
                    .unwrap()
                    .insert(script.id.clone(), "waiting for target process".into());
                continue;
            }
            if self.spawn_session(script) {
                started += 1;
            }
        }
        let version = self.host_version().unwrap_or_default();
        self.set_status(format!(
            "frida-server {version} running · {started}/{total} script(s) active"
        ));
    }

    fn spawn_session(&self, script: &FridaScript) -> bool {
        self.last_attempt
            .lock()
            .unwrap()
            .insert(script.id.clone(), Instant::now());
        let scripts_dir = self.base_dir.join("scripts");
        let logs_dir = self.base_dir.join("logs");
        let _ = fs::create_dir_all(&scripts_dir);
        let _ = fs::create_dir_all(&logs_dir);
        let script_path = scripts_dir.join(format!("{}.js", script.id));
        if let Err(e) = fs::write(&script_path, &script.source) {
            self.last_exit
                .lock()
                .unwrap()
                .insert(script.id.clone(), format!("failed to write script: {e}"));
            return false;
        }
        let log_path = logs_dir.join(format!("{}.log", script.id));
        let Ok(mut log) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
        else {
            return false;
        };
        let _ = writeln!(
            log,
            "=== [{}] {} '{}' -> {} ===",
            unix_now(),
            script.mode,
            script.name,
            script.target
        );
        let Ok(err_log) = log.try_clone() else {
            return false;
        };

        let mut cmd = Command::new(&self.frida_bin);
        cmd.arg("-U");
        if script.mode == "spawn" {
            cmd.args(["-f", &script.target]);
        } else {
            cmd.args(["-N", &script.target]);
        }
        cmd.arg("-l")
            .arg(&script_path)
            .args(["-q", "-t", "inf"])
            .env("PYTHONUNBUFFERED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(err_log));

        match cmd.spawn() {
            Ok(child) => {
                println!(
                    "    [frida] Loaded '{}' ({} {})",
                    script.name, script.mode, script.target
                );
                self.last_exit.lock().unwrap().remove(&script.id);
                self.sessions.lock().unwrap().push(Session {
                    script_id: script.id.clone(),
                    child,
                });
                true
            }
            Err(e) => {
                self.last_exit
                    .lock()
                    .unwrap()
                    .insert(script.id.clone(), format!("failed to launch frida: {e}"));
                false
            }
        }
    }

    fn reap_sessions(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        let mut exits = self.last_exit.lock().unwrap();
        sessions.retain_mut(|s| match s.child.try_wait() {
            Ok(Some(st)) => {
                exits.insert(s.script_id.clone(), describe_exit(st));
                false
            }
            _ => true,
        });
    }

    fn stop_sessions_locked(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        for s in sessions.iter_mut() {
            // SIGTERM first so frida can detach cleanly, then force-kill
            let _ = Command::new("kill")
                .args(["-TERM", &s.child.id().to_string()])
                .status();
        }
        for s in sessions.iter_mut() {
            let mut exited = false;
            for _ in 0..10 {
                if let Ok(Some(_)) = s.child.try_wait() {
                    exited = true;
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            if !exited {
                let _ = s.child.kill();
                let _ = s.child.wait();
            }
        }
        sessions.clear();
    }
}

fn detect_frida_bin() -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(ref h) = home {
        candidates.push(h.join(".local/bin/frida"));
    }
    candidates.push(PathBuf::from("/usr/local/bin/frida"));
    candidates.push(PathBuf::from("/opt/homebrew/bin/frida"));
    // Prefer anything on PATH
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':').filter(|d| !d.is_empty()) {
            let p = Path::new(dir).join("frida");
            if p.is_file() {
                return p.display().to_string();
            }
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "frida".to_string())
}

fn generate_id(idx: usize) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("s{:x}{idx}", nanos & 0xffff_ffff_ffff)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn describe_exit(st: std::process::ExitStatus) -> String {
    match st.code() {
        Some(c) => format!("exited (code {c})"),
        None => "terminated".to_string(),
    }
}

fn set_private(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

fn shell_out(adb: &Path, cmd: &str) -> String {
    Command::new(adb)
        .args(["shell", cmd])
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn root_shell_command(adb: &Path, mode: RootMode, cmd: &str) -> Command {
    let mut c = Command::new(adb);
    match mode {
        RootMode::Adbd => {
            c.args(["shell", cmd]);
        }
        RootMode::Su => {
            c.args(["shell", &format!("su 0 sh -c '{cmd}'")]);
        }
    }
    c
}

fn root_shell(adb: &Path, mode: RootMode, cmd: &str) -> std::io::Result<Output> {
    root_shell_command(adb, mode, cmd).stdin(Stdio::null()).output()
}

fn ensure_root(adb: &Path) -> Result<RootMode, String> {
    if shell_out(adb, "id -u") == "0" {
        return Ok(RootMode::Adbd);
    }
    let _ = Command::new(adb)
        .arg("root")
        .stdin(Stdio::null())
        .output();
    // adbd restarts; poll until it is back as root
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(500));
        if shell_out(adb, "id -u") == "0" {
            return Ok(RootMode::Adbd);
        }
    }
    if shell_out(adb, "su 0 id -u") == "0" {
        return Ok(RootMode::Su);
    }
    Err("Root access unavailable on the emulator. Frida requires a rootable system image (google_apis / default, not google_apis_playstore).".into())
}

fn device_frida_arch(adb: &Path) -> Result<&'static str, String> {
    let abi = shell_out(adb, "getprop ro.product.cpu.abi");
    match abi.as_str() {
        "x86_64" => Ok("x86_64"),
        "x86" => Ok("x86"),
        "arm64-v8a" => Ok("arm64"),
        a if a.starts_with("armeabi") => Ok("arm"),
        other => Err(format!("Unsupported emulator ABI for frida-server: '{other}'")),
    }
}

fn server_pid_alive(adb: &Path, mode: RootMode) -> bool {
    root_shell(adb, mode, "pidof frida-server")
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

fn target_running(adb: &Path, target: &str) -> bool {
    !shell_out(adb, &format!("pidof {target}")).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg(dir: &Path) -> EffectiveConfig {
        EffectiveConfig {
            avd: None,
            apk: None,
            port: 8080,
            abi: "x86_64".into(),
            headless: true,
            gpu: None,
            build_only: false,
            supervise: true,
            config_path: None,
            project_root: dir.to_path_buf(),
            state_dir: dir.to_path_buf(),
            android_home: dir.to_path_buf(),
            java_home: None,
            ndk_home: None,
            latitude: None,
            longitude: None,
            altitude: None,
            frida_enabled: true,
            frida_server: None,
            frida_bin: Some("/nonexistent/frida".into()),
        }
    }

    fn script(name: &str, target: &str) -> FridaScript {
        FridaScript {
            id: String::new(),
            name: name.into(),
            target: target.into(),
            mode: "spawn".into(),
            enabled: true,
            source: "console.log(1);".into(),
        }
    }

    #[test]
    fn settings_validate_persist_and_reload() {
        let dir = std::env::temp_dir().join(format!("frida-test-{}", unix_now() ^ std::process::id() as u64));
        let cfg = test_cfg(&dir);
        let mgr = FridaManager::new(&cfg);
        assert!(mgr.is_enabled(), "config default should apply without settings.json");

        let bad = FridaSettings { enabled: true, scripts: vec![script("x", "com.app; rm -rf /")] };
        assert!(mgr.save_settings(bad).is_err());
        let empty_src = FridaSettings {
            enabled: true,
            scripts: vec![FridaScript { source: "  ".into(), ..script("x", "com.app") }],
        };
        assert!(mgr.save_settings(empty_src).is_err());

        let mut weird_mode = script("", "com.example.two");
        weird_mode.mode = "bogus".into();
        let saved = mgr
            .save_settings(FridaSettings {
                enabled: false,
                scripts: vec![script("One", "com.example.one"), weird_mode],
            })
            .unwrap();
        assert_eq!(saved.scripts.len(), 2);
        assert!(saved.scripts.iter().all(|s| !s.id.is_empty()));
        assert_ne!(saved.scripts[0].id, saved.scripts[1].id);
        assert_eq!(saved.scripts[1].mode, "attach");
        assert_eq!(saved.scripts[1].name, "Script 2");

        // Reload from disk: UI setting (disabled) overrides config default (enabled)
        let mgr2 = FridaManager::new(&cfg);
        assert!(!mgr2.is_enabled());
        assert_eq!(mgr2.settings().scripts[0].target, "com.example.one");
        let status = mgr2.status_json(false);
        assert!(status["scripts"][0].get("source").is_none());
        assert_eq!(mgr2.status_json(true)["scripts"][0]["source"], "console.log(1);");
        assert!(mgr2.log_tail("../etc/passwd", 10).is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
