# Download Node.js portable and put into src-tauri/node-runtime/windows/
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

Write-Host "Target Platform: Windows/$Arch"

$Url = "https://nodejs.org/dist/$NodeVersion/node-$NodeVersion-win-$Arch.zip"
$CacheDir = Join-Path $RuntimeDir ".cache"
New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null
$ZipFile = Join-Path $CacheDir "node-win.zip"

if (-not (Test-Path $ZipFile)) {
    Write-Host "Downloading Node.js portable..."
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile
}

Write-Host "Extracting to $PlatformDir\"
$tmpDir = Join-Path $RuntimeDir "tmp_node"
if (Test-Path $tmpDir) { Remove-Item -Recurse -Force $tmpDir }
Expand-Archive -Path $ZipFile -DestinationPath $tmpDir -Force

$extracted = Get-ChildItem -Path $tmpDir -Directory | Select-Object -First 1
if ($extracted) {
    Copy-Item -Path (Join-Path $extracted.FullName "*") -Destination $PlatformDir -Recurse -Force
}

Remove-Item -Recurse -Force $tmpDir
Write-Host "Node.js runtime prepared successfully."
