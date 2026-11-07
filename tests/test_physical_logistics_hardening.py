"""Unreleased source-only regression contracts for physical logistics hardening.

These are NOT a substitute for cargo check/test or live Horizon gameplay.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


class PhysicalLogisticsHardeningContracts(unittest.TestCase):
    def test_bakery_native_transfer_rules_are_inside_saved_journal_scope(self):
        section = CORE.split("if let Some(t) = &village.barrel_transfer {", 1)[1]
        section = section.split("if village.hall_complete && village.hall_job.is_some()", 1)[0]
        self.assertEqual(section.count("match t.direction"), 2)
        self.assertIn("BarrelDirection::ToBakerIngredients", section)
        self.assertIn("BarrelDirection::ToBakeryBuilder", section)
        self.assertIn("b.building.as_ref().is_none_or(|job| job.blocked)", section)
        self.assertIn("!b.complete || b.bread_in_transit != 0", section)
        self.assertTrue(section.rstrip().endswith("}"))

    def test_combined_bills_are_distinct_and_subtract_native_and_inbound_stock_once(self):
        method = CORE.split("pub fn physical_construction_shortfall(", 1)[1].split("\n}", 1)[0]
        for token in (
            "BAKERY_WOOD_LOGS", "BAKERY_COBBLESTONE",
            "HUT_WOOD_LOGS", "HUT_COBBLESTONE",
            "b.construction_escrow.get(&good)",
            "village.build_escrow.get(&good)",
            "bakery_bill.saturating_add(hut_bill)",
            "village.warehouse_quantity(good)",
            "unpaid(Good::Wood).saturating_sub(available(Good::Wood))",
        ):
            self.assertIn(token, method)

    def test_planner_waits_for_unresolved_native_journal_and_unavailable_barrels(self):
        method = GLUE.split("fn autonomous_village_step(", 1)[1].split("\n}", 1)[0]
        block = method.split("let (wood_needed, stone_needed)", 1)[1].split(
            "if village.gather_job.is_none()", 1
        )[0]
        self.assertIn("village.barrel_transfer.is_some()", block)
        self.assertIn("village.market_purchase.is_some()", block)
        self.assertIn("physical_barrel_stock(game, &dimension, village.center)", block)
        self.assertIn("physical_construction_shortfall(", block)
        self.assertLess(block.index("village.barrel_transfer.is_some()"),
                        block.index("physical_barrel_stock("))

    def test_blocked_bakery_does_not_withdraw_further_materials(self):
        section = GLUE.split("let bakery_builder =", 1)[1].split("if bakery_builder", 1)[0]
        self.assertIn("job.blocked", section)
        self.assertIn("!b.supplies_reserved", section)

    def test_operator_civic_warehouse_reports_combined_need_not_single_hut(self):
        section = GLUE.split('if action == "hall" || action == "warehouse"', 1)[1]
        section = section.split('if action == "bakery"', 1)[0]
        self.assertIn("physical_construction_shortfall(", section)
        self.assertIn("Combined pending construction deficit", section)
        self.assertNotIn("Next hut needs", section)


if __name__ == "__main__":
    unittest.main()
