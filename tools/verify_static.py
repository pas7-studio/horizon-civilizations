#!/usr/bin/env python3
"""Offline structural checks. Not a substitute for cargo check or game tests."""
from pathlib import Path
import json
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "mixins" / "civilizations"

sys.path.insert(0, str(ROOT / "tools"))
import mixin_sources  # noqa: E402

manifest = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text())
compat = json.loads((ROOT / "compatibility.json").read_text())
assert manifest["compatibility"]["horizon_revision"] == compat["horizon_revision"]
assert len(compat["horizon_revision"]) == 40
assert manifest["mod"]["id"] == "civilizations"
modules = [m["name"] for m in manifest["module"]]
assert len(modules) == len(set(modules))
for entry in manifest["module"]:
    assert entry["file"] == "crates/horizon-client/src/game.rs"
    assert (BUNDLE / entry["source"]).is_file()
for entry in manifest["injection"]:
    assert entry["kind"] == "HEAD"
    assert "native_" in entry["target"]
    assert (BUNDLE / entry["helper"]).is_file()
assert len(manifest["injection"]) == 11
cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
version = cargo["package"]["version"]
assert manifest["mod"]["version"] == version
assert (ROOT / cargo["lib"]["path"]).is_file()
core = mixin_sources.payload_text(BUNDLE, "core")
glue = mixin_sources.payload_text(BUNDLE, "glue")
for label in ["pub fn found", "pub fn advance", "pub fn buy", "pub fn sell", "pub fn declare_war", "pub fn make_peace", "pub fn validate", "pub fn workplace", "pub fn actor_name", "pub fn hut_plan", "pub fn start_visual_house", "pub fn finish_visual_house"]:
    assert label in core, label
for label in ["native_spawn_mob", "native_world_dir", "native_feedback", "native_take_items", "native_give_items", "native_mob_navigate", "loaded_actors", "guide_actors", "build_one_step", "check_empty_hut_site", "native_run_command", "level_tick", "interact_entity", "native_entity_uuid", "native_tell"]:
    assert label in glue, label
assert "Millenaire" not in core
bundle_path = ROOT / "target" / f"civilizations-{version}.hnmod"
if bundle_path.exists():
    with zipfile.ZipFile(bundle_path) as z:
        assert z.testzip() is None
        names = set(z.namelist())
        assert "horizon-native.json" in names
        for entry in manifest["module"]:
            name = (BUNDLE / entry["source"]).relative_to(ROOT).as_posix()
            assert name in names, name
            payload = z.read(name).decode("utf-8")
            assert not any(mixin_sources.MOD_DECL.match(line.rstrip())
                           for line in payload.splitlines()), name
print("Static structure: PASS (Rust compilation and Horizon gameplay: NOT TESTED)")
