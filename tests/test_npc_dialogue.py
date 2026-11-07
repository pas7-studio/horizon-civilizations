"""Source contracts for in-world NPC dialogue and vanilla safety.

These checks are in addition to (not a replacement for) pinned native compilation
and in-game right-click E2E.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "mixins" / "civilizations"


class NpcDialogueContracts(unittest.TestCase):
    def test_named_hook_is_present_and_included_in_bundle(self):
        manifest = tomllib.loads((SRC / "horizon-mixin.toml").read_text())
        hooks = {hook["id"]: hook for hook in manifest["injection"]}
        self.assertIn("interact_entity", hooks)
        self.assertEqual(
            "horizon_client::game::native_hooks::Game<'_>::native_interact_entity",
            hooks["interact_entity"]["target"],
        )
        self.assertTrue((SRC / hooks["interact_entity"]["helper"]).is_file())

    def test_vanilla_mobs_are_not_intercepted_or_misidentified(self):
        code = (SRC / "glue.rs").read_text()
        self.assertIn('kind != "minecraft:villager"', code)
        self.assertIn('resident.entity_uuid == Some(uuid)', code)
        self.assertIn('game.native_on(MOD_ID)', code)
        self.assertIn('game.native_tell(player.uuid', code)
        self.assertIn('*claimed = true;', code)


if __name__ == "__main__":
    unittest.main()
