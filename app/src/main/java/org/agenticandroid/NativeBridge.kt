package org.agenticandroid

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import org.json.JSONObject
import java.io.File

object NativeBridge {
    init {
        System.loadLibrary("agentic_android")
    }

    external fun nativeStartServer(port: Int, configDir: String, backupDir: String, backupSecret: String, callback: Any): Boolean
    external fun nativeStopServer()
    external fun nativeIsServerRunning(): Boolean
    external fun nativeGetServerPort(): Int
    external fun nativePushVideoFrame(jpegBytes: ByteArray, width: Int, height: Int)
    external fun nativePushAudioPcm(pcmBytes: ByteArray, sampleRate: Int, channels: Int)

    private fun resolveDeviceName(service: AgentAccessibilityService?): String {
        val globalName = try {
            service?.let {
                Settings.Global.getString(it.contentResolver, Settings.Global.DEVICE_NAME)
            }?.trim()
        } catch (_: Throwable) {
            null
        }
        if (!globalName.isNullOrEmpty()) return globalName

        val model = Build.MODEL?.trim().orEmpty()
        val manufacturer = Build.MANUFACTURER?.trim().orEmpty()
        return when {
            model.isEmpty() -> "Android Device"
            manufacturer.isEmpty() || model.startsWith(manufacturer, ignoreCase = true) -> model
            else -> "$manufacturer $model"
        }
    }

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
                    put("device_name", resolveDeviceName(service))
                    put("screen_width", dims.first)
                    put("screen_height", dims.second)
                }.toString()
            }

            if (tool == "start_high_fps") {
                if (AgentForegroundService.isMediaCastActive) {
                    return JSONObject().apply {
                        put("ok", true)
                        put("media_cast_active", true)
                    }.toString()
                }
                val ctx = service ?: AgentForegroundService.appContext
                    ?: return JSONObject().apply {
                        put("ok", false)
                        put("error", "Service context unavailable")
                    }.toString()
                val activityIntent = Intent(ctx, MainActivity::class.java).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
                    putExtra("start_media_cast", true)
                }
                ctx.startActivity(activityIntent)
                return JSONObject().apply {
                    put("ok", true)
                    put("prompted", true)
                    put("media_cast_active", false)
                }.toString()
            }

            if (tool == "stop_high_fps") {
                val ctx = service ?: AgentForegroundService.appContext
                    ?: return JSONObject().apply {
                        put("ok", false)
                        put("error", "Service context unavailable")
                    }.toString()
                val svcIntent = Intent(ctx, AgentForegroundService::class.java).apply {
                    action = AgentForegroundService.ACTION_STOP_MEDIA
                }
                ctx.startService(svcIntent)
                return JSONObject().apply {
                    put("ok", true)
                    put("media_cast_active", false)
                }.toString()
            }

            if (tool == "install_apk") {
                val ctx = service ?: AgentForegroundService.appContext
                    ?: return JSONObject().apply {
                        put("ok", false)
                        put("error", "Service context unavailable")
                    }.toString()
                val apkPath = params.optString("path", "").trim()
                val apkFile = File(apkPath)
                if (apkPath.isEmpty() || !apkFile.exists() || !apkFile.isFile) {
                    return JSONObject().apply {
                        put("ok", false)
                        put("error", "APK file not found: $apkPath")
                    }.toString()
                }
                val canInstall = Build.VERSION.SDK_INT < Build.VERSION_CODES.O ||
                    ctx.packageManager.canRequestPackageInstalls()
                if (!canInstall && Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    val settingsIntent = Intent(
                        Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                        Uri.parse("package:${ctx.packageName}")
                    ).apply {
                        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    }
                    ctx.startActivity(settingsIntent)
                    return JSONObject().apply {
                        put("ok", true)
                        put("needs_permission", true)
                        put("message", "Enable 'Allow from this source' on the device, then upload again.")
                    }.toString()
                }
                val apkUri = FileProvider.getUriForFile(
                    ctx,
                    "${ctx.packageName}.fileprovider",
                    apkFile
                )
                val installIntent = Intent(Intent.ACTION_VIEW).apply {
                    setDataAndType(apkUri, "application/vnd.android.package-archive")
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                ctx.startActivity(installIntent)
                return JSONObject().apply {
                    put("ok", true)
                    put("needs_permission", false)
                    put("launched", true)
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
