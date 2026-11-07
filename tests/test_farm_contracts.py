"""Offline source contracts for the physical wheat field.

Runtime semantics are exercised by Rust unit tests and pinned native CI.
"""
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / "mixins" / "civilizations"

class FarmContracts(unittest.TestCase):
    def test_farm_writes_require_authoritative_loaded_block_checks(self):
        glue = (MIXINS / "glue.rs").read_text()
        for feature in ["check_empty_farm_site", "farm_one_step", "world_farm_block",
                        "native_run_command", "native_entity_by_uuid", "inspect_farm",
                        "game.native_on(MOD_ID)", '"civ farm <name:word>"']:
            self.assertIn(feature, glue)
        # Rustfmt may split this method chain across lines; assert behavior,
        # not a particular whitespace arrangement.
        self.assertIn('.properties', glue)
        self.assertIn('.get("age")', glue)
        self.assertIn("farm_one_step(game, state)", glue)

    def test_visual_harvest_mints_no_economic_stock(self):
        core = (MIXINS / "core.rs").read_text()
        beginning = core.index("pub fn inspect_farm(")
        end = core.index("#[derive(Clone, Debug, Serialize, Deserialize)]\npub struct Settlement", beginning)
        self.assertNotIn("stock.insert", core[beginning:end])
        self.assertNotIn("native_give_items", (MIXINS / "glue.rs").read_text().split("fn farm_one_step(")[1].split("pub fn interact_entity(")[0])
        self.assertIn("minecraft:wheat[age=7]", core)
        self.assertIn("minecraft:air", core)

    def test_manifest_and_package_are_current(self):
        manifest = (MIXINS / "horizon-mixin.toml").read_text()
        self.assertIn('version = "0.1.4"', manifest)
        self.assertIn('civilizations-0.1.4.hnmod', (ROOT / "scripts" / "install-local.ps1").read_text())

if __name__ == "__main__":
    unittest.main()
