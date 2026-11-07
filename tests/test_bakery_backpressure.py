"""Bounded native bakery production source contracts, NOT runtime tests."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


class NativeBakeryBackpressureContracts(unittest.TestCase):
    def test_target_depends_on_residents_but_has_an_explicit_limit(self):
        section = CORE.split("pub fn physical_bread_target(", 1)[1].split("\n}", 1)[0]
        self.assertIn("village.population().saturating_mul(2).min(64)", section)

    def test_restock_uses_only_real_items_and_confirmed_pending_receipts(self):
        section = CORE.split("pub fn needs_real_bread(", 1)[1].split("\n}", 1)[0]
        self.assertIn("village.warehouse_quantity(Good::Bread)", section)
        self.assertIn("b.bread_in_transit", section)
        self.assertIn("observed_barrel_bread.saturating_add(pending)", section)
        self.assertNotIn("village.stock", section)

    def test_wheat_pickup_rechecks_actual_bread_before_using_native_barrel(self):
        section = GLUE.split("let bakery_needs_wheat =", 1)[1].split(
            "let can_collect =", 1
        )[0]
        self.assertIn("physical_barrel_stock(game, &dimension, village.center)", section)
        self.assertIn("needs_real_bread(&village, physical_bread)", section)
        self.assertIn("request_workshop_pickup(", section)
        self.assertNotIn("else { continue; }", section)
        self.assertNotIn("!crate::game::civilizations_core::needs_real_bread", section)

    def test_existing_wheat_is_not_crafted_when_real_bread_stock_full(self):
        section = GLUE.split("fn baker_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("physical_barrel_stock(game, &dimension, village.center)", section)
        self.assertIn("needs_real_bread(village, physical_bread)", section)
        self.assertLess(section.index("needs_real_bread(village, physical_bread)"),
                        section.index("craft_bread(id)?"))

    def test_bread_delivery_is_unchanged_and_happens_before_restock_guard(self):
        section = GLUE.split("fn baker_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertLess(section.index("deliver_bakery_bread(id)?"),
                        section.index("needs_real_bread(village, physical_bread)"))
        self.assertIn("b.bread_in_transit > 0", section)


if __name__ == "__main__":
    unittest.main()
