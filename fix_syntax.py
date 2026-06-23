import re

file_path = "autocast-mobile/app/src/main/java/com/make/autocast/mobile/MainActivity.kt"
with open(file_path, 'r') as f:
    content = f.read()

# Fix vertical scroll import
if 'import androidx.compose.foundation.verticalScroll' not in content:
    content = content.replace('import androidx.compose.foundation.layout.*', 'import androidx.compose.foundation.layout.*\nimport androidx.compose.foundation.verticalScroll\nimport androidx.compose.foundation.rememberScrollState')

# Fix text block syntax errors (escaping newlines)
content = content.replace('Text("1. 前往「开发者选项」打开「无线调试」\\n2. 点击「使用配对码配对设备」\\n3. 填入显示的 IP:端口 (例: 192.168.1.5:38541) 和配对码"', 'Text("1. 前往「开发者选项」打开「无线调试」\\n2. 点击「使用配对码配对设备」\\n3. 填入显示的 IP:端口 和 配对码"')

with open(file_path, 'w') as f:
    f.write(content)
