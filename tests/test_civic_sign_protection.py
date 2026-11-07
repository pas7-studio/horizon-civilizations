import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MIXIN = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()


class CivicSignProtectionContracts(unittest.TestCase):
    def test_both_hooks_are_registered_with_existing_helpers(self):
        by_id = {entry["id"]: entry for entry in MIXIN["injection"]}
        self.assertTrue(by_id["sign_click"]["target"].endswith("native_right_click_block"))
        self.assertTrue(by_id["sign_edit"]["target"].endswith("native_sign_changed"))
        for key in ("sign_click", "sign_edit"):
            self.assertTrue((ROOT / "mixins/civilizations" / by_id[key]["helper"]).is_file())

    def test_glue_cancels_only_registered_signs_for_everyone(self):
        click = GLUE.split("pub fn protect_civic_sign_click(", 1)[1].split("\n}", 1)[0]
        self.assertIn("civic_sign_at(game, position)", click)
        self.assertIn("*canceled = true;", click)
        # No operator exemption: board text is only rewritten on state change.
        self.assertNotIn("permission", click)
        probe = GLUE.split("fn civic_sign_at(", 1)[1].split("\n}", 1)[0]
        self.assertIn('ends_with("_sign")', probe)
        self.assertIn("world.is_civic_sign(", probe)

    def test_core_registry_covers_every_board_kind(self):
        body = CORE.split("pub fn is_civic_sign(", 1)[1].split("pub fn player_may_break", 1)[0]
        for needle in ("hall_all_board_positions", "legacy_hall_board_positions",
                       "site_board_position", "v.posts"):
            self.assertIn(needle, body)


if __name__ == "__main__":
    unittest.main()
