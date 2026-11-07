"""Offline source contracts for short real-oak multi-log delivery.

These prove structural invariants only, never Rust compilation or in-game E2E.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def section(text: str, needle: str, closing: str = "\n    }") -> str:
    return text.split(needle, 1)[1].split(closing, 1)[0]


class ShortOakStemContracts(unittest.TestCase):
    def test_serde_optional_stem_does_not_break_old_world_saves(self):
        self.assertIn("#[serde(default)]\n    pub oak_stem: Option<OakStem>", CORE)
        self.assertIn("oak_stem: None,", CORE)
        self.assertIn("pub struct OakStem {", CORE)

    def test_candidate_height_is_small_and_rooted(self):
        method = section(CORE, "pub fn short_oak_stem_height(", "\n}")
        self.assertIn("is_rooted_oak_harvest_origin(root,", method)
        self.assertIn("offset < 4", method)
        self.assertIn("(3..=4).contains(&count)", method)

    def test_each_segment_requires_reliable_native_world_snapshot(self):
        method = section(CORE, "pub fn inspect_oak_stem(", "\n}")
        self.assertIn("Some(expected)", method)
        self.assertIn("minecraft:air", method)
        self.assertIn("minecraft:oak_log", method)
        self.assertIn("minecraft:oak_leaves", method)
        self.assertIn("Some(block) = read([x, top_y, z]) else { return false; }", method)
        self.assertIn("stem.cursor == 0", method)

    def test_planner_creates_durable_whole_stem_not_one_unrelated_log(self):
        planner = section(GLUE, "fn autonomous_village_step(", "\n}")
        self.assertIn("short_oak_stem_height(position,", planner)
        self.assertIn("plan_short_oak_stem(id, position, stand, height)", planner)
        self.assertIn("plan_gather(id, good, position, stand)", planner)
        self.assertIn("let prior = state.world.as_ref().unwrap().clone()", planner)
        self.assertIn("save_autonomous(state, prior)?;", planner)
        self.assertNotIn("prior.settlements.get_mut(&id).unwrap().gather_job = None;", planner)

    def test_each_followup_has_its_own_new_unissued_gather(self):
        confirm = section(CORE, "pub fn confirm_gather(")
        self.assertIn("let continuation = if let Some(stem)", confirm)
        self.assertIn("v.warehouse_credit(good, 1)?;", confirm)
        self.assertIn("issued: false, blocked: false", confirm)
        self.assertIn("v.oak_stem.as_mut().unwrap().cursor = cursor", confirm)
        self.assertIn("v.oak_stem = None", confirm)
        self.assertIn("v.gather_job = None", confirm)

    def test_abort_discards_entire_stem_with_zero_credit(self):
        discard = section(CORE, "pub fn discard_blocked_gather(")
        self.assertIn("v.gather_job = None;", discard)
        self.assertIn("v.oak_stem = None;", discard)
        self.assertNotIn("warehouse_credit(", discard)

    def test_load_rejects_orphaned_or_corrupted_tree_cursor(self):
        self.assertIn('ok_or("oak stem saved without an active physical gather")?', CORE)
        self.assertIn('Err("invalid saved short oak stem continuation".into())', CORE)

    def test_resource_is_rechecked_before_every_native_edit(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("inspect_oak_stem(stem, |pos| world_block(game, &dimension, pos))", runner)
        self.assertIn("|| !valid_resource", runner)
        self.assertIn("gather_site_unclaimed(id, job.position, job.stand)", runner)
        self.assertLess(runner.index("|| !valid_resource"),
                        runner.index("mark_gather_issued(id)?"))
        self.assertLess(runner.index("mark_gather_issued(id)?"),
                        runner.index("game.native_run_command(&cmd)"))

    def test_no_free_sapling_or_unverified_materials(self):
        planner = section(GLUE, "fn autonomous_village_step(", "\n}")
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        for source in (planner, runner):
            self.assertNotIn("minecraft:oak_sapling", source)
            self.assertNotIn("native_give_items(", source)
        self.assertIn("v.warehouse_credit(good, 1)?;", section(CORE, "pub fn confirm_gather("))


if __name__ == "__main__":
    unittest.main()
