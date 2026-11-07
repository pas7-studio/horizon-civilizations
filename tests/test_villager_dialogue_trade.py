import re
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MIXIN = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()


class VillagerDialogueContracts(unittest.TestCase):
    def test_no_chest_menu_hooks_remain(self):
        ids = {entry["id"] for entry in MIXIN["injection"]}
        self.assertFalse({"menu_click", "menu_input", "menu_close"} & ids)
        for name in ("menu_click.rs", "menu_input.rs", "menu_close.rs"):
            self.assertFalse((ROOT / "mixins/civilizations" / name).exists(), name)
        self.assertNotIn("native_open_menu", GLUE)

    def test_dialogs_are_shown_only_from_the_server_tick(self):
        # The show command refuses to run inside a player's command or hook.
        tick = GLUE.split("pub fn tick(game: &mut Game<'_>) {", 1)[1].split("\n}", 1)[0]
        self.assertIn("process_dialog_queue(game);", tick)
        flush = GLUE.split("pub fn process_dialog_queue(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn('native_run_command(&format!("dialog show {name} {snbt}"))', flush)
        talk = GLUE.split("fn show_talk(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("dialog_queue().push(", talk)
        self.assertNotIn("native_run_command", talk)

    def test_player_names_are_validated_before_building_a_command(self):
        flush = GLUE.split("pub fn process_dialog_queue(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("is_ascii_alphanumeric() || c == '_'", flush)
        self.assertIn("name.len() <= 16", flush)

    def test_right_click_claims_every_village_villager_and_shows_the_window(self):
        body = GLUE.split("pub fn interact_entity(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("show_talk(game, &player, village_id, resident_id, TalkPage::Main, None)", body)
        self.assertIn("Found::Unbound", body)
        self.assertIn("160.0 * 160.0", body)
        self.assertIn("Default villager trading is turned off", body)
        self.assertIn("*claimed = true;", body)

    def test_talk_command_is_public_registered_with_slider_arities(self):
        for usage in ('"civ talk <name:word> <resident:integer>"',
                      '"civ talk <name:word> <resident:integer> <page:word>"',
                      '"civ talk <name:word> <resident:integer> <page:word> <a:word>"',
                      '"civ talk <name:word> <resident:integer> <page:word> <a:word> <b:word>"'):
            self.assertIn(usage, GLUE)
        # Registered with the public list, not the operator list.
        public = GLUE.split("for usage in [", 2)[1].split("] {", 1)[0]
        self.assertIn("civ talk", public)
        perform = GLUE.split("pub fn perform(", 1)[1]
        self.assertLess(perform.index('Some("talk")'), perform.index("let Some(world_dir)"))

    def test_talk_command_rechecks_everything_the_window_sent(self):
        body = GLUE.split("fn talk_command(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("player.dimension != dimension", body)
        self.assertIn("64.0 * 64.0", body)
        self.assertIn("TalkPage::parse(other)", body)
        sale = GLUE.split("fn sell_to_resident(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("parse_sale_args(&talk.wants, &talk.carried, words)", sale)
        self.assertIn("if !talk.is_member", sale)

    def test_sale_is_all_or_nothing_with_rollback_before_and_after_the_save(self):
        sale = GLUE.split("fn sell_to_resident(", 1)[1].split("\n}\n", 1)[0]
        self.assertIn(".sell(&actor, village_id, *good, *count)", sale)
        self.assertIn("native_take_items(good.item(), *count)", sale)
        self.assertIn("native_give_items(", sale)
        self.assertGreaterEqual(sale.count("state.world = Some(previous);"), 3)
        self.assertLess(sale.index("native_take_items"), sale.index("save(state)"))

    def test_only_mayor_and_baker_trade_and_by_theme(self):
        self.assertIn("matches!(job, Job::Merchant | Job::Baker)", CORE)
        wants = CORE.split("pub fn resident_wants(", 1)[1].split("\n}", 1)[0]
        for needle in ("Job::Merchant", "wood_missing", "stone_missing", "Job::Baker"):
            self.assertIn(needle, wants)

    def test_dialog_uses_only_run_command_buttons_and_never_pauses(self):
        snbt = CORE.split("pub fn dialog_snbt(", 1)[1]
        self.assertIn("minecraft:multi_action", snbt)
        self.assertIn("pause:0b", snbt)
        self.assertIn("can_close_with_escape:1b", snbt)
        for kind in ("minecraft:run_command", "minecraft:dynamic/run_command"):
            self.assertIn(kind, snbt)
        # No button may open links or suggest commands.
        self.assertNotIn("open_url", snbt)
        self.assertNotIn("suggest_command", snbt)

    def test_global_sell_buy_commands_are_operator_only(self):
        arm = GLUE.split('"sell" | "buy" => (||', 1)[1].split("let good = Good::parse", 1)[0]
        self.assertIn("player.permission < 2", arm)
        self.assertIn("Villagers trade in person", arm)

    def test_every_dialog_command_is_a_civ_talk_command(self):
        commands = re.findall(r'format!\("civ talk \{\} \{\} \{\}"', CORE)
        self.assertEqual(len(commands), 1)


if __name__ == "__main__":
    unittest.main()
