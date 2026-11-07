"""First-play diagnosis source contracts; not native integration proof."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


class FirstPlayDiagnosticsContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.section = GLUE.split('if action == "diagnose" {', 1)[1].split(
            'if action == "gathercheck" {', 1
        )[0]

    def test_player_can_call_diagnostic_without_op(self):
        self.assertIn('"civ diagnose <name:word>"', GLUE)
        self.assertNotIn("player.permission", self.section)
        self.assertIn("if v.dimension != player.dimension", self.section)

    def test_live_uuid_actors_and_actual_container_are_observed(self):
        self.assertIn("loaded_actors(game, world, v.id)", self.section)
        self.assertIn("resident.alive && resident.job == role", self.section)
        self.assertIn("physical_barrel_stock(game, &v.dimension, v.center)", self.section)
        self.assertIn("physical_construction_shortfall(v, &stock)", self.section)

    def test_never_mints_virtual_stock_or_alters_native_world(self):
        for forbidden in (
            "state.world.as_mut(", "native_run_command(", "native_spawn_mob(",
            "native_give_items(", "native_take_items(", "native_extract_item(",
            "insert_stack_through(", "save(state)", "warehouse_credit(",
        ):
            self.assertNotIn(forbidden, self.section, forbidden)

    def test_actionable_fail_closed_blocker_order(self):
        sources = (
            "!v.autonomy_enabled", "!v.actors_spawned", "linked.is_empty()",
            "v.market_purchase.is_some()", "v.barrel_transfer.is_some()",
            "v.hall_job.as_ref().is_some_and(|job| job.blocked)",
            "v.gather_job.as_ref().is_some_and(|job| job.blocked)",
            "!loaded_role(Job::Builder)",
        )
        positions = [self.section.index(token) for token in sources]
        self.assertEqual(positions, sorted(positions))

    def test_unloaded_vanilla_barrels_are_not_reported_as_empty(self):
        self.assertIn("real native barrels missing or unloaded", self.section)
        self.assertIn("official hall barrel block entities unavailable", self.section)
        self.assertIn("not a gameplay pass", self.section)

    def test_reports_important_role_and_workshop_states(self):
        for token in ("Job::Baker", "Job::Lumberjack", "Job::Miner",
                      "Job::Merchant", "b.bread_in_transit", "b.wheat_escrow",
                      "b.supplies_reserved", "phase.name()"):
            self.assertIn(token, self.section)


if __name__ == "__main__":
    unittest.main()
