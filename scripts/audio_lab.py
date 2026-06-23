#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AutoCast AI 本地音频能力助手。

目标：把「语音识别」和「语音克隆」做成可本地部署、按需下载模型的能力，
避免把大模型塞进安装包。

ASR：复用 wechat/stt_helper.py 的 sherpa-onnx + SenseVoice int8 ONNX 模型。
TTS/克隆：优先使用本地已安装的 f5-tts 命令行；未安装时给出明确安装提示。
        F5-TTS 模型由其自身缓存到用户目录/HF 缓存，不进入安装包。

命令协议：单次 CLI 调用，stdout 只输出 JSON，stderr 打日志/进度。
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import wave
from pathlib import Path
from typing import Any, Dict, List, Optional


def _data_dir() -> Path:
    env = os.environ.get("AUTOCAST_DATA_DIR")
    if env:
        return Path(env)
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "AutoCastAI"
    if os.name == "nt":
        return Path(os.environ.get("APPDATA", str(Path.home()))) / "AutoCastAI"
    return Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local" / "share"))) / "AutoCastAI"


def _json_out(payload: Dict[str, Any]) -> None:
    print(json.dumps(payload, ensure_ascii=False), flush=True)


def _json_err(message: str, **extra: Any) -> None:
    payload = {"type": "audio_lab_log", "message": message, **extra}
    print(json.dumps(payload, ensure_ascii=False), file=sys.stderr, flush=True)


def _run_streaming(cmd: list[str], progress: int, stage: str, timeout: int = 3600) -> None:
    _json_err("执行安装命令", command=" ".join(cmd), progress=progress, stage=stage)
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    tail: list[str] = []
    started = time.time()
    assert proc.stdout is not None
    for line in proc.stdout:
        line = line.rstrip()
        if not line:
            continue
        tail.append(line)
        if len(tail) > 80:
            tail.pop(0)
        lower = line.lower()
        guessed = progress
        if "collecting" in lower or "looking in indexes" in lower:
            guessed = max(progress, 35)
        elif "downloading" in lower:
            guessed = max(progress, 45)
        elif "installing collected packages" in lower:
            guessed = max(progress, 70)
        elif "successfully installed" in lower:
            guessed = max(progress, 90)
        _json_err(line, progress=guessed, stage=stage)
        if time.time() - started > timeout:
            proc.kill()
            raise RuntimeError(f"安装命令超时: {' '.join(cmd)}")
    rc = proc.wait(timeout=30)
    if rc != 0:
        tail_text = "\n".join(tail[-20:])
        raise RuntimeError(f"安装命令失败(exit={rc}): {tail_text}")


def _safe_name(name: str) -> str:
    keep = []
    for ch in name.strip():
        if ch.isalnum() or ch in ("-", "_", "."):
            keep.append(ch)
        elif ch.isspace():
            keep.append("_")
    s = "".join(keep).strip("._")
    return s[:80] or f"voice_{int(time.time())}"


def _audio_duration(path: Path) -> Optional[float]:
    if path.suffix.lower() == ".wav":
        try:
            with wave.open(str(path), "rb") as w:
                fr = w.getframerate() or 1
                return round(w.getnframes() / fr, 3)
        except Exception:
            return None
    return None


def _writable_dir(path: Path) -> bool:
    try:
        path.mkdir(parents=True, exist_ok=True)
        probe = path / f".autocast_write_test_{os.getpid()}"
        probe.write_text("ok", encoding="utf-8")
        probe.unlink(missing_ok=True)
        return True
    except Exception:
        return False


def _cache_dir(name: str) -> Path:
    preferred = _data_dir() / "voice_lab" / "cache" / name
    if _writable_dir(preferred):
        return preferred
    fallback = Path(tempfile.gettempdir()) / "autocast_voice_lab_cache" / name
    fallback.mkdir(parents=True, exist_ok=True)
    return fallback


def _find_ffmpeg() -> str:
    env = os.environ.get("IMAGEIO_FFMPEG_EXE") or os.environ.get("WECHAT_FFMPEG")
    if env:
        return env
    found = shutil.which("ffmpeg")
    if found:
        return found
    try:
        import importlib
        imageio_ffmpeg = importlib.import_module("imageio_ffmpeg")
        bundled = imageio_ffmpeg.get_ffmpeg_exe()
        if bundled:
            return bundled
    except Exception:
        pass
    return "ffmpeg"


def _convert_audio_to_wav(source: Path) -> Path:
    """把任意常见音频转成 STT 可读取的 16k/mono/16-bit WAV。"""
    ffmpeg = _find_ffmpeg()
    tmp = tempfile.NamedTemporaryFile(prefix="autocast_asr_", suffix=".wav", delete=False)
    tmp_path = Path(tmp.name)
    tmp.close()
    cmd = [
        ffmpeg,
        "-hide_banner",
        "-loglevel", "error",
        "-y",
        "-i", str(source),
        "-vn",
        "-ac", "1",
        "-ar", "16000",
        "-sample_fmt", "s16",
        str(tmp_path),
    ]
    proc = subprocess.run(cmd, text=True, capture_output=True, timeout=600)
    if proc.returncode != 0 or not tmp_path.exists() or tmp_path.stat().st_size == 0:
        try:
            tmp_path.unlink(missing_ok=True)
        except Exception:
            pass
        detail = (proc.stderr or proc.stdout or "").strip()
        raise RuntimeError(f"音频转码失败，请确认文件是有效音频且 FFmpeg 可用: {detail[-800:]}")
    return tmp_path


def cmd_asr_check(_: argparse.Namespace) -> None:
    from wechat.stt_helper import STTHelper

    helper = STTHelper.get_instance()
    _json_out({
        "ok": True,
        "ready": helper.is_model_ready(),
        "model_dir": helper.get_model_dir(),
        "engine": "sherpa-onnx-sensevoice-int8",
    })


def cmd_asr_download(_: argparse.Namespace) -> None:
    from wechat.stt_helper import STTHelper

    helper = STTHelper.get_instance()
    ok = helper.download_model()
    _json_out({
        "ok": bool(ok),
        "ready": helper.is_model_ready(),
        "model_dir": helper.get_model_dir(),
        "engine": "sherpa-onnx-sensevoice-int8",
    })


def cmd_asr_transcribe(args: argparse.Namespace) -> None:
    from wechat.stt_helper import STTHelper

    audio = Path(args.audio).expanduser().resolve()
    if not audio.exists():
        raise FileNotFoundError(f"音频文件不存在: {audio}")

    helper = STTHelper.get_instance()
    if not helper.is_model_ready():
        raise RuntimeError("ASR 模型未就绪，请先下载 SenseVoice 模型")

    wav_path: Optional[Path] = None
    cleanup_wav = False
    try:
        if audio.suffix.lower() == ".wav":
            try:
                with wave.open(str(audio), "rb"):
                    pass
                wav_path = audio
            except wave.Error:
                # 扩展名是 .wav 但内容不是 RIFF/WAVE（常见于 m4a/mp3 改后缀或容器 WAV），走 FFmpeg 兜底。
                wav_path = _convert_audio_to_wav(audio)
                cleanup_wav = True
        else:
            wav_path = _convert_audio_to_wav(audio)
            cleanup_wav = True

        text = helper.transcribe(str(wav_path))
        _json_out({
            "ok": True,
            "text": text,
            "audio_path": str(audio),
            "normalized_audio_path": str(wav_path),
            "duration": _audio_duration(wav_path),
            "engine": "sherpa-onnx-sensevoice-int8",
        })
    finally:
        if cleanup_wav and wav_path:
            try:
                wav_path.unlink(missing_ok=True)
            except Exception:
                pass


def _f5_venv_bin_dir() -> Path:
    venv = _data_dir() / "voice_lab" / "f5_tts_venv"
    if os.name == "nt":
        return venv / "Scripts"
    return venv / "bin"


def _find_f5_cli() -> Optional[str]:
    env = os.environ.get("AUTOCAST_F5_TTS_BIN")
    if env and Path(env).exists():
        return env
    env_path = os.environ.get("AUTOCAST_F5_TTS_BIN")
    if env_path and shutil.which(env_path):
        return env_path

    local_bin = _f5_venv_bin_dir()
    for name in ("f5-tts_infer-cli", "f5-tts_infer_cli", "f5-tts"):
        candidate = local_bin / (name + (".exe" if os.name == "nt" else ""))
        if candidate.exists():
            return str(candidate)

    for name in ("f5-tts_infer-cli", "f5-tts_infer_cli", "f5-tts"):
        p = shutil.which(name)
        if p:
            return p
    return None


def _run_f5_once(
    cli: str,
    reference_audio: Path,
    reference_text: str,
    gen_text: str,
    output_path: Path,
    device_name: Optional[str],
) -> List[str]:
    cmd = [
        cli,
        "--ref_audio", str(reference_audio),
        "--ref_text", reference_text,
        "--gen_text", gen_text,
        "--output_dir", str(output_path.parent),
        "--output_file", output_path.name,
    ]
    if device_name:
        cmd.extend(["--device", device_name])
    _json_err(
        "启动 F5-TTS 推理" if not device_name else f"启动 F5-TTS 推理（{device_name}）",
        command=" ".join(cmd),
        progress=25,
        stage="infer",
    )
    numba_cache = _cache_dir("numba")
    mpl_cache = _cache_dir("matplotlib")
    env = os.environ.copy()
    env.setdefault("NUMBA_CACHE_DIR", str(numba_cache))
    env.setdefault("MPLCONFIGDIR", str(mpl_cache))
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
        env=env,
    )
    tail: list[str] = []
    started = time.time()
    assert proc.stdout is not None
    for line in proc.stdout:
        line = line.rstrip()
        if not line:
            continue
        tail.append(line)
        if len(tail) > 80:
            tail.pop(0)
        lower = line.lower()
        guessed = 45
        if "100%" in line:
            guessed = 88
        elif "download" in lower or "huggingface" in lower:
            guessed = 50
        elif "load" in lower or "vocos" in lower:
            guessed = 60
        elif "infer" in lower or "sample" in lower or "generate" in lower:
            guessed = 75
        _json_err(line, progress=guessed, stage="infer")
        if time.time() - started > 1800:
            proc.kill()
            raise RuntimeError("F5-TTS 推理超时")

    rc = proc.wait(timeout=30)
    if rc != 0:
        tail_text = "\n".join(tail[-30:])
        raise RuntimeError(f"F5-TTS 推理失败(exit={rc}): {tail_text[-2000:]}")
    return tail


def _run_f5(cli: str, reference_audio: Path, reference_text: str, gen_text: str, output_path: Path) -> List[str]:
    """兼容 F5-TTS 常见 CLI 参数。失败时返回 stderr 给上层诊断。"""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    configured_device = os.environ.get("AUTOCAST_F5_TTS_DEVICE", "").strip() or None
    try:
        tail = _run_f5_once(cli, reference_audio, reference_text, gen_text, output_path, configured_device)
    except RuntimeError as e:
        err = str(e)
        should_retry_cpu = (
            configured_device is None
            and ("exit=-11" in err or "SIGSEGV" in err or "segmentation" in err.lower())
        )
        if not should_retry_cpu:
            raise
        try:
            output_path.unlink(missing_ok=True)
        except Exception:
            pass
        _json_err("F5-TTS 默认设备推理崩溃，自动切换 CPU 重试", progress=35, stage="infer")
        tail = _run_f5_once(cli, reference_audio, reference_text, gen_text, output_path, "cpu")

    _json_err("检查合成音频文件", progress=90, stage="verify")
    if not output_path.exists() or output_path.stat().st_size == 0:
        # 有些版本会忽略 output_file，兜底找最新 wav/mp3。
        candidates = sorted(
            [p for p in output_path.parent.glob("*") if p.suffix.lower() in {".wav", ".mp3", ".flac"}],
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        if candidates:
            candidates[0].replace(output_path)
        else:
            raise RuntimeError("F5-TTS 未生成音频文件")
    _json_err("F5-TTS 推理完成", progress=96, stage="done")
    return ["\n".join(tail[-30:]), ""]


def cmd_clone_check(_: argparse.Namespace) -> None:
    cli = _find_f5_cli()
    _json_out({
        "ok": True,
        "ready": bool(cli),
        "engine": "f5-tts",
        "cli": cli,
        "venv_dir": str(_data_dir() / "voice_lab" / "f5_tts_venv"),
        "install_hint": None if cli else "未检测到 F5-TTS。可在系统设置 → 系统组件中下载/安装，或设置 AUTOCAST_F5_TTS_BIN 指向 f5-tts_infer-cli。",
    })


def cmd_clone_install(_: argparse.Namespace) -> None:
    existing = _find_f5_cli()
    if existing:
        _json_out({
            "ok": True,
            "ready": True,
            "engine": "f5-tts",
            "cli": existing,
            "message": "F5-TTS 已安装",
        })
        return

    venv = _data_dir() / "voice_lab" / "f5_tts_venv"
    venv.parent.mkdir(parents=True, exist_ok=True)
    if not venv.exists():
        _json_err("创建 F5-TTS 独立 Python venv", venv=str(venv), progress=8, stage="venv")
        subprocess.run([sys.executable, "-m", "venv", str(venv)], check=True, timeout=300)
        _json_err("F5-TTS 独立 Python venv 创建完成", venv=str(venv), progress=18, stage="venv")
    else:
        _json_err("复用已存在的 F5-TTS 独立 Python venv", venv=str(venv), progress=18, stage="venv")

    python = _f5_venv_bin_dir() / ("python.exe" if os.name == "nt" else "python")
    if not python.exists():
        raise RuntimeError(f"F5-TTS venv Python 缺失: {python}")

    _run_streaming([str(python), "-m", "pip", "install", "--upgrade", "pip", "setuptools", "wheel"], progress=22, stage="pip", timeout=1800)
    _run_streaming([str(python), "-m", "pip", "install", "f5-tts"], progress=35, stage="f5-tts", timeout=7200)

    _json_err("检查 F5-TTS CLI", progress=94, stage="check")
    cli = _find_f5_cli()
    if not cli:
        raise RuntimeError("F5-TTS 安装完成但未找到 f5-tts_infer-cli，请检查 pip 安装日志")
    _json_out({
        "ok": True,
        "ready": True,
        "engine": "f5-tts",
        "cli": cli,
        "venv_dir": str(venv),
    })


def cmd_clone_register(args: argparse.Namespace) -> None:
    source = Path(args.audio).expanduser().resolve()
    if not source.exists():
        raise FileNotFoundError(f"参考音频不存在: {source}")
    name = _safe_name(args.name)
    voice_dir = _data_dir() / "voice_lab" / "voices" / name
    voice_dir.mkdir(parents=True, exist_ok=True)
    dest = voice_dir / ("reference" + source.suffix.lower())
    shutil.copy2(source, dest)
    meta = {
        "name": name,
        "display_name": args.name.strip() or name,
        "reference_audio": str(dest),
        "reference_text": args.text.strip(),
        "created_at": int(time.time()),
        "engine": "f5-tts",
    }
    (voice_dir / "voice.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8")
    _json_out({"ok": True, "voice": meta})


def _load_voice(name: str) -> Dict[str, Any]:
    voice_dir = _data_dir() / "voice_lab" / "voices" / _safe_name(name)
    meta_path = voice_dir / "voice.json"
    if not meta_path.exists():
        raise FileNotFoundError(f"未找到音色: {name}")
    return json.loads(meta_path.read_text(encoding="utf-8"))


def cmd_clone_list(_: argparse.Namespace) -> None:
    root = _data_dir() / "voice_lab" / "voices"
    voices = []
    if root.exists():
        for meta_path in sorted(root.glob("*/voice.json")):
            try:
                meta = json.loads(meta_path.read_text(encoding="utf-8"))
                voices.append(meta)
            except Exception as e:
                _json_err(f"跳过损坏音色配置: {meta_path}: {e}")
    _json_out({"ok": True, "voices": voices})


def cmd_clone_synthesize(args: argparse.Namespace) -> None:
    _json_err("检查 F5-TTS 环境", progress=5, stage="check")
    cli = _find_f5_cli()
    if not cli:
        raise RuntimeError("未检测到 F5-TTS。请执行 pip install f5-tts，或设置 AUTOCAST_F5_TTS_BIN。")
    _json_err("读取本地音色", progress=12, stage="voice")
    meta = _load_voice(args.voice)
    ref_audio = Path(meta["reference_audio"])
    if not ref_audio.exists():
        raise FileNotFoundError(f"参考音频缺失: {ref_audio}")
    ref_text = (args.reference_text or meta.get("reference_text") or "").strip()
    if not ref_text:
        raise RuntimeError("参考音频文字不能为空；注册音色时请填写参考音频对应文本")
    text = args.text.strip()
    if not text:
        raise RuntimeError("合成文本不能为空")

    _json_err("准备输出文件", progress=18, stage="prepare")
    if args.output:
        out = Path(args.output).expanduser().resolve()
    else:
        filename = f"{_safe_name(args.voice)}_{int(time.time())}.wav"
        output_dir = _data_dir() / "voice_lab" / "outputs"
        if not _writable_dir(output_dir):
            output_dir = Path(tempfile.gettempdir()) / "autocast_voice_lab_outputs"
            output_dir.mkdir(parents=True, exist_ok=True)
            _json_err("默认输出目录不可写，已切换到临时目录", output_dir=str(output_dir), progress=20, stage="prepare")
        out = output_dir / filename
    _run_f5(cli, ref_audio, ref_text, text, out)
    _json_out({
        "ok": True,
        "audio_path": str(out),
        "voice": meta,
        "engine": "f5-tts",
    })


def main() -> int:
    parser = argparse.ArgumentParser(description="AutoCast AI local audio lab helper")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("asr-check").set_defaults(func=cmd_asr_check)
    sub.add_parser("asr-download").set_defaults(func=cmd_asr_download)
    p = sub.add_parser("asr-transcribe")
    p.add_argument("--audio", required=True)
    p.set_defaults(func=cmd_asr_transcribe)

    sub.add_parser("clone-check").set_defaults(func=cmd_clone_check)
    sub.add_parser("clone-install").set_defaults(func=cmd_clone_install)
    p = sub.add_parser("clone-register")
    p.add_argument("--name", required=True)
    p.add_argument("--audio", required=True)
    p.add_argument("--text", required=True)
    p.set_defaults(func=cmd_clone_register)

    sub.add_parser("clone-list").set_defaults(func=cmd_clone_list)
    p = sub.add_parser("clone-synthesize")
    p.add_argument("--voice", required=True)
    p.add_argument("--text", required=True)
    p.add_argument("--reference-text", default="")
    p.add_argument("--output", default="")
    p.set_defaults(func=cmd_clone_synthesize)

    args = parser.parse_args()
    try:
        args.func(args)
        return 0
    except Exception as e:
        _json_err(str(e), error=True)
        _json_out({"ok": False, "error": str(e)})
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
