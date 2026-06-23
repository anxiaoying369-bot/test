import re

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/MainActivity.kt"
with open(file_path, 'r') as f:
    content = f.read()

# Add adbPort variable
if 'var adbPort by remember' not in content:
    content = content.replace(
        'var serverIp by remember { mutableStateOf(prefs.getString("server_ip", "") ?: "") }',
        'var serverIp by remember { mutableStateOf(prefs.getString("server_ip", "") ?: "") }\n            var adbPort by remember { mutableStateOf(prefs.getString("adb_port", "") ?: "") }'
    )

new_ui = """                        Spacer(modifier = Modifier.height(20.dp))
                        HorizontalDivider()
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("远程 ADB 调试 (免数据线强拉)", style = MaterialTheme.typography.titleMedium)
                        Spacer(modifier = Modifier.height(4.dp))
                        Text("请先前往「开发者选项」开启「无线调试」，并记下系统分配的 5 位数端口号填入下方。", style = MaterialTheme.typography.bodySmall, color = Color(0xFF888888))
                        
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
                                label = { Text("端口 (如 38541)") },
                                modifier = Modifier.weight(1f),
                                singleLine = true
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Button(onClick = {
                                if (adbPort.isNotBlank()) {
                                    ConnectionService.instance?.sendAdbInfo(adbPort)
                                }
                            }) {
                                Text("发送到电脑")
                            }
                        }

                        Spacer(modifier = Modifier.height(20.dp))
                        HorizontalDivider()
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("后台保活（拔数据线后必做）","""

if "远程 ADB 调试" not in content:
    content = content.replace(
        """                        Spacer(modifier = Modifier.height(20.dp))
                        Divider()
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("后台保活（拔数据线后必做）",""", new_ui)
    
    content = content.replace(
        """                        Spacer(modifier = Modifier.height(20.dp))
                        HorizontalDivider()
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("后台保活（拔数据线后必做）",""", new_ui)

with open(file_path, 'w') as f:
    f.write(content)

print("Patch applied.")
