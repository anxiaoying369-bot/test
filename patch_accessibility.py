import sys

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/service/AutoCastAccessibilityService.kt"
with open(file_path, 'r') as f:
    content = f.read()

new_logic = """    private var lastSentPairingCode: String = ""

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event == null) return
        val rootNode = rootInActiveWindow ?: return
        
        if (event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED || 
            event.eventType == AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED) {
            extractPairingInfo(rootNode)
        }
    }

    private fun extractPairingInfo(node: AccessibilityNodeInfo) {
        val texts = mutableListOf<String>()
        extractAllText(node, texts)
        
        var ipPort = ""
        var code = ""
        
        val ipRegex = Regex(\"\"\"\\b(?:[0-9]{1,3}\\.){3}[0-9]{1,3}:[0-9]{4,5}\\b\"\"\")
        val codeRegex = Regex(\"\"\"^\\d{6}$\"\"\")
        
        for (text in texts) {
            val s = text.trim()
            if (ipRegex.containsMatchIn(s)) {
                ipPort = ipRegex.find(s)?.value ?: ""
            }
            if (codeRegex.matches(s)) {
                code = s
            }
        }
        
        if (ipPort.isNotEmpty() && code.isNotEmpty() && code != lastSentPairingCode) {
            Log.i("AutoCast", "Auto-extracted Pair Info: target=$ipPort, code=$code")
            lastSentPairingCode = code
            ConnectionService.instance?.sendAdbInfo(ipPort, code)
        }
    }

    private fun extractAllText(node: AccessibilityNodeInfo, texts: MutableList<String>) {
        if (node.text != null) texts.add(node.text.toString())
        if (node.contentDescription != null) texts.add(node.contentDescription.toString())
        for (i in 0 until node.childCount) {
            val child = node.getChild(i)
            if (child != null) extractAllText(child, texts)
        }
    }"""

content = content.replace("    override fun onAccessibilityEvent(event: AccessibilityEvent?) {}", new_logic)

with open(file_path, 'w') as f:
    f.write(content)

print("Accessibility patch applied")
