# AUTO-SPLIT LOADER: original source moved to audio_lab_parts/ in <=450-line text chunks.
# Keep this file as the public import/CLI entrypoint so existing imports and commands continue to work.
from pathlib import Path as _AutoSplitPath

_auto_split_dir = _AutoSplitPath(__file__).with_name('audio_lab_parts')
_auto_split_source = "".join(
    (_auto_split_dir / _name).read_text(encoding="utf-8")
    for _name in ['audio_lab.part01.txt', 'audio_lab.part02.txt']
)
exec(compile(_auto_split_source, __file__, "exec"), globals(), globals())
