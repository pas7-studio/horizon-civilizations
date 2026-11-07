"""Native worker 3D reach / item delivery guards.

These checks are source contracts, not Rust/native gameplay execution.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def body(src: str, func: str) -> str:
    return src.split(func, 1)[1].split("\n}", 1)[0]


class NativeWorkerProximityContracts(unittest.TestCase):
    def test_physical_presence_checks_all_three_coordinates(self):
        section = body(CORE, "pub fn native_worker_near(")
        for part in ("actor[0]", "actor[1]", "actor[2]",
                     "dx * dx + dz * dz", "dy.abs() <= vertical_radius"):
            self.assertIn(part, section)
        self.assertIn("actor.iter().all(|v| v.is_finite())", section)
        self.assertIn("vertical_radius < 0.0", section)

    def test_physical_block_reach_checks_actual_eye_height(self):
        section = body(CORE, "pub fn native_worker_reaches_block(")
        self.assertIn("actor[1] + 1.5", section)
        self.assertIn("dy * dy", section)
        self.assertIn("dx * dx + dy * dy + dz * dz <= reach * reach", section)
        self.assertIn("!reach.is_finite()", section)

    def test_native_actor_binding_rejects_remote_y_locations(self):
        section = body(GLUE, "fn loaded_actors(")
        self.assertIn("native_worker_near(mob.pos, village.center, 96.0, 48.0)", section)

    def test_native_navigation_corrects_wrong_floor(self):
        section = body(GLUE, "fn guide_actors(")
        self.assertIn("let dy = mob.pos[1] - pos[1];", section)
        self.assertIn("|| dy.abs() > 2.5", section)

    def test_npc_construction_cannot_be_authorized_from_cave_below(self):
        for function in ("fn build_town_hall_one_step(",
                         "fn build_bakery_one_step(", "fn build_one_step("):
            section = body(GLUE, function)
            self.assertIn("native_worker_near(", section, function)

    def test_farmer_and_baker_must_exist_at_3d_worksites(self):
        for function in ("fn farm_one_step(", "fn baker_one_step("):
            section = body(GLUE, function)
            self.assertIn("native_worker_near(", section, function)

    def test_courier_credit_requires_real_vertical_hall_presence(self):
        for function in ("fn farmer_delivery_one_step(", "fn gather_one_step("):
            section = body(GLUE, function)
            self.assertIn("native_worker_near(actor.pos, v.center, 8.0, 4.0)", section, function)
        warehouse = body(GLUE, "fn storage_worker_near(")
        self.assertIn("native_worker_near(actor.pos, v.center, 8.0, 4.0)", warehouse)

    def test_tree_and_stone_cannot_be_removed_without_3d_reach(self):
        section = body(GLUE, "fn gather_one_step(")
        self.assertIn("native_worker_reaches_block(actor.pos, job.position, 4.5)", section)
        self.assertLess(section.index("native_worker_reaches_block("),
                        section.index("mark_gather_issued(id)?"))

    def test_planned_work_stand_is_revalidated_before_any_mining(self):
        stand = body(GLUE, "fn gather_stand_still_clear(")
        self.assertIn("world_block(game, dimension, stand)", stand)
        self.assertIn("world_block(game, dimension, [x, head, z])", stand)
        self.assertIn("world_block(game, dimension, [x, floor, z])", stand)
        self.assertIn("is_ground(&block)", stand)
        runner = body(GLUE, "fn gather_one_step(")
        self.assertIn("!gather_stand_still_clear(game, &dimension, job.stand)", runner)
        self.assertLess(runner.index("!gather_stand_still_clear("),
                        runner.index("mark_gather_issued(id)?"))

    def test_worker_must_be_at_saved_stand_as_well_as_within_block_reach(self):
        runner = body(GLUE, "fn gather_one_step(")
        self.assertIn("native_worker_near(actor.pos, job.stand, 2.5, 2.0)", runner)
        self.assertIn("native_worker_reaches_block(actor.pos, job.position, 4.5)", runner)
        self.assertLess(runner.index("native_worker_near(actor.pos, job.stand"),
                        runner.index("native_worker_reaches_block(actor.pos, job.position"))

    def test_no_data_migration_or_virtual_resource_grant(self):
        self.assertNotIn("native_worker_near:", CORE)
        self.assertNotIn("warehouse_credit(", body(CORE, "pub fn native_worker_near("))
        self.assertNotIn("native_run_command(", body(CORE, "pub fn native_worker_reaches_block("))


if __name__ == "__main__":
    unittest.main()
