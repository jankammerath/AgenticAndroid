package com.agenticandroid

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import java.net.Inet4Address
import java.net.NetworkInterface

class MainActivity : AppCompatActivity() {

    private lateinit var editPort: EditText
    private lateinit var txtServerStatus: TextView
    private lateinit var txtServerUrl: TextView
    private lateinit var txtAccessibilityStatus: TextView
    private lateinit var txtCaptureStatus: TextView

    private val mediaProjectionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK && result.data != null) {
            val svcIntent = Intent(this, AgentForegroundService::class.java).apply {
                action = AgentForegroundService.ACTION_START_MEDIA
                putExtra(AgentForegroundService.EXTRA_RESULT_CODE, result.resultCode)
                putExtra(AgentForegroundService.EXTRA_RESULT_DATA, result.data)
            }
            ContextCompat.startForegroundService(this, svcIntent)
            refreshUi()
        }
    }

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        editPort = findViewById(R.id.editPort)
        txtServerStatus = findViewById(R.id.txtServerStatus)
        txtServerUrl = findViewById(R.id.txtServerUrl)
        txtAccessibilityStatus = findViewById(R.id.txtAccessibilityStatus)
        txtCaptureStatus = findViewById(R.id.txtCaptureStatus)

        val prefs = getSharedPreferences("agentic_prefs", Context.MODE_PRIVATE)
        val savedPort = intent.getIntExtra("port", prefs.getInt("port", 8080))
        editPort.setText(savedPort.toString())

        findViewById<Button>(R.id.btnStartServer).setOnClickListener {
            startServer()
        }

        findViewById<Button>(R.id.btnStopServer).setOnClickListener {
            val svcIntent = Intent(this, AgentForegroundService::class.java).apply {
                action = AgentForegroundService.ACTION_STOP_SERVER
            }
            startService(svcIntent)
            txtServerStatus.postDelayed({ refreshUi() }, 250)
        }

        findViewById<Button>(R.id.btnOpenAccessibility).setOnClickListener {
            startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        }

        findViewById<Button>(R.id.btnStartMediaCast).setOnClickListener {
            if (!NativeBridge.nativeIsServerRunning()) {
                startServer()
            }
            val mpm = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
            mediaProjectionLauncher.launch(mpm.createScreenCaptureIntent())
        }

        requestRuntimePermissions()

        // Auto-start server if launched with auto_start=true or first launch
        if (intent.getBooleanExtra("auto_start", true) && !NativeBridge.nativeIsServerRunning()) {
            startServer()
        }
    }

    override fun onResume() {
        super.onResume()
        refreshUi()
    }

    private fun requestRuntimePermissions() {
        val needed = mutableListOf<String>()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            needed.add(Manifest.permission.POST_NOTIFICATIONS)
        }
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            needed.add(Manifest.permission.RECORD_AUDIO)
        }
        if (needed.isNotEmpty()) {
            permissionLauncher.launch(needed.toTypedArray())
        }
    }

    private fun startServer() {
        val port = editPort.text.toString().toIntOrNull()?.coerceIn(1024, 65535) ?: 8080
        getSharedPreferences("agentic_prefs", Context.MODE_PRIVATE)
            .edit()
            .putInt("port", port)
            .apply()

        val svcIntent = Intent(this, AgentForegroundService::class.java).apply {
            action = AgentForegroundService.ACTION_START_SERVER
            putExtra(AgentForegroundService.EXTRA_PORT, port)
        }
        ContextCompat.startForegroundService(this, svcIntent)
        txtServerStatus.postDelayed({ refreshUi() }, 300)
    }

    private fun refreshUi() {
        val running = NativeBridge.nativeIsServerRunning()
        val port = NativeBridge.nativeGetServerPort().takeIf { it > 0 }
            ?: editPort.text.toString().toIntOrNull() ?: 8080

        if (running) {
            txtServerStatus.text = "Server: Running (Rust NDK Core)"
            txtServerStatus.setTextColor(0xFF22C55E.toInt())
            val ip = getLocalIpAddress() ?: "127.0.0.1"
            txtServerUrl.text = "Web UI: http://$ip:$port\n(Emulator via adb forward: http://localhost:$port)"
        } else {
            txtServerStatus.text = "Server: Stopped"
            txtServerStatus.setTextColor(0xFFF59E0B.toInt())
            txtServerUrl.text = ""
        }

        val accEnabled = AgentAccessibilityService.instance != null
        txtAccessibilityStatus.text = if (accEnabled) {
            "Accessibility Service: Active"
        } else {
            "Accessibility Service: Disabled"
        }
        txtAccessibilityStatus.setTextColor(if (accEnabled) 0xFF22C55E.toInt() else 0xFFF59E0B.toInt())

        txtCaptureStatus.text = if (AgentForegroundService.isMediaCastActive) {
            "Media Cast: Active (High-FPS Video + Audio)"
        } else {
            "Media Cast: Fallback (Accessibility Screenshots)"
        }
    }

    private fun getLocalIpAddress(): String? {
        return try {
            NetworkInterface.getNetworkInterfaces().toList()
                .flatMap { it.inetAddresses.toList() }
                .firstOrNull { !it.isLoopbackAddress && it is Inet4Address }
                ?.hostAddress
        } catch (_: Throwable) {
            null
        }
    }
}
