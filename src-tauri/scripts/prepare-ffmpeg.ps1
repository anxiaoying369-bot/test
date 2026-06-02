#
# Download FFmpeg static binaries and extract to src-tauri/ffmpeg-runtime/windows/
#

$FFMPEG_VERSION = "7.0.1"
$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RUNTIME_DIR = Join-Path (Split-Path -Parent $SCRIPT_DIR) "ffmpeg-runtime"
$PLATFORM_DIR = Join-Path $RUNTIME_DIR "windows"
$CACHE_DIR = Join-Path $RUNTIME_DIR ".cache"

if (-not (Test-Path $CACHE_DIR)) {
    New-Item -ItemType Directory -Force -Path $CACHE_DIR | Out-Null
}
if (-not (Test-Path $PLATFORM_DIR)) {
    New-Item -ItemType Directory -Force -Path $PLATFORM_DIR | Out-Null
}

Write-Host "Target Platform: windows/x64"

$url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
$zipfile = Join-Path $CACHE_DIR "ffmpeg-win.zip"

if (-not (Test-Path $zipfile)) {
    Write-Host "Downloading FFmpeg (Windows)..."
    Invoke-WebRequest -Uri $url -OutFile $zipfile
}

Write-Host "Extracting to $PLATFORM_DIR\"
$tmpDir = Join-Path $RUNTIME_DIR "tmp_ffmpeg"
if (Test-Path $tmpDir) { Remove-Item -Recurse -Force $tmpDir }
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

Expand-Archive -Path $zipfile -DestinationPath $tmpDir -Force

$binFiles = Get-ChildItem -Path $tmpDir -Filter "*.exe" -Recurse | Where-Object { $_.DirectoryName -like "*bin*" }
foreach ($file in $binFiles) {
    Copy-Item -Path $file.FullName -Destination $PLATFORM_DIR -Force
}

Remove-Item -Recurse -Force $tmpDir

Write-Host ""
Write-Host "FFmpeg runtime prepared successfully."
Get-ChildItem -Path $PLATFORM_DIR -Filter "*.exe"
