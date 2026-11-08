"""Source contracts for the player-facing physical supply request board.

No runtime tests, Cargo, client build, or Minecraft E2E are run here.
The contracts guard integration points until the user's local build.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def body(source, marker):
    start = source.index(marker)
    start_of_line = source.rfind("\n", 0, start) + 1
    closing = "\n" + " " * (start - start_of_line) + "}\n"
    return source[start:].split(closing, 1)[0]


class PhysicalCivicSupplyRequestContracts(unittest.TestCase):
    def test_pre_hall_demand_counts_only_saved_warehouse_receipts(self):
        requests = body(CORE, "pub fn physical_supply_requests(")
        self.assertIn("if !village.hall_complete {", requests)
        self.assertIn("village.hall_materials_missing()", requests)
        self.assertIn('purpose: "first town hall"', requests)
        self.assertLess(requests.index("if !village.hall_complete {"),
                        requests.index("let stored = stored?;"))

    def test_unavailable_stock_never_looks_like_an_empty_warehouse(self):
        requests = body(CORE, "pub fn physical_supply_requests(")
        self.assertIn("village.barrel_transfer.is_some() || village.market_purchase.is_some()", requests)
        self.assertIn("return None;", requests)
        self.assertIn("let stored = stored?;", requests)
        self.assertIn("physical_construction_shortfall(village, stored)", requests)

    def test_bread_demand_ignores_abstract_stocks_and_counts_incoming(self):
        requests = body(CORE, "pub fn physical_supply_requests(")
        self.assertIn("physical_bread_target(village)", requests)
        self.assertIn("have(Good::Bread)", requests)
        self.assertIn("village.warehouse_quantity(Good::Bread)", requests)
        self.assertIn("bakery.bread_in_transit", requests)
        self.assertNotIn("village.quantity(", requests)

    def test_wheat_request_requires_real_bakery_and_subtracts_existing_supply(self):
        requests = body(CORE, "pub fn physical_supply_requests(")
        self.assertIn("village.bakery.as_ref().filter(|b| b.complete)", requests)
        self.assertIn("BREAD_WHEAT_RECIPE", requests)
        self.assertIn("village.wheat_in_transit", requests)
        self.assertIn("bakery.wheat_escrow", requests)
        self.assertIn("target_wheat.saturating_sub(available_wheat)", requests)

    def test_physical_supply_requests_remain_bounded_without_new_inventory(self):
        requests = body(CORE, "pub fn physical_supply_requests(")
        self.assertIn("amount.min(MAX_HAND_SALE)", requests)
        self.assertIn("bread_missing.min(MAX_HAND_SALE)", requests)
        self.assertNotIn("warehouse_credit(", requests)
        self.assertNotIn("world.advance(", requests)
        self.assertNotIn("native_", requests)

    def test_mayor_accepts_building_goods_and_bread_baker_accepts_wheat(self):
        mapping = body(CORE, "pub fn resident_wants_for_requests(")
        self.assertIn("Job::Merchant => matches!(request.good, Good::Wood | Good::Stone | Good::Bread)", mapping)
        self.assertIn("Job::Baker => request.good == Good::Wheat", mapping)
        self.assertIn("_ => false", mapping)

    def test_steward_is_staffed_before_hall_to_accept_first_materials(self):
        staffing = body(CORE, "pub fn workplace_demand(")
        self.assertIn("want(Job::Merchant, 1);", staffing)
        self.assertNotIn("if self.hall_complete || self.hall_job.is_some()", staffing)

    def test_dialogue_rechecks_live_demand_and_inventory_including_bread(self):
        talk = body(GLUE, "fn read_talk(")
        self.assertIn("physical_supply_requests(", talk)
        self.assertIn("resident_wants_for_requests(", talk)
        self.assertIn("Good::Bread, Good::Wheat", talk)
        self.assertIn("stock_known: requests.is_some()", talk)
        sale = body(GLUE, "fn sell_to_resident(")
        self.assertIn("parse_sale_args(&talk.wants, &talk.carried, words)?", sale)
        self.assertIn("game.native_take_items(", sale)
        self.assertIn("save(state)", sale)

    def test_each_dialogue_command_checks_original_uuid_and_distance(self):
        command = body(GLUE, "fn talk_command(")
        self.assertIn("resident.entity_uuid?", command)
        self.assertIn("game.native_entity_by_uuid(resident_uuid)", command)
        self.assertIn('kind == "minecraft:villager"', command)
        self.assertIn("6.0 * 6.0", command)
        self.assertIn("if !close_to_real_resident", command)
        self.assertLess(command.index("if !close_to_real_resident"),
                        command.index('let (page, notice) = match page_word'))

    def test_public_requests_are_read_only_and_explicit_about_unknown_stock(self):
        block = GLUE.split('if action == "requests" {', 1)[1].split(
            'if action == "meals" {', 1
        )[0]
        self.assertIn("physical_supply_requests(", block)
        self.assertIn("village.dimension == player.dimension", block)
        self.assertIn("let Some(requests) = requests else", block)
        self.assertNotIn("save(state)", block)
        self.assertNotIn("state.world.as_mut()", block)
        self.assertNotIn("native_take_items(", block)
        self.assertIn('"civ requests <name:word>"', GLUE)

    def test_three_slider_sale_has_registered_command_signature(self):
        self.assertIn(
            '"civ talk <name:word> <resident:integer> <page:word> <a:word> <b:word> <c:word>"',
            GLUE
        )
        self.assertIn("format!(\"{} {}\", command_for(input, \"sell\"), args.join(\" \"))", CORE)

    def test_new_rust_regressions_cover_first_hall_missed_stock_and_bakery(self):
        tests = CORE.split("mod physical_civic_supply_requests_tests {", 1)[1].split(
            "mod physical_pit_courier_tests {", 1
        )[0]
        self.assertEqual(tests.count("#[test]"), 5)
        for marker in (
            "first_town_hall_requests_real_missing_materials_without_barrels",
            "completed_hall_never_forges_zero_stock_when_unloaded_or_journalled",
            "bread_is_based_on_real_stock_and_inflight_not_abstract_goods",
            "wheat_demand_appears_only_with_a_completed_physical_bakery",
            "mayor_and_baker_take_only_their_requests_at_existing_prices",
        ):
            self.assertIn(marker, tests)


if __name__ == "__main__":
    unittest.main()
