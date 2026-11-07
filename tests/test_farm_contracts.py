"""Offline source contracts for the physical wheat field.

Runtime semantics are exercised by Rust unit tests and pinned native CI.
"""
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
MIXINS = ROOT / "mixins" / "civilizations"

class FarmContracts(unittest.TestCase):
    def test_farm_writes_require_authoritative_loaded_block_checks(self):
        glue = glue_source()
        for feature in ["check_empty_farm_site", "farm_one_step", "world_farm_block",
                        "native_run_command", "native_entity_by_uuid", "inspect_farm",
                        "game.native_on(MOD_ID)", '"civ farm <name:word>"',
                        '"civ recoverfarm <name:word>"', "recover_farm"]:
            self.assertIn(feature, glue)
        # Rustfmt may split this method chain across lines; assert behavior,
        # not a particular whitespace arrangement.
        self.assertIn('.properties', glue)
        self.assertIn('.get("age")', glue)
        self.assertIn("farm_one_step(game, state)", glue)

    def test_visual_harvest_mints_no_economic_stock(self):
        core = core_source()
        beginning = core.index("pub fn inspect_farm(")
        end = core.index("#[derive(Clone, Debug, Serialize, Deserialize)]\npub struct Settlement", beginning)
        self.assertNotIn("stock.insert", core[beginning:end])
        self.assertNotIn("native_give_items", glue_source().split("fn farm_one_step(")[1].split("pub fn interact_entity(")[0])
        self.assertIn("minecraft:wheat[age=7]", core)
        self.assertIn("minecraft:air", core)

    def test_manifest_and_package_are_current(self):
        manifest = (MIXINS / "horizon-mixin.toml").read_text()
        import tomllib
        mixin = tomllib.loads(manifest)
        crate = tomllib.loads((ROOT / "Cargo.toml").read_text())
        version = crate["package"]["version"]
        self.assertEqual(version, mixin["mod"]["version"])
        self.assertIn(f"civilizations-{version}.hnmod",
                      (ROOT / "scripts" / "install-local.ps1").read_text())

if __name__ == "__main__":
    unittest.main()
