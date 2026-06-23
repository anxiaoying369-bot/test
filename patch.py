import sys

def patch():
    path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/service/ConnectionService.kt"
    with open(path, "r") as f:
        content = f.read()

    new_imports = """import java.util.concurrent.TimeUnit
import android.net.wifi.WifiManager"""
    content = content.replace("import java.util.concurrent.TimeUnit", new_imports)

    new_companion = """    companion object {
        var connectionStatus by mutableStateOf("未连接")
        var lastError by mutableStateOf("")
        var isServiceRunning by mutableStateOf(false)
        var instance: ConnectionService? = null
    }"""
    
    old_companion = """    companion object {
        var connectionStatus by mutableStateOf("未连接")
        var lastError by mutableStateOf("")
        var isServiceRunning by mutableStateOf(false)
    }"""
    content = content.replace(old_companion, new_companion)

    old_oncreate = """    override fun onCreate() {
        super.onCreate()
        isServiceRunning = true
        startForeground(1, createNotification())
        setupCallListener()
    }"""

    new_oncreate = """    override fun onCreate() {
        super.onCreate()
        isServiceRunning = true
        instance = this
        startForeground(1, createNotification())
        setupCallListener()
    }"""
    content = content.replace(old_oncreate, new_oncreate)

    new_methods = """    private fun getLocalIpAddress(): String {
        try {
            val wifiManager = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
            val ipAddress = wifiManager.connectionInfo.ipAddress
            if (ipAddress != 0) {
                return String.format("%d.%d.%d.%d",
                    ipAddress and 0xff,
                    ipAddress shr 8 and 0xff,
                    ipAddress shr 16 and 0xff,
                    ipAddress shr 24 and 0xff
                )
            }
        } catch (e: Exception) {}
        return ""
    }

    fun sendAdbInfo(port: String) {
        val ip = getLocalIpAddress()
        if (ip.isEmpty()) {
            Toast.makeText(this, "无法获取局域网 IP，请确保已连接 WiFi", Toast.LENGTH_SHORT).show()
            return
        }
        try {
            val info = JSONObject().apply {
                put("type", "adb_info")
                put("ip", ip)
                put("port", port)
            }
            webSocket?.send(info.toString())
            Toast.makeText(this, "已发送远程调试信息至电脑", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Toast.makeText(this, "发送失败", Toast.LENGTH_SHORT).show()
        }
    }

    private fun initWebSocket() {"""

    content = content.replace("    private fun initWebSocket() {", new_methods)

    with open(path, "w") as f:
        f.write(content)

patch()
