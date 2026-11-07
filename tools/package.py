#!/usr/bin/env python3
"""Build a reproducible Horizon native-mod source archive (.hnmod).

This script does NOT compile or test the native Rust code.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "mixins" / "civilizations"


def package() -> Path:
    compat = json.loads((ROOT / "compatibility.json").read_text(encoding="utf-8"))
    mixin = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text(encoding="utf-8"))
    mod = mixin["mod"]
    if mod["id"] != BUNDLE.name:
        raise ValueError("bundle directory and mod id differ")
    if mixin["compatibility"]["horizon_revision"] != compat["horizon_revision"]:
        raise ValueError("mod manifest and compatibility.json pin different Horizon revisions")
    entry = {
        "schema": 1,
        "id": mod["id"],
        "version": mod["version"],
        "game": "26.3",
        "horizon": compat["horizon_revision"],
        "mixins": compat["horizon_mixins_revision"],
    }
    files: dict[str, bytes] = {
        "horizon-native.json": (json.dumps(entry, indent=2, sort_keys=True) + "\n").encode("utf-8")
    }
    for source in sorted(BUNDLE.iterdir()):
        if source.is_file() and source.suffix in {".rs", ".toml"}:
            files[source.relative_to(ROOT).as_posix()] = source.read_bytes()
    for module in mixin["module"]:
        name = (BUNDLE / module["source"]).relative_to(ROOT).as_posix()
        if name not in files:
            raise ValueError(f"missing module source: {name}")
    for injection in mixin["injection"]:
        name = (BUNDLE / injection["helper"]).relative_to(ROOT).as_posix()
        if name not in files:
            raise ValueError(f"missing hook helper: {name}")
    target = ROOT / "target" / f"{mod['id']}-{mod['version']}.hnmod"
    target.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as z:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            z.writestr(info, data)
    print(json.dumps({
        "file": str(target),
        "bytes": target.stat().st_size,
        "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
        "validation": "packaging only; Rust compile and Horizon E2E not yet executed",
    }, indent=2))
    return target


if __name__ == "__main__":
    package()
