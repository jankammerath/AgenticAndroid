package org.agenticandroid

import android.Manifest
import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.graphics.Bitmap
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioPlaybackCaptureConfiguration
import android.media.AudioRecord
import android.media.ImageReader
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.IBinder
import android.util.DisplayMetrics
import android.view.WindowManager
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import java.io.ByteArrayOutputStream
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class AgentForegroundService : Service() {

    companion object {
        const val ACTION_START_SERVER = "org.agenticandroid.START_SERVER"
        const val ACTION_STOP_SERVER = "org.agenticandroid.STOP_SERVER"
        const val ACTION_START_MEDIA = "org.agenticandroid.START_MEDIA"
        const val EXTRA_PORT = "port"
        const val EXTRA_RESULT_CODE = "result_code"
        const val EXTRA_RESULT_DATA = "result_data"

        private const val CHANNEL_ID = "agentic_android_channel"
        private const val NOTIFICATION_ID = 1001

        @Volatile
        var isMediaCastActive: Boolean = false
            private set
    }

    private var mediaProjection: MediaProjection? = null
    private var virtualDisplay: VirtualDisplay? = null
    private var imageReader: ImageReader? = null
    private var captureThread: HandlerThread? = null
    private var audioRecord: AudioRecord? = null
    private var audioThread: Thread? = null
    private val audioRunning = AtomicBoolean(false)

    private val fallbackScheduler = Executors.newSingleThreadScheduledExecutor()
    private var fallbackTask: ScheduledFuture<*>? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    private fun getBackupDirPath(): String {
        val ext = getExternalFilesDir(null) ?: return ""
        val dir = java.io.File(ext, "state_backup")
        dir.mkdirs()
        return dir.absolutePath
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP_SERVER -> {
                stopMediaCast()
                stopFallbackPoller()
                NativeBridge.nativeStopServer()
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
                return START_NOT_STICKY
            }
            ACTION_START_MEDIA -> {
                val code = intent.getIntExtra(EXTRA_RESULT_CODE, Activity.RESULT_CANCELED)
                @Suppress("DEPRECATION")
                val data = intent.getParcelableExtra<Intent>(EXTRA_RESULT_DATA)
                val port = NativeBridge.nativeGetServerPort().takeIf { it > 0 } ?: 8080
                if (code == Activity.RESULT_OK && data != null) {
                    try {
                        startAsForeground(port, withMediaProjection = true)
                        if (!NativeBridge.nativeIsServerRunning()) {
                            NativeBridge.nativeStartServer(port, filesDir.absolutePath, getBackupDirPath(), NativeBridge)
                            startFallbackPoller()
                        }
                        startMediaCast(code, data)
                    } catch (_: Throwable) {
                        stopMediaCast()
                        startAsForeground(port, withMediaProjection = false)
                    }
                } else {
                    startAsForeground(port, withMediaProjection = false)
                }
                return START_NOT_STICKY
            }
            else -> {
                val port = intent?.getIntExtra(EXTRA_PORT, 8080) ?: 8080
                startAsForeground(port, withMediaProjection = isMediaCastActive)
                NativeBridge.nativeStartServer(port, filesDir.absolutePath, getBackupDirPath(), NativeBridge)
                startFallbackPoller()
                return START_STICKY
            }
        }
    }

    private fun startAsForeground(port: Int, withMediaProjection: Boolean) {
        val openIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
        val notification: Notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Agentic Android Running")
            .setContentText("Web UI active on port $port")
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentIntent(openIntent)
            .setOngoing(true)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val type = if (withMediaProjection) {
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC or ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION
            } else {
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
            }
            startForeground(NOTIFICATION_ID, notification, type)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun startFallbackPoller() {
        if (fallbackTask != null) return
        fallbackTask = fallbackScheduler.scheduleWithFixedDelay({
            if (!isMediaCastActive && NativeBridge.nativeIsServerRunning()) {
                AgentAccessibilityService.instance?.captureScreenshotToRust()
            }
        }, 1200, 1200, TimeUnit.MILLISECONDS)
    }

    private fun stopFallbackPoller() {
        fallbackTask?.cancel(false)
        fallbackTask = null
    }

    private fun startMediaCast(resultCode: Int, data: Intent) {
        stopMediaCast()
        val mpm = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
        val projection = mpm.getMediaProjection(resultCode, data) ?: return
        mediaProjection = projection
        isMediaCastActive = true

        val wm = getSystemService(Context.WINDOW_SERVICE) as WindowManager
        val (fullW, fullH) = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val bounds = wm.maximumWindowMetrics.bounds
            Pair(bounds.width(), bounds.height())
        } else {
            val dm = DisplayMetrics()
            @Suppress("DEPRECATION")
            wm.defaultDisplay.getRealMetrics(dm)
            Pair(dm.widthPixels, dm.heightPixels)
        }
        val densityDpi = resources.displayMetrics.densityDpi
        val streamW = if (fullW > 720) 720 else fullW
        val streamH = Math.round(fullH.toFloat() * streamW / fullW).coerceAtLeast(1)

        val ht = HandlerThread("ScreenCapture").apply { start() }
        captureThread = ht
        val handler = Handler(ht.looper)

        projection.registerCallback(object : MediaProjection.Callback() {
            override fun onStop() {
                stopMediaCast()
            }
        }, handler)

        val reader = ImageReader.newInstance(streamW, streamH, PixelFormat.RGBA_8888, 2)
        imageReader = reader

        var lastFrameTs = 0L
        reader.setOnImageAvailableListener({ r ->
            val img = r.acquireLatestImage() ?: return@setOnImageAvailableListener
            try {
                val now = System.currentTimeMillis()
                if (now - lastFrameTs < 120) return@setOnImageAvailableListener // ~8 fps
                lastFrameTs = now

                val plane = img.planes[0]
                val buffer = plane.buffer
                val pixelStride = plane.pixelStride
                val rowStride = plane.rowStride
                val rowPadding = rowStride - pixelStride * streamW

                val bitmap = Bitmap.createBitmap(
                    streamW + rowPadding / pixelStride,
                    streamH,
                    Bitmap.Config.ARGB_8888
                )
                bitmap.copyPixelsFromBuffer(buffer)
                val cropped = Bitmap.createBitmap(bitmap, 0, 0, streamW, streamH)
                bitmap.recycle()

                val baos = ByteArrayOutputStream()
                cropped.compress(Bitmap.CompressFormat.JPEG, 65, baos)
                cropped.recycle()

                NativeBridge.nativePushVideoFrame(baos.toByteArray(), fullW, fullH)
            } catch (_: Throwable) {
            } finally {
                img.close()
            }
        }, handler)

        virtualDisplay = projection.createVirtualDisplay(
            "AgenticAndroidScreen",
            streamW,
            streamH,
            densityDpi,
            DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
            reader.surface,
            null,
            handler
        )

        startAudioPlaybackCapture(projection)
    }

    private fun startAudioPlaybackCapture(projection: MediaProjection) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) return
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            return
        }
        try {
            val sampleRate = 16000
            val config = AudioPlaybackCaptureConfiguration.Builder(projection)
                .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                .addMatchingUsage(AudioAttributes.USAGE_GAME)
                .addMatchingUsage(AudioAttributes.USAGE_UNKNOWN)
                .build()
            val format = AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(sampleRate)
                .setChannelMask(AudioFormat.CHANNEL_IN_MONO)
                .build()
            val minBuf = AudioRecord.getMinBufferSize(
                sampleRate,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT
            ).coerceAtLeast(3200)

            val record = AudioRecord.Builder()
                .setAudioFormat(format)
                .setBufferSizeInBytes(minBuf * 2)
                .setAudioPlaybackCaptureConfig(config)
                .build()

            audioRecord = record
            audioRunning.set(true)
            record.startRecording()

            audioThread = Thread {
                val buf = ByteArray(3200) // 100ms at 16kHz 16-bit mono
                while (audioRunning.get()) {
                    val read = record.read(buf, 0, buf.size)
                    if (read > 0) {
                        NativeBridge.nativePushAudioPcm(buf.copyOf(read), sampleRate, 1)
                    }
                }
            }.apply {
                name = "AudioCaptureThread"
                start()
            }
        } catch (_: Throwable) {
        }
    }

    private fun stopMediaCast() {
        isMediaCastActive = false
        audioRunning.set(false)
        try {
            audioRecord?.stop()
            audioRecord?.release()
        } catch (_: Throwable) {}
        audioRecord = null
        audioThread = null

        try { virtualDisplay?.release() } catch (_: Throwable) {}
        virtualDisplay = null
        try { imageReader?.close() } catch (_: Throwable) {}
        imageReader = null
        try { mediaProjection?.stop() } catch (_: Throwable) {}
        mediaProjection = null
        try { captureThread?.quitSafely() } catch (_: Throwable) {}
        captureThread = null
    }

    override fun onDestroy() {
        super.onDestroy()
        stopMediaCast()
        stopFallbackPoller()
        fallbackScheduler.shutdownNow()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Agentic Android Server",
                NotificationManager.IMPORTANCE_LOW
            )
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
    }
}
