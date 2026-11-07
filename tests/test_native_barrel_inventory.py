"""v0.1.13 structural guardrails for real native barrel storage.

These contracts inspect source only. A Windows native Horizon compile, Rust
unit tests and actual in-world hopper/player/chunk interaction still need
separate execution before a PR may be considered playable.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()
MIXIN = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())


class NativeBarrelInventorySourceContracts(unittest.TestCase):
    def test_exact_real_barrels_are_in_hall_plan(self):
        self.assertIn("pub fn hall_storage_positions(origin:", CORE)
        self.assertIn("pub fn town_hall_plan(origin:", CORE)
        self.assertIn('(-1, 1, "minecraft:barrel")', CORE)
        self.assertIn('(1, 1, "minecraft:barrel")', CORE)

    def test_pinned_horizon_real_inventory_api_not_nbt_emulation(self):
        self.assertIn("use horizon_client::items::ItemStack;", GLUE)
        self.assertIn("use horizon_core::geometry::Direction;", GLUE)
        self.assertIn("game.native_container_slots(", GLUE)
        self.assertIn("game.native_extract_item(", GLUE)
        self.assertIn("game.insert_stack_through(", GLUE)
        self.assertIn("Direction::Up", GLUE)
        self.assertIn("stack.damage == 0 && stack.extra.is_none()", GLUE)

    def test_durable_transfers_are_single_outstanding_and_bounded(self):
        self.assertIn("pub struct BarrelTransfer", CORE)
        self.assertIn("pub barrel_transfer: Option<BarrelTransfer>", CORE)
        self.assertIn("pub enum BarrelPhase", CORE)
        self.assertIn("pub enum BarrelDirection", CORE)
        self.assertIn("pub fn plan_barrel_transfer(", CORE)
        self.assertIn("pub fn issue_barrel_transfer(", CORE)
        self.assertIn("pub fn confirm_barrel_transfer(", CORE)
        self.assertIn("pub fn block_barrel_transfer(", CORE)
        self.assertIn("(1..=8).contains(&count)", CORE)
        self.assertIn("t.expected_after() != Some(after)", CORE)

    def test_native_write_ahead_saved_before_first_item_move(self):
        method = GLUE.split("fn barrel_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("transfer.phase == BarrelPhase::Blocked", method)
        self.assertIn("transfer.phase == BarrelPhase::Issued", method)
        self.assertIn("issue_barrel_transfer(id)?", method)
        self.assertIn("save_autonomous(state, before)?;", method)
        issued_index = method.index("issue_barrel_transfer(id)?")
        save_index = method.index("save_autonomous(state, before)?;", issued_index)
        insert_index = method.index("game.insert_stack_through(", issued_index)
        extract_index = method.index("game.native_extract_item(", issued_index)
        self.assertLess(issued_index, save_index)
        self.assertLess(save_index, insert_index)
        self.assertLess(save_index, extract_index)
        self.assertIn("confirm_barrel_transfer(id, actual)?", method)

    def test_real_hut_uses_extracted_barrel_escrow_not_abstract_stock(self):
        self.assertIn("pub build_escrow: BTreeMap<Good, u32>", CORE)
        reserve = CORE.split("pub fn reserve_hut_supplies(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("if v.hall_complete", reserve)
        self.assertIn("v.build_escrow.get(&Good::Wood)", reserve)
        self.assertIn("v.build_escrow.get(&Good::Stone)", reserve)
        self.assertIn("v.hut_supplies_reserved = true", reserve)
        self.assertIn("ToBuilder", GLUE)
        self.assertIn("reserve_hut_supplies(id)?", GLUE)

    def test_npc_couriers_and_builder_route_to_real_hall_first(self):
        routes = GLUE.split("fn guide_actors(", 1)[1].split("\n}", 1)[0]
        self.assertIn("if target.issued", routes)
        self.assertIn("if village.wheat_delivery_pending() > 0", routes)
        self.assertIn("village.hall_complete && !village.hut_supplies_reserved", routes)
        self.assertIn("fn farmer_delivery_one_step(", GLUE)
        self.assertIn("deliver_farm_wheat(id)?", GLUE)
        self.assertIn("farmer_delivery_one_step(game, state)", GLUE)

    def test_actual_stock_and_signs_do_not_display_phantom_json_count(self):
        self.assertIn("fn physical_barrel_stock(", GLUE)
        self.assertIn("fn plain_barrel_count(", GLUE)
        self.assertIn("physical_barrel_stock(game, &dimension, village.center)", GLUE)
        self.assertIn("ACTUAL BARRELS", GLUE)
        self.assertIn("Awaiting steward delivery", GLUE)
        self.assertIn("Market order:", GLUE)
        self.assertIn('if action == "hall" || action == "warehouse"', GLUE)

    def test_market_prepays_then_delivers_one_verified_native_slot(self):
        self.assertIn("pub struct MarketPurchase", CORE)
        self.assertIn("pub fn begin_market_purchase(", CORE)
        self.assertIn("pub fn complete_market_purchase(", CORE)
        self.assertIn("pub fn block_market_purchase(", CORE)
        self.assertIn("self.players.get_mut(buyer).unwrap().coins -= price", CORE)
        source = GLUE.split("fn buy_from_real_barrel(", 1)[1].split("\n}", 1)[0]
        self.assertIn("begin_market_purchase(", source)
        self.assertIn("game.native_extract_item(", source)
        self.assertIn("game.native_give_items(", source)
        self.assertIn("complete_market_purchase(", source)
        self.assertIn("quarantined_native_purchase", source)
        self.assertIn("*already_saved = true", source)

    def test_public_barrel_server_inventory_open_is_protected(self):
        self.assertIn("pub fn protect_hall_inventory(", GLUE)
        self.assertIn("native_player_permission(player)", GLUE)
        self.assertIn("*cancel = true;", GLUE)
        self.assertIn("hall_storage_positions(v.center).contains(&position)", GLUE)
        ids = [entry["id"] for entry in MIXIN["injection"]]
        self.assertEqual(len(ids), 7)
        self.assertIn("warehouse_inventory", ids)
        entry = next(item for item in MIXIN["injection"]
                     if item["id"] == "warehouse_inventory")
        self.assertIn("native_inventory_block_start", entry["target"])
        self.assertEqual(entry["helper"], "warehouse_block_start.rs")
        self.assertTrue((ROOT / "mixins/civilizations/warehouse_block_start.rs").is_file())

    def test_simulated_daily_economy_does_not_fund_native_barrels(self):
        self.assertIn("pub stock: BTreeMap<Good, u32>", CORE)
        self.assertIn("pub warehouse: BTreeMap<Good, u32>", CORE)
        self.assertIn("wheat_in_transit", CORE)
        self.assertIn("barrel_transfer: None", CORE)
        self.assertIn("build_escrow: BTreeMap::new()", CORE)
        self.assertIn("market_purchase: None", CORE)

    def test_package_version_and_older_src_contracts(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        self.assertEqual(cargo["package"]["version"], "0.1.13")
        self.assertEqual(MIXIN["mod"]["version"], "0.1.13")
        self.assertIn("pub const SCHEMA: u32 = 1;", CORE)
        self.assertIn('#[serde(default)]\n    pub barrel_transfer:', CORE)
        self.assertIn('#[serde(default)]\n    pub market_purchase:', CORE)
        self.assertIn('#[serde(default)]\n    pub build_escrow:', CORE)
        self.assertIn('#[serde(default)]\n    pub wheat_in_transit:', CORE)


if __name__ == "__main__":
    unittest.main()
