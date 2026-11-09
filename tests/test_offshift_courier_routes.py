"""Source contracts for off-shift verified delivery and safe hall navigation.

Authored for the local Horizon gate. They do not run the native game,
prove a native path succeeded, or replace actual Rust compile/test.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def body(name: str) -> str:
    return GLUE.split("fn " + name + "(", 1)[1].split("\n}", 1)[0]


class OffShiftCourierRouteContracts(unittest.TestCase):
    def test_verified_delivery_precedes_meals_and_sleep(self):
        route = body("guide_actors")
        self.assertIn("resident_verified_cargo(", route)
        self.assertIn("let pos = if delivery_due {", route)
        self.assertIn("} else if meal_due {", route)
        self.assertLess(route.index("if delivery_due {"), route.index("else if meal_due {"))
        self.assertLess(route.index("if delivery_due {"), route.index("else if let Some(home) = home_target"))

    def test_loaded_courier_return_is_independent_of_work_shift(self):
        route = body("guide_actors")
        delivery = route.split("let pos = if delivery_due {", 1)[1].split(
            "} else if meal_due {", 1
        )[0]
        self.assertNotIn("phase.on_shift", delivery)
        self.assertIn("loaded_civic_receiving_goal(", delivery)
        self.assertIn("else { continue };", delivery)

    def test_fresh_work_still_depends_on_daily_shift(self):
        route = body("guide_actors")
        self.assertIn("phase.on_shift(resident.job) && resident.job == Job::Farmer", route)
        self.assertIn("phase.on_shift(resident.job) && resident.job == Job::Baker", route)
        farm = body("farm_one_step")
        self.assertIn("on_shift(Job::Farmer)", farm)
        baking = body("baker_one_step")
        self.assertIn("on_shift(Job::Baker)", baking)

    def test_gather_issued_receipt_reconciles_before_new_work_shift_gate(self):
        executor = body("gather_one_step")
        self.assertIn("if job.issued {", executor)
        self.assertIn("confirm_gather(id)?", executor)
        self.assertIn("if !DailyPhase::at(game.world.day_time).on_shift(role)", executor)
        self.assertLess(executor.index("if job.issued {"),
                        executor.index("if !DailyPhase::at(game.world.day_time).on_shift(role)"))
        self.assertLess(executor.index("confirm_gather(id)?"),
                        executor.index("if !DailyPhase::at(game.world.day_time).on_shift(role)"))
        self.assertLess(executor.index("if !DailyPhase::at(game.world.day_time).on_shift(role)"),
                        executor.index("let saved_stem ="))

    def test_farmer_return_is_allowed_after_shift(self):
        executor = body("farmer_delivery_one_step")
        self.assertNotIn("on_shift(Job::Farmer)", executor)
        self.assertIn("v.wheat_carrier_id", executor)
        self.assertIn("r.id == carrier", executor)
        self.assertIn("deliver_farm_wheat_from(id, carrier)", executor)
        self.assertIn("native_worker_near(actor.pos, v.center, 8.0, 4.0)", executor)

    def test_baker_return_precedes_shift_gate_without_night_production(self):
        executor = body("baker_one_step")
        self.assertIn("deliver_bakery_bread_from(id, carrier)", executor)
        self.assertIn("on_shift(Job::Baker)", executor)
        self.assertIn("craft_bread_with_carrier(id, baker_id)", executor)
        self.assertLess(executor.index("deliver_bakery_bread_from(id, carrier)"),
                        executor.index("on_shift(Job::Baker)"))
        self.assertLess(executor.index("on_shift(Job::Baker)"),
                        executor.index("craft_bread_with_carrier(id, baker_id)"))

    def test_landing_has_bounded_loaded_ground_and_two_high_air(self):
        goal = body("loaded_civic_receiving_goal")
        self.assertIn("world_block(game, dimension, [x, floor, z])", goal)
        self.assertIn("world_block(game, dimension, [x, y, z])", goal)
        self.assertIn("world_block(game, dimension, [x, head, z])", goal)
        self.assertIn("is_ground(block)", goal)
        self.assertIn('Some("minecraft:air")', goal)
        self.assertIn("checked_add", goal)
        self.assertIn("checked_sub", goal)
        self.assertNotIn("native_run_command(", goal)

    def test_navigator_skips_unloaded_or_nonfinite_goal(self):
        route = body("guide_actors")
        self.assertIn("!pos.iter().all(|component| component.is_finite())", route)
        self.assertIn("!game.world.loaded(pos[0].floor() as i32, pos[2].floor() as i32)", route)
        self.assertLess(route.index("!game.world.loaded("),
                        route.index("game.native_mob_navigate("))

    def test_routes_command_registered_as_read_only(self):
        self.assertIn('"civ routes <name:word>"', GLUE)
        self.assertIn('if action == "routes" {', GLUE)
        cmd = GLUE.split('if action == "routes" {', 1)[1].split(
            'if action == "logistics" {', 1
        )[0]
        for forbidden in ("save(", "native_run_command(", "native_mob_navigate(",
                          "native_give_items(", "native_take_items(", "world.as_mut()"):
            self.assertNotIn(forbidden, cmd)
        self.assertIn("loaded_actors(game, world, village.id)", cmd)

    def test_routes_do_not_confuse_unloaded_actors_with_completed_delivery(self):
        cmd = GLUE.split('if action == "routes" {', 1)[1].split(
            'if action == "logistics" {', 1
        )[0]
        self.assertIn("resident_verified_cargo(village, resident)", cmd)
        self.assertIn("UNLOADED actor: custody saved", cmd)
        self.assertIn("no clear LOADED receiving approach", cmd)
        self.assertIn("native path success unknown", cmd)
        self.assertIn("Position is only an observation", cmd)
        self.assertNotIn("warehouse_credit(", cmd)

    def test_routes_preserve_dimension_and_uuid_checks(self):
        cmd = GLUE.split('if action == "routes" {', 1)[1].split(
            'if action == "logistics" {', 1
        )[0]
        self.assertIn("village.dimension != player.dimension", cmd)
        self.assertIn("native_worker_near(actor.pos, village.center, 8.0, 4.0)", cmd)
        self.assertIn("actor.pos.iter().all(|n| n.is_finite())", cmd)
        self.assertIn("loaded_civic_receiving_goal(game, &village.dimension, village.center)", cmd)

    def test_only_original_uuid_can_confirm_delivery_not_route_ui(self):
        self.assertIn("deliver_farm_wheat_from(id, carrier)", GLUE)
        self.assertIn("deliver_bakery_bread_from(id, carrier)", GLUE)
        self.assertIn("witness_gather_pickup(id, assigned)?", GLUE)
        self.assertIn("confirm_gather(id)?", GLUE)
        self.assertIn("pub fn resident_verified_cargo(", CORE)


if __name__ == "__main__":
    unittest.main()
