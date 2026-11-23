package com.agenticandroid

import org.json.JSONObject

object NativeBridge {
    init {
        System.loadLibrary("agentic_android")
    }

    external fun nativeStartServer(port: Int, configDir: String, callback: Any): Boolean
    external fun nativeStopServer()
    external fun nativeIsServerRunning(): Boolean
    external fun nativeGetServerPort(): Int
    external fun nativePushVideoFrame(jpegBytes: ByteArray, width: Int, height: Int)
    external fun nativePushAudioPcm(pcmBytes: ByteArray, sampleRate: Int, channels: Int)

    @JvmStatic
    fun dispatchAction(actionJson: String): String {
        return try {
            val req = JSONObject(actionJson)
            val action = req.optString("action", "")
            val service = AgentAccessibilityService.instance

            if (action == "status") {
                val dims = service?.getScreenSize() ?: Pair(1080, 2400)
                return JSONObject().apply {
                    put("ok", true)
                    put("accessibility_enabled", service != null)
                    put("media_cast_active", AgentForegroundService.isMediaCastActive)
                    put("screen_width", dims.first)
                    put("screen_height", dims.second)
                }.toString()
            }

            if (service == null) {
                return JSONObject().apply {
                    put("ok", false)
                    put("error", "Accessibility service is not enabled. Enable 'Agentic Android Control' in Android Accessibility settings.")
                }.toString()
            }

            val result = when (action) {
                "tap" -> service.performTap(
                    req.optDouble("x", 0.0).toFloat(),
                    req.optDouble("y", 0.0).toFloat()
                )
                "double_tap" -> service.performDoubleTap(
                    req.optDouble("x", 0.0).toFloat(),
                    req.optDouble("y", 0.0).toFloat()
                )
                "long_press" -> service.performLongPress(
                    req.optDouble("x", 0.0).toFloat(),
                    req.optDouble("y", 0.0).toFloat(),
                    req.optLong("duration_ms", 700L)
                )
                "swipe" -> service.performSwipe(
                    req.optDouble("start_x", 0.0).toFloat(),
                    req.optDouble("start_y", 0.0).toFloat(),
                    req.optDouble("end_x", 0.0).toFloat(),
                    req.optDouble("end_y", 0.0).toFloat(),
                    req.optLong("duration_ms", 350L)
                )
                "type_text" -> service.performTypeText(
                    req.optString("text", ""),
                    req.optBoolean("clear_first", true)
                )
                "click_node" -> service.performClickNode(req.optString("query", ""))
                "global_action" -> service.performNamedGlobalAction(req.optString("action", req.optString("name", "")))
                "launch_app" -> service.performLaunchApp(req.optString("package_or_name", ""))
                "get_ui_tree" -> service.dumpUiTree()
                "request_screenshot" -> service.captureScreenshotToRust()
                else -> JSONObject().apply {
                    put("ok", false)
                    put("error", "Unknown action: $action")
                }
            }
            result.toString()
        } catch (t: Throwable) {
            JSONObject().apply {
                put("ok", false)
                put("error", t.message ?: t.javaClass.simpleName)
            }.toString()
        }
    }
}
