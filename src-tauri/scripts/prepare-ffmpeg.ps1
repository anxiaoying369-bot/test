#
# 下载 FFmpeg 静态二进制文件并解压到 src-tauri/ffmpeg-runtime/ (Windows)
#

$FFMPEG_VERSION = "7.0.1"
$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RUNTIME_DIR = Join-Path (Split-Path -Parent $SCRIPT_DIR) "ffmpeg-runtime"
$CACHE_DIR = Join-Path $RUNTIME_DIR ".cache"

if (-not (Test-Path $CACHE_DIR)) {
    New-Item -ItemType Directory -Force -Path $CACHE_DIR
}

Write-Host "▸ 目标平台: windows/x64"

$url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
$zipfile = Join-Path $CACHE_DIR "ffmpeg-win.zip"

if (-not (Test-Path $zipfile)) {
    Write-Host "▸ 下载 FFmpeg (Windows)..."
    Invoke-WebRequest -Uri $url -OutFile $zipfile
}

Write-Host "▸ 解压到 $RUNTIME_DIR/"
$tmpDir = Join-Path $RUNTIME_DIR "tmp"
if (Test-Path $tmpDir) { Remove-Item -Recurse -Force $tmpDir }

Expand-Archive -Path $zipfile -DestinationPath $tmpDir

$binFiles = Get-ChildItem -Path $tmpDir -Filter "*.exe" -Recurse | Where-Object { $_.DirectoryName -like "*bin" }
foreach ($file in $binFiles) {
    Copy-Item -Path $file.FullName -Destination $RUNTIME_DIR -Force
}

Remove-Item -Recurse -Force $tmpDir

Write-Host ""
Write-Host "✅ FFmpeg 运行时准备完成"
Get-ChildItem -Path $RUNTIME_DIR -Filter "*.exe"
