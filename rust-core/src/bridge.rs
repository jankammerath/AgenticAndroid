use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::JavaVM;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct DeviceBridge {
    vm: Arc<JavaVM>,
    callback: Arc<GlobalRef>,
}

impl DeviceBridge {
    pub fn new(vm: JavaVM, callback: GlobalRef) -> Self {
        Self {
            vm: Arc::new(vm),
            callback: Arc::new(callback),
        }
    }

    pub fn dispatch_action(&self, action: Value) -> Result<Value, String> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|e| format!("Failed to attach JVM thread: {e}"))?;

        let payload = serde_json::to_string(&action)
            .map_err(|e| format!("Failed to serialize action: {e}"))?;

        let j_payload = env
            .new_string(&payload)
            .map_err(|e| format!("Failed to create JNI string: {e}"))?;

        let result_val = env
            .call_method(
                self.callback.as_obj(),
                "dispatchAction",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&JObject::from(j_payload))],
            )
            .map_err(|e| {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
                format!("JNI dispatchAction call failed: {e}")
            })?;

        let result_obj = result_val
            .l()
            .map_err(|e| format!("Invalid return type from dispatchAction: {e}"))?;

        if result_obj.is_null() {
            return Ok(json!({ "ok": false, "error": "Null response from Android bridge" }));
        }

        let j_str = JString::from(result_obj);
        let resp_str: String = env
            .get_string(&j_str)
            .map_err(|e| format!("Failed to read JNI return string: {e}"))?
            .into();

        serde_json::from_str(&resp_str)
            .map_err(|e| format!("Invalid JSON from Android bridge: {e} (raw: {resp_str})"))
    }

    pub async fn dispatch_async(&self, action: Value) -> Result<Value, String> {
        let bridge = self.clone();
        tokio::task::spawn_blocking(move || bridge.dispatch_action(action))
            .await
            .map_err(|e| format!("Bridge task panicked: {e}"))?
    }

    pub async fn get_status(&self) -> Value {
        self.dispatch_async(json!({ "action": "status" }))
            .await
            .unwrap_or_else(|e| json!({ "ok": false, "error": e }))
    }

    pub async fn get_ui_tree(&self) -> Value {
        self.dispatch_async(json!({ "action": "get_ui_tree" }))
            .await
            .unwrap_or_else(|e| json!({ "ok": false, "error": e }))
    }

    pub async fn request_screenshot(&self) -> Value {
        self.dispatch_async(json!({ "action": "request_screenshot" }))
            .await
            .unwrap_or_else(|e| json!({ "ok": false, "error": e }))
    }
}
