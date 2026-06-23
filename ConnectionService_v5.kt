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
            .setContentText("无线助手运行中")
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
            when (type) {
                "sync_recordings" -> {
                    mainHandler.post { Toast.makeText(this, "正在同步录音...", Toast.LENGTH_SHORT).show() }
                    syncRecordings(forceFullScan = true)
                }
                "click" -> AutoCastAccessibilityService.getInstance()?.performClick(json.getDouble("x").toFloat(), json.getDouble("y").toFloat())
                "swipe" -> AutoCastAccessibilityService.getInstance()?.performSwipe(json.getDouble("x1").toFloat(), json.getDouble("y1").toFloat(), json.getDouble("x2").toFloat(), json.getDouble("y2").toFloat(), json.optLong("duration", 300))
                "key" -> AutoCastAccessibilityService.getInstance()?.performGlobalAction(json.optString("name"))
                "screenshot" -> AutoCastAccessibilityService.getInstance()?.takeAppScreenshot { bitmap, _ -> bitmap?.let { sendScreenshot(it) } }
            }
        } catch (e: Exception) { Log.e("AutoCast", "Err: ${e.message}") }
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
                if (state == TelephonyManager.CALL_STATE_OFFHOOK) {
                    mainHandler.postDelayed({ AutoCastAccessibilityService.getInstance()?.smartClickRecordButton() }, 1500)
                } else if (state == TelephonyManager.CALL_STATE_IDLE) {
                    mainHandler.postDelayed({ syncRecordings() }, 5000)
                    mainHandler.postDelayed({ syncRecordings() }, 20000)
                }
            }
        }, PhoneStateListener.LISTEN_CALL_STATE)
    }

    private val callRecordingDirs = listOf(
        "/sdcard/Sounds/CallRecord",
        "/storage/emulated/0/Sounds/CallRecord",
        "/sdcard/Recordings/Call",
        "/sdcard/Music/Recordings/Call Recordings"
    )

    @Volatile private var syncing = false

    private fun syncRecordings(forceFullScan: Boolean = false) {
        if (!isConnected || syncing) return
        syncing = true
        Thread {
            try {
                val now = System.currentTimeMillis()
                val threshold = if (forceFullScan) 48 * 3600 * 1000L else 15 * 60 * 1000L
                val filesToUpload = mutableListOf<File>()

                for (dirPath in callRecordingDirs) {
                    val dir = File(dirPath)
                    if (dir.exists() && dir.isDirectory) {
                        dir.listFiles()?.filter { it.isFile && it.length() > 1000 && (now - it.lastModified()) < threshold }?.let {
                            filesToUpload.addAll(it)
                        }
                    }
                }
                
                try {
                    val projection = arrayOf(MediaStore.Audio.Media.DATA, MediaStore.Audio.Media.DATE_ADDED)
                    val cutOff = (now - threshold) / 1000
                    contentResolver.query(MediaStore.Audio.Media.EXTERNAL_CONTENT_URI, projection, "${MediaStore.Audio.Media.DATE_ADDED} >= ?", arrayOf(cutOff.toString()), null)?.use { cursor ->
                        while (cursor.moveToNext()) {
                            val path = cursor.getString(cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.DATA))
                            val file = File(path)
                            if (file.exists() && (path.contains("Call", true) || path.contains("通话", true))) {
                                if (!filesToUpload.any { it.absolutePath == file.absolutePath }) filesToUpload.add(file)
                            }
                        }
                    }
                } catch (e: Exception) { Log.e("AutoCast", "MediaStore err: ${e.message}") }

                filesToUpload.sortBy { it.lastModified() }
                
                val toProcess = if (forceFullScan) filesToUpload else {
                    val latest = filesToUpload.lastOrNull()
                    if (latest != null && latest.absolutePath != lastUploadedPath) {
                        lastUploadedPath = latest.absolutePath
                        listOf(latest)
                    } else emptyList()
                }
                
                for (file in toProcess) {
                    Log.i("AutoCast", "Syncing: ${file.name}")
                    uploadFileInChunks(file)
                }
            } catch (e: Exception) { Log.e("AutoCast", "Sync err: ${e.message}")
            } finally { syncing = false }
        }.start()
    }

    private fun uploadFileInChunks(file: File) {
        try {
            webSocket?.send("{\"type\":\"file_start\", \"name\":\"${file.name}\", \"size\":${file.length()}, \"file_type\":\"audio\"}")
            val buffer = ByteArray(64 * 1024)
            FileInputStream(file).use { fis ->
                var bytesRead: Int
                while (fis.read(buffer).also { bytesRead = it } != -1) {
                    webSocket?.send(buffer.toByteString(0, bytesRead))
                }
            }
            webSocket?.send("{\"type\":\"file_end\", \"name\":\"${file.name}\"}")
        } catch (e: Exception) { Log.e("AutoCast", "Upload err: ${e.message}") }
    }

    override fun onDestroy() { isServiceRunning = false; super.onDestroy() }
    override fun onBind(intent: Intent?) = null
}
