# 下载 Node.js 便携版并放入 src-tauri/node-runtime/windows/
#
$ErrorActionPreference = "Stop"

$NodeVersion = "v20.15.1"
$Arch = "x64"

$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Path
$RuntimeDir = Resolve-Path (Join-Path $ScriptDir "..")
$RuntimeDir = Join-Path $RuntimeDir "node-runtime"
$PlatformDir = Join-Path $RuntimeDir "windows"

if (-not (Test-Path $PlatformDir)) {
    New-Item -ItemType Directory -Force -Path $PlatformDir | Out-Null
}

Write-Host "▸ 目标平台: Windows/$Arch"

$Url = "https://nodejs.org/dist/$NodeVersion/node-$NodeVersion-win-$Arch.zip"
$CacheDir = Join-Path $RuntimeDir ".cache"
New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null
$ZipFile = Join-Path $CacheDir "node-win.zip"

if (-not (Test-Path $ZipFile)) {
    Write-Host "▸ 下载 Node.js 便携版..."
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile
}

Write-Host "▸ 解压到 $PlatformDir\"
$tmpDir = Join-Path $RuntimeDir "tmp_node"
if (Test-Path $tmpDir) { Remove-Item -Recurse -Force $tmpDir }
Expand-Archive -Path $ZipFile -DestinationPath $tmpDir -Force

$extracted = Get-ChildItem -Path $tmpDir -Directory | Select-Object -First 1
if ($extracted) {
    Copy-Item -Path (Join-Path $extracted.FullName "*") -Destination $PlatformDir -Recurse -Force
}

Remove-Item -Recurse -Force $tmpDir
Write-Host "✅ Node.js 运行时准备完成"
