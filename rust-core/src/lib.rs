mod agent;
mod auth;
mod bridge;
mod llm;
mod server;
mod state;
mod telegram;

use crate::bridge::DeviceBridge;
use crate::server::start_http_server;
use crate::state::AppState;
use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jint, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

struct RunningServer {
    state: Arc<AppState>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    port: u16,
}

static RUNTIME: OnceCell<Runtime> = OnceCell::new();
static SERVER: OnceCell<Mutex<Option<RunningServer>>> = OnceCell::new();

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("AgenticAndroidRust"),
        );
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

fn server_slot() -> &'static Mutex<Option<RunningServer>> {
    SERVER.get_or_init(|| Mutex::new(None))
}

#[no_mangle]
pub extern "system" fn Java_org_agenticandroid_NativeBridge_nativeStartServer(
    mut env: JNIEnv,
    _class: JClass,
    port: jint,
    config_dir: JString,
    backup_dir: JString,
    backup_secret: JString,
    callback: JObject,
) -> jboolean {
    let rt = runtime();

    let dir_str: String = match env.get_string(&config_dir) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Invalid config_dir string: {e}");
            return JNI_FALSE;
        }
    };

    let backup_opt: Option<PathBuf> = env
        .get_string(&backup_dir)
        .ok()
        .map(|s| String::from(s))
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from);

    let secret_opt: Option<String> = env
        .get_string(&backup_secret)
        .ok()
        .map(|s| String::from(s))
        .filter(|s| !s.trim().is_empty());

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(e) => {
            log::error!("Failed to get JavaVM: {e}");
            return JNI_FALSE;
        }
    };

    let global_cb = match env.new_global_ref(callback) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Failed to create GlobalRef: {e}");
            return JNI_FALSE;
        }
    };

    let port_u16 = (port as u16).max(1024);
    let bridge = DeviceBridge::new(vm, global_cb);
    let state = AppState::new(PathBuf::from(dir_str), backup_opt, secret_opt, port_u16);

    let mut slot = server_slot().lock().unwrap();
    if let Some(mut existing) = slot.take() {
        if let Some(tx) = existing.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let state_clone = state.clone();
    rt.spawn(async move {
        if let Err(e) = start_http_server(state_clone, bridge, port_u16, shutdown_rx).await {
            log::error!("HTTP server terminated with error: {e}");
        }
    });

    *slot = Some(RunningServer {
        state,
        shutdown_tx: Some(shutdown_tx),
        port: port_u16,
    });

    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_org_agenticandroid_NativeBridge_nativeStopServer(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut slot = server_slot().lock().unwrap();
    if let Some(mut existing) = slot.take() {
        if let Some(tx) = existing.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_agenticandroid_NativeBridge_nativeIsServerRunning(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let slot = server_slot().lock().unwrap();
    if slot.is_some() {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_agenticandroid_NativeBridge_nativeGetServerPort(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let slot = server_slot().lock().unwrap();
    slot.as_ref().map(|s| s.port as jint).unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_org_agenticandroid_NativeBridge_nativePushVideoFrame(
    env: JNIEnv,
    _class: JClass,
    jpeg_bytes: JByteArray,
    width: jint,
    height: jint,
) {
    let state = {
        let slot = server_slot().lock().unwrap();
        match slot.as_ref() {
            Some(s) => s.state.clone(),
            None => return,
        }
    };

    if let Ok(bytes) = env.convert_byte_array(jpeg_bytes) {
        let rt = runtime();
        rt.spawn(async move {
            state
                .push_video_frame(bytes, width.max(0) as u32, height.max(0) as u32)
                .await;
        });
    }
}

#[no_mangle]
pub extern "system" fn Java_org_agenticandroid_NativeBridge_nativePushAudioPcm(
    env: JNIEnv,
    _class: JClass,
    pcm_bytes: JByteArray,
    sample_rate: jint,
    channels: jint,
) {
    let state = {
        let slot = server_slot().lock().unwrap();
        match slot.as_ref() {
            Some(s) => s.state.clone(),
            None => return,
        }
    };

    if let Ok(bytes) = env.convert_byte_array(pcm_bytes) {
        state.push_audio_pcm(bytes, sample_rate.max(8000) as u32, channels.max(1) as u16);
    }
}
