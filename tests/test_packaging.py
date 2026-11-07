"""Executable checks that do not require Rust or Minecraft assets."""
import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import tomllib
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_packager():
    spec = importlib.util.spec_from_file_location('civ_packager', ROOT / 'tools/package.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PackagingContracts(unittest.TestCase):
    def test_manifest_tracks_compatibility_pin(self):
        compat = json.loads((ROOT / 'compatibility.json').read_text())
        manifest = tomllib.loads((ROOT / 'mixins/civilizations/horizon-mixin.toml').read_text())
        self.assertEqual(compat['horizon_revision'], manifest['compatibility']['horizon_revision'])
        self.assertEqual('civilizations', manifest['mod']['id'])
        cargo = tomllib.loads((ROOT / 'Cargo.toml').read_text())
        self.assertEqual(cargo['package']['version'], manifest['mod']['version'])
        self.assertEqual(6, len(manifest['injection']))

    def test_native_bundle_is_reproducible_and_source_only(self):
        build = load_packager()
        result = build.package()
        first = hashlib.sha256(result.read_bytes()).hexdigest()
        result = build.package()
        self.assertEqual(first, hashlib.sha256(result.read_bytes()).hexdigest())
        with zipfile.ZipFile(result) as z:
            self.assertIsNone(z.testzip())
            items = set(z.namelist())
            self.assertIn('horizon-native.json', items)
            self.assertIn('mixins/civilizations/core.rs', items)
            self.assertIn('mixins/civilizations/glue.rs', items)
            self.assertIn('mixins/civilizations/level_tick.rs', items)
            self.assertFalse(any(x.endswith(('.exe', '.dll', '.so', '.class', '.jar', '.png')) for x in items))
            index = json.loads(z.read('horizon-native.json'))
            cargo = tomllib.loads((ROOT / 'Cargo.toml').read_text())
            self.assertEqual(cargo['package']['version'], index['version'])
            self.assertEqual('civilizations', index['id'])

    def test_static_contracts(self):
        result = subprocess.run([sys.executable, str(ROOT / 'tools/verify_static.py')],
                                capture_output=True, text=True, check=True)
        self.assertIn('PASS', result.stdout)

    def test_mod_keeps_upstream_separate(self):
        sources = list((ROOT / 'mixins/civilizations').glob('*.rs'))
        combined = '\n'.join(x.read_text() for x in sources)
        self.assertNotIn('org.millenaire', combined)
        self.assertFalse((ROOT / 'crates/horizon-client').exists())
        self.assertIn('native_mob_navigate', combined)
        self.assertIn('native_level_tick_post', (ROOT / 'mixins/civilizations/horizon-mixin.toml').read_text())
        self.assertIn('check_empty_hut_site', combined)
        self.assertIn('native_run_command', combined)


if __name__ == '__main__':
    unittest.main()
