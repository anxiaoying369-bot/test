# Download python-build-standalone (Windows x86_64), extract to src-tauri\python-runtime\windows\
# Install requirements.txt using its pip.
#
# Usage (in PowerShell):
#   .\src-tauri\scripts\prepare-python-runtime.ps1
#
$ErrorActionPreference = "Stop"

$PythonVersion = "3.11.10"
$ReleaseTag    = "20241016"

$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Path
$RuntimeDir = Resolve-Path (Join-Path $ScriptDir "..")
$RuntimeDir = Join-Path $RuntimeDir "python-runtime"
$PlatformDir = Join-Path $RuntimeDir "windows"
$RepoRoot   = Resolve-Path (Join-Path $ScriptDir "..\..")
$Requirements = Join-Path $RepoRoot "requirements.txt"

# -- Platform Detection -----------------------------------------
$Arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$PlatformTag = "$Arch-pc-windows-msvc"
Write-Host "Target Platform: Windows/$Arch  ->  $PlatformTag"

# -- Download and Extract ---------------------------------------
# $CacheDir 必须在 if/else 外定义：即使 Python 已存在、跳过下载，
# 后面 bootstrap pip 下载 get-pip.py 时仍要用到它。否则走 "skipping download"
# 分支时 $CacheDir 为 null，导致 Join-Path 报 "Cannot bind argument to parameter 'Path'"。
$CacheDir = Join-Path $RuntimeDir ".cache"
if (-not (Test-Path $CacheDir)) { New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null }

$Marker = Join-Path $PlatformDir ".version"
$ExpectedMarker = "$PythonVersion-$PlatformTag"

if ((Test-Path $Marker) -and ((Get-Content $Marker) -eq $ExpectedMarker)) {
    Write-Host "Python $PythonVersion ($PlatformTag) already exists, skipping download."
} else {
    if (-not (Test-Path $PlatformDir)) { New-Item -ItemType Directory -Force -Path $PlatformDir | Out-Null }

    $Tarball = Join-Path $CacheDir "python-$PythonVersion-$PlatformTag.tar.gz"
    $Url = "https://github.com/astral-sh/python-build-standalone/releases/download/$ReleaseTag/cpython-$PythonVersion+$ReleaseTag-$PlatformTag-install_only.tar.gz"

    if (-not (Test-Path $Tarball)) {
        Write-Host "Downloading python-build-standalone..."
        Write-Host "  $Url"
        Invoke-WebRequest -Uri $Url -OutFile "$Tarball.tmp" -UseBasicParsing
        Move-Item -Force "$Tarball.tmp" $Tarball
    }

    Write-Host "Extracting to $PlatformDir\"
    $tmpDir = Join-Path $RuntimeDir "tmp_python"
    if (Test-Path $tmpDir) { Remove-Item -Recurse -Force $tmpDir }
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    tar -xzf $Tarball -C $tmpDir

    # python-build-standalone 解压后顶层目录名是 "python"。把它原样
    # 移到 $PlatformDir 下，**保留** "python/" 这一层（不要平铺）。
    # 这样 Windows 上最终路径是 src-tauri\python-runtime\windows\python\python.exe，
    # 跟 macOS 端 prepare-python-runtime.sh 的 mv python macos 行为一致，
    # 也跟 src-tauri\src\utils.rs:320-321 期望的
    #   python-runtime/<platform>/python/python.exe (windows)
    #   python-runtime/<platform>/python/bin/python3   (macos)
    # 完全对齐。
    $extracted = Get-ChildItem -Path $tmpDir -Directory | Select-Object -First 1
    if ($extracted) {
        $dest = Join-Path $PlatformDir "python"
        if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
        Move-Item -Path $extracted.FullName -Destination $dest
    }
    Remove-Item -Recurse -Force $tmpDir

    Set-Content -Path $Marker -Value $ExpectedMarker
}

# -- Install Dependencies ---------------------------------------
# After the extract step, the python-build-standalone tree lives at
#   $PlatformDir\python\python.exe
# (we keep the "python/" subdirectory so it matches src-tauri/src/utils.rs
# expectations and the macOS prepare-python-runtime.sh layout).
$PythonBin = Join-Path $PlatformDir "python\python.exe"
if (-not (Test-Path $PythonBin)) {
    Write-Error "Cannot find python.exe: $PythonBin"
    exit 1
}
if (-not (Test-Path $Requirements)) {
    Write-Error "Cannot find requirements.txt: $Requirements"
    exit 1
}

# python-build-standalone install_only tarball does NOT ship pip.
# Bootstrap pip using get-pip.py.
Write-Host "Bootstrapping pip (python-build-standalone install_only lacks pip)..."
$GetPip = Join-Path $CacheDir "get-pip.py"
if (-not (Test-Path $GetPip)) {
    Write-Host "  Downloading get-pip.py..."
    Invoke-WebRequest -Uri "https://bootstrap.pypa.io/get-pip.py" -OutFile $GetPip -UseBasicParsing
}

$PipArgs = @()
if ($env:PIP_INDEX_URL) {
    $PipArgs += "-i", $env:PIP_INDEX_URL
    Write-Host "Using mirror: $($env:PIP_INDEX_URL)"
}

& $PythonBin $GetPip @PipArgs --quiet

# Ensure numpy 1.x is installed for X86_V1 CPU compatibility (pyarrow 16.x requires it)
Write-Host "Installing numpy 1.x for X86_V1 CPU compatibility..."
& $PythonBin -m pip install "numpy==1.26.4" @PipArgs --no-cache-dir --force-reinstall --quiet

Write-Host "Installing dependencies (from ${Requirements})..."
$PipInstallArgs = @("-m", "pip", "install", "-r", $Requirements)
if ($env:PIP_INDEX_URL) {
    $PipInstallArgs += "-i", $env:PIP_INDEX_URL
}
# 关键：在 CI 环境下通过额外参数强制使用 CPU 版 torch 以减小体积
if ($env:GITHUB_ACTIONS -eq "true") {
    Write-Host "CI detected: Forcing CPU versions of torch/torchaudio to save space..."
    & $PythonBin -m pip install torch torchaudio --index-url https://download.pytorch.org/whl/cpu @PipArgs --no-cache-dir --quiet
}
& $PythonBin @PipInstallArgs --no-cache-dir --quiet

Write-Host "Aggressively cleaning up runtime to save space..."
# 删除所有的调试符号文件、静态库和头文件（运行时不需要）
Get-ChildItem -Path $PlatformDir -Recurse -File -Include "*.pdb", "*.lib", "*.a", "*.h", "*.cpp" | Remove-Item -Force -ErrorAction SilentlyContinue

# 删除不必要的文件夹
$UnneededDirs = @("__pycache__", "tests", "test", "Include", "share", "tcl", "tk", "idlelib", "ensurepip")
foreach ($dir in $UnneededDirs) {
    Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter $dir | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
}

# 删除 site-packages 里的测试文件夹
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Include "tests", "test" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

# -- Report Size ------------------------------------------------
$Size = (Get-ChildItem -Path $PlatformDir -Recurse | Measure-Object -Property Length -Sum).Sum
$SizeMB = [math]::Round($Size / 1MB, 1)
Write-Host ""
Write-Host "Python runtime prepared successfully."
Write-Host "   Path: $PlatformDir"
Write-Host "   Size: $SizeMB MB"
Write-Host ""
Write-Host "Next step: Run npm run tauri build"
