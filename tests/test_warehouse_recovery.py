"""Source-level regression contracts for explicit operator warehouse recovery.

Does not certify Rust compilation, native hook integration or in-game behavior.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


class OperatorWarehouseRecoveryContracts(unittest.TestCase):
    def test_operator_commands_are_registered_with_op_permissions(self):
        self.assertIn('"civ barrelcheck <name:word>"', GLUE)
        self.assertIn('"civ resolvebarrel <name:word> <mode:word> <count:integer>"', GLUE)
        registration = GLUE.split("pub fn register(", 1)[1].split("\n}", 1)[0]
        self.assertIn("NativeCommand::op(usage, 2)", registration)

    def test_recovery_does_not_trigger_new_external_item_mutations(self):
        action = GLUE.split('"resolvebarrel" if player.permission >= 2', 1)[1]
        action = action.split('"recoverfarm" if player.permission >= 2', 1)[0]
        self.assertIn("plain_barrel_count(game,", action)
        self.assertIn("if actual != manually_observed", action)
        self.assertIn("t.expected_after() != Some(actual)", action)
        self.assertIn("t.phase != BarrelPhase::Blocked", action)
        self.assertIn("t.phase != BarrelPhase::Planned", action)
        self.assertNotIn("native_extract_item(", action)
        self.assertNotIn("insert_stack_through(", action)
        self.assertNotIn("native_give_items(", action)

    def test_cancel_only_never_issued_intents(self):
        cancel = CORE.split("pub fn cancel_planned_barrel_transfer(", 1)[1]
        cancel = cancel.split("\n    }", 1)[0]
        self.assertIn("t.phase != BarrelPhase::Planned", cancel)
        self.assertIn("v.barrel_transfer = None;", cancel)
        self.assertIn("v.market_purchase.is_some()", cancel)

    def test_confirm_blocked_receipt_uses_existing_once_only_commit(self):
        confirm = CORE.split("pub fn reconcile_blocked_barrel_transfer(", 1)[1]
        confirm = confirm.split("\n    }", 1)[0]
        self.assertIn("t.phase != BarrelPhase::Blocked", confirm)
        self.assertIn("t.expected_after() != Some(observed_after)", confirm)
        self.assertIn("let backup = self.clone();", confirm)
        self.assertIn("self.confirm_barrel_transfer(id, observed_after)", confirm)
        self.assertIn("*self = backup;", confirm)
        self.assertNotIn("warehouse_credit(", confirm)

    def test_read_only_audit_requires_operator_and_actual_native_snapshot(self):
        audit = GLUE.split('if action == "barrelcheck" {', 1)[1]
        audit = audit.split('if action == "bakery" {', 1)[0]
        self.assertIn("player.permission < 2", audit)
        self.assertIn("v.dimension != player.dimension", audit)
        self.assertIn("plain_barrel_count(game, &v.dimension, t.position, t.good)", audit)
        self.assertIn("current native count", audit)

    def test_hall_signs_reuse_actual_combined_demand_and_pause_on_journal(self):
        boards = GLUE.split("fn refresh_hall_boards(", 1)[1]
        boards = boards.split("\n}", 1)[0]
        self.assertIn("physical_barrel_stock(game, &dimension, village.center)", boards)
        self.assertIn("physical_construction_shortfall(village, &stored)", boards)
        self.assertIn("village.barrel_transfer.is_some()", boards)
        self.assertIn("village.market_purchase.is_some()", boards)
        self.assertIn('"PENDING TX".to_owned()', boards)
        self.assertNotIn("HUT_WOOD_LOGS", boards)


if __name__ == "__main__":
    unittest.main()
