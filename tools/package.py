"""Build the source-only Horizon native mod bundle; no game data included."""
import hashlib
import json
from pathlib import Path
import zipfile

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / "horizon-native.json").read_text())
bundle = root / "dist" / f"continuity-native-{manifest['version']}.hnmod"
bundle.parent.mkdir(exist_ok=True)
sources = [root / "horizon-native.json"]
sources += sorted((root / "mixins" / manifest["id"]).rglob("*"))
with zipfile.ZipFile(bundle, "w", zipfile.ZIP_DEFLATED) as archive:
    for path in sources:
        if path.is_file():
            info = zipfile.ZipInfo(path.relative_to(root).as_posix(), (2026, 10, 31, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, path.read_bytes())
print(bundle)
print("SHA-256:", hashlib.sha256(bundle.read_bytes()).hexdigest())
