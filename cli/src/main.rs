use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const APP_PACKAGE: &str = "org.agenticandroid";
const ACCESSIBILITY_SERVICE: &str = "org.agenticandroid/org.agenticandroid.AgentAccessibilityService";
const MAIN_ACTIVITY: &str = "org.agenticandroid/.MainActivity";

#[derive(Parser, Debug)]
#[command(
    name = "agentic-android",
    version,
    about = "Build, launch, and supervise Agentic Android on an Android emulator (GUI or headless daemon)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    run_opts: RunOpts,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build (or use --apk), launch emulator, install app, enable Accessibility, and forward port
    Run(RunOpts),

    /// Terminate any running Android emulator (and stop daemon if active)
    #[command(alias = "kill")]
    Stop {
        /// Optional specific emulator serial (e.g. emulator-5554); defaults to all running emulators
        #[arg(short, long)]
        serial: Option<String>,
        /// Directory for PID and log files
        #[arg(long)]
        state_dir: Option<PathBuf>,
    },

    /// Manage the background supervisor daemon (ideal for headless Linux servers)
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// List available Android Virtual Devices (AVDs)
    Avds,

    /// Set the running emulator's GPS location (latitude / longitude)
    Geo {
        /// Latitude in decimal degrees (e.g. 50.7374)
        #[arg(long, allow_hyphen_values = true)]
        lat: f64,
        /// Longitude in decimal degrees (e.g. 7.0982)
        #[arg(long, allow_hyphen_values = true)]
        lon: f64,
        /// Optional altitude in meters
        #[arg(long, allow_hyphen_values = true)]
        alt: Option<f64>,
    },
}

#[derive(Subcommand, Debug)]
enum DaemonAction {
    /// Start the emulator and Agentic Android supervisor in the background
    Start(RunOpts),
    /// Stop the running background daemon and its managed emulator
    Stop {
        /// Directory for PID and log files
        #[arg(long)]
        state_dir: Option<PathBuf>,
    },
    /// Show daemon, emulator, and webserver health status
    Status {
        /// Webserver port to probe
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
        /// Directory for PID and log files
        #[arg(long)]
        state_dir: Option<PathBuf>,
    },
    /// Print a ready-to-use Linux systemd service unit file
    Systemd(RunOpts),
}

#[derive(Args, Debug, Clone, Default)]
struct RunOpts {
    /// AVD name to launch (defaults to first phone AVD, or creates one if none exist)
    #[arg(short, long, env = "AVD_NAME")]
    avd: Option<String>,

    /// Path to a prebuilt APK to install instead of building from source
    #[arg(long, env = "APK_PATH")]
    apk: Option<PathBuf>,

    /// Webserver & adb forward port
    #[arg(short, long, env = "PORT")]
    port: Option<u16>,

    /// Target Android ABI when building from source (arm64-v8a or x86_64)
    #[arg(long, env = "ABI")]
    abi: Option<String>,

    /// Run the Android emulator headless (-no-window -no-audio -gpu swiftshader_indirect -feature -Vulkan)
    #[arg(long, env = "HEADLESS")]
    headless: bool,

    /// Override Android emulator GPU mode (e.g. swiftshader_indirect, guest, host, auto)
    #[arg(long, env = "EMU_GPU")]
    gpu: Option<String>,

    /// Build from source only without launching an emulator
    #[arg(long)]
    build_only: bool,

    /// Detach into background supervisor daemon mode
    #[arg(short, long)]
    daemon: bool,

    /// Keep running in foreground as a supervisor (used internally by daemon & systemd)
    #[arg(long)]
    supervise: bool,

    /// Path to TOML config file (default: ~/.agentic-android/config.toml if present)
    #[arg(short, long, env = "AGENTIC_CONFIG")]
    config: Option<PathBuf>,

    /// Path to AgenticAndroid repo root (for source builds)
    #[arg(long, env = "AGENTIC_ROOT")]
    project_root: Option<PathBuf>,

    /// Directory for PID and log files (default: ~/.agentic-android)
    #[arg(long)]
    state_dir: Option<PathBuf>,

    /// Optional GPS latitude in decimal degrees to set on the emulator (e.g. 50.7374)
    #[arg(long, env = "EMU_LATITUDE", allow_hyphen_values = true)]
    lat: Option<f64>,

    /// Optional GPS longitude in decimal degrees to set on the emulator (e.g. 7.0982)
    #[arg(long, env = "EMU_LONGITUDE", allow_hyphen_values = true)]
    lon: Option<f64>,

    /// Optional GPS altitude in meters
    #[arg(long, env = "EMU_ALTITUDE", allow_hyphen_values = true)]
    alt: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    avd: Option<String>,
    apk: Option<PathBuf>,
    port: Option<u16>,
    abi: Option<String>,
    headless: Option<bool>,
    gpu: Option<String>,
    project_root: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    android_home: Option<PathBuf>,
    java_home: Option<PathBuf>,
    android_ndk_home: Option<PathBuf>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f64>,
}

#[derive(Debug, Clone)]
struct EffectiveConfig {
    avd: Option<String>,
    apk: Option<PathBuf>,
    port: u16,
    abi: String,
    headless: bool,
    gpu: Option<String>,
    build_only: bool,
    supervise: bool,
    project_root: PathBuf,
    state_dir: PathBuf,
    android_home: PathBuf,
    java_home: Option<PathBuf>,
    ndk_home: Option<PathBuf>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f64>,
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        Some(Commands::Avds) => cmd_list_avds(),
        Some(Commands::Geo { lat, lon, alt }) => cmd_set_geo(lat, lon, alt),
        Some(Commands::Stop { serial, state_dir }) => cmd_stop_emulator(serial, state_dir),
        Some(Commands::Daemon { action }) => match action {
            DaemonAction::Start(opts) => cmd_daemon_start(opts),
            DaemonAction::Stop { state_dir } => cmd_daemon_stop(state_dir),
            DaemonAction::Status { port, state_dir } => cmd_daemon_status(port, state_dir),
            DaemonAction::Systemd(opts) => cmd_print_systemd(opts),
        },
        Some(Commands::Run(opts)) => {
            if opts.daemon {
                cmd_daemon_start(opts)
            } else {
                cmd_run(opts)
            }
        }
        None => {
            if cli.run_opts.daemon {
                cmd_daemon_start(cli.run_opts)
            } else {
                cmd_run(cli.run_opts)
            }
        }
    };

    if let Err(e) = res {
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    }
}

fn default_state_dir() -> PathBuf {
    home_dir().join(".agentic-android")
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn detect_android_home(override_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = override_dir {
        if p.exists() {
            return Ok(p);
        }
    }
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(val) = env::var(var) {
            let p = PathBuf::from(val);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    let home = home_dir();
    for candidate in [
        home.join("Library/Android/sdk"),
        home.join("Android/Sdk"),
        PathBuf::from("/usr/local/lib/android/sdk"),
        PathBuf::from("/opt/android-sdk"),
    ] {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("ANDROID_HOME not found. Set ANDROID_HOME or configure android_home in config.toml.".into())
}

fn detect_java_home(override_dir: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = override_dir {
        if p.join("bin/java").exists() {
            return Some(p);
        }
    }
    if let Ok(val) = env::var("JAVA_HOME") {
        let p = PathBuf::from(val);
        if p.join("bin/java").exists() {
            return Some(p);
        }
    }
    let studio_jbr = PathBuf::from("/Applications/Android Studio.app/Contents/jbr/Contents/Home");
    if studio_jbr.join("bin/java").exists() {
        return Some(studio_jbr);
    }
    if cfg!(target_os = "macos") {
        if let Ok(out) = Command::new("/usr/libexec/java_home").args(["-v", "17"]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(PathBuf::from(s));
                }
            }
        }
    }
    None
}

fn detect_ndk_home(android_home: &Path, override_dir: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = override_dir {
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(val) = env::var("ANDROID_NDK_HOME") {
        let p = PathBuf::from(val);
        if p.exists() {
            return Some(p);
        }
    }
    let ndk_root = android_home.join("ndk");
    let mut versions: Vec<PathBuf> = fs::read_dir(ndk_root)
        .ok()?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| p.is_dir())
        .collect();
    versions.sort();
    versions.pop()
}

fn detect_project_root(override_dir: Option<PathBuf>) -> PathBuf {
    if let Some(p) = override_dir {
        return p;
    }
    if let Ok(cwd) = env::current_dir() {
        if cwd.join("rust-core/Cargo.toml").exists() {
            return cwd;
        }
        if let Some(parent) = cwd.parent() {
            if parent.join("rust-core/Cargo.toml").exists() {
                return parent.to_path_buf();
            }
        }
    }
    if let Ok(exe) = env::current_exe() {
        for ancestor in exe.ancestors() {
            if ancestor.join("rust-core/Cargo.toml").exists() {
                return ancestor.to_path_buf();
            }
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn load_config(opts: &RunOpts) -> Result<EffectiveConfig, String> {
    let default_cfg_path = default_state_dir().join("config.toml");
    let cfg_path = opts
        .config
        .clone()
        .or_else(|| default_cfg_path.exists().then_some(default_cfg_path));

    let file_cfg: FileConfig = if let Some(path) = cfg_path {
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config {}: {e}", path.display()))?;
        toml::from_str(&raw)
            .map_err(|e| format!("Failed to parse TOML config {}: {e}", path.display()))?
    } else {
        FileConfig::default()
    };

    let android_home = detect_android_home(file_cfg.android_home)?;
    let java_home = detect_java_home(file_cfg.java_home);
    let ndk_home = detect_ndk_home(&android_home, file_cfg.android_ndk_home);

    let default_abi = if cfg!(target_arch = "aarch64") {
        "arm64-v8a"
    } else {
        "x86_64"
    };

    Ok(EffectiveConfig {
        avd: opts.avd.clone().or(file_cfg.avd),
        apk: opts.apk.clone().or(file_cfg.apk),
        port: opts.port.or(file_cfg.port).unwrap_or(8080),
        abi: opts
            .abi
            .clone()
            .or(file_cfg.abi)
            .unwrap_or_else(|| default_abi.to_string()),
        headless: opts.headless || file_cfg.headless.unwrap_or(false),
        gpu: opts.gpu.clone().or(file_cfg.gpu),
        build_only: opts.build_only,
        supervise: opts.supervise,
        project_root: detect_project_root(opts.project_root.clone().or(file_cfg.project_root)),
        state_dir: opts
            .state_dir
            .clone()
            .or(file_cfg.state_dir)
            .unwrap_or_else(default_state_dir),
        android_home,
        java_home,
        ndk_home,
        latitude: opts.lat.or(file_cfg.latitude),
        longitude: opts.lon.or(file_cfg.longitude),
        altitude: opts.alt.or(file_cfg.altitude),
    })
}

fn adb_bin(android_home: &Path) -> PathBuf {
    android_home.join("platform-tools/adb")
}

fn emulator_bin(android_home: &Path) -> PathBuf {
    android_home.join("emulator/emulator")
}

fn avdmanager_bin(android_home: &Path) -> PathBuf {
    let latest = android_home.join("cmdline-tools/latest/bin/avdmanager");
    if latest.exists() {
        return latest;
    }
    android_home.join("cmdline-tools/bin/avdmanager")
}

fn find_running_emulators(android_home: &Path) -> Vec<String> {
    let Ok(out) = Command::new(adb_bin(android_home))
        .arg("devices")
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?;
            let state = parts.next()?;
            if serial.starts_with("emulator-") && (state == "device" || state == "offline") {
                Some(serial.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn cmd_stop_emulator(serial: Option<String>, state_dir: Option<PathBuf>) -> Result<(), String> {
    // Stop background daemon first if running so it does not restart the emulator
    let dir = state_dir.clone().unwrap_or_else(default_state_dir);
    if dir.join("daemon.pid").exists() {
        let _ = cmd_daemon_stop(state_dir);
    }

    let android_home = detect_android_home(None)?;
    let adb = adb_bin(&android_home);
    let targets = match serial {
        Some(s) => vec![s],
        None => find_running_emulators(&android_home),
    };

    if targets.is_empty() {
        println!("No running Android emulators found.");
        return Ok(());
    }

    for emu in &targets {
        println!("Stopping emulator '{emu}'...");
        let _ = Command::new(&adb).args(["-s", emu, "emu", "kill"]).status();
    }

    // Wait briefly for emulators to exit
    for _ in 0..15 {
        let remaining = find_running_emulators(&android_home);
        if targets.iter().all(|t| !remaining.contains(t)) {
            println!("Emulator(s) terminated.");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }

    println!("Sent shutdown signal to emulator(s): {}", targets.join(", "));
    Ok(())
}

fn cmd_set_geo(lat: f64, lon: f64, alt: Option<f64>) -> Result<(), String> {
    let android_home = detect_android_home(None)?;
    set_emulator_location(&android_home, lat, lon, alt)?;
    println!("Set emulator GPS location to lat={lat}, lon={lon}");
    Ok(())
}

fn set_emulator_location(
    android_home: &Path,
    lat: f64,
    lon: f64,
    alt: Option<f64>,
) -> Result<(), String> {
    // Note: `adb emu geo fix` takes <longitude> <latitude> [altitude]
    let mut args = vec![
        "emu".to_string(),
        "geo".to_string(),
        "fix".to_string(),
        format!("{lon}"),
        format!("{lat}"),
    ];
    if let Some(a) = alt {
        args.push(format!("{a}"));
    }
    let status = Command::new(adb_bin(android_home))
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to run adb emu geo fix: {e}"))?;
    if !status.success() {
        return Err("adb emu geo fix failed (is an emulator running?).".into());
    }
    Ok(())
}

fn apply_configured_location(cfg: &EffectiveConfig) {
    if let (Some(lat), Some(lon)) = (cfg.latitude, cfg.longitude) {
        match set_emulator_location(&cfg.android_home, lat, lon, cfg.altitude) {
            Ok(()) => println!("    Set emulator GPS location: lat={lat}, lon={lon}"),
            Err(e) => eprintln!("WARNING: Could not set emulator GPS location: {e}"),
        }
    }
}

fn cmd_list_avds() -> Result<(), String> {
    let android_home = detect_android_home(None)?;
    let avds = list_avds(&android_home)?;
    if avds.is_empty() {
        println!("No AVDs found.");
    } else {
        for avd in avds {
            println!("{avd}");
        }
    }
    Ok(())
}

fn list_avds(android_home: &Path) -> Result<Vec<String>, String> {
    let mut cmd = Command::new(emulator_bin(android_home));
    cmd.arg("-list-avds");
    if env::var_os("ANDROID_AVD_HOME").is_none() {
        let server_avd_home = PathBuf::from("/var/lib/agentic-android/.android/avd");
        if server_avd_home.exists() {
            cmd.env("ANDROID_AVD_HOME", server_avd_home);
        }
    }
    let out = cmd
        .output()
        .map_err(|e| format!("Failed to run emulator -list-avds: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

fn build_from_source(cfg: &EffectiveConfig) -> Result<PathBuf, String> {
    let ndk_home = cfg
        .ndk_home
        .as_ref()
        .ok_or("Android NDK not found in ANDROID_HOME/ndk. Install NDK or pass --apk <path>.")?;

    let (rust_target, clang_prefix, linker_env, cc_env, ar_env) = match cfg.abi.as_str() {
        "arm64-v8a" => (
            "aarch64-linux-android",
            "aarch64-linux-android29",
            "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
            "CC_aarch64_linux_android",
            "AR_aarch64_linux_android",
        ),
        "x86_64" => (
            "x86_64-linux-android",
            "x86_64-linux-android29",
            "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
            "CC_x86_64_linux_android",
            "AR_x86_64_linux_android",
        ),
        other => {
            return Err(format!(
                "Unsupported ABI '{other}'. Use 'arm64-v8a' or 'x86_64'."
            ));
        }
    };

    let host_tag = if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "linux-x86_64"
    };
    let toolchain_bin = ndk_home.join(format!("toolchains/llvm/prebuilt/{host_tag}/bin"));
    let clang_path = toolchain_bin.join(format!("{clang_prefix}-clang"));
    let ar_path = toolchain_bin.join("llvm-ar");

    println!(
        "==> [1/5] Building Rust NDK core ({rust_target} -> {})...",
        cfg.abi
    );

    let cargo_bin_dir = home_dir().join(".cargo/bin");
    let path_env = if cargo_bin_dir.exists() {
        format!(
            "{}:{}",
            cargo_bin_dir.display(),
            env::var("PATH").unwrap_or_default()
        )
    } else {
        env::var("PATH").unwrap_or_default()
    };

    let status = Command::new("cargo")
        .current_dir(cfg.project_root.join("rust-core"))
        .args(["build", "--release", "--target", rust_target])
        .env("PATH", &path_env)
        .env(linker_env, &clang_path)
        .env(cc_env, &clang_path)
        .env(ar_env, &ar_path)
        .status()
        .map_err(|e| format!("Failed to invoke cargo: {e}"))?;

    if !status.success() {
        return Err("Rust NDK core build failed.".into());
    }

    let built_so = cfg
        .project_root
        .join(format!("rust-core/target/{rust_target}/release/libagentic_android.so"));
    let jnilibs_dir = cfg
        .project_root
        .join(format!("app/src/main/jniLibs/{}", cfg.abi));
    fs::create_dir_all(&jnilibs_dir).map_err(|e| format!("Failed to create jniLibs dir: {e}"))?;
    fs::copy(&built_so, jnilibs_dir.join("libagentic_android.so"))
        .map_err(|e| format!("Failed to copy libagentic_android.so: {e}"))?;

    println!("==> [2/5] Building Android APK...");
    let local_props = cfg.project_root.join("local.properties");
    if !local_props.exists() {
        let _ = fs::write(&local_props, format!("sdk.dir={}\n", cfg.android_home.display()));
    }

    let gradlew = cfg.project_root.join("gradlew");
    let mut cmd = if gradlew.exists() {
        Command::new(&gradlew)
    } else {
        Command::new("gradle")
    };
    cmd.current_dir(&cfg.project_root)
        .arg("assembleDebug")
        .env("ANDROID_HOME", &cfg.android_home)
        .env("ANDROID_SDK_ROOT", &cfg.android_home);
    if let Some(ref jh) = cfg.java_home {
        cmd.env("JAVA_HOME", jh);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to invoke Gradle: {e}"))?;
    if !status.success() {
        return Err("Android APK build failed.".into());
    }

    Ok(cfg
        .project_root
        .join("app/build/outputs/apk/debug/app-debug.apk"))
}

fn find_running_device(android_home: &Path) -> Option<String> {
    let out = Command::new(adb_bin(android_home))
        .arg("devices")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines().skip(1) {
        let mut parts = line.split_whitespace();
        if let (Some(serial), Some(state)) = (parts.next(), parts.next()) {
            if state == "device" {
                return Some(serial.to_string());
            }
        }
    }
    None
}

fn select_or_create_avd(cfg: &EffectiveConfig) -> Result<String, String> {
    let avds = list_avds(&cfg.android_home)?;

    if let Some(ref requested) = cfg.avd {
        if avds.iter().any(|a| a == requested) {
            return Ok(requested.clone());
        }
        println!("    Configured AVD '{requested}' not found. Creating it...");
        return create_avd(cfg, requested);
    }

    if let Some(phone) = avds
        .iter()
        .find(|a| {
            let l = a.to_lowercase();
            l.contains("pixel") || l.contains("phone")
        })
        .cloned()
    {
        return Ok(phone);
    }
    if let Some(first) = avds.into_iter().next() {
        return Ok(first);
    }

    println!("    No AVD found. Creating 'agentic_android_avd'...");
    create_avd(cfg, "agentic_android_avd")
}

fn create_avd(cfg: &EffectiveConfig, avd_name: &str) -> Result<String, String> {
    let sys_img_pkg = find_system_image_package(&cfg.android_home, &cfg.abi)?;
    println!("    Using system image: {sys_img_pkg}");

    let mut child = Command::new(avdmanager_bin(&cfg.android_home))
        .args([
            "create",
            "avd",
            "--force",
            "--name",
            avd_name,
            "--package",
            &sys_img_pkg,
            "--device",
            "pixel_6",
        ])
        .env("ANDROID_HOME", &cfg.android_home)
        .env("ANDROID_SDK_ROOT", &cfg.android_home)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to invoke avdmanager: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(b"no\n");
    }
    let status = child
        .wait()
        .map_err(|e| format!("avdmanager failed: {e}"))?;
    if !status.success() {
        return Err(format!("Failed to create AVD '{avd_name}'."));
    }
    Ok(avd_name.to_string())
}

fn find_system_image_package(android_home: &Path, abi: &str) -> Result<String, String> {
    let base = android_home.join("system-images");
    let mut found = Vec::new();
    if let Ok(apis) = fs::read_dir(&base) {
        for api in apis.flatten() {
            if let Ok(tags) = fs::read_dir(api.path()) {
                for tag in tags.flatten() {
                    let abi_dir = tag.path().join(abi);
                    if abi_dir.is_dir() {
                        if let (Some(a), Some(t)) = (
                            api.file_name().to_str(),
                            tag.file_name().to_str(),
                        ) {
                            found.push(format!("system-images;{a};{t};{abi}"));
                        }
                    }
                }
            }
        }
    }
    found.sort();
    // Prefer Google Play Store system images when available
    if let Some(play) = found
        .iter()
        .rev()
        .find(|pkg| pkg.contains(";google_apis_playstore;"))
        .cloned()
    {
        return Ok(play);
    }
    found
        .pop()
        .ok_or_else(|| format!("No Android system image found for ABI '{abi}' in {}", base.display()))
}

fn cleanup_stale_avd_locks(avd_name: &str) {
    let avd_home = env::var_os("ANDROID_AVD_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let server_home = PathBuf::from("/var/lib/agentic-android/.android/avd");
            if server_home.exists() {
                server_home
            } else {
                home_dir().join(".android/avd")
            }
        });
    let avd_dir = avd_home.join(format!("{avd_name}.avd"));
    if let Ok(entries) = fs::read_dir(&avd_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("lock") {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

fn spawn_emulator(cfg: &EffectiveConfig, avd_name: &str) -> Result<Child, String> {
    fs::create_dir_all(&cfg.state_dir).map_err(|e| format!("Failed to create state dir: {e}"))?;
    cleanup_stale_avd_locks(avd_name);
    let log_path = cfg.state_dir.join("emulator.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open {}: {e}", log_path.display()))?;
    let err_file = log_file
        .try_clone()
        .map_err(|e| format!("Failed to clone log handle: {e}"))?;

    let mut args = vec![
        "-avd".to_string(),
        avd_name.to_string(),
        "-no-snapshot-load".to_string(),
        "-no-boot-anim".to_string(),
    ];

    if cfg.headless {
        let gpu_mode = cfg
            .gpu
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("swiftshader_indirect");
        args.extend([
            "-no-window".to_string(),
            "-no-audio".to_string(),
            "-gpu".to_string(),
            gpu_mode.to_string(),
        ]);
        // Disable emulated Vulkan on software GPUs to prevent headless SwiftShader JIT SIGSEGVs
        if gpu_mode == "swiftshader_indirect" || gpu_mode == "swiftshader" || gpu_mode == "guest" {
            args.extend(["-feature".to_string(), "-Vulkan".to_string()]);
        }
    } else if let Some(ref gpu_mode) = cfg.gpu {
        if !gpu_mode.trim().is_empty() {
            args.extend(["-gpu".to_string(), gpu_mode.clone()]);
        }
    }

    println!(
        "    Starting emulator '{avd_name}' ({})...",
        if cfg.headless { "headless" } else { "GUI" }
    );

    Command::new(emulator_bin(&cfg.android_home))
        .args(&args)
        .env("ANDROID_HOME", &cfg.android_home)
        .env("ANDROID_SDK_ROOT", &cfg.android_home)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(err_file))
        .spawn()
        .map_err(|e| format!("Failed to start emulator: {e}"))
}

fn wait_for_emulator_boot(
    cfg: &EffectiveConfig,
    mut emulator_child: Option<&mut Child>,
    timeout_secs: u64,
) -> Result<(), String> {
    println!("    Waiting for emulator to boot...");
    let adb = adb_bin(&cfg.android_home);
    let _ = Command::new(&adb).arg("start-server").status();

    let attempts = (timeout_secs / 2).max(1);
    let mut restarted_adb_for_unauth = false;
    for _ in 0..attempts {
        if let Some(child) = emulator_child.as_deref_mut() {
            if let Ok(Some(exit_status)) = child.try_wait() {
                let log_path = cfg.state_dir.join("emulator.log");
                let tail = fs::read_to_string(&log_path)
                    .ok()
                    .map(|s| {
                        let lines: Vec<&str> = s.lines().collect();
                        let start = lines.len().saturating_sub(20);
                        lines[start..].join("\n")
                    })
                    .unwrap_or_default();
                return Err(format!(
                    "Emulator exited prematurely ({exit_status}). Recent emulator.log:\n{tail}"
                ));
            }
        }

        // Self-heal if an existing adb server was started with a different HOME/adbkey
        if !restarted_adb_for_unauth {
            if let Ok(devs) = Command::new(&adb).arg("devices").output() {
                if String::from_utf8_lossy(&devs.stdout).contains("unauthorized") {
                    eprintln!("    Detected unauthorized emulator (mismatched adbkey); restarting adb server...");
                    let _ = Command::new(&adb).arg("kill-server").status();
                    let _ = Command::new(&adb).arg("start-server").status();
                    restarted_adb_for_unauth = true;
                }
            }
        }

        if let Ok(out) = Command::new(&adb)
            .args(["shell", "getprop", "sys.boot_completed"])
            .output()
        {
            if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "1" {
                thread::sleep(Duration::from_secs(2));
                return Ok(());
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err("Timed out waiting for Android emulator to finish booting.".into())
}

fn ensure_backup_secret(state_dir: &Path) -> String {
    let _ = fs::create_dir_all(state_dir);
    let key_path = state_dir.join("backup.key");
    if let Ok(existing) = fs::read_to_string(&key_path) {
        let trimmed = existing.trim().to_string();
        if trimmed.len() >= 16 {
            return trimmed;
        }
    }
    let mut bytes = [0u8; 24];
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut bytes);
    } else {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(42);
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = ((nanos >> ((i % 16) * 8)) & 0xff) as u8 ^ (i as u8).wrapping_mul(37);
        }
    }
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let _ = fs::write(&key_path, format!("{hex}\n"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
    }
    hex
}

fn backup_has_user_data(json: &str) -> bool {
    if json.contains("\"enabled\":true") || json.contains("\"messages\":[{") {
        return true;
    }
    for key in ["claude", "gemini", "grok", "openai", "bot_token", "totp_secret"] {
        let needle = format!("\"{key}\":\"");
        if let Some(pos) = json.find(&needle) {
            let rest = &json[pos + needle.len()..];
            if let Some(first) = rest.chars().next() {
                if first != '"' {
                    return true;
                }
            }
        }
    }
    false
}

fn pull_app_state_backup(cfg: &EffectiveConfig) {
    let host_dir = cfg.state_dir.join("state_backup");
    if fs::create_dir_all(&host_dir).is_err() {
        return;
    }
    let secret = ensure_backup_secret(&cfg.state_dir);
    let url = format!("http://127.0.0.1:{}/api/internal/backup", cfg.port);
    let tmp_path = host_dir.join("backup.json.tmp");
    let dst_path = host_dir.join("backup.json");
    let prev_path = host_dir.join("backup.prev.json");

    if let Ok(out) = Command::new("curl")
        .args([
            "-fsS",
            "--max-time",
            "5",
            "-H",
            &format!("X-Backup-Secret: {secret}"),
            "-o",
        ])
        .arg(&tmp_path)
        .arg(&url)
        .output()
    {
        if out.status.success() {
            if let Ok(new_content) = fs::read_to_string(&tmp_path) {
                if new_content.len() > 20 {
                    // Never clobber a populated backup with an empty/freshly-wiped state
                    let existing_has_data = fs::read_to_string(&dst_path)
                        .map(|s| backup_has_user_data(&s))
                        .unwrap_or(false);
                    let new_has_data = backup_has_user_data(&new_content);
                    if existing_has_data && !new_has_data {
                        let _ = fs::remove_file(&tmp_path);
                        return;
                    }
                    if existing_has_data {
                        let _ = fs::copy(&dst_path, &prev_path);
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = fs::set_permissions(&prev_path, fs::Permissions::from_mode(0o600));
                        }
                    }
                    let _ = fs::rename(&tmp_path, &dst_path);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&dst_path, fs::Permissions::from_mode(0o600));
                    }
                    return;
                }
            }
        }
    }
    let _ = fs::remove_file(&tmp_path);
}

fn restore_app_state_backup(cfg: &EffectiveConfig) {
    let src_path = cfg.state_dir.join("state_backup/backup.json");
    if !src_path.exists() {
        return;
    }
    let secret = ensure_backup_secret(&cfg.state_dir);
    let url = format!("http://127.0.0.1:{}/api/internal/backup", cfg.port);
    for attempt in 1..=5 {
        if let Ok(out) = Command::new("curl")
            .args([
                "-fsS",
                "--max-time",
                "10",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-H",
                &format!("X-Backup-Secret: {secret}"),
                "--data-binary",
            ])
            .arg(format!("@{}", src_path.display()))
            .arg(&url)
            .output()
        {
            if out.status.success() {
                let body = String::from_utf8_lossy(&out.stdout);
                if body.contains("\"restored\":true") {
                    println!(
                        "    Restored Agentic Android state from host backup ({}).",
                        src_path.display()
                    );
                }
                return;
            } else if attempt == 5 {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!(
                    "    WARNING: Failed to restore host backup ({}): {}",
                    src_path.display(),
                    stderr.trim()
                );
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn install_and_configure(cfg: &EffectiveConfig, apk_path: &Path) -> Result<(), String> {
    let adb = adb_bin(&cfg.android_home);

    // Pull latest state backup over HTTP if the old app is currently reachable
    pull_app_state_backup(cfg);

    println!(
        "==> [4/5] Installing APK ({}) & configuring Accessibility Service...",
        apk_path.display()
    );
    let install_out = Command::new(&adb)
        .args(["install", "-r"])
        .arg(apk_path)
        .output()
        .map_err(|e| format!("Failed to run adb install: {e}"))?;

    if !install_out.status.success() {
        let stderr = String::from_utf8_lossy(&install_out.stderr);
        let stdout = String::from_utf8_lossy(&install_out.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") {
            eprintln!(
                "    Existing package signature mismatch; uninstalling old {APP_PACKAGE} and reinstalling..."
            );
            let _ = Command::new(&adb).args(["uninstall", APP_PACKAGE]).status();
            let retry = Command::new(&adb)
                .args(["install", "-r"])
                .arg(apk_path)
                .status()
                .map_err(|e| format!("Failed to retry adb install: {e}"))?;
            if !retry.success() {
                return Err("adb install failed after uninstalling incompatible package.".into());
            }
        } else {
            eprintln!("{combined}");
            return Err("adb install failed.".into());
        }
    }

    // Allow restricted settings (required on Android 13+ for sideloaded Accessibility services)
    let _ = Command::new(&adb)
        .args(["shell", "appops", "set", APP_PACKAGE, "ACCESS_RESTRICTED_SETTINGS", "allow"])
        .status();

    // Allow sideloading APKs via the Web UI without extra unknown-sources prompt
    let _ = Command::new(&adb)
        .args(["shell", "appops", "set", APP_PACKAGE, "REQUEST_INSTALL_PACKAGES", "allow"])
        .status();

    // Grant runtime permissions
    for perm in [
        "android.permission.RECORD_AUDIO",
        "android.permission.POST_NOTIFICATIONS",
    ] {
        let _ = Command::new(&adb)
            .args(["shell", "pm", "grant", APP_PACKAGE, perm])
            .status();
    }

    ensure_accessibility_enabled(&cfg.android_home)?;

    println!(
        "==> [5/5] Forwarding port {} and launching Agentic Android...",
        cfg.port
    );
    ensure_port_forwarded(&cfg.android_home, cfg.port)?;

    let backup_secret = ensure_backup_secret(&cfg.state_dir);
    let status = Command::new(&adb)
        .args([
            "shell",
            "am",
            "start",
            "-S",
            "-n",
            MAIN_ACTIVITY,
            "--ez",
            "auto_start",
            "true",
            "--ei",
            "port",
            &cfg.port.to_string(),
            "--es",
            "backup_secret",
            &backup_secret,
        ])
        .status()
        .map_err(|e| format!("Failed to launch MainActivity: {e}"))?;
    if !status.success() {
        return Err("Failed to start MainActivity via adb.".into());
    }

    // Re-verify accessibility binding after force-stop/start (-S)
    ensure_accessibility_enabled(&cfg.android_home)?;

    // Apply configured GPS coordinates if set in config.toml or CLI flags
    apply_configured_location(cfg);

    Ok(())
}

fn is_accessibility_bound(android_home: &Path) -> bool {
    let Ok(out) = Command::new(adb_bin(android_home))
        .args(["shell", "dumpsys", "accessibility"])
        .output()
    else {
        return false;
    };
    let dump = String::from_utf8_lossy(&out.stdout);
    for line in dump.lines() {
        if line.contains("Bound services:")
            && (line.contains(APP_PACKAGE) || line.contains("Agentic Android Control"))
        {
            return true;
        }
    }
    false
}

fn ensure_accessibility_enabled(android_home: &Path) -> Result<(), String> {
    let adb = adb_bin(android_home);

    // Ensure restricted settings are allowed on Android 13+
    let _ = Command::new(&adb)
        .args([
            "shell",
            "appops",
            "set",
            APP_PACKAGE,
            "ACCESS_RESTRICTED_SETTINGS",
            "allow",
        ])
        .status();

    if is_accessibility_bound(android_home) {
        return Ok(());
    }

    for attempt in 1..=5 {
        let out = Command::new(&adb)
            .args([
                "shell",
                "settings",
                "get",
                "secure",
                "enabled_accessibility_services",
            ])
            .output()
            .map_err(|e| format!("Failed to query accessibility settings: {e}"))?;
        let existing = String::from_utf8_lossy(&out.stdout).trim().to_string();

        // Filter out our own service (and any legacy com.agenticandroid entry) so toggling forces rebind
        let others: Vec<&str> = if existing.is_empty() || existing == "null" {
            Vec::new()
        } else {
            existing
                .split(':')
                .filter(|s| {
                    !s.is_empty()
                        && !s.contains(APP_PACKAGE)
                        && !s.contains("com.agenticandroid")
                })
                .collect()
        };

        // Step 1: Temporarily write without our service so Android's ContentObserver sees a real change
        let without_ours = others.join(":");
        if without_ours.is_empty() {
            let _ = Command::new(&adb)
                .args([
                    "shell",
                    "settings",
                    "delete",
                    "secure",
                    "enabled_accessibility_services",
                ])
                .status();
        } else {
            let _ = Command::new(&adb)
                .args([
                    "shell",
                    "settings",
                    "put",
                    "secure",
                    "enabled_accessibility_services",
                    &without_ours,
                ])
                .status();
        }
        thread::sleep(Duration::from_millis(400));

        // Step 2: Write with our service enabled and accessibility_enabled = 1
        let mut with_ours = others;
        with_ours.push(ACCESSIBILITY_SERVICE);
        let new_services = with_ours.join(":");

        let _ = Command::new(&adb)
            .args([
                "shell",
                "settings",
                "put",
                "secure",
                "enabled_accessibility_services",
                &new_services,
            ])
            .status();
        let _ = Command::new(&adb)
            .args([
                "shell",
                "settings",
                "put",
                "secure",
                "accessibility_enabled",
                "1",
            ])
            .status();

        thread::sleep(Duration::from_millis(800));
        if is_accessibility_bound(android_home) {
            println!("    Accessibility Service bound and active.");
            return Ok(());
        }
        if attempt < 5 {
            println!("    Waiting for Accessibility Service to bind (attempt {attempt}/5)...");
            thread::sleep(Duration::from_secs(1));
        }
    }

    eprintln!("WARNING: Accessibility Service enabled in settings, but Android has not reported it bound yet.");
    Ok(())
}

fn ensure_port_forwarded(android_home: &Path, port: u16) -> Result<(), String> {
    let spec = format!("tcp:{port}");
    let status = Command::new(adb_bin(android_home))
        .args(["forward", &spec, &spec])
        .status()
        .map_err(|e| format!("Failed to run adb forward: {e}"))?;
    if !status.success() {
        return Err(format!("adb forward {spec} failed."));
    }
    Ok(())
}

fn probe_health(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let Ok(mut stream) = TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_secs(2),
    ) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let req = format!("GET /api/health HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = String::new();
    let _ = stream.read_to_string(&mut buf);
    buf.contains("200 OK") && buf.contains("agentic-android-rust")
}

fn wait_for_health(port: u16, timeout_secs: u64) -> bool {
    for _ in 0..timeout_secs {
        if probe_health(port) {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
    }
    false
}

fn cmd_run(opts: RunOpts) -> Result<(), String> {
    let cfg = load_config(&opts)?;

    let apk_path = if let Some(ref apk) = cfg.apk {
        if !apk.exists() {
            return Err(format!("APK not found: {}", apk.display()));
        }
        println!("==> Using prebuilt APK: {}", apk.display());
        apk.clone()
    } else {
        build_from_source(&cfg)?
    };

    if cfg.build_only {
        println!("Build complete: {}", apk_path.display());
        return Ok(());
    }

    println!("==> [3/5] Checking Android emulator...");
    let mut emulator_child: Option<Child> = None;
    let mut active_avd = cfg.avd.clone();

    if find_running_device(&cfg.android_home).is_none() {
        let avd_name = select_or_create_avd(&cfg)?;
        active_avd = Some(avd_name.clone());
        emulator_child = Some(spawn_emulator(&cfg, &avd_name)?);
        wait_for_emulator_boot(&cfg, emulator_child.as_mut(), 180)?;
    }

    install_and_configure(&cfg, &apk_path)?;

    println!(
        "    Waiting for Rust webserver on http://localhost:{} ...",
        cfg.port
    );
    if wait_for_health(cfg.port, 25) {
        restore_app_state_backup(&cfg);
        pull_app_state_backup(&cfg);
        println!();
        println!("============================================================");
        println!(" Agentic Android is running!");
        println!(" Web UI: http://localhost:{}", cfg.port);
        println!("============================================================");
    } else {
        eprintln!(
            "WARNING: App launched, but http://localhost:{}/api/health did not respond yet.",
            cfg.port
        );
    }

    if !cfg.supervise {
        return Ok(());
    }

    // Supervisor loop (used by daemon mode and systemd)
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let _ = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    });

    println!("==> Supervisor active (checking health every 10s, Ctrl+C / SIGTERM to stop)...");
    let mut tick: u64 = 0;
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(10));
        if !running.load(Ordering::SeqCst) {
            break;
        }
        tick += 1;
        if tick % 6 == 0 {
            pull_app_state_backup(&cfg);
        }

        // Check if managed emulator exited
        if let Some(ref mut child) = emulator_child {
            if let Ok(Some(exit_status)) = child.try_wait() {
                eprintln!(
                    "Emulator exited ({exit_status}); restarting..."
                );
                if let Some(ref avd_name) = active_avd {
                    if let Ok(new_child) = spawn_emulator(&cfg, avd_name) {
                        *child = new_child;
                        let _ = wait_for_emulator_boot(&cfg, Some(child), 180);
                        let _ = install_and_configure(&cfg, &apk_path);
                    }
                }
                continue;
            }
        }

        // Ensure accessibility stays bound even if app health is OK
        if !is_accessibility_bound(&cfg.android_home) {
            eprintln!("Accessibility Service unbound; re-enabling...");
            let _ = ensure_accessibility_enabled(&cfg.android_home);
        }

        // Ensure port forward, accessibility, and app health
        if !probe_health(cfg.port) {
            eprintln!("Health check failed on port {}; recovering...", cfg.port);
            let _ = ensure_accessibility_enabled(&cfg.android_home);
            let _ = ensure_port_forwarded(&cfg.android_home, cfg.port);
            let backup_secret = ensure_backup_secret(&cfg.state_dir);
            let _ = Command::new(adb_bin(&cfg.android_home))
                .args([
                    "shell",
                    "am",
                    "start",
                    "-n",
                    MAIN_ACTIVITY,
                    "--ez",
                    "auto_start",
                    "true",
                    "--ei",
                    "port",
                    &cfg.port.to_string(),
                    "--es",
                    "backup_secret",
                    &backup_secret,
                ])
                .status();
        }
    }

    println!("==> Shutting down supervisor...");
    pull_app_state_backup(&cfg);
    if let Some(mut child) = emulator_child {
        let _ = Command::new(adb_bin(&cfg.android_home))
            .args(["emu", "kill"])
            .status();
        // Give QEMU up to 20s to flush userdata-qemu.img cleanly before force-killing
        let mut exited = false;
        for _ in 0..40 {
            if let Ok(Some(_)) = child.try_wait() {
                exited = true;
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }
        if !exited {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    let pid_file = cfg.state_dir.join("daemon.pid");
    let _ = fs::remove_file(pid_file);
    Ok(())
}

fn cmd_daemon_start(opts: RunOpts) -> Result<(), String> {
    let state_dir = opts.state_dir.clone().unwrap_or_else(default_state_dir);
    fs::create_dir_all(&state_dir).map_err(|e| format!("Failed to create state dir: {e}"))?;

    let pid_path = state_dir.join("daemon.pid");
    if let Ok(pid_str) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if is_pid_alive(pid) {
                return Err(format!(
                    "Daemon is already running (PID {pid}). Run `agentic-android daemon stop` first."
                ));
            }
        }
    }

    let exe = env::current_exe().map_err(|e| format!("Cannot locate current binary: {e}"))?;
    let log_path = state_dir.join("daemon.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open {}: {e}", log_path.display()))?;
    let err_file = log_file
        .try_clone()
        .map_err(|e| format!("Failed to clone log handle: {e}"))?;

    let mut args = vec!["run".to_string(), "--supervise".to_string()];
    if let Some(ref avd) = opts.avd {
        args.extend(["--avd".into(), avd.clone()]);
    }
    if let Some(ref apk) = opts.apk {
        args.extend(["--apk".into(), apk.display().to_string()]);
    }
    if let Some(port) = opts.port {
        args.extend(["--port".into(), port.to_string()]);
    }
    if let Some(ref abi) = opts.abi {
        args.extend(["--abi".into(), abi.clone()]);
    }
    if opts.headless {
        args.push("--headless".into());
    }
    if let Some(ref gpu) = opts.gpu {
        args.extend(["--gpu".into(), gpu.clone()]);
    }
    if let Some(ref c) = opts.config {
        args.extend(["--config".into(), c.display().to_string()]);
    }
    if let Some(ref root) = opts.project_root {
        args.extend(["--project-root".into(), root.display().to_string()]);
    }
    if let Some(lat) = opts.lat {
        args.extend(["--lat".into(), lat.to_string()]);
    }
    if let Some(lon) = opts.lon {
        args.extend(["--lon".into(), lon.to_string()]);
    }
    if let Some(alt) = opts.alt {
        args.extend(["--alt".into(), alt.to_string()]);
    }
    args.extend(["--state-dir".into(), state_dir.display().to_string()]);

    let child = Command::new(&exe)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(err_file))
        .spawn()
        .map_err(|e| format!("Failed to spawn daemon: {e}"))?;

    let pid = child.id();
    fs::write(&pid_path, format!("{pid}\n"))
        .map_err(|e| format!("Failed to write PID file: {e}"))?;

    println!("Agentic Android daemon started (PID {pid})");
    println!("Logs: {}", log_path.display());
    println!("Check status with: agentic-android daemon status");
    Ok(())
}

fn cmd_daemon_stop(state_dir: Option<PathBuf>) -> Result<(), String> {
    let dir = state_dir.unwrap_or_else(default_state_dir);
    let pid_path = dir.join("daemon.pid");
    let pid_str = fs::read_to_string(&pid_path)
        .map_err(|_| format!("No running daemon found (missing {})", pid_path.display()))?;
    let pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|_| "Invalid PID file contents".to_string())?;

    if is_pid_alive(pid) {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
        for _ in 0..15 {
            if !is_pid_alive(pid) {
                break;
            }
            thread::sleep(Duration::from_millis(400));
        }
        println!("Stopped Agentic Android daemon (PID {pid}).");
    } else {
        println!("Daemon PID {pid} was not running; cleaned up stale PID file.");
    }
    let _ = fs::remove_file(&pid_path);
    Ok(())
}

fn cmd_daemon_status(port: u16, state_dir: Option<PathBuf>) -> Result<(), String> {
    let dir = state_dir.unwrap_or_else(default_state_dir);
    let pid_path = dir.join("daemon.pid");
    let daemon_running = fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&pid| is_pid_alive(pid));

    match daemon_running {
        Some(pid) => println!("Daemon:    running (PID {pid})"),
        None => println!("Daemon:    stopped"),
    }

    if let Ok(android_home) = detect_android_home(None) {
        match find_running_device(&android_home) {
            Some(serial) => println!("Emulator:  connected ({serial})"),
            None => println!("Emulator:  not detected"),
        }
    }

    if probe_health(port) {
        println!("Web UI:    healthy (http://localhost:{port})");
    } else {
        println!("Web UI:    not reachable on port {port}");
    }
    println!("Logs:      {}", dir.join("daemon.log").display());
    Ok(())
}

fn cmd_print_systemd(opts: RunOpts) -> Result<(), String> {
    let exe = env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/usr/local/bin/agentic-android".to_string());
    let avd_flag = opts
        .avd
        .map(|a| format!(" --avd {a}"))
        .unwrap_or_default();
    let apk_flag = opts
        .apk
        .map(|p| format!(" --apk {}", p.display()))
        .unwrap_or_default();
    let port = opts.port.unwrap_or(8080);

    println!(
        r#"[Unit]
Description=Agentic Android Headless Emulator & Supervisor
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={exe} run --supervise --headless --port {port}{avd_flag}{apk_flag}
Restart=on-failure
RestartSec=10
Environment=ANDROID_HOME=/opt/android-sdk

[Install]
WantedBy=multi-user.target"#
    );
    Ok(())
}

fn is_pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
