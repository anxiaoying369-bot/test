# AUTO-SPLIT LOADER: original source moved to wechat_engine_parts/ in <=450-line text chunks.
# Keep this file as the public import/CLI entrypoint so existing imports and commands continue to work.
from pathlib import Path as _AutoSplitPath

_auto_split_dir = _AutoSplitPath(__file__).with_name('wechat_engine_parts')
_auto_split_source = "".join(
    (_auto_split_dir / _name).read_text(encoding="utf-8")
    for _name in ['wechat_engine.part01.txt', 'wechat_engine.part02.txt', 'wechat_engine.part03.txt']
)
exec(compile(_auto_split_source, __file__, "exec"), globals(), globals())
