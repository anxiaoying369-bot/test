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
$Marker = Join-Path $PlatformDir ".version"
$ExpectedMarker = "$PythonVersion-$PlatformTag"

if ((Test-Path $Marker) -and ((Get-Content $Marker) -eq $ExpectedMarker)) {
    Write-Host "Python $PythonVersion ($PlatformTag) already exists, skipping download."
} else {
    $CacheDir = Join-Path $RuntimeDir ".cache"
    if (-not (Test-Path $CacheDir)) { New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null }
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

    $extracted = Get-ChildItem -Path $tmpDir -Directory | Select-Object -First 1
    if ($extracted) {
        Copy-Item -Path (Join-Path $extracted.FullName "*") -Destination $PlatformDir -Recurse -Force
        Remove-Item -Recurse -Force $tmpDir
    }

    Set-Content -Path $Marker -Value $ExpectedMarker
}

# -- Install Dependencies ---------------------------------------
$PythonBin = Join-Path $PlatformDir "python.exe"
if (-not (Test-Path $PythonBin)) {
    Write-Error "Cannot find python.exe: $PythonBin"
    exit 1
}
if (-not (Test-Path $Requirements)) {
    Write-Error "Cannot find requirements.txt: $Requirements"
    exit 1
}

Write-Host "Upgrading pip..."
& $PythonBin -m pip install --upgrade pip --quiet

# Ensure numpy 2.x is NOT present (it breaks compatibility on older CPUs)
Write-Host "Checking for incompatible numpy versions..."
& $PythonBin -m pip uninstall numpy -y --quiet

Write-Host "Installing dependencies (from ${Requirements})..."
& $PythonBin -m pip install -r $Requirements --no-cache-dir --force-reinstall

Write-Host "Cleaning __pycache__ and .pyc..."
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter "__pycache__" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path $PlatformDir -Recurse -File -Filter "*.pyc" | Remove-Item -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter "tests" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter "test"  | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

# -- Report Size ------------------------------------------------
$Size = (Get-ChildItem -Path $PlatformDir -Recurse | Measure-Object -Property Length -Sum).Sum
$SizeMB = [math]::Round($Size / 1MB, 1)
Write-Host ""
Write-Host "Python runtime prepared successfully."
Write-Host "   Path: $PlatformDir"
Write-Host "   Size: $SizeMB MB"
Write-Host ""
Write-Host "Next step: Run npm run tauri build"
