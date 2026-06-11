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

# 强制 Python 在 Windows 上使用 UTF-8 读取文件，防止 requirements.txt 中的中文注释导致 UnicodeDecodeError
$env:PYTHONUTF8 = "1"

Write-Host "Installing dependencies from ${Requirements}..."
# 注意：STT 已从 funasr+torch 切到 sherpa-onnx（纯 ONNX，无 torch），不再安装 torch，
# 避免内置 Python 体积过大导致 Windows NSIS 安装包 32 位偏移溢出而打包失败。
& $PythonBin -m pip install -r $Requirements @PipArgs

Write-Host "Aggressively cleaning up runtime to save space..."
# 1. 深度清理 __pycache__ (无论在哪里，一律干掉)
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter "__pycache__" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

# 2. 删除所有的调试符号文件、静态库、头文件和源码
Get-ChildItem -Path $PlatformDir -Recurse -File -Include "*.pdb", "*.lib", "*.a", "*.h", "*.cpp", "*.c", "*.pyi", "*.pxd" | Remove-Item -Force -ErrorAction SilentlyContinue

# 3. 删除标准库和 site-packages 中无用的文件夹
$UnneededDirs = @("tests", "test", "Include", "share", "tcl", "tk", "idlelib", "ensurepip", "doc", "docs", "examples", "example", "tutorials")
foreach ($dir in $UnneededDirs) {
    Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter $dir | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
}

# 5. 清理 llvmlite 和 scipy 等体积巨大的额外二进制（如果存在开发版本遗留）
Get-ChildItem -Path $PlatformDir -Recurse -Directory -Filter "llvmlite" | ForEach-Object {
    Get-ChildItem -Path $_.FullName -Recurse -File -Include "*.lib", "*.a", "*.h" | Remove-Item -Force -ErrorAction SilentlyContinue
}

# 6. 体积清理：删未使用 / 非必需的包与目录（NSIS 2GB mmap 上限保护）
#    每条都经过验证：项目代码（scripts/ + src-tauri/src/）中未 import 这些包。
#    删除这些包后 site-packages 通常可减 200~400MB。
$SitePackages = Join-Path $PlatformDir "python\Lib\site-packages"

# 6.1 整个未使用的包 —— 这些包在项目里 grep 不到任何 import
$UnusedPackages = @(
    "jieba",        # 37MB  中文分词；项目未使用
    "sklearn",      # 24MB  scikit-learn；项目未使用（pandas/numpy 不依赖它）
    "sympy",        # 18MB  数学符号库；项目未使用
    "numba",        # 7.7MB JIT 编译器；项目未使用
    "hf_xet",       # 7.3MB huggingface 内部存储后端；huggingface_hub 默认 fallback，不影响功能
    "pygments"      # 4.9MB 代码高亮；项目未使用
    # 注意：不删 pip —— stt_helper.py 在首次使用 STT 时会调用
    #   `python -m pip install modelscope` 作 fallback（脚本中已实现），
    # 删了会导致 Windows 用户首次 STT 时 pip 不可用。
)
foreach ($pkg in $UnusedPackages) {
    $pkgDir = Join-Path $SitePackages $pkg
    if (Test-Path $pkgDir) {
        $sz = (Get-ChildItem -Path $pkgDir -Recurse -File | Measure-Object -Property Length -Sum).Sum
        Remove-Item -Recurse -Force $pkgDir -ErrorAction SilentlyContinue
        Write-Host "  - Removed unused package: $pkg  ($([math]::Round($sz/1MB,1)) MB)"
    }
}

# 6.2 transformers/models/ —— 41MB 的 481 个模型定义目录
#     运行时按需 lazy import，用户只用到 SenseVoice（走 funasr 而非 transformers 直接加载）
$TxModelsDir = Join-Path $SitePackages "transformers\models"
if (Test-Path $TxModelsDir) {
    $sz = (Get-ChildItem -Path $TxModelsDir -Recurse -File | Measure-Object -Property Length -Sum).Sum
    Remove-Item -Recurse -Force $TxModelsDir -ErrorAction SilentlyContinue
    Write-Host "  - Removed transformers\models\  ($([math]::Round($sz/1MB,1)) MB)"
}

# 6.3 pyarrow C++ 头文件 —— 5.3MB .h / 716KB src/ ，运行时不需要
$PyArrowInclude = Join-Path $SitePackages "pyarrow\include"
$PyArrowSrc     = Join-Path $SitePackages "pyarrow\src"
foreach ($d in @($PyArrowInclude, $PyArrowSrc)) {
    if (Test-Path $d) {
        $sz = (Get-ChildItem -Path $d -Recurse -File | Measure-Object -Property Length -Sum).Sum
        Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
        Write-Host "  - Removed $d  ($([math]::Round($sz/1MB,1)) MB)"
    }
}

# 6.4 jieba/lac_small —— 12MB LAC 序列标注模型，项目未使用 jieba
# （6.1 已删整个 jieba，保留此条仅作历史兼容性，实际不会执行）

# 6.5 清理第三方包里的 tests/、examples/、docs/、bin/、__pycache__/、*.dist-info/RECORD
#     （之前的 #1/#3 步已处理全 site-packages，再扫一遍确保 transformers/models 等目录被删前的 cleanup 干净）
$DeepUnneededDirs = @(
    "tests", "test", "testing", "tests-*",
    "examples", "example", "extras", "tutorials", "demo", "demos",
    "docs", "doc", "documentation",
    "bin",          # 多数 Python 包的 CLI 入口目录，运行时不调用
    "__pycache__", "*.egg-info"
)
foreach ($pattern in $DeepUnneededDirs) {
    Get-ChildItem -Path $SitePackages -Recurse -Directory -Filter $pattern -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch 'transformers[\\/]models' } |  # 已被 6.2 删
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
}

# 6.6 删除 *.dist-info/ 下的 RECORD 文件（pip 元数据，安装完已无用处；不删整个目录以免破坏 importlib.metadata）
#     RECORD 文件通常 1-10KB，但 transformers/models/ 大量包各自带一份累计也不小
#     限定在 *.dist-info 目录下避免误伤同名文件（极少见但稳妥起见）
#     注意：必须保留 METADATA / WHEEL —— importlib.metadata.version() 依赖 METADATA，
#     很多包（openai 等）import 时会查自身版本，删了会导致装好的 app 一启动就崩。
Get-ChildItem -Path $SitePackages -Directory -Filter "*.dist-info" -ErrorAction SilentlyContinue | ForEach-Object {
    $distInfo = $_.FullName
    foreach ($f in @("RECORD", "INSTALLER")) {
        $fp = Join-Path $distInfo $f
        if (Test-Path $fp) { Remove-Item -Force $fp -ErrorAction SilentlyContinue }
    }
}

# -- Report Size ------------------------------------------------
$Size = (Get-ChildItem -Path $PlatformDir -Recurse | Measure-Object -Property Length -Sum).Sum
$SizeMB = [math]::Round($Size / 1MB, 1)
Write-Host ""
Write-Host "Python runtime prepared successfully."
Write-Host "   Path: $PlatformDir"
Write-Host "   Size: $SizeMB MB"
Write-Host ""
Write-Host "Next step: Run npm run tauri build"
