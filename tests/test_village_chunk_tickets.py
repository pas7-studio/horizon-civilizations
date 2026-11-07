"""Source contracts for keeping an autonomous village's own area loaded.

Horizontally the engine loads whole 16x16 chunk columns, so the honest
optimisation is to hold FEW chunks: the civic block, the chunk of every real
workplace, and the corridor a villager must walk. Not a square of countryside.
These checks are not a Rust compiler or a Horizon game test.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


class VillageChunkTicketContracts(unittest.TestCase):
    def test_footprint_is_shaped_not_a_square(self):
        self.assertIn("pub const HALL_CHUNK_RADIUS: i32 = 1;", CORE)
        self.assertIn("pub const MAX_HELD_VILLAGE_CHUNKS: usize = 48;", CORE)
        body = CORE.split("pub fn village_chunk_footprint(", 1)[1].split("\n}", 1)[0]
        self.assertIn("div_euclid(16)", body)
        self.assertIn("for site in sites {", body)
        self.assertIn("let steps = (dx.abs().max(dz.abs()) / 4).max(1);", body)
        self.assertIn("if full.len() <= MAX_HELD_VILLAGE_CHUNKS", body)
        self.assertNotIn("native_", body)

    def test_tickets_are_held_by_the_native_api_not_a_command(self):
        hold = GLUE.split("fn hold_village_chunks(", 1)[1].split("\n}", 1)[0]
        self.assertIn("native_forced_chunks(&owner)", hold)
        self.assertIn("native_force_chunk_in(&owner, &dimension, chunk, false)", hold)
        self.assertIn("native_force_chunk_in(&owner, &dimension, *chunk, true)", hold)
        self.assertIn('format!("{MOD_ID}:village/{}", village.id)', hold)
        self.assertIn("if !village.autonomy_enabled {", hold)
        self.assertNotIn("run_command", hold)
        self.assertNotIn("save(", hold)

    def test_only_real_workplaces_extend_the_footprint(self):
        sites = GLUE.split("fn settlement_work_sites(", 1)[1].split("\n}", 1)[0]
        self.assertIn("village.posts.iter()", sites)
        self.assertIn("village.farm", sites)
        self.assertIn("village.bakery", sites)
        self.assertIn("village.visual_sites", sites)
        hold = GLUE.split("fn hold_village_chunks(", 1)[1].split("\n}", 1)[0]
        self.assertIn("let sites = settlement_work_sites(village);", hold)

    def test_holding_happens_before_planning_in_the_level_tick(self):
        tick = GLUE.split("pub fn level_tick(", 1)[1].split("\n}", 1)[0]
        self.assertIn("hold_village_chunks(game, world)", tick)
        self.assertLess(tick.index("hold_village_chunks(game, world)"),
                        tick.index("autonomous_village_step(game, state)"))

    def test_players_can_inspect_the_loaded_area(self):
        self.assertIn('"civ chunks <name:word>"', GLUE)
        self.assertIn('if action == "chunks" {', GLUE)
        self.assertIn("settlement_work_sites(s)", GLUE)


if __name__ == "__main__":
    unittest.main()
