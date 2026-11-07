"""Civic hall and receipt-backed physical warehouse structural contracts.

These tests inspect source and do not certify Horizon native compilation,
Minecraft sign NBT support, AI navigation or container inventory behavior.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()


class CivicHallWarehouseContracts(unittest.TestCase):
    def test_hall_is_distinct_from_abstract_houses(self):
        self.assertIn("pub fn town_hall_plan(origin:", CORE)
        self.assertIn("pub fn inspect_town_hall(", CORE)
        for block in ("minecraft:barrel", "minecraft:oak_sign", "minecraft:lectern"):
            self.assertIn(block, CORE)
        self.assertIn("pub hall_job: Option<VisualBuildJob>", CORE)
        self.assertIn("pub hall_complete: bool", CORE)
        self.assertIn("pub fn finish_town_hall(", CORE)

    def test_hall_funds_only_from_receipted_real_goods(self):
        reserve = CORE.split("pub fn reserve_town_hall(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("v.hall_materials_missing()", reserve)
        self.assertIn("v.warehouse_debit(Good::Wood, HALL_WOOD_LOGS)?", reserve)
        self.assertIn("v.warehouse_debit(Good::Stone, HALL_COBBLESTONE)?", reserve)
        self.assertNotIn("v.take(", reserve)
        self.assertIn("pub const HALL_WOOD_LOGS: u32 = 24;", CORE)
        self.assertIn("pub const HALL_COBBLESTONE: u32 = 25;", CORE)

    def test_hut_reconciliation_has_real_warehouse_funding_gate(self):
        reserve = CORE.split("pub fn reserve_hut_supplies(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("v.hut_supplies_reserved", reserve)
        self.assertIn("v.warehouse_debit(Good::Wood, HUT_WOOD_LOGS)?", reserve)
        self.assertIn("v.warehouse_debit(Good::Stone, HUT_COBBLESTONE)?", reserve)
        build = GLUE.split("fn build_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("reserve_hut_supplies(village_id)?", build)
        self.assertIn("save_autonomous(state, previous)?", build)
        self.assertLess(build.index("save_autonomous(state, previous)?"),
                        build.index("game.native_run_command(&command)"))

    def test_gather_is_two_stage_save_first_then_world_observation(self):
        self.assertIn("pub struct GatherJob", CORE)
        self.assertIn("pub fn plan_gather(", CORE)
        self.assertIn("pub fn mark_gather_issued(", CORE)
        self.assertIn("pub fn confirm_gather(", CORE)
        self.assertIn("pub fn discard_blocked_gather(", CORE)
        executor = GLUE.split("fn gather_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn('if name == "minecraft:air"', executor)
        self.assertIn("confirm_gather(id)?", executor)
        self.assertIn("job.source_block()", executor)
        self.assertIn("game.native_run_command(&cmd)", executor)
        self.assertIn("mark_gather_issued(id)?", executor)
        self.assertIn("save_autonomous(state, before)", executor)

    def test_gather_prevents_registered_property_and_structure_damage(self):
        plan = CORE.split("pub fn plan_gather(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("near_plot(other.center, position)", plan)
        self.assertIn("other.player_parcels", plan)
        self.assertIn("other.visual_sites", plan)
        self.assertIn("other.visual_job", plan)
        self.assertIn("other.farm", plan)
        self.assertIn("worker_job", plan)
        self.assertIn("another physical gather is pending", plan)

    def test_miner_and_lumberjack_act_in_loaded_world(self):
        self.assertIn("fn seems_natural_resource(", GLUE)
        self.assertIn('Some("minecraft:oak_leaves")', GLUE)
        self.assertIn('Some("minecraft:stone")', GLUE)
        self.assertIn("fn gather_stand(", GLUE)
        self.assertIn("fn autonomous_village_step(", GLUE)
        self.assertIn("plan_gather(id, good, position, stand)", GLUE)
        self.assertIn("loaded_actors(game, state.world.as_ref().unwrap(), id)", GLUE)
        self.assertIn("state.physical_ticks % 8 == 0", GLUE)
        self.assertIn("gather_one_step(game, state)", GLUE)

    def test_physical_wheat_only_from_confirmed_harvest_receipt(self):
        farm = CORE.split("pub fn advance_farm(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("FarmStage::Harvesting", farm)
        self.assertIn("new_wheat", farm)
        self.assertIn("village.wheat_in_transit = count", farm)
        self.assertIn("pub fn deliver_farm_wheat(", CORE)
        self.assertIn("v.warehouse.insert(Good::Wheat, next)", CORE)
        self.assertIn("farm.harvest_receipts.push(HarvestReceipt", farm)
        self.assertNotIn("village.add(Good::Wheat", farm)

    def test_buy_sell_physically_received_items_not_virtual_daily_stock(self):
        sell = CORE.split("pub fn sell(", 1)[1].split("\n    }", 1)[0]
        buy = CORE.split("pub fn buy(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("s.warehouse_credit(good, count)?", sell)
        self.assertNotIn("s.add(good, count)", sell)
        self.assertIn("s.warehouse_debit(good, count)?", buy)
        self.assertNotIn("s.take(good, count)", buy)
        self.assertIn("game.native_take_items(item, count)", GLUE)
        self.assertIn("game.native_give_items(item, count)", GLUE)

    def test_hall_build_native_loaded_observation_and_builder_proximity(self):
        section = GLUE.split("fn build_town_hall_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("DailyPhase::at(game.world.day_time)", section)
        self.assertIn("inspect_town_hall(&job", section)
        self.assertIn("loaded_actors(game, world, village_id)", section)
        self.assertIn("BuildInspection::Blocked", section)
        self.assertIn("BuildInspection::Complete", section)
        self.assertIn("finish_town_hall(village_id)?", section)
        self.assertIn("game.native_run_command(&cmd)", section)

    def test_real_signs_show_mayor_and_material_shortages(self):
        self.assertIn("pub fn hall_board_positions(", CORE)
        self.assertIn("fn refresh_hall_boards(", GLUE)
        self.assertIn("minecraft:oak_sign", GLUE)
        self.assertIn("MAYOR {mayor}", GLUE)
        self.assertIn("WOOD -{wood_missing}", GLUE)
        self.assertIn("STONE -{stone_missing}", GLUE)
        self.assertIn("NO HOME {homeless}", GLUE)
        self.assertIn("front_text:", GLUE)
        self.assertIn('if action == "hall" || action == "warehouse"', GLUE)

    def test_version_mixin_and_legacy_schema(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        mixin = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        self.assertEqual(cargo["package"]["version"], mixin["mod"]["version"])
        self.assertEqual(mixin["mod"]["version"], "0.1.14")
        self.assertEqual(len(mixin["injection"]), 9)
        self.assertIn("pub const SCHEMA: u32 = 1;", CORE)
        for field in ("warehouse", "hall_job", "hall_complete",
                      "hut_supplies_reserved", "gather_job"):
            self.assertIn(f"pub {field}:", CORE)


if __name__ == "__main__":
    unittest.main()
