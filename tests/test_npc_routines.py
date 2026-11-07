"""Offline source contracts for deterministic day/night NPC routines."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()


class RoutineContracts(unittest.TestCase):
    def test_schedule_derives_from_authoritative_clock(self):
        self.assertIn("pub enum DailyPhase", CORE)
        self.assertIn("day_time.rem_euclid(24_000)", CORE)
        for phase in ("Dawn", "Work", "Evening", "Night"):
            self.assertIn(phase, CORE)
        self.assertIn("let phase = DailyPhase::at(game.world.day_time);", GLUE)

    def test_read_only_schedule_command(self):
        self.assertIn('"civ schedule <name:word>"', GLUE)
        self.assertIn('if action == "schedule"', GLUE)
        self.assertIn("phase.on_shift(resident.job)", GLUE)
        self.assertIn("game.world.day_time.rem_euclid(24_000)", GLUE)

    def test_farm_and_build_gate_even_if_actor_is_close(self):
        for method, profession in (("build_one_step", "Builder"), ("farm_one_step", "Farmer")):
            body = GLUE.split(f"fn {method}(", 1)[1].split("\n}", 1)[0]
            self.assertIn(
                f"DailyPhase::at(game.world.day_time).on_shift(Job::{profession})",
                body,
            )
            self.assertIn("return Ok(())", body)

    def test_loaded_uuid_actors_only_and_no_new_hooks(self):
        body = GLUE.split("fn guide_actors(", 1)[1].split("\n}", 1)[0]
        self.assertIn("loaded_actors(game, world, village.id)", body)
        self.assertIn("phase.offset(resident.job, resident_id)", body)
        self.assertIn("game.native_mob_navigate", body)
        self.assertIn("game.native_on(MOD_ID)", GLUE)

    def test_version_consistent_and_save_schema_unchanged(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        manifest = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        self.assertEqual(cargo["package"]["version"], manifest["mod"]["version"])
        self.assertIn("pub const SCHEMA: u32 = 1", CORE)


if __name__ == "__main__":
    unittest.main()
