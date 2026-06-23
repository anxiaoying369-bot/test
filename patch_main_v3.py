import re

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/MainActivity.kt"
with open(file_path, 'r') as f:
    content = f.read()

# Make the Column scrollable
content = content.replace('Column(modifier = Modifier.padding(24.dp)) {', 'Column(modifier = Modifier.padding(24.dp).verticalScroll(androidx.compose.foundation.rememberScrollState())) {')
if 'androidx.compose.foundation.rememberScrollState' not in content:
    content = content.replace('import androidx.compose.foundation.layout.*', 'import androidx.compose.foundation.layout.*\nimport androidx.compose.foundation.verticalScroll')

# Add pairing code state
if 'var pairingCode by remember' not in content:
    content = content.replace(
        'var adbPort by remember { mutableStateOf(prefs.getString("adb_port", "") ?: "") }',
        'var adbPort by remember { mutableStateOf(prefs.getString("adb_port", "") ?: "") }\n            var pairingCode by remember { mutableStateOf(prefs.getString("pairing_code", "") ?: "") }'
    )

new_ui = """                        Spacer(modifier = Modifier.height(20.dp))
                        HorizontalDivider()
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("远程 ADB 调试 (免数据线强拉)", style = MaterialTheme.typography.titleMedium)
                        Spacer(modifier = Modifier.height(4.dp))
                        Text("1. 前往「开发者选项」打开「无线调试」\n2. 点击「使用配对码配对设备」\n3. 填入显示的 IP:端口 (例: 192.168.1.5:38541) 和配对码", style = MaterialTheme.typography.bodySmall, color = Color(0xFF888888))
                        
                        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            OutlinedButton(onClick = { 
                                runCatching { startActivity(Intent(Settings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS)) }
                            }, modifier = Modifier.weight(1f)) {
                                Text("前往开发者选项")
                            }
                        }
                        
                        Spacer(modifier = Modifier.height(8.dp))
                        
                        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            OutlinedTextField(
                                value = adbPort,
                                onValueChange = { 
                                    adbPort = it
                                    prefs.edit().putString("adb_port", it).apply()
                                },
                                label = { Text("IP:端口") },
                                modifier = Modifier.weight(1f),
                                singleLine = true
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            OutlinedTextField(
                                value = pairingCode,
                                onValueChange = { 
                                    pairingCode = it
                                    prefs.edit().putString("pairing_code", it).apply()
                                },
                                label = { Text("配对码") },
                                modifier = Modifier.weight(0.6f),
                                singleLine = true
                            )
                        }
                        Spacer(modifier = Modifier.height(8.dp))
                        Button(onClick = {
                            if (adbPort.isNotBlank()) {
                                ConnectionService.instance?.sendAdbInfo(adbPort, pairingCode)
                            }
                        }, modifier = Modifier.fillMaxWidth()) {
                            Text("发送配对信息至电脑")
                        }

                        Spacer(modifier = Modifier.height(20.dp))
                        HorizontalDivider()
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("后台保活（拔数据线后必做）","""

# Replace old ADB UI with new one
old_ui_start = 'Spacer(modifier = Modifier.height(20.dp))\n                        HorizontalDivider()\n                        Spacer(modifier = Modifier.height(12.dp))\n                        Text("远程 ADB 调试'
old_ui_end = 'Text("后台保活（拔数据线后必做）",'

start_idx = content.find(old_ui_start)
end_idx = content.find(old_ui_end, start_idx) + len(old_ui_end)

if start_idx != -1 and end_idx != -1:
    content = content[:start_idx] + new_ui + content[end_idx:]

with open(file_path, 'w') as f:
    f.write(content)

print("Patch applied to MainActivity.kt")
