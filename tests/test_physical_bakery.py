"""v0.1.14 source contracts for actual functional village bakery.

These inspect source text and are NOT Rust/native compile or gameplay proof.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()
MIXIN = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())


class PhysicalBakerySourceContracts(unittest.TestCase):
    def test_native_bakery_is_its_own_visible_physical_structure(self):
        self.assertIn("pub fn bakery_plan(origin:", CORE)
        self.assertIn('"minecraft:crafting_table"', CORE)
        self.assertIn('"minecraft:smoker"', CORE)
        self.assertIn("pub fn inspect_bakery(", CORE)
        self.assertIn("pub bakery: Option<BakeryWorkshop>", CORE)
        self.assertIn("pub fn start_bakery(", CORE)
        self.assertIn("pub fn finish_bakery(", CORE)

    def test_construction_requires_materials_extracted_from_real_barrels(self):
        self.assertIn("pub const BAKERY_WOOD_LOGS: u32 = 24;", CORE)
        self.assertIn("pub const BAKERY_COBBLESTONE: u32 = 33;", CORE)
        self.assertIn("pub fn reserve_bakery_supplies(", CORE)
        section = CORE.split("pub fn reserve_bakery_supplies(",1)[1].split("\n    }",1)[0]
        self.assertIn("bakery.construction_escrow.get(&Good::Wood)", section)
        self.assertIn("bakery.construction_escrow.get(&Good::Stone)", section)
        self.assertNotIn("village.stock", section)
        self.assertNotIn("warehouse_debit", section)
        self.assertIn("pub enum BarrelDirection", CORE)
        self.assertIn("ToBakeryBuilder", CORE)
        self.assertIn("BarrelDirection::ToBakeryBuilder", GLUE)

    def test_real_crafting_uses_three_wheat_from_native_barrel(self):
        self.assertIn("pub const BREAD_WHEAT_RECIPE: u32 = 3;", CORE)
        self.assertIn("ToBakerIngredients", CORE)
        self.assertIn("pub fn craft_bread(", CORE)
        recipe = CORE.split("pub fn craft_bread(",1)[1].split("\n    }",1)[0]
        self.assertIn("b.wheat_escrow -= BREAD_WHEAT_RECIPE", recipe)
        self.assertIn("b.bread_in_transit += 1", recipe)
        self.assertIn("b.batches = next", recipe)
        self.assertNotIn("village.stock", recipe)
        self.assertNotIn("warehouse_credit", recipe)

    def test_baker_needs_native_unchanged_crafting_table_and_real_actor(self):
        method = GLUE.split("fn baker_one_step(",1)[1].split("\n}",1)[0]
        self.assertIn("bakery_plan(b.origin).iter().all", method)
        self.assertIn("world_block(game, &dimension, step.pos)", method)
        self.assertIn("loaded_actors(game, world, id)", method)
        self.assertIn("game.native_mob(&dimension, *mob)", method)
        self.assertIn("craft_bread(id)?", method)
        self.assertIn("save_autonomous(state, before)?", method)

    def test_baker_hauls_bread_back_to_town_hall_before_deposit(self):
        routes = GLUE.split("fn guide_actors(",1)[1].split("\n}",1)[0]
        self.assertIn("resident.job == Job::Baker", routes)
        self.assertIn("bakery.bread_in_transit > 0", routes)
        self.assertIn("bakery.wheat_escrow < ", routes)
        self.assertIn("bakery.origin[0]", routes)
        method = GLUE.split("fn baker_one_step(",1)[1].split("\n}",1)[0]
        self.assertIn("storage_worker_near(game, world, id, Job::Baker)", method)
        self.assertIn("deliver_bakery_bread(id)?", method)
        self.assertIn("v.warehouse_credit(Good::Bread, pending)?", CORE)

    def test_native_bread_created_only_as_bounded_verified_deposit(self):
        self.assertIn("fn barrel_one_step(", GLUE)
        self.assertIn("fn request_workshop_pickup(", GLUE)
        self.assertIn("BarrelDirection::ToBakerIngredients", GLUE)
        self.assertIn("game.native_extract_item(", GLUE)
        self.assertIn("game.insert_stack_through(", GLUE)
        self.assertIn("BarrelDirection::ToBarrel => v.warehouse_debit(good, count)", CORE)

    def test_bakery_workstations_reconcile_block_by_block(self):
        method = GLUE.split("fn build_bakery_one_step(",1)[1].split("\n}",1)[0]
        self.assertIn("inspect_bakery(&job", method)
        self.assertIn("BuildInspection::Blocked", method)
        self.assertIn("BuildInspection::Advance", method)
        self.assertIn("BuildInspection::Complete", method)
        self.assertIn("finish_bakery(id)?", method)
        self.assertIn("game.native_run_command(&command)", method)
        self.assertIn("nearby_builder", method)

    def test_default_on_autonomy_reserves_bakery_on_safe_loaded_land(self):
        method=GLUE.split("fn autonomous_village_step(",1)[1].split("\n}",1)[0]
        self.assertIn("let need_bakery =", method)
        self.assertIn("start_bakery(id, site)?", method)
        self.assertIn("check_empty_hut_site(game, &dimension, site)", method)
        self.assertIn("has_baker && village.hall_complete", method)
        self.assertIn("save_autonomous(state, old)?", method)

    def test_native_inventory_demand_includes_stored_goods_and_transit(self):
        planner = GLUE.split("fn autonomous_village_step(", 1)[1].split("\n}", 1)[0]
        demand = CORE.split("pub fn physical_construction_shortfall(", 1)[1].split("\n}", 1)[0]
        self.assertIn("physical_barrel_stock(game, &dimension, village.center)", planner)
        self.assertIn("physical_construction_shortfall(", planner)
        self.assertIn("village.barrel_transfer.is_some()", planner)
        self.assertIn("village.market_purchase.is_some()", planner)
        self.assertIn("village.warehouse_quantity(good)", demand)
        self.assertIn("village.build_escrow.get(&good)", demand)
        self.assertIn("b.construction_escrow.get(&good)", demand)
        self.assertIn("bakery_bill.saturating_add(hut_bill)", demand)
        self.assertIn("unpaid(Good::Wood).saturating_sub(available(Good::Wood))", demand)

    def test_player_buyout_and_workshop_claim_exclude_npc_damage(self):
        self.assertIn("pub fn player_may_break(",CORE)
        self.assertIn("lot.owner != player",CORE)
        self.assertIn("v.bakery.as_ref().is_some_and(|b| in_hut(b.origin))",CORE)
        self.assertIn("other.bakery.as_ref().is_some_and", CORE)
        self.assertIn("pub fn protect_civilization_block(",GLUE)
        self.assertIn("*cancel = true;", GLUE)

    def test_hoppers_can_neither_siphon_nor_insert_public_stock(self):
        self.assertIn("pub fn protect_public_hopper(",GLUE)
        self.assertIn("positions.contains(&source) || positions.contains(&destination)",GLUE)
        self.assertIn("native_inventory_move_item",str(MIXIN))
        self.assertTrue((ROOT/"mixins/civilizations/hopper_transfer.rs").is_file())

    def test_native_player_break_hook_is_public_only_and_does_not_touch_other_land(self):
        self.assertIn("native_destroy_block",str(MIXIN))
        self.assertTrue((ROOT/"mixins/civilizations/protect_block.rs").is_file())
        self.assertIn("player.permission >= 2",GLUE)
        self.assertIn("player_may_break(",GLUE)

    def test_new_workshop_and_escrow_fields_migrate_old_worlds(self):
        self.assertIn("#[serde(default)]\n    pub bakery: Option<BakeryWorkshop>",CORE)
        for f in ("construction_escrow","wheat_escrow","bread_in_transit","batches"):
            self.assertIn(f"pub {f}:",CORE)
        self.assertIn('pub const SCHEMA: u32 = 1;',CORE)

    def test_civic_status_includes_physical_workshop_batches(self):
        self.assertIn('"civ bakery <name:word>"',GLUE)
        self.assertIn('if action == "bakery"',GLUE)
        self.assertIn("Completed 3-wheat-to-1-bread batches",GLUE)

    def test_manifest_uses_nine_real_horizon_injections(self):
        cargo=tomllib.loads((ROOT/"Cargo.toml").read_text())
        self.assertEqual(cargo["package"]["version"],MIXIN["mod"]["version"])
        self.assertEqual(len(MIXIN["injection"]),9)
        ids={x["id"] for x in MIXIN["injection"]}
        self.assertTrue({"hopper_transfer","protect_break","warehouse_inventory"} <= ids)


if __name__ == "__main__":
    unittest.main()
