package org.agenticandroid

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Path
import android.graphics.Rect
import android.os.Build
import android.os.Bundle
import android.util.DisplayMetrics
import android.view.Display
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class AgentAccessibilityService : AccessibilityService() {

    companion object {
        @Volatile
        var instance: AgentAccessibilityService? = null
            private set
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Events are observed on-demand via UI tree queries and live streaming.
    }

    override fun onInterrupt() {}

    override fun onDestroy() {
        super.onDestroy()
        if (instance === this) {
            instance = null
        }
    }

    fun getScreenSize(): Pair<Int, Int> {
        val wm = getSystemService(WINDOW_SERVICE) as WindowManager
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val bounds = wm.currentWindowMetrics.bounds
            Pair(bounds.width(), bounds.height())
        } else {
            val dm = DisplayMetrics()
            @Suppress("DEPRECATION")
            wm.defaultDisplay.getRealMetrics(dm)
            Pair(dm.widthPixels, dm.heightPixels)
        }
    }

    fun performTap(x: Float, y: Float): JSONObject {
        val path = Path().apply { moveTo(x, y) }
        val stroke = GestureDescription.StrokeDescription(path, 0, 50)
        return dispatchGestureSync(GestureDescription.Builder().addStroke(stroke).build(), "tap($x, $y)")
    }

    fun performDoubleTap(x: Float, y: Float): JSONObject {
        val p1 = Path().apply { moveTo(x, y) }
        val s1 = GestureDescription.StrokeDescription(p1, 0, 40)
        val s2 = GestureDescription.StrokeDescription(Path().apply { moveTo(x, y) }, 110, 40)
        val desc = GestureDescription.Builder().addStroke(s1).addStroke(s2).build()
        return dispatchGestureSync(desc, "double_tap($x, $y)")
    }

    fun performLongPress(x: Float, y: Float, durationMs: Long): JSONObject {
        val path = Path().apply { moveTo(x, y) }
        val stroke = GestureDescription.StrokeDescription(path, 0, durationMs.coerceIn(300, 5000))
        return dispatchGestureSync(GestureDescription.Builder().addStroke(stroke).build(), "long_press($x, $y)")
    }

    fun performSwipe(sx: Float, sy: Float, ex: Float, ey: Float, durationMs: Long): JSONObject {
        val path = Path().apply {
            moveTo(sx, sy)
            lineTo(ex, ey)
        }
        val stroke = GestureDescription.StrokeDescription(path, 0, durationMs.coerceIn(80, 3000))
        return dispatchGestureSync(GestureDescription.Builder().addStroke(stroke).build(), "swipe")
    }

    private fun dispatchGestureSync(gesture: GestureDescription, label: String): JSONObject {
        val latch = CountDownLatch(1)
        var completed = false
        val accepted = dispatchGesture(gesture, object : GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription?) {
                completed = true
                latch.countDown()
            }

            override fun onCancelled(gestureDescription: GestureDescription?) {
                completed = false
                latch.countDown()
            }
        }, null)

        if (!accepted) {
            return JSONObject().put("ok", false).put("error", "Gesture rejected: $label")
        }
        latch.await(3, TimeUnit.SECONDS)
        return JSONObject().put("ok", completed).put("action", label)
    }

    fun performTypeText(text: String, clearFirst: Boolean): JSONObject {
        val root = rootInActiveWindow ?: return JSONObject().put("ok", false).put("error", "No active window")
        val target = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT) ?: findFirstEditable(root)
            ?: return JSONObject().put("ok", false).put("error", "No editable text field found")

        val finalText = if (clearFirst) {
            text
        } else {
            val existing = target.text?.toString() ?: ""
            existing + text
        }
        val args = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, finalText)
        }
        val ok = target.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
        return JSONObject().put("ok", ok).put("typed", text)
    }

    private fun findFirstEditable(node: AccessibilityNodeInfo): AccessibilityNodeInfo? {
        if (node.isEditable) return node
        for (i in 0 until node.childCount) {
            val child = node.getChild(i) ?: continue
            val found = findFirstEditable(child)
            if (found != null) return found
        }
        return null
    }

    fun performClickNode(query: String): JSONObject {
        if (query.isBlank()) return JSONObject().put("ok", false).put("error", "Empty query")
        val root = rootInActiveWindow ?: return JSONObject().put("ok", false).put("error", "No active window")
        val q = query.lowercase()
        val match = findNodeMatching(root, q)
            ?: return JSONObject().put("ok", false).put("error", "No node matching '$query'")

        var current: AccessibilityNodeInfo? = match
        while (current != null) {
            if (current.isClickable) {
                val ok = current.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                if (ok) return JSONObject().put("ok", true).put("clicked", query)
            }
            current = current.parent
        }

        val bounds = Rect()
        match.getBoundsInScreen(bounds)
        return performTap(bounds.exactCenterX(), bounds.exactCenterY())
    }

    private fun findNodeMatching(node: AccessibilityNodeInfo, q: String): AccessibilityNodeInfo? {
        val text = node.text?.toString()?.lowercase() ?: ""
        val desc = node.contentDescription?.toString()?.lowercase() ?: ""
        val id = node.viewIdResourceName?.lowercase() ?: ""
        if (text.contains(q) || desc.contains(q) || id.contains(q)) {
            return node
        }
        for (i in 0 until node.childCount) {
            val child = node.getChild(i) ?: continue
            val found = findNodeMatching(child, q)
            if (found != null) return found
        }
        return null
    }

    fun performNamedGlobalAction(name: String): JSONObject {
        val actionId = when (name.lowercase()) {
            "back" -> GLOBAL_ACTION_BACK
            "home" -> GLOBAL_ACTION_HOME
            "recents" -> GLOBAL_ACTION_RECENTS
            "notifications" -> GLOBAL_ACTION_NOTIFICATIONS
            "quick_settings" -> GLOBAL_ACTION_QUICK_SETTINGS
            "power_dialog" -> GLOBAL_ACTION_POWER_DIALOG
            "lock_screen" -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) GLOBAL_ACTION_LOCK_SCREEN else -1
            "take_screenshot" -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) GLOBAL_ACTION_TAKE_SCREENSHOT else -1
            else -> -1
        }
        if (actionId == -1) {
            return JSONObject().put("ok", false).put("error", "Unsupported global action: $name")
        }
        val ok = performGlobalAction(actionId)
        return JSONObject().put("ok", ok).put("global_action", name)
    }

    fun performLaunchApp(packageOrName: String): JSONObject {
        val pm = packageManager
        var intent = pm.getLaunchIntentForPackage(packageOrName)
        if (intent == null) {
            val query = packageOrName.lowercase()
            val apps = pm.getInstalledApplications(0)
            val match = apps.firstOrNull { app ->
                val label = pm.getApplicationLabel(app).toString().lowercase()
                label == query || label.contains(query) || app.packageName.lowercase().contains(query)
            }
            if (match != null) {
                intent = pm.getLaunchIntentForPackage(match.packageName)
            }
        }
        if (intent == null) {
            return JSONObject().put("ok", false).put("error", "App not found: $packageOrName")
        }
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        startActivity(intent)
        return JSONObject().put("ok", true).put("launched", intent.component?.packageName ?: packageOrName)
    }

    fun dumpUiTree(): JSONObject {
        val root = rootInActiveWindow ?: return JSONObject().put("ok", false).put("error", "No active window")
        val (w, h) = getScreenSize()
        val pkg = root.packageName?.toString() ?: ""
        var appVersion = ""
        var appLabel = ""
        if (pkg.isNotEmpty()) {
            try {
                val pInfo = packageManager.getPackageInfo(pkg, 0)
                appVersion = pInfo.versionName ?: ""
                val appInfo = pInfo.applicationInfo
                if (appInfo != null) {
                    appLabel = packageManager.getApplicationLabel(appInfo).toString()
                }
            } catch (_: Throwable) {
            }
        }
        val nodes = JSONArray()
        collectInteractiveNodes(root, nodes, 0)
        return JSONObject().apply {
            put("ok", true)
            put("package", pkg)
            if (appLabel.isNotEmpty()) put("app_label", appLabel)
            if (appVersion.isNotEmpty()) put("app_version", appVersion)
            put("screen_width", w)
            put("screen_height", h)
            put("elements", nodes)
        }
    }

    private fun collectInteractiveNodes(node: AccessibilityNodeInfo, out: JSONArray, depth: Int) {
        if (out.length() >= 180 || depth > 24) return
        val text = node.text?.toString()?.trim() ?: ""
        val desc = node.contentDescription?.toString()?.trim() ?: ""
        val id = node.viewIdResourceName?.substringAfterLast("/") ?: ""
        val interesting = text.isNotEmpty() || desc.isNotEmpty() || node.isClickable || node.isEditable || node.isScrollable

        if (interesting && node.isVisibleToUser) {
            val r = Rect()
            node.getBoundsInScreen(r)
            if (r.width() > 0 && r.height() > 0) {
                out.put(JSONObject().apply {
                    if (text.isNotEmpty()) put("text", text)
                    if (desc.isNotEmpty()) put("desc", desc)
                    if (id.isNotEmpty()) put("id", id)
                    put("class", node.className?.toString()?.substringAfterLast(".") ?: "")
                    put("cx", r.centerX())
                    put("cy", r.centerY())
                    put("bounds", "[${r.left},${r.top},${r.right},${r.bottom}]")
                    if (node.isClickable) put("clickable", true)
                    if (node.isEditable) put("editable", true)
                    if (node.isScrollable) put("scrollable", true)
                })
            }
        }
        for (i in 0 until node.childCount) {
            val child = node.getChild(i) ?: continue
            collectInteractiveNodes(child, out, depth + 1)
        }
    }

    fun captureScreenshotToRust(): JSONObject {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            return JSONObject().put("ok", false).put("error", "Accessibility takeScreenshot requires Android 11+")
        }
        val latch = CountDownLatch(1)
        var captured = false
        takeScreenshot(Display.DEFAULT_DISPLAY, mainExecutor, object : TakeScreenshotCallback {
            override fun onSuccess(screenshot: ScreenshotResult) {
                try {
                    val hwBitmap = Bitmap.wrapHardwareBuffer(screenshot.hardwareBuffer, screenshot.colorSpace)
                    screenshot.hardwareBuffer.close()
                    if (hwBitmap != null) {
                        val soft = hwBitmap.copy(Bitmap.Config.ARGB_8888, false)
                        hwBitmap.recycle()
                        val w = soft.width
                        val h = soft.height
                        val scaledW = if (w > 720) 720 else w
                        val scaledH = (h.toFloat() * scaledW / w).toInt().coerceAtLeast(1)
                        val resized = if (scaledW != w) Bitmap.createScaledBitmap(soft, scaledW, scaledH, true) else soft
                        val baos = ByteArrayOutputStream()
                        resized.compress(Bitmap.CompressFormat.JPEG, 70, baos)
                        if (resized !== soft) resized.recycle()
                        soft.recycle()
                        NativeBridge.nativePushVideoFrame(baos.toByteArray(), w, h)
                        captured = true
                    }
                } catch (_: Throwable) {
                } finally {
                    latch.countDown()
                }
            }

            override fun onFailure(errorCode: Int) {
                latch.countDown()
            }
        })
        latch.await(2, TimeUnit.SECONDS)
        return JSONObject().put("ok", captured)
    }
}
