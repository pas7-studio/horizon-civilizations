"""Source-only safety contracts for v0.1.9 village housing proposals.

These guard the interface without claiming Rust compilation or game E2E.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()


class VillageHousingProposalContracts(unittest.TestCase):
    def test_deterministic_proposal_inventory_is_bounded(self):
        self.assertIn("pub const HOUSING_PLOT_OFFSETS: [[i32; 2]; 20]", CORE)
        self.assertIn("pub const MAX_HOUSING_SURVEY_RESULTS: usize = 5;", CORE)
        self.assertIn("slot.checked_sub(1)", CORE)
        self.assertIn("HOUSING_PLOT_OFFSETS.get(", CORE)
        self.assertIn("pub fn proposed_hut(&self", CORE)

    def test_domain_checks_all_same_dimension_structure_and_farm_overlaps(self):
        check = CORE.split("pub fn check_visual_house_site(", 1)[1]
        check = check.split("\n    }", 1)[0]
        self.assertIn("other.dimension == village.dimension", check)
        for part in ("other.visual_sites", "other.visual_job", "other.farm"):
            self.assertIn(part, check)
        self.assertIn("distance(site, origin)", check)
        self.assertIn("distance(job.origin, origin)", check)
        self.assertIn("distance(farm.origin, origin)", check)

    def test_manual_and_proposed_construction_share_same_safety_policy(self):
        proposal = CORE.split("pub fn proposed_hut(", 1)[1].split("\n    }", 1)[0]
        approval = CORE.split("pub fn start_visual_house(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("self.check_visual_house_site(village_id, origin)?", proposal)
        self.assertIn("self.check_visual_house_site(village_id, origin)?", approval)
        self.assertIn("village.physical_housing().2 == 0", proposal)
        self.assertNotIn("visual_job = Some(", proposal)

    def test_survey_is_read_only_and_uses_actual_loaded_world(self):
        survey = GLUE.split('if action == "survey" {', 1)[1].split('\n    if action == "homes"', 1)[0]
        self.assertIn("world.proposed_hut(village.id, slot, y)", survey)
        self.assertIn("check_empty_hut_site(game, &village.dimension, site)", survey)
        self.assertIn("MAX_HOUSING_SURVEY_RESULTS", survey)
        self.assertNotIn("save(state)", survey)
        self.assertNotIn("start_visual_house(", survey)

    def test_approval_requires_operator_proximity_and_rechecks_real_blocks(self):
        handler = GLUE.split('"approvehouse" if player.permission >= 2 => (|| {', 1)[1]
        handler = handler.split("\n        })(),", 1)[0]
        self.assertIn("previous.proposed_hut(village.id, slot, y)?", handler)
        self.assertIn("dx * dx + dz * dz > 24.0 * 24.0", handler)
        self.assertIn("(f64::from(y) - py).abs() > 8.0", handler)
        terrain = handler.index("check_empty_hut_site(game, &village.dimension, site)?")
        reserve = handler.index("start_visual_house(village.id, site)?")
        self.assertLess(terrain, reserve)

    def test_no_new_horizon_hooks_or_save_schema(self):
        mixin = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        self.assertEqual(cargo["package"]["version"], mixin["mod"]["version"])
        self.assertEqual(mixin["mod"]["version"], "0.1.9")
        self.assertEqual(len(mixin["injection"]), 6)
        self.assertIn("pub const SCHEMA: u32 = 1", CORE)
        self.assertNotIn("pub proposed_sites:", CORE)

    def test_explicit_operator_command_and_read_only_survey_registration(self):
        self.assertIn('"civ survey <name:word>"', GLUE)
        self.assertIn('"civ approvehouse <name:word> <slot:integer> <y:integer>"', GLUE)
        self.assertIn('if action == "survey"', GLUE)
        self.assertIn('"approvehouse" if player.permission >= 2', GLUE)
        self.assertIn('"approvehouse" | "farm"', GLUE)


if __name__ == "__main__":
    unittest.main()
