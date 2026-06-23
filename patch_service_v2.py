import re

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/service/ConnectionService.kt"
with open(file_path, 'r') as f:
    content = f.read()

old_func = """    fun sendAdbInfo(port: String) {
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
    }"""

new_func = """    fun sendAdbInfo(ipAndPort: String, pairingCode: String) {
        try {
            val info = JSONObject().apply {
                put("type", "adb_info")
                put("target", ipAndPort)
                put("code", pairingCode)
            }
            webSocket?.send(info.toString())
            Toast.makeText(this, "已发送配对信息至电脑", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Toast.makeText(this, "发送失败", Toast.LENGTH_SHORT).show()
        }
    }"""

content = content.replace(old_func, new_func)

with open(file_path, 'w') as f:
    f.write(content)

print("Patch applied to ConnectionService.kt")
