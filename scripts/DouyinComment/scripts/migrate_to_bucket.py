# AUTO-SPLIT LOADER: original source moved to migrate_to_bucket_parts/ in <=450-line text chunks.
# Keep this file as the public import/CLI entrypoint so existing imports and commands continue to work.
from pathlib import Path as _AutoSplitPath

_auto_split_dir = _AutoSplitPath(__file__).with_name('migrate_to_bucket_parts')
_auto_split_source = "".join(
    (_auto_split_dir / _name).read_text(encoding="utf-8")
    for _name in ['migrate_to_bucket.part01.txt', 'migrate_to_bucket.part02.txt', 'migrate_to_bucket.part03.txt', 'migrate_to_bucket.part04.txt']
)
exec(compile(_auto_split_source, __file__, "exec"), globals(), globals())
