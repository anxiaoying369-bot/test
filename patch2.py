import sys

def patch():
    path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/MainActivity.kt"
    with open(path, "r") as f:
        content = f.read()

    new_state = """            var serverIp by remember { mutableStateOf(prefs.getString("server_ip", "") ?: "") }
            var adbPort by remember { mutableStateOf(prefs.getString("adb_port", "") ?: "") }"""
    content = content.replace('            var serverIp by remember { mutableStateOf(prefs.getString("server_ip", "") ?: "") }', new_state)

    new_ui = """                        Spacer(modifier = Modifier.height(20.dp))
                        Divider()
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

                        // ───── 后台保活引导 ─────"""
    
    content = content.replace("                        // ───── 后台保活引导 ─────", new_ui)

    with open(path, "w") as f:
        f.write(content)

patch()
