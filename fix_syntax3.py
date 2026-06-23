import re

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/MainActivity.kt"
with open(file_path, 'r') as f:
    content = f.read()

bad_str = 'Text("""1. 前往「开发者选项」打开「无线调试」\n2. 点击「使用配对码配对设备」\n3. 填入显示的 IP:端口 (例: 192.168.1.5:38541) 和配对码"""), style = MaterialTheme.typography.bodySmall, color = Color(0xFF888888))'
good_str = 'Text("""1. 前往「开发者选项」打开「无线调试」\n2. 点击「使用配对码配对设备」\n3. 填入显示的 IP:端口 (例: 192.168.1.5:38541) 和配对码""", style = MaterialTheme.typography.bodySmall, color = Color(0xFF888888))'

content = content.replace(bad_str, good_str)

with open(file_path, 'w') as f:
    f.write(content)
