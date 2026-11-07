"""Offline source contracts for the visual harvest receipt journal."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()

class HarvestReceiptContracts(unittest.TestCase):
    def test_source_contains_bounded_receipt_journal(self):
        for part in ("pub struct HarvestReceipt", "harvest_receipts.push",
                     "MAX_HARVEST_RECEIPTS", "checked_add(1)",
                     "pub harvest_sequence: u64"):
            self.assertIn(part, CORE)

    def test_read_only_command_and_no_stock_credit(self):
        self.assertIn('"civ harvests <name:word>"', GLUE)
        self.assertIn('if action == "harvests"', GLUE)
        self.assertIn("No physical stock has been credited", GLUE)
        start = CORE.index("FarmStage::Harvesting => {")
        end = CORE.index("FarmStage::Replanting => {", start)
        self.assertNotIn("stock.insert", CORE[start:end])

    def test_package_and_windows_installer_version_match(self):
        self.assertIn('version = "0.1.5"',
                      (ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        self.assertIn("civilizations-0.1.5.hnmod",
                      (ROOT / "scripts/install-local.ps1").read_text())
