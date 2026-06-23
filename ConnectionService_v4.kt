package com.make.autocast.mobile.service

import android.app.*
import android.content.*
import android.graphics.Bitmap
import android.os.*
import android.telephony.PhoneStateListener
import android.telephony.TelephonyManager
import android.util.Log
import android.provider.MediaStore
import android.widget.Toast
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.app.NotificationCompat
import com.arthenica.ffmpegkit.FFmpegKit
import com.arthenica.ffmpegkit.ReturnCode
import okhttp3.*
import okio.ByteString.Companion.toByteString
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileInputStream
import java.util.concurrent.TimeUnit

class ConnectionService : Service() {

    companion object {
        var connectionStatus by mutableStateOf("未连接")
        var lastError by mutableStateOf("")
        var isServiceRunning by mutableStateOf(false)
    }

    private var client: OkHttpClient? = null
    private var webSocket: WebSocket? = null
    private val mainHandler = Handler(Looper.getMainLooper())
    private var isConnected = false
    private var currentServerAddress: String = ""
    @Volatile private var reconnectScheduled = false

    private var lastUploadedPath: String? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        intent?.getStringExtra("SERVER_IP")?.let { ip ->
            val addr = if (ip.contains(":")) ip else "$ip:1422"
            if (addr != currentServerAddress || !isConnected) {
                currentServerAddress = addr
                initWebSocket()
            }
        }
        return START_STICKY
    }

    override fun onCreate() {
        super.onCreate()
        isServiceRunning = true
        startForeground(1, createNotification())
        setupCallListener()
    }

    private fun createNotification(): Notification {
        val channelId = "autocast_conn"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(channelId, "AutoCast Connection", NotificationManager.IMPORTANCE_LOW)
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
        return NotificationCompat.Builder(this, channelId)
            .setContentTitle("AutoCast Mobile")
            .setContentText("无线助手运行中 (支持手机端录音回传)")
            .setSmallIcon(android.R.drawable.ic_menu_share)
            .build()
    }

    private fun initWebSocket() {
        if (currentServerAddress.isEmpty()) return
        connectionStatus = "正在连接..."
        if (client == null) {
            client = OkHttpClient.Builder().connectTimeout(5, TimeUnit.SECONDS).build()
        }
        val androidId = android.provider.Settings.Secure.getString(contentResolver, android.provider.Settings.Secure.ANDROID_ID)
            ?: Build.MODEL.replace(" ", "_")
        val model = "${Build.BRAND}_${Build.MODEL}".replace(" ", "_")
        val url = "ws://$currentServerAddress/ws?device_id=phone_$androidId&model=$model"
        Log.i("AutoCast", "Connecting to $url")
        val request = Request.Builder().url(url).build()
        webSocket = client?.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                if (ws != webSocket) { ws.cancel(); return }
                isConnected = true
                reconnectScheduled = false
                connectionStatus = "已连接"
                sendDeviceInfo()
                startHeartbeat()
                mainHandler.postDelayed({ syncRecordings(forceFullScan = true) }, 2000)
            }
            override fun onMessage(ws: WebSocket, text: String) { handleCommand(text) }
            override fun onClosing(ws: WebSocket, code: Int, reason: String) { ws.close(1000, null) }
            override fun onClosed(ws: WebSocket, code: Int, reason: String) {
                if (ws == webSocket) { isConnected = false; connectionStatus = "已断开"; scheduleReconnect() }
            }
            override fun onFailure(ws: WebSocket, t: Throwable, response: Response?) {
                if (ws != webSocket) return
                isConnected = false
                connectionStatus = "连接失败"
                scheduleReconnect()
            }
        })
    }

    private fun scheduleReconnect() {
        if (reconnectScheduled) return
        reconnectScheduled = true
        mainHandler.postDelayed({
            reconnectScheduled = false
            if (!isConnected) initWebSocket()
        }, 5000)
    }

    private fun sendDeviceInfo() {
        try {
            val wm = getSystemService(Context.WINDOW_SERVICE) as android.view.WindowManager
            val (width, height) = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val bounds = wm.currentWindowMetrics.bounds
                bounds.width() to bounds.height()
            } else {
                val metrics = android.util.DisplayMetrics()
                @Suppress("DEPRECATION")
                wm.defaultDisplay.getRealMetrics(metrics)
                metrics.widthPixels to metrics.heightPixels
            }
            val info = JSONObject().apply {
                put("type", "device_info"); put("width", width); put("height", height)
            }
            webSocket?.send(info.toString())
        } catch (e: Exception) {}
    }

    private fun startHeartbeat() {
        mainHandler.removeCallbacksAndMessages(null)
        mainHandler.postDelayed(object : Runnable {
            override fun run() {
                if (isConnected) {
                    webSocket?.send("{\"type\":\"ping\"}")
                    mainHandler.postDelayed(this, 15000)
                }
            }
        }, 15000)
    }

    private fun handleCommand(text: String) {
        try {
            val json = JSONObject(text)
            val type = json.optString("type")
            Log.i("AutoCast", "Received command: $type")
            
            when (type) {
                "sync_recordings" -> {
                    mainHandler.post {
                        Toast.makeText(this, "正在同步录音...", Toast.LENGTH_SHORT).show()
                    }
                    syncRecordings(forceFullScan = true)
                }
                "click" -> {
                    val accService = AutoCastAccessibilityService.getInstance()
                    accService?.performClick(json.getDouble("x").toFloat(), json.getDouble("y").toFloat())
                }
                "swipe" -> {
                    val accService = AutoCastAccessibilityService.getInstance()
                    accService?.performSwipe(json.getDouble("x1").toFloat(), json.getDouble("y1").toFloat(), json.getDouble("x2").toFloat(), json.getDouble("y2").toFloat(), json.optLong("duration", 300))
                }
                "key" -> {
                    val accService = AutoCastAccessibilityService.getInstance()
                    accService?.performGlobalAction(json.optString("name"))
                }
                "screenshot" -> {
                    val accService = AutoCastAccessibilityService.getInstance()
                    accService?.takeAppScreenshot { bitmap, error -> bitmap?.let { sendScreenshot(it) } }
                }
            }
        } catch (e: Exception) {
            Log.e("AutoCast", "handleCommand error: ${e.message}")
        }
    }

    private fun sendScreenshot(bitmap: Bitmap) {
        Thread {
            try {
                val stream = ByteArrayOutputStream()
                bitmap.compress(Bitmap.CompressFormat.JPEG, 70, stream)
                val bytes = stream.toByteArray()
                val fileName = "screenshot_${System.currentTimeMillis()}.jpg"
                webSocket?.send("{\"type\":\"file_start\", \"name\":\"$fileName\", \"size\":${bytes.size}, \"file_type\":\"image\"}")
                val chunkSize = 64 * 1024
                var offset = 0
                while (offset < bytes.size) {
                    val length = Math.min(chunkSize, bytes.size - offset)
                    webSocket?.send(bytes.toByteString(offset, length))
                    offset += length
                }
                webSocket?.send("{\"type\":\"file_end\", \"name\":\"$fileName\"}")
            } catch (e: Exception) {}
        }.start()
    }

    private fun setupCallListener() {
        val tm = getSystemService(Context.TELEPHONY_SERVICE) as TelephonyManager
        tm.listen(object : PhoneStateListener() {
            override fun onCallStateChanged(state: Int, phoneNumber: String?) {
                Log.i("AutoCast", "Call state changed: $state")
                when (state) {
                    TelephonyManager.CALL_STATE_OFFHOOK -> {
                        mainHandler.postDelayed({
                            AutoCastAccessibilityService.getInstance()?.smartClickRecordButton()
                        }, 1500)
                    }
                    TelephonyManager.CALL_STATE_IDLE -> {
                        Log.i("AutoCast", "Call idle, scheduling sync...")
                        mainHandler.postDelayed({ syncRecordings() }, 5000)
                        mainHandler.postDelayed({ syncRecordings() }, 15000)
                        mainHandler.postDelayed({ syncRecordings() }, 40000)
                    }
                }
            }
        }, PhoneStateListener.LISTEN_CALL_STATE)
    }

    private val callRecordingDirs = listOf(
        "/sdcard/Sounds/CallRecord",
        "/sdcard/Music/Recordings/Call Recordings",
        "/sdcard/Recordings/Call Recordings",
        "/sdcard/MIUI/sound_recorder/call_rec",
        "/sdcard/Recordings/Call",
        "/sdcard/Recordings/CallRecord"
    )

    @Volatile private var syncing = false

    private fun syncRecordings(forceFullScan: Boolean = false) {
        if (!isConnected || syncing) {
            Log.w("AutoCast", "Sync skipped: isConnected=$isConnected, syncing=$syncing")
            return
        }
        syncing = true
        Thread {
            try {
                findAndProcessNativeRecording(forceFullScan)
            } catch (e: Exception) {
                Log.e("AutoCast", "syncRecordings error: ${e.message}")
            } finally {
                syncing = false
            }
        }.start()
    }

    private fun findAndProcessNativeRecording(forceFullScan: Boolean) {
        val now = System.currentTimeMillis()
        Log.i("AutoCast", "findAndProcessNativeRecording, forceFullScan=$forceFullScan")
        
        val filesToProcess = mutableListOf<File>()
        
        for (dirPath in callRecordingDirs) {
            val dir = File(dirPath)
            if (dir.exists() && dir.isDirectory) {
                val files = dir.listFiles() ?: continue
                for (f in files) {
                    if (f.isFile && f.length() > 2000) {
                        val threshold = if (forceFullScan) 24 * 3600 * 1000L else 15 * 60 * 1000L
                        if ((now - f.lastModified()) < threshold) {
                            filesToProcess.add(f)
                        }
                    }
                }
            }
        }
        
        filesToProcess.sortBy { it.lastModified() }

        if (filesToProcess.isEmpty()) {
            Log.i("AutoCast", "No new recording found.")
            return
        }

        for (file in filesToProcess) {
            if (file.absolutePath == lastUploadedPath) continue
            
            Log.i("AutoCast", "Found file to process: ${file.absolutePath}")
            lastUploadedPath = file.absolutePath
            convertAndUpload(file)
            
            if (!forceFullScan) break
        }
    }

    private fun convertAndUpload(sourceFile: File) {
        Thread {
            try {
                val ext = sourceFile.extension.lowercase()
                if (ext == "mp3" || ext == "m4a" || ext == "amr" || ext == "aac") {
                    Log.i("AutoCast", "Uploading directly: ${sourceFile.name}")
                    uploadFileInChunks(sourceFile)
                } else {
                    val mp3File = File(externalCacheDir, sourceFile.nameWithoutExtension + ".mp3")
                    Log.i("AutoCast", "Converting to MP3: ${mp3File.absolutePath}")
                    val session = FFmpegKit.execute("-y -i \"${sourceFile.absolutePath}\" -acodec libmp3lame -ab 64k \"${mp3File.absolutePath}\"")
                    if (ReturnCode.isSuccess(session.returnCode)) {
                        uploadFileInChunks(mp3File)
                    } else {
                        uploadFileInChunks(sourceFile)
                    }
                }
            } catch (e: Exception) {
                Log.e("AutoCast", "convertAndUpload error: ${e.message}")
                uploadFileInChunks(sourceFile)
            }
        }.start()
    }

    private fun uploadFileInChunks(file: File) {
        try {
            Log.i("AutoCast", "Uploading chunked: ${file.name} (${file.length()} bytes)")
            webSocket?.send("{\"type\":\"file_start\", \"name\":\"${file.name}\", \"size\":${file.length()}, \"file_type\":\"audio\"}")
            val buffer = ByteArray(64 * 1024)
            FileInputStream(file).use { fis ->
                var bytesRead: Int
                while (fis.read(buffer).also { bytesRead = it } != -1) {
                    webSocket?.send(buffer.toByteString(0, bytesRead))
                }
            }
            webSocket?.send("{\"type\":\"file_end\", \"name\":\"${file.name}\"}")
        } catch (e: Exception) {
            Log.e("AutoCast", "Upload error: ${e.message}")
        }
    }

    override fun onDestroy() { isServiceRunning = false; super.onDestroy() }
    override fun onBind(intent: Intent?) = null
}
