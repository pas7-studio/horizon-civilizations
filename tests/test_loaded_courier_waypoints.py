"""Static source contracts for loaded-world courier A* checkpoints.

Authored for later local execution only; these checks do not certify
the real native pathfinder, a Rust compile or Minecraft gameplay.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def section(text: str, start: str, end: str) -> str:
    return text.split(start, 1)[1].split(end, 1)[0]


class LoadedCourierWaypointContracts(unittest.TestCase):
    def test_core_exposes_bounded_path_and_condensed_checkpoints(self):
        self.assertIn("pub fn plan_loaded_courier_path(", CORE)
        self.assertIn("pub fn courier_route_checkpoints(", CORE)
        self.assertIn("pub fn courier_walkable(", CORE)
        self.assertIn("pub const COURIER_PATH_MAX_EXPANSIONS: usize = 4_096;", CORE)
        self.assertIn("pub const COURIER_PATH_MAX_SPAN: u32 = 112;", CORE)
        self.assertIn("pub const COURIER_ROUTE_WAYPOINT_SPACING: usize = 6;", CORE)

    def test_all_steps_have_a_real_ground_and_two_loaded_air_blocks(self):
        check = section(CORE, "pub fn courier_walkable(", "/// Bounded A*")
        self.assertIn("is_ground(name)", check)
        self.assertIn('"minecraft:oak_planks" | "minecraft:cobblestone"', check)
        self.assertIn("read(feet).as_deref() == Some(\"minecraft:air\")", check)
        self.assertIn("read([feet[0], head, feet[2]])", check)
        self.assertNotIn("unwrap_or(", check)

    def test_search_refuses_unloaded_ends_and_faraway_destinations(self):
        route = section(CORE, "pub fn plan_loaded_courier_path(", "/// Split a completely loaded")
        self.assertIn("start[0].abs_diff(goal[0]) > COURIER_PATH_MAX_SPAN", route)
        self.assertIn("start[1].abs_diff(goal[1]) > COURIER_PATH_MAX_ELEVATION", route)
        self.assertIn("!courier_walkable(start, &mut read)", route)
        self.assertIn("!courier_walkable(goal, &mut read)", route)
        self.assertIn("if expanded > COURIER_PATH_MAX_EXPANSIONS", route)

    def test_a_star_moves_cardinally_and_at_most_one_block_up_or_down(self):
        route = section(CORE, "pub fn plan_loaded_courier_path(", "/// Split a completely loaded")
        self.assertIn("[(0_i32,-1_i32), (-1,0), (1,0), (0,1)]", route)
        self.assertIn("for dy in [0_i32, 1, -1]", route)
        self.assertIn("at[1].checked_add(dy)", route)
        self.assertIn("dy == 1", route)
        self.assertIn("read([at[0], above, at[2]])", route)
        self.assertIn("BinaryHeap::new()", route)
        self.assertIn("BTreeMap::new()", route)

    def test_route_steps_are_observed_not_world_edits_or_inventory_grants(self):
        planner = section(CORE, "pub fn plan_loaded_courier_path(", "/// Choose one loaded, active village gather")
        for forbidden in ("native_run_command(", "native_spawn_mob(", "native_give_items(",
                          "warehouse_credit(", "warehouse_debit(", "teleport(", "world.stock"):
            self.assertNotIn(forbidden, planner)

    def test_checkpoint_compression_keeps_every_corner_and_six_block_spacing(self):
        condense = section(CORE, "pub fn courier_route_checkpoints(", "/// Choose one loaded, active village gather")
        self.assertIn("next_direction != direction", condense)
        self.assertIn("COURIER_ROUTE_WAYPOINT_SPACING", condense)
        self.assertIn("out.push(full[index-1])", condense)
        self.assertIn("out.push(end)", condense)

    def test_actor_start_uses_loaded_nearby_real_native_geometry(self):
        native = section(GLUE, "fn loaded_courier_route_start(", "/// Profession-aware day/night")
        self.assertIn("actor.iter().all(|v| v.is_finite())", native)
        self.assertIn("checked_add", native)
        self.assertIn("native_worker_near(actor, candidate, 1.9, 1.5)", native)
        self.assertIn("courier_walkable(candidate", native)
        self.assertIn("world_block(game, dimension, pos)", native)
        self.assertNotIn("native_run_command(", native)

    def test_only_receipt_verified_couriers_get_native_waypoint_plans(self):
        route = section(GLUE, "fn guide_actors(", "/// Reads only loaded blocks")
        self.assertIn("resident_verified_cargo(", route)
        self.assertIn("CourierNavAction::Navigate { alternative }", route)
        self.assertIn("plan_loaded_courier_path(start, dest,", route)
        self.assertIn("courier_route_checkpoints(&full)", route)
        self.assertIn("monitor.set_waypoints(alternative, route)", route)

    def test_native_nav_uses_next_verified_checkpoint_never_unsurveyed_direct_goal(self):
        route = section(GLUE, "fn guide_actors(", "/// Reads only loaded blocks")
        cargo = section(route, "let pos = if delivery_due {", "} else if meal_due {")
        self.assertIn("let Some(checkpoint) = monitor.next_waypoint(mob.pos)", cargo)
        self.assertIn("courier_walkable(checkpoint,", cargo)
        self.assertIn("f64::from(checkpoint[0]) + 0.5", cargo)
        self.assertNotIn("return Some(goal)", cargo)
        self.assertIn("game.native_mob_navigate(", route)

    def test_path_is_volatility_bounded_and_surveys_do_not_block_server_forever(self):
        route = section(GLUE, "fn guide_actors(", "/// Reads only loaded blocks")
        self.assertIn("route_surveys_remaining = COURIER_MAX_PATH_SURVEYS_PER_NAV_TICK", route)
        self.assertIn("if route_surveys_remaining == 0", route)
        self.assertIn("route_surveys_remaining -= 1", route)
        self.assertIn("courier_routes.retain(", route)
        self.assertIn("COURIER_MAX_PATH_SURVEYS_PER_NAV_TICK: usize = 2", GLUE)

    def test_saved_uuid_cargo_never_depends_on_volatile_waypoints(self):
        monitor = section(CORE, "pub struct CourierRouteMonitor {", "impl CourierRouteMonitor")
        self.assertIn("waypoints: Vec<[i32; 3]>", monitor)
        self.assertIn("waypoint_cursor: usize", monitor)
        self.assertIn("route_alternative: Option<u8>", monitor)
        self.assertNotIn("Serialize", monitor)
        settlement = section(CORE, "pub struct Settlement {", "\n}")
        self.assertNotIn("waypoint_cursor", settlement)
        self.assertIn("pub wheat_carrier_id: Option<u64>", settlement)

    def test_live_routes_displays_checkpoint_or_unknown_walkway_without_grants(self):
        route = section(GLUE, 'if action == "routes" {', 'if action == "logistics" {')
        self.assertIn("NO VERIFIED WALKWAY", route)
        self.assertIn("loaded walkway checkpoint", route)
        self.assertIn("monitor.waypoints.get(monitor.waypoint_cursor)", route)
        self.assertIn("native path success unknown", route)
        for forbidden in ("warehouse_credit(", "native_give_items(", "native_run_command("):
            self.assertNotIn(forbidden, route)

    def test_rust_regressions_cover_detours_stairs_unknown_chunks_and_monitoring(self):
        cases = section(CORE, "mod loaded_courier_waypoint_tests {",
                        "mod native_courier_stall_monitor_tests {")
        self.assertEqual(cases.count("#[test]"), 7)
        for name in (
            "loaded_flat_native_corridor_reaches_hall_with_sparse_checkpoints",
            "solid_wall_forces_a_real_loaded_cardinal_detour",
            "one_block_stair_is_allowed_only_with_observed_head_clearance",
            "unknown_chunk_wall_and_invalid_standing_cell_fail_closed",
            "standing_and_waypoints_cannot_jump_diagonally",
            "monitor_advances_only_near_loaded_waypoints",
            "genuine_movement_preserves_active_alternative",
        ):
            self.assertIn(name, cases)


if __name__ == "__main__":
    unittest.main()
