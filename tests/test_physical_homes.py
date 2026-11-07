"""Read-only source contracts for physical-hut-only NPC home routing."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


class PhysicalHomesContracts(unittest.TestCase):
    def test_only_completed_sites_supply_homes(self):
        self.assertIn("pub const PHYSICAL_HOME_CAPACITY: usize = 3;", CORE)
        section = CORE.split("pub fn home_slot(&self", 1)[1].split("\n    }", 1)[0]
        self.assertIn("self.visual_sites.get(position / PHYSICAL_HOME_CAPACITY)", section)
        self.assertNotIn("self.houses", section)
        self.assertNotIn("pending_visual_buildings", section)

    def test_allocation_is_stable_by_living_resident_identity(self):
        section = CORE.split("pub fn home_slot(&self", 1)[1].split("\n    }", 1)[0]
        self.assertIn("resident.alive", section)
        self.assertIn("ids.sort_unstable()", section)
        self.assertIn("ids.binary_search(&resident_id)", section)

    def test_minecraft_block_inspection_is_fail_closed(self):
        section = CORE.split("pub fn inspect_home(", 1)[1].split("\n}", 1)[0]
        self.assertIn("fn", CORE)
        self.assertIn('"minecraft:cobblestone"', section)
        self.assertIn('"minecraft:air"', section)
        self.assertIn("return false", section)
        self.assertIn("read(position).as_deref() != Some(expected)", section)

    def test_night_routes_only_loaded_verified_homes(self):
        self.assertIn("fn loaded_home_target(", GLUE)
        self.assertIn("inspect_home(home, |position| world_block(game, dimension, position))", GLUE)
        section = GLUE.split("fn guide_actors(", 1)[1].split("\n}", 1)[0]
        self.assertIn("phase == DailyPhase::Night", section)
        self.assertIn("!phase.on_shift(resident.job)", section)
        self.assertIn("village.home_slot(resident_id)", section)
        self.assertIn("loaded_home_target(game, &current_dim, home)", section)
        self.assertIn("loaded_actors(game, world, village.id)", section)

    def test_homes_command_and_dialogue_present(self):
        self.assertIn('"civ homes <name:word>"', GLUE)
        self.assertIn('if action == "homes"', GLUE)
        self.assertIn("village.physical_housing()", GLUE)
        self.assertIn("village.home_slot(resident.id)", CORE)
        self.assertIn("Lives in a village hut, place", CORE)

    def test_version_and_persistence_contracts(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        mixin = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        self.assertEqual(cargo["package"]["version"], mixin["mod"]["version"])
        self.assertIn("pub const SCHEMA: u32 = 1", CORE)
        self.assertNotIn("pub home_slot:", CORE)
        self.assertNotIn("pub home_site:", CORE)


if __name__ == "__main__":
    unittest.main()
