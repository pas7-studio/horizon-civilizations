import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()


class VillagerDialogueTradeContracts(unittest.TestCase):
    def test_right_click_opens_a_display_only_menu_with_chat_fallback(self):
        body = GLUE.split("pub fn interact_entity(", 1)[1].split("\nfn sell_held_to_resident(", 1)[0]
        self.assertIn("native_open_menu(player.uuid, DIALOGUE_ROWS", body)
        self.assertIn("native_tell(", body)  # fallback when no menu can open
        self.assertIn("dialogue_menu(&DialogueInput", body)
        # Display-only: the menu must not register click handlers or move items.
        self.assertNotIn("native_menu_clicked", GLUE)
        self.assertNotIn("native_exchange_items", GLUE)

    def test_only_mayor_and_baker_trade_and_by_theme(self):
        self.assertIn("matches!(job, Job::Merchant | Job::Baker)", CORE)
        wants = CORE.split("pub fn resident_wants(", 1)[1].split("\n}", 1)[0]
        self.assertIn("Job::Merchant", wants)
        self.assertIn("wood_missing", wants)
        self.assertIn("stone_missing", wants)
        self.assertIn("Job::Baker", wants)

    def test_hand_sale_uses_checked_saved_and_rolled_back_path(self):
        sale = GLUE.split("fn sell_held_to_resident(", 1)[1].split("\n}", 1)[0]
        self.assertIn(".sell(&actor, village_id, good, count)", sale)
        self.assertIn("native_take_items(item, count)", sale)
        self.assertIn("state.world = Some(previous);", sale)
        self.assertIn("save(state)", sale)
        # Rollback appears for both a failed removal and a failed save.
        self.assertGreaterEqual(sale.count("state.world = Some(previous);"), 2)

    def test_global_sell_buy_commands_are_operator_only(self):
        arm = GLUE.split('"sell" | "buy" => (||', 1)[1].split("let good = Good::parse", 1)[0]
        self.assertIn("player.permission < 2", arm)
        self.assertIn("Villagers trade in person", arm)


if __name__ == "__main__":
    unittest.main()
