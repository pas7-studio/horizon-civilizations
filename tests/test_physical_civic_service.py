"""Static contracts for physical supply service and visible courier state.

These tests are authored for later local execution. They do not replace
Cargo/Rust or the actual Horizon first-play gate.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def routine(source, marker):
    start = source.index(marker)
    start_of_line = source.rfind("\n", 0, start) + 1
    indent = start - start_of_line
    return source[start:].split("\n" + " " * indent + "}\n", 1)[0]


class CivicServiceAndLogisticsContracts(unittest.TestCase):
    def test_no_duplicate_top_level_settlement_definition(self):
        self.assertEqual(CORE.count("pub struct Settlement {"), 1)
        settlement = CORE.split("pub struct Settlement {", 1)[1].split("\n}", 1)[0]
        self.assertEqual(len(re.findall(r"^    pub houses: u32,$", settlement, re.M)), 1)

    def test_persistent_player_delivery_map_is_backward_compatible(self):
        player = CORE.split("pub struct Player {", 1)[1].split("\n}", 1)[0]
        self.assertIn('skip_serializing_if = "BTreeMap::is_empty"', player)
        self.assertIn("pub civic_deliveries: BTreeMap<u64, BTreeMap<Good, u64>>", player)
        self.assertIn("civic_deliveries: BTreeMap::new()", CORE)

    def test_service_tiers_are_derived_not_player_editable_flags(self):
        tiers = routine(CORE, "pub fn civic_service_tier(")
        for amount in ("0..=31", "32..=127", "128..=511"):
            self.assertIn(amount, tiers)
        self.assertIn("CivicServiceTier::Benefactor", tiers)
        self.assertNotIn("warehouse_credit", tiers)

    def test_sale_records_volume_on_existing_real_item_transfer_path(self):
        sale = routine(CORE, "pub fn sell(")
        self.assertIn("s.warehouse_credit(good, count)?", sale)
        self.assertIn("p.civic_deliveries.entry(village_id)", sale)
        self.assertIn("contributed.saturating_add(u64::from(count))", sale)
        self.assertLess(sale.index("s.warehouse_credit(good, count)?"),
                        sale.index("p.civic_deliveries.entry(village_id)"))

    def test_bonuses_are_once_per_threshold_crossing_not_repeatable_claims(self):
        sale = routine(CORE, "pub fn sell(")
        self.assertIn("previous_tier = civic_service_tier(", sale)
        self.assertIn("next_tier = civic_service_tier(", sale)
        self.assertIn("next_tier.ordinal() - previous_tier.ordinal()", sale)
        self.assertIn("rep.saturating_add(1 + tier_bonus)", sale)
        self.assertNotIn("coins += tier_bonus", sale)

    def test_service_counts_have_persisted_validation_and_upper_cap(self):
        self.assertIn("MAX_CIVIC_DELIVERIES_PER_GOOD: u64 = 10_000_000", CORE)
        validate = routine(CORE, "pub fn validate(")
        self.assertIn("p.civic_deliveries.iter()", validate)
        self.assertIn("count > MAX_CIVIC_DELIVERIES_PER_GOOD", validate)
        self.assertIn("invalid saved physical civic contribution ledger", validate)

    def test_service_command_exposes_only_current_authenticated_player(self):
        route = GLUE.split('if action == "service" {', 1)[1].split(
            'if action == "logistics" {', 1
        )[0]
        self.assertIn("world.players.get(&actor)", route)
        self.assertIn("p.civic_delivery_total(village.id)", route)
        self.assertIn("tier.next_at()", route)
        self.assertNotIn("world.players.values()", route)
        self.assertNotIn("state.world.as_mut()", route)

    def test_logistics_shows_real_gather_receipt_sequence(self):
        route = GLUE.split('if action == "logistics" {', 1)[1].split(
            'if action == "requests" {', 1
        )[0]
        for marker in ("job.blocked", "!job.issued", "!job.native_edit_confirmed",
                       "!job.pickup_witnessed", "job.worker_id", "v.residents.iter()"):
            self.assertIn(marker, route)
        self.assertIn("waiting for same worker at the hall", route)

    def test_logistics_shows_inbound_food_and_pending_native_journal(self):
        route = GLUE.split('if action == "logistics" {', 1)[1].split(
            'if action == "requests" {', 1
        )[0]
        for marker in ("v.wheat_in_transit", "b.bread_in_transit",
                       "v.warehouse_quantity(*good)", "v.barrel_transfer",
                       "v.market_purchase"):
            self.assertIn(marker, route)

    def test_logistics_unloaded_stock_is_not_fake_zero(self):
        route = GLUE.split('if action == "logistics" {', 1)[1].split(
            'if action == "requests" {', 1
        )[0]
        self.assertIn("v.hall_complete && v.dimension == player.dimension", route)
        self.assertIn("physical_barrel_stock(game, &v.dimension, v.center)", route)
        self.assertIn("unavailable/unloaded", route)
        self.assertIn("other dimension (not observed)", route)

    def test_both_commands_are_registered_and_read_only(self):
        for command in ('"civ service <name:word>"', '"civ logistics <name:word>"'):
            self.assertIn(command, GLUE)
        route = GLUE.split('if action == "service" {', 1)[1].split(
            'if action == "requests" {', 1
        )[0]
        for forbidden in ("save(state)", "state.world.as_mut()", "native_take_items(",
                          "native_run_command(", "native_spawn_mob("):
            self.assertNotIn(forbidden, route)

    def test_rust_tests_cover_migration_thresholds_restart_and_invalid_records(self):
        cases = CORE.split("mod physical_civic_service_tests {", 1)[1].split(
            "mod physical_civic_supply_requests_tests {", 1
        )[0]
        self.assertEqual(cases.count("#[test]"), 5)
        for marker in (
            "legacy_player_saves_default_to_zero_real_deliveries",
            "physically_paid_sale_credits_exact_volume_and_one_tier_bonus",
            "service_milestone_does_not_reaward_on_purchase_or_restart",
            "supply_records_belong_only_to_authenticated_player_and_village",
            "malformed_saved_service_counts_fail_closed_and_counts_are_bounded",
        ):
            self.assertIn(marker, cases)


if __name__ == "__main__":
    unittest.main()
