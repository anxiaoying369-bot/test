import re

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/service/ConnectionService.kt"
with open(file_path, 'r') as f:
    content = f.read()

# Replace the IP checking logic since the target format is "IP:PORT" or "IP",
# but the user might just input the port. Actually, the user inputs "IP:Port" now!
# Let's check what the frontend asks for.
# Ah, the UI says: "填入显示的 IP:端口" and the variable is adbPort.
# If adbPort contains ":", we don't need getLocalIpAddress() at all!

new_func = """    fun sendAdbInfo(targetInfo: String, pairingCode: String) {
        try {
            val info = JSONObject().apply {
                put("type", "adb_info")
                put("target", targetInfo)
                put("code", pairingCode)
            }
            webSocket?.send(info.toString())
            Toast.makeText(this, "已发送配对信息至电脑", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Toast.makeText(this, "发送失败", Toast.LENGTH_SHORT).show()
        }
    }"""

# Find the existing sendAdbInfo
if "fun sendAdbInfo(" in content:
    start_idx = content.find("fun sendAdbInfo(")
    end_idx = content.find("private fun initWebSocket() {")
    if start_idx != -1 and end_idx != -1:
        # Also remove getLocalIpAddress if it's there
        get_ip_idx = content.find("private fun getLocalIpAddress()")
        if get_ip_idx != -1 and get_ip_idx < start_idx:
            content = content[:get_ip_idx] + new_func + "\n\n" + content[end_idx:]
        else:
            content = content[:start_idx] + new_func + "\n\n" + content[end_idx:]

with open(file_path, 'w') as f:
    f.write(content)

print("Patch applied to ConnectionService.kt for IP logic")
