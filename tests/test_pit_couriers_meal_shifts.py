"""Source-level regression contracts for physical pit couriers and meal shifts.

These tests inspect integration structure; they are NOT native Rust or
Horizon gameplay evidence. Run with python -m unittest discover -s tests.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def routine(source, marker):
    start = source.index(marker)
    prefix = source.rfind("\n", 0, start) + 1
    closing = "\n" + " " * (start - prefix) + "}\n"
    return source[start:].split(closing, 1)[0]


class PitCourierAndMealShiftContracts(unittest.TestCase):
    def test_both_pit_goods_require_digger_not_miner(self):
        job = routine(CORE, "pub fn job(self) -> Option<Job>")
        self.assertIn("Self::Sand | Self::Dirt => Some(Job::Digger)", job)
        self.assertIn("Self::Stone => Some(Job::Miner)", job)

    def test_assignment_uses_canonical_good_to_job_mapping(self):
        assignment = routine(CORE, "pub fn assign_gather_worker(")
        self.assertIn("job.good.job().ok_or(", assignment)
        self.assertNotIn("job.good == Good::Stone", assignment)
        self.assertIn("resident.job == required", assignment)
        self.assertIn("job.worker_id.is_some_and(", assignment)

    def test_saved_courier_validates_against_same_canonical_mapping(self):
        validate = routine(CORE, "pub fn validate(")
        self.assertIn("let role = gather.good.job()", validate)
        self.assertIn("r.id == worker_id && r.job == role", validate)
        self.assertNotIn("let role = if gather.good == Good::Wood", validate)

    def test_native_navigation_routes_digger_to_source_then_hall(self):
        route = routine(GLUE, "fn guide_actors(")
        self.assertIn("Job::Lumberjack | Job::Miner | Job::Digger", route)
        self.assertIn("job.good.job() == Some(resident.job)", route)
        self.assertIn("target.issued && target.pickup_witnessed", route)
        self.assertIn("target.stand[0]", route)

    def test_native_gather_witness_uses_correct_job_on_both_phases(self):
        executor = routine(GLUE, "fn gather_one_step(")
        self.assertEqual(
            executor.count("let Some(role) = job.good.job() else { return Ok(()); };"),
            2,
        )
        self.assertIn("let profession = role;", executor)
        self.assertIn("r.job == role && r.id == assigned", executor)
        self.assertIn("native_worker_near(actor.pos, v.center, 8.0, 4.0)", executor)
        self.assertNotIn("let profession = job.good.job().unwrap_or(Job::Miner)", executor)

    def test_food_cannot_keep_every_worker_at_hall_for_the_entire_day(self):
        route = routine(GLUE, "fn guide_actors(")
        self.assertIn("matches!(phase, DailyPhase::Dawn | DailyPhase::Evening)", route)
        self.assertIn("village.meal_day == Some(game.world.day_time.div_euclid(24_000))", route)
        self.assertIn("let pos = if delivery_due {", route)
        self.assertIn("} else if meal_due {", route)
        self.assertLess(route.index("if delivery_due {"), route.index("else if meal_due {"))
        self.assertIn("phase.on_shift(resident.job) && resident.job == Job::Farmer", route)
        self.assertIn("phase.on_shift(resident.job) && resident.job == Job::Baker", route)

    def test_receipt_still_requires_native_air_pickup_and_return(self):
        executor = routine(GLUE, "fn gather_one_step(")
        self.assertIn("job.native_edit_confirmed", executor)
        self.assertIn("witness_gather_pickup(id, assigned)?", executor)
        self.assertIn("confirm_gather(id)?", executor)
        self.assertLess(
            executor.index("witness_gather_pickup(id, assigned)?"),
            executor.index("confirm_gather(id)?"),
        )

    def test_rust_regressions_cover_both_goods_restart_and_exactly_once(self):
        cases = CORE.split("mod physical_pit_courier_tests {", 1)[1].split(
            "mod physical_meals_tests {", 1
        )[0]
        self.assertEqual(cases.count("#[test]"), 2)
        self.assertIn("real_sand_pit_digger_delivers_one_witnessed_item_after_reboot", cases)
        self.assertIn("real_dirt_pit_digger_delivers_one_witnessed_item_after_reboot", cases)
        self.assertIn("serde_json::from_str(&saved)", cases)
        self.assertIn("restored.validate().unwrap()", cases)
        self.assertIn("warehouse_quantity(good), 1", cases)
        self.assertIn("restored.confirm_gather(id).is_err()", cases)


if __name__ == "__main__":
    unittest.main()
