"""Structural guard: the mixin sources stay a module tree, not one giant file.

A single 300 KB `core.rs` (plus a 180 KB `glue.rs`) made parallel work
unmergeable. These checks keep the split from silently collapsing back, and
prove that the payload the Horizon mixin tool receives is still self-contained.
"""
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest

from civil_sources import BUNDLE, ROOT

import mixin_sources

MAX_MODULE_LINES = 700
MAX_ROOT_LINES = 120


def lines(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


class ModuleLayoutContracts(unittest.TestCase):
    def test_cargo_library_is_the_module_root(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertEqual("mixins/civilizations/core/mod.rs", cargo["lib"]["path"])
        self.assertTrue((ROOT / cargo["lib"]["path"]).is_file())

    def test_every_module_declaration_resolves_and_flattens(self):
        root = BUNDLE / "core" / "mod.rs"
        directory = mixin_sources.children_dir(root)
        for line in root.read_text(encoding="utf-8").splitlines():
            match = mixin_sources.MOD_DECL.match(line.rstrip())
            if not match:
                continue
            name = match.group("name")
            self.assertTrue((directory / f"{name}.rs").is_file()
                            or (directory / name / "mod.rs").is_file(), name)
        for module in ("core", "glue"):
            payload = mixin_sources.payload_text(BUNDLE, module)
            leftover = [line for line in payload.splitlines()
                        if mixin_sources.MOD_DECL.match(line.rstrip())]
            self.assertEqual([], leftover, module)

    def test_no_source_file_is_an_unreviewable_monolith(self):
        oversized = {
            path.relative_to(BUNDLE).as_posix(): lines(path)
            for path in (BUNDLE / "core").rglob("*.rs") if lines(path) > MAX_MODULE_LINES
        }
        self.assertEqual({}, oversized)
        self.assertLessEqual(lines(BUNDLE / "core" / "mod.rs"), MAX_ROOT_LINES)
        self.assertLessEqual(lines(BUNDLE / "glue" / "mod.rs"), MAX_ROOT_LINES)
        oversized_glue = {
            path.name: lines(path) for path in (BUNDLE / "glue").glob("*.rs")
            if lines(path) > MAX_MODULE_LINES
        }
        self.assertEqual({}, oversized_glue)

    def test_world_impl_is_split_per_domain(self):
        world = BUNDLE / "core" / "world"
        impl_files = sorted(p.stem for p in world.glob("*.rs") if p.stem != "mod")
        self.assertGreaterEqual(len(impl_files), 8)
        self.assertIn("state", impl_files)
        self.assertIn("gather", impl_files)

    def test_staged_payload_parses_as_rust(self):
        """The planner only reports `cannot parse string into token stream`.

        A missing brace from a bad module move is caught here with a line
        number instead of a full native build.
        """
        rustfmt = shutil.which("rustfmt")
        if not rustfmt:
            self.skipTest("rustfmt not installed")
        with tempfile.TemporaryDirectory(prefix="civ-syntax-") as work:
            mods = mixin_sources.stage(BUNDLE, Path(work))
            staged = mods / BUNDLE.name
            manifest = tomllib.loads((staged / "horizon-mixin.toml").read_text(encoding="utf-8"))
            for entry in manifest["module"]:
                payload = staged / entry["source"]
                proc = subprocess.run(
                    [rustfmt, "--edition", "2024", "--check", str(payload)],
                    capture_output=True, text=True, errors="replace")
                syntax_errors = [line for line in (proc.stdout + proc.stderr).splitlines()
                                 if line.startswith("error:")]
                self.assertEqual([], syntax_errors, f"{entry['source']}: {syntax_errors[:3]}")

    def test_staged_payload_is_self_contained_and_keeps_its_unit_tests(self):
        with tempfile.TemporaryDirectory(prefix="civ-layout-") as work:
            mods = mixin_sources.stage(BUNDLE, Path(work))
            staged = mods / BUNDLE.name
            manifest = tomllib.loads((staged / "horizon-mixin.toml").read_text(encoding="utf-8"))
            matrix = {m["id"]: m for m in manifest["module"]}
            self.assertEqual("core/mod.rs", matrix["core"]["source"])
            payload = (staged / matrix["core"]["source"]).read_text(encoding="utf-8")
            self.assertIn("pub fn found", payload)
            # The native gate selects `civilizations_core::`/`civilizations_glue::`
            # tests, so staged payloads must keep their #[cfg(test)] modules.
            self.assertIn("mod domain_tests", payload)
            self.assertIn("#[cfg(test)]", payload)
            for injection in manifest["injection"]:
                self.assertTrue((staged / injection["helper"]).is_file(), injection["id"])


if __name__ == "__main__":
    unittest.main()
