"""Static integration contracts for safe courier stall detection and reroutes.

These are SOURCE checks only; native path success, movement and inventory
correctness require the user's later build and disposable-world E2E.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def body(start: str, end: str) -> str:
    return GLUE.split(start, 1)[1].split(end, 1)[0]


class SafeCourierStuckRecoveryContracts(unittest.TestCase):
    def test_monitor_is_volatile_and_not_part_of_save_schema(self):
        active = body("struct ActiveWorld {", "\n}")
        self.assertIn("courier_routes: BTreeMap<u128, CourierRouteMonitor>", active)
        self.assertNotIn("serde", active)
        self.assertIn("courier_routes: BTreeMap::new()", GLUE)
        settlement = CORE.split("pub struct Settlement {", 1)[1].split("\n}", 1)[0]
        self.assertNotIn("courier_routes", settlement)

    def test_only_verified_loaded_uuid_cargo_enters_monitor(self):
        guide = body("fn guide_actors(", "\n}")
        self.assertIn("resident_verified_cargo(", guide)
        self.assertIn("let Some(uuid) = resident.entity_uuid", guide)
        self.assertIn("game.native_mob(&current_dim, mob_id)", guide)
        self.assertIn("observed_couriers.insert(uuid)", guide)
        self.assertIn("CourierRouteMonitor::start(mob.pos", guide)

    def test_monitor_never_grants_or_deletes_items(self):
        logic = CORE.split("impl CourierRouteMonitor {", 1)[1].split(
            "/// Choose one loaded, active village gather", 1
        )[0]
        for forbidden in ("warehouse_credit(", "warehouse_debit(", "native_run_command(",
                          "native_give_items(", "native_take_items(", "teleport("):
            self.assertNotIn(forbidden, logic)
        self.assertIn("CourierNavAction::NeedsHelp", logic)

    def test_real_movement_not_small_jitter_can_reset_stall(self):
        logic = CORE.split("pub fn observe(", 1)[1].split(
            "/// Choose one loaded, active village gather", 1
        )[0]
        self.assertIn("self.best_distance - distance >= COURIER_PROGRESS_METERS", logic)
        self.assertIn("movement_sq >= COURIER_MOVEMENT_METERS", logic)
        self.assertIn("self.motion_anchor = position", logic)
        self.assertIn("self.reroutes = 0", logic)

    def test_alternatives_have_bounded_cooldown_and_exhaustion(self):
        self.assertIn("pub const COURIER_MAX_REROUTES: u8 = 4", CORE)
        self.assertIn("pub const COURIER_STALL_TICKS: u64 = 400", CORE)
        self.assertIn("pub const COURIER_RETRY_COOLDOWN_TICKS: u64 = 120", CORE)
        self.assertIn("self.reroutes >= COURIER_MAX_REROUTES", CORE)
        self.assertIn("let alternative = self.route_alternative", CORE)
        self.assertIn("return CourierNavAction::Navigate { alternative };", CORE)

    def test_refreshed_native_navigation_is_not_spammed_per_20_tick(self):
        self.assertIn("pub const COURIER_REPATH_TICKS: u64 = 100", CORE)
        self.assertIn("since_request >= COURIER_REPATH_TICKS", CORE)
        guide = body("fn guide_actors(", "\n}")
        self.assertIn("CourierNavAction::Wait | CourierNavAction::NeedsHelp => continue", guide)

    def test_alternatives_still_use_loaded_ground_and_two_air_blocks(self):
        landing = body("fn loaded_civic_receiving_goal(", "\n}")
        self.assertIn("const APPROACHES: [(i32, i32); 10]", landing)
        self.assertIn("usize::from(alternative)", landing)
        self.assertIn("checked_add", landing)
        self.assertIn("world_block(game, dimension, [x, floor, z])", landing)
        self.assertIn("world_block(game, dimension, [x, y, z])", landing)
        self.assertIn("world_block(game, dimension, [x, head, z])", landing)
        self.assertNotIn("native_run_command(", landing)

    def test_unloaded_receiving_area_does_not_exhaust_retries(self):
        guide = body("fn guide_actors(", "\n}")
        self.assertIn("loaded_civic_receiving_goal(", guide)
        self.assertIn(").is_none() { continue; }", guide)
        self.assertIn("courier_routes.retain(|uuid, _| observed_couriers.contains(uuid))", guide)

    def test_routes_reports_real_stalled_retries_without_repairing_stock(self):
        routes = body('if action == "routes" {', 'if action == "logistics" {')
        self.assertIn("state.courier_routes.get(&uuid)", routes)
        self.assertIn("STALLED:", routes)
        self.assertIn("reroutes exhausted", routes)
        self.assertIn("native path success unknown", routes)
        for forbidden in ("save(", "warehouse_credit(", "native_give_items(",
                          "native_mob_navigate(", "world.as_mut()"):
            self.assertNotIn(forbidden, routes)

    def test_guarded_manual_retry_requires_operator_native_identity_and_approval(self):
        retry = body('if action == "retryroute" {', 'if action == "routes" {')
        self.assertIn("player.permission < 2", retry)
        self.assertIn('Some("confirm")', retry)
        self.assertIn("village.dimension != player.dimension", retry)
        self.assertIn("dx * dx + dz * dz > 32.0 * 32.0", retry)
        self.assertIn("resident_verified_cargo(village, citizen)", retry)
        self.assertIn("loaded_actors(game, world, village.id)", retry)

    def test_manual_retry_resets_only_volatile_route_not_receipts(self):
        retry = body('if action == "retryroute" {', 'if action == "routes" {')
        self.assertIn("state.courier_routes.remove(&uuid)", retry)
        for forbidden in ("save(", "warehouse_credit(", "warehouse_debit(", "native_run_command(",
                          "native_give_items(", "native_take_items(", "gather_job =",
                          "wheat_in_transit ="):
            self.assertNotIn(forbidden, retry)

    def test_five_rust_monitor_scenarios_written_for_local_gate(self):
        cases = CORE.split("mod native_courier_stall_monitor_tests {", 1)[1].split(
            "mod verified_dialog_cargo_tests {", 1
        )[0]
        self.assertEqual(cases.count("#[test]"), 5)
        self.assertIn("bounded_alternative_retries_exhaust_to_help", cases)
        self.assertIn("a_three_meter_sideways_detour_counts", cases)
        self.assertIn("invalid_input_and_tick_epoch_reset", cases)


if __name__ == "__main__":
    unittest.main()
