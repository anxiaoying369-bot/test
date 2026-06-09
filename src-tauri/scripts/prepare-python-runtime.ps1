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
$PythonBin = Join-Path $PlatformDir "python\python.exe"
$PipArgs = @("--no-cache-dir", "--quiet")

# 在 CI 环境下，强制安装 CPU 版 torch，并确保不被后续安装覆盖
if ($env:GITHUB_ACTIONS -eq "true") {
    Write-Host "CI detected: Installing CPU-only torch..."
    & $PythonBin -m pip install torch==2.4.1+cpu torchaudio==2.4.1+cpu --index-url https://download.pytorch.org/whl/cpu @PipArgs
}

Write-Host "Installing other dependencies from ${Requirements}..."
# 使用 --extra-index-url 确保 pip 能找到 CPU 版 torch 的依赖，同时不触发 GPU 版升级
& $PythonBin -m pip install -r $Requirements --extra-index-url https://download.pytorch.org/whl/cpu @PipArgs

Write-Host "Aggressively cleaning up runtime to save space..."
# 删除所有的调试符号文件、静态库、头文件和源码
Get-ChildItem -Path $PlatformDir -Recurse -File -Include "*.pdb", "*.lib", "*.a", "*.h", "*.cpp", "*.c", "*.pyi" | Remove-Item -Force -ErrorAction SilentlyContinue

# 删除不必要的文件夹
$UnneededDirs = @("__pycache__", "tests", "test", "Include", "share", "tcl", "tk", "idlelib", "ensurepip", "doc", "docs")
foreach ($dir in $UnneededDirs) {
    Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter $dir | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
}

# 深度清理 torch（非常重要）
$TorchDir = Join-Path $PlatformDir "python\Lib\site-packages\torch"
if (Test-Path $TorchDir) {
    Write-Host "Cleaning up torch internals..."
    $TorchUnneeded = @("test", "bin", "include", "lib\*.lib")
    foreach ($sub in $TorchUnneeded) {
        $subPath = Join-Path $TorchDir $sub
        if (Test-Path $subPath) { Remove-Item -Recurse -Force $subPath -ErrorAction SilentlyContinue }
    }
}

# 删除 site-packages 里的其它测试文件夹和无用数据
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Include "tests", "test", "data" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

# -- Report Size ------------------------------------------------
$Size = (Get-ChildItem -Path $PlatformDir -Recurse | Measure-Object -Property Length -Sum).Sum
$SizeMB = [math]::Round($Size / 1MB, 1)
Write-Host ""
Write-Host "Python runtime prepared successfully."
Write-Host "   Path: $PlatformDir"
Write-Host "   Size: $SizeMB MB"
Write-Host ""
Write-Host "Next step: Run npm run tauri build"
