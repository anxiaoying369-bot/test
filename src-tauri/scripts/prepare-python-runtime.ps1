# 下载 python-build-standalone (Windows x86_64)，解压到 src-tauri\python-runtime\，
# 用它的 pip 安装 requirements.txt。
#
# 用法（在 PowerShell 中）：
#   .\src-tauri\scripts\prepare-python-runtime.ps1
#
$ErrorActionPreference = "Stop"

$PythonVersion = "3.11.10"
$ReleaseTag    = "20241016"

$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Path
$RuntimeDir = Resolve-Path (Join-Path $ScriptDir "..")
$RuntimeDir = Join-Path $RuntimeDir "python-runtime"
$RepoRoot   = Resolve-Path (Join-Path $ScriptDir "..\..")
$Requirements = Join-Path $RepoRoot "requirements.txt"

# ── 平台检测 ─────────────────────────────────────────
$Arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$PlatformTag = "$Arch-pc-windows-msvc"
Write-Host "▸ 目标平台: Windows/$Arch  →  $PlatformTag"

# ── 下载与解压 ───────────────────────────────────────
$Marker = Join-Path $RuntimeDir ".version"
$ExpectedMarker = "$PythonVersion-$PlatformTag"

if ((Test-Path $Marker) -and ((Get-Content $Marker) -eq $ExpectedMarker)) {
    Write-Host "✓ Python $PythonVersion ($PlatformTag) 已存在，跳过下载"
} else {
    $CacheDir = Join-Path $RuntimeDir ".cache"
    New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null

    $Tarball = Join-Path $CacheDir "python-$PythonVersion-$PlatformTag.tar.gz"
    $Url = "https://github.com/astral-sh/python-build-standalone/releases/download/$ReleaseTag/cpython-$PythonVersion+$ReleaseTag-$PlatformTag-install_only.tar.gz"

    if (-not (Test-Path $Tarball)) {
        Write-Host "▸ 下载 python-build-standalone..."
        Write-Host "  $Url"
        Invoke-WebRequest -Uri $Url -OutFile "$Tarball.tmp" -UseBasicParsing
        Move-Item -Force "$Tarball.tmp" $Tarball
    }

    Write-Host "▸ 解压到 $RuntimeDir\python\"
    $PythonDir = Join-Path $RuntimeDir "python"
    if (Test-Path $PythonDir) { Remove-Item -Recurse -Force $PythonDir }
    # Windows 自带 tar (Win10 1803+)
    tar -xzf $Tarball -C $RuntimeDir
    Set-Content -Path $Marker -Value $ExpectedMarker
}

# ── 安装依赖 ─────────────────────────────────────────
$PythonBin = Join-Path $RuntimeDir "python\python.exe"
if (-not (Test-Path $PythonBin)) {
    Write-Error "❌ 找不到 python.exe: $PythonBin"
    exit 1
}
if (-not (Test-Path $Requirements)) {
    Write-Error "❌ 找不到 requirements.txt: $Requirements"
    exit 1
}

Write-Host "▸ 升级 pip..."
& $PythonBin -m pip install --upgrade pip --quiet

Write-Host "▸ 安装依赖（来自 ${Requirements}）..."
& $PythonBin -m pip install -r $Requirements

Write-Host "▸ 清理 __pycache__ 和 .pyc..."
Get-ChildItem -Path (Join-Path $RuntimeDir "python") -Recurse -Directory -Filter "__pycache__" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path (Join-Path $RuntimeDir "python") -Recurse -File -Filter "*.pyc" | Remove-Item -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path (Join-Path $RuntimeDir "python") -Recurse -Directory -Filter "tests" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path (Join-Path $RuntimeDir "python") -Recurse -Directory -Filter "test"  | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

# ── 汇报体积 ─────────────────────────────────────────
$Size = (Get-ChildItem -Path (Join-Path $RuntimeDir "python") -Recurse | Measure-Object -Property Length -Sum).Sum
$SizeMB = [math]::Round($Size / 1MB, 1)
Write-Host ""
Write-Host "✅ Python 运行时准备完成"
Write-Host "   位置: $RuntimeDir\python"
Write-Host "   体积: $SizeMB MB"
Write-Host ""
Write-Host "下一步: 运行 npm run tauri build"
