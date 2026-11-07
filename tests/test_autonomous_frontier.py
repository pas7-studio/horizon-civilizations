"""Source contracts for autonomous first-settlement discovery.

These guard native integration structure; Rust/Horizon gameplay still needs
real compilation and a disposable-world acceptance run.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def routine(source, start):
    return source.split(start, 1)[1].split("\n}", 1)[0]


class AutonomousFrontierContracts(unittest.TestCase):
    def test_npc_founder_never_receives_human_lordship_or_credits(self):
        npc = routine(CORE, "pub fn found_autonomous(")
        self.assertIn("NPC_FOUNDER, false", npc)
        shared = routine(CORE, "fn found_internal(")
        self.assertIn("if player_founded {", shared)
        self.assertIn("self.players.entry(founder.to_owned())", shared)
        self.assertIn("self.settlements.insert(id, settlement)", shared)

    def test_autonomous_founder_limited_to_overworld_and_cap(self):
        npc = routine(CORE, "pub fn found_autonomous(")
        self.assertIn('dimension != "minecraft:overworld"', npc)
        self.assertIn("MAX_NPC_SETTLEMENTS", npc)
        self.assertIn('pub const MAX_NPC_SETTLEMENTS: usize = 8;', CORE)

    def test_first_village_can_appear_for_integrated_host_or_guests(self):
        start = routine(GLUE, "fn autonomous_frontier_found(")
        self.assertIn("game.player.as_ref()", start)
        self.assertIn("game.native_server_players()", start)
        self.assertIn("player.dimension == dimension", start)
        self.assertIn("if explorers", start) if "if explorers" in start else self.assertIn("for explorer in explorers", start)

    def test_new_villages_avoid_players_existing_towns_and_world_border(self):
        founding = routine(GLUE, "fn autonomous_frontier_found(")
        self.assertIn("FRONTIER_OFFSETS", founding)
        self.assertIn("29_999_000.0", founding)
        self.assertIn("192_i64 * 192", founding)
        self.assertIn("if near_existing { continue; }", founding)
        self.assertIn("checked_add", founding)

    def test_terrain_and_ten_actor_preflight_before_persistent_foundation(self):
        founding = routine(GLUE, "fn autonomous_frontier_found(")
        self.assertLess(founding.index("check_empty_hut_site(game"),
                        founding.index("trial.found_autonomous("))
        self.assertLess(founding.index("trial.found_autonomous("),
                        founding.index("surveyed_initial_actor_positions("))
        self.assertLess(founding.index("surveyed_initial_actor_positions("),
                        founding.index("state.world.as_mut().unwrap()"))
        self.assertIn("save_autonomous(state, before)?;", founding)

    def test_founding_has_no_direct_native_spawn_or_world_edits(self):
        founding = routine(GLUE, "fn autonomous_frontier_found(")
        self.assertNotIn("native_spawn_mob(", founding)
        self.assertNotIn("native_run_command(", founding)
        self.assertLess(founding.index("save_autonomous(state, before)?;"),
                        founding.index("game.native_broadcast("))

    def test_one_bounded_foundation_attempt_every_2400_ticks(self):
        tick = routine(GLUE, "pub fn level_tick(")
        self.assertIn("if state.physical_ticks % 2_400 == 0", tick)
        self.assertIn("autonomous_frontier_found(game, state)", tick)
        self.assertIn("if error.starts_with(\"persistence:\")", tick)

    def test_existing_npc_spawning_receives_new_foundations(self):
        tick = routine(GLUE, "pub fn level_tick(")
        autonomous = routine(GLUE, "fn autonomous_village_step(")
        self.assertIn("if state.physical_ticks % 100 == 0", tick)
        self.assertIn("return auto_spawn_villagers(game, state, id);", autonomous)
        self.assertLess(tick.index("autonomous_frontier_found(game, state)"),
                        tick.index("autonomous_village_step(game, state)"))

    def test_nearby_is_a_public_command_and_explains_location(self):
        self.assertIn('"civ nearby"', GLUE)
        self.assertIn('if action == "nearby" {', GLUE)
        self.assertIn("/civ list | nearby | info", GLUE)
        route = GLUE.split('if action == "nearby" {', 1)[1].split(
            'if action == "hall"', 1
        )[0]
        self.assertIn("player.dimension", route)
        self.assertIn("v.center[0]", route)
        self.assertIn("v.center[2]", route)
        self.assertIn("dx.hypot(dz)", route)

    def test_nearby_does_not_modify_authoritative_world_or_inventory(self):
        route = GLUE.split('if action == "nearby" {', 1)[1].split(
            'if action == "hall"', 1
        )[0]
        self.assertNotIn("state.world.as_mut()", route)
        self.assertNotIn("native_run_command(", route)
        self.assertNotIn("native_spawn_mob(", route)
        self.assertNotIn("save(state)", route)
        self.assertIn("sort_by(|a, b| a.0.total_cmp", route)
        self.assertIn(".take(4)", route)

    def test_real_rust_regressions_cover_join_limit_and_roundtrip(self):
        tests = CORE.split("mod autonomous_frontier_tests", 1)[1]
        self.assertEqual(tests.count("#[test]"), 6)
        self.assertIn("reload_cannot_found_a_second_npc_town_on_the_same_site", tests)
        self.assertIn("npc_founding_creates_full_independent_society_without_player_gifts", tests)
        self.assertIn("a_real_player_may_join_an_independent_village", tests)
        self.assertIn("autonomous_villages_are_bounded", tests)
        self.assertIn("npc_founding_roundtrip", tests)


if __name__ == "__main__":
    unittest.main()
