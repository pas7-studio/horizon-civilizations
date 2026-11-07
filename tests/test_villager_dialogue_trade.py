import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MIXIN = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()


class VillagerMenuContracts(unittest.TestCase):
    def test_menu_hooks_are_registered_with_existing_helpers(self):
        by_id = {entry["id"]: entry for entry in MIXIN["injection"]}
        expected = {
            "menu_click": "native_menu_clicked",
            "menu_input": "native_menu_input_closed",
            "menu_close": "native_menu_closed",
        }
        for key, hook in expected.items():
            self.assertTrue(by_id[key]["target"].endswith(hook), key)
            self.assertTrue((ROOT / "mixins/civilizations" / by_id[key]["helper"]).is_file())

    def test_right_click_queues_the_menu_instead_of_opening_it_in_the_hook(self):
        body = GLUE.split("pub fn interact_entity(", 1)[1].split("\n/// The conversation as", 1)[0]
        # The hub is out of reach inside the click hook: only queue there.
        self.assertIn("menu_queue().push(PendingOpen", body)
        self.assertNotIn("native_open_menu", body)
        self.assertIn("*claimed = true;", body)
        # Opening happens from the server tick, every tick.
        tick = GLUE.split("pub fn tick(game: &mut Game<'_>) {", 1)[1].split("\n}", 1)[0]
        self.assertIn("process_menu_queue(game);", tick)
        queue = GLUE.split("pub fn process_menu_queue(", 1)[1].split("\n}", 1)[0]
        self.assertIn("open_page(game", queue)

    def test_default_villager_trading_is_blocked_for_unlinked_villagers_too(self):
        body = GLUE.split("pub fn interact_entity(", 1)[1].split("\n/// The conversation as", 1)[0]
        self.assertIn("Found::Unbound", body)
        self.assertIn("160.0 * 160.0", body)
        self.assertIn("Default villager trading is turned off", body)

    def test_pages_open_with_input_slots_only_for_trading(self):
        opener = GLUE.split("fn open_page(", 1)[1].split("\n}", 1)[0]
        self.assertIn("native_open_menu_with_input(uuid, DIALOGUE_ROWS, title, items, TRADE_INPUT)", opener)
        self.assertIn("native_open_menu(uuid, DIALOGUE_ROWS, title, items)", opener)
        self.assertIn("page == MenuPage::Trade", opener)

    def test_clicks_are_dispatched_by_action_and_never_move_items(self):
        click = GLUE.split("pub fn menu_clicked(", 1)[1].split("\n}\n", 1)[0]
        for action in ("MenuAction::Close", "MenuAction::Go(page)", "MenuAction::Join",
                       "MenuAction::Sell"):
            self.assertIn(action, click)
        self.assertNotIn("native_exchange_items", GLUE)

    def test_only_a_confirmed_sale_takes_goods_and_failures_return_them(self):
        closed = GLUE.split("pub fn menu_input_closed(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("s.confirm_sale", closed)
        self.assertIn("MenuPage::Trade", closed)
        sale = GLUE.split("fn sell_offered(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("settle_offer(&session.wants", sale)
        self.assertIn(".sell(&actor, session.village_id", sale)
        self.assertGreaterEqual(sale.count("state.world = Some(previous);"), 2)
        # Slots are emptied only after the sale has been saved.
        self.assertLess(sale.index("save(state)"), sale.index("slots.iter_mut()"))

    def test_only_mayor_and_baker_trade_and_by_theme(self):
        self.assertIn("matches!(job, Job::Merchant | Job::Baker)", CORE)
        wants = CORE.split("pub fn resident_wants(", 1)[1].split("\n}", 1)[0]
        for needle in ("Job::Merchant", "wood_missing", "stone_missing", "Job::Baker"):
            self.assertIn(needle, wants)

    def test_global_sell_buy_commands_are_operator_only(self):
        arm = GLUE.split('"sell" | "buy" => (||', 1)[1].split("let good = Good::parse", 1)[0]
        self.assertIn("player.permission < 2", arm)
        self.assertIn("Villagers trade in person", arm)


if __name__ == "__main__":
    unittest.main()
