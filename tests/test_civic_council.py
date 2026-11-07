"""Offline contracts for deterministic, read-only settlement council advice.

These source checks are not a Rust compiler or a Horizon game test.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()


class CivicCouncilContracts(unittest.TestCase):
    def test_council_needs_are_explicit_and_ranked_deterministically(self):
        self.assertIn("pub enum CivicNeed", CORE)
        for need in ("Food", "Security", "Stability", "Identity", "Housing", "Materials"):
            self.assertIn(f"    {need},", CORE)
        self.assertIn("b.urgency.cmp(&a.urgency).then(a.need.cmp(&b.need))", CORE)
        self.assertIn("priorities.truncate(5)", CORE)

    def test_forecast_is_bounded_and_uses_the_existing_daily_economy(self):
        self.assertIn("pub const COUNCIL_FORECAST_DAYS: u8 = 3;", CORE)
        self.assertIn("pub const MAX_COUNCIL_FORECAST_DAYS: u8 = 7;", CORE)
        body = CORE.split("pub fn forecast_abstract_days(&self", 1)[1].split("\n    }", 1)[0]
        self.assertIn("let mut hypothetical = self.clone();", body)
        self.assertIn("hypothetical.day();", body)
        self.assertIn("days.min(MAX_COUNCIL_FORECAST_DAYS)", body)
        self.assertNotIn("self.day();", body)
        self.assertNotIn("self.advance(", body)
        self.assertNotIn("save(", body)

    def test_risk_model_separates_hunger_war_housing_materials(self):
        body = CORE.split("pub fn council_priorities(&self", 1)[1].split("\n    }", 1)[0]
        self.assertIn("self.starving_days > 0", body)
        self.assertIn("forecast_abstract_days(COUNCIL_FORECAST_DAYS)", body)
        self.assertIn("self.active_wars.is_empty()", body)
        self.assertIn("self.physical_housing()", body)
        self.assertIn("self.pending_visual_buildings > 0", body)
        self.assertIn("self.actors_spawned", body)
        self.assertIn("self.count(Job::Builder) > 0", body)
        self.assertNotIn("native_give_items", body)
        self.assertNotIn("native_run_command", body)

    def test_read_only_commands_register_without_operator_privileges(self):
        self.assertIn('"civ council <name:word>"', GLUE)
        self.assertIn('"civ forecast <name:word>"', GLUE)
        self.assertIn('if action == "council" {', GLUE)
        self.assertIn('if action == "forecast" {', GLUE)
        for name in ("council", "forecast"):
            body = GLUE.split(f'if action == "{name}" {{', 1)[1]
            body = body.split('    if action == "', 1)[0]
            self.assertNotIn("save(state)", body)
            self.assertNotIn("start_visual_house", body)
            self.assertNotIn("native_run_command", body)
            self.assertIn("return;", body)

    def test_abstract_forecast_exclusions_are_visible_in_chat(self):
        self.assertIn("Excludes World war attrition", GLUE)
        self.assertIn("player trades, inventory, physical crops", GLUE)
        self.assertIn("Read-only advice; no items, land claims, orders or block edits.", GLUE)

    def test_npc_dialogue_shows_the_actual_top_council_concern(self):
        # The conversation text is built by the pure dialogue_menu in the core.
        self.assertIn("village.council_priorities().first()", CORE)
        self.assertIn("The council worries about {} ({}/100).", CORE)
        self.assertIn("The council is calm.", CORE)
        self.assertIn('resident.entity_uuid == Some(uuid)', GLUE)

    def test_local_validator_does_not_enable_ci_or_run_gameplay(self):
        validator = (ROOT / "scripts/validate-local.ps1").read_text()
        self.assertIn("cargo", validator)
        self.assertIn("rustfmt", validator)
        self.assertIn("clippy", validator)
        self.assertIn("unittest", validator)
        self.assertIn("tools/package.py", validator)
        self.assertIn("tools/verify_static.py", validator)
        self.assertIn("tools/native_build.py", validator)
        self.assertIn("BuildClient", validator)
        self.assertNotIn("gh workflow run", validator)
        self.assertNotIn("gh api", validator)
        self.assertNotIn("actions/workflows", validator)

    def test_version_and_legacy_schema_are_explicit(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        mixin = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        self.assertEqual(cargo["package"]["version"], mixin["mod"]["version"])
        self.assertEqual(mixin["mod"]["version"], cargo["package"]["version"])
        self.assertEqual(len(mixin["injection"]), 11)
        self.assertIn("pub const SCHEMA: u32 = 1;", CORE)
        self.assertNotIn("pub council:", CORE)
        self.assertNotIn("pub forecast:", CORE)


if __name__ == "__main__":
    unittest.main()
