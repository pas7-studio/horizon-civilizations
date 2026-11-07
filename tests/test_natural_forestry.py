"""Source-level contracts for rooted-tree harvesting and durable gather quarantine.

These textual checks are not Rust compile, native block execution, or gameplay.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


class NaturalForestryAndGatherJournalContracts(unittest.TestCase):
    def test_tree_candidate_is_rooted_and_has_upper_crown(self):
        section = CORE.split("pub fn is_rooted_oak_harvest_origin(", 1)[1].split(
            "\n}", 1
        )[0]
        for token in ("minecraft:oak_log", "minecraft:grass_block",
                      "minecraft:dirt", "minecraft:oak_leaves", "upper_logs >= 2 && crown"):
            self.assertIn(token, section)
        self.assertIn("None => return false", section)

    def test_native_selector_does_not_guess_from_single_adjacent_leaf(self):
        selector = GLUE.split("fn seems_natural_resource(", 1)[1].split(
            "\n}", 1
        )[0]
        self.assertIn("is_rooted_oak_harvest_origin(point,", selector)
        self.assertIn("Good::Stone => world_block(", selector)
        self.assertNotIn("for (dx, dy, dz)", selector)

    def test_issued_marker_persists_before_native_block_mutation(self):
        runner = GLUE.split("fn gather_one_step(", 1)[1].split("\n}", 1)[0]
        issued = runner.index("mark_gather_issued(id)?")
        durable = runner.index("save_autonomous(state, before)?", issued)
        native = runner.index("game.native_run_command(&cmd)", issued)
        self.assertLess(issued, durable)
        self.assertLess(durable, native)
        self.assertIn("block_gather(id)?", runner)

    def test_live_source_provenance_is_checked_at_issue_time(self):
        runner = GLUE.split("fn gather_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("let valid_resource = if let Some(stem) = saved_stem", runner)
        self.assertIn("inspect_oak_stem(stem, |pos| world_block(", runner)
        self.assertIn("seems_natural_resource(game, &dimension, job.position, job.good)", runner)
        self.assertIn("|| !valid_resource", runner)
        self.assertIn("!state.world.as_ref().unwrap().gather_site_unclaimed(id, job.position, job.stand)", runner)
        self.assertLess(runner.index("|| !valid_resource"),
                        runner.index("mark_gather_issued(id)?"))

    def test_uncertain_issued_gather_is_not_automatically_discarded(self):
        planner = GLUE.split("fn autonomous_village_step(", 1)[1].split(
            "\n}", 1
        )[0]
        self.assertIn("job.blocked && !job.issued", planner)
        self.assertIn("discard_blocked_gather(id)?", planner)
        self.assertIn("if village.gather_job.is_none()", planner)
        self.assertNotIn("if village.gather_job.as_ref().is_some_and(|job| job.blocked) {", planner)

    def test_operator_abandonment_never_creates_native_items(self):
        method = GLUE.split('"discardgather" if player.permission >= 2', 1)[1].split(
            '"resolvebarrel" if player.permission >= 2', 1
        )[0]
        self.assertIn("job.blocked || !job.issued", method)
        self.assertIn('"intact" => observed == job.source_block()', method)
        self.assertIn('"empty" => observed == "minecraft:air"', method)
        self.assertIn("abandon_blocked_issued_gather(v.id)?", method)
        for unsafe in ("game.native_run_command(", "native_extract_item(",
                       "native_give_items(", "warehouse_credit("):
            self.assertNotIn(unsafe, method)

    def test_registered_land_is_revalidated_using_shared_policy(self):
        checker = CORE.split("pub fn gather_site_unclaimed(", 1)[1].split("\n    }", 1)[0]
        plan = CORE.split("pub fn plan_gather(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("other.player_parcels.iter().any", checker)
        self.assertIn("other.visual_sites.iter().any", checker)
        self.assertIn("other.bakery.as_ref().is_some_and", checker)
        self.assertIn("other.farm.as_ref().is_some_and", checker)
        self.assertIn("self.gather_site_unclaimed(village_id, position, stand)", plan)

    def test_diagnostics_distinguish_issued_quarantine(self):
        diagnosis = GLUE.split('if action == "diagnose" {', 1)[1].split(
            'if action == "gathercheck" {', 1
        )[0]
        self.assertIn("job.issued", diagnosis)
        self.assertIn("issued wood/stone gather QUARANTINED", diagnosis)
        self.assertIn("never-issued gather candidate blocked", diagnosis)

    def test_operators_can_audit_live_source_without_mutating(self):
        self.assertIn('"civ gathercheck <name:word>"', GLUE)
        audit = GLUE.split('if action == "gathercheck" {', 1)[1].split(
            'if action == "barrelcheck" {', 1
        )[0]
        self.assertIn("player.permission < 2", audit)
        self.assertIn("world_block(game, &v.dimension, job.position)", audit)
        self.assertNotIn("save(state)", audit)


if __name__ == "__main__":
    unittest.main()
