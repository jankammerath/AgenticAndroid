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

    fun dispatchAction(actionJson: String): String {
        return try {
            val req = JSONObject(actionJson)
            val tool = req.optString("tool", req.optString("action", ""))
            val params = req.optJSONObject("params") ?: req
            val service = AgentAccessibilityService.instance

            if (tool == "status") {
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

            val result = when (tool) {
                "tap" -> service.performTap(
                    params.optDouble("x", 0.0).toFloat(),
                    params.optDouble("y", 0.0).toFloat()
                )
                "double_tap" -> service.performDoubleTap(
                    params.optDouble("x", 0.0).toFloat(),
                    params.optDouble("y", 0.0).toFloat()
                )
                "long_press" -> service.performLongPress(
                    params.optDouble("x", 0.0).toFloat(),
                    params.optDouble("y", 0.0).toFloat(),
                    params.optLong("duration_ms", 700L)
                )
                "swipe" -> service.performSwipe(
                    params.optDouble("start_x", 0.0).toFloat(),
                    params.optDouble("start_y", 0.0).toFloat(),
                    params.optDouble("end_x", 0.0).toFloat(),
                    params.optDouble("end_y", 0.0).toFloat(),
                    params.optLong("duration_ms", 350L)
                )
                "type_text" -> service.performTypeText(
                    params.optString("text", ""),
                    params.optBoolean("clear_first", true)
                )
                "click_node" -> service.performClickNode(params.optString("query", ""))
                "global_action" -> service.performNamedGlobalAction(
                    params.optString("action", params.optString("name", ""))
                )
                "launch_app" -> service.performLaunchApp(params.optString("package_or_name", ""))
                "get_ui_tree" -> service.dumpUiTree()
                "request_screenshot" -> service.captureScreenshotToRust()
                else -> JSONObject().apply {
                    put("ok", false)
                    put("error", "Unknown action: $tool")
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
