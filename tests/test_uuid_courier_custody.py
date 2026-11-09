"""Static integration contracts for NPC wheat/bread courier custody.

Authored only. Does not execute Rust, Horizon builds or game E2E.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


class CourierCustodyContracts(unittest.TestCase):
    def test_save_fields_are_backward_compatible(self):
        for needle in (
            "pub wheat_carrier_id: Option<u64>",
            "pub bread_carrier_id: Option<u64>",
            "wheat_carrier_id: None",
            "bread_carrier_id: None",
        ):
            self.assertIn(needle, CORE)

    def test_native_farm_requires_loaded_exact_uuid_linked_farmer(self):
        farm = GLUE.split("fn farm_one_step(", 1)[1].split("pub fn protect_public_hopper", 1)[0]
        self.assertIn("resident.entity_uuid.is_some()", farm)
        self.assertIn("native_worker_near(actor.pos, farm.origin", farm)
        self.assertIn("advance_farm_with_carrier(village_id, decision.clone(), farmer_id)", farm)

    def test_harvest_receipt_and_carrier_are_committed_together(self):
        domain = CORE.split("pub fn advance_farm_with_carrier(", 1)[1].split(
            "pub fn deliver_farm_wheat_from(", 1
        )[0]
        self.assertIn("farm.stage == FarmStage::Harvesting", domain)
        self.assertIn("r.job == Job::Farmer", domain)
        self.assertIn("v.wheat_in_transit > 0 && v.wheat_carrier_id != Some(worker_id)", domain)
        self.assertIn("self.advance_farm(village_id, decision)?", domain)
        self.assertIn(".wheat_carrier_id = Some(worker_id)", domain)

    def test_wheat_delivery_rejects_a_different_farmer(self):
        native = GLUE.split("fn farmer_delivery_one_step(", 1)[1].split(
            "fn baker_one_step(", 1
        )[0]
        self.assertIn("v.wheat_carrier_id", native)
        self.assertIn("r.id == carrier", native)
        self.assertIn("deliver_farm_wheat_from(id, carrier)", native)
        self.assertNotIn("deliver_farm_wheat(id)", native)

    def test_native_baker_gets_proven_witness_identity_at_workshop(self):
        native = GLUE.split("fn baker_one_step(", 1)[1].split(
            "///", 1
        )[0]
        self.assertIn("r.entity_uuid.is_some()", native)
        self.assertIn("native_worker_near(actor.pos, b.origin", native)
        self.assertIn("craft_bread_with_carrier(id, baker_id)", native)

    def test_native_baker_delivery_matches_the_saved_worker(self):
        native = GLUE.split("fn baker_one_step(", 1)[1].split(
            "///", 1
        )[0]
        self.assertIn("village.bread_carrier_id", native)
        self.assertIn("r.id == carrier", native)
        self.assertIn("deliver_bakery_bread_from(id, carrier)", native)
        self.assertNotIn("deliver_bakery_bread(id)", native)

    def test_town_roster_cannot_reassign_a_worker_who_holds_a_receipt(self):
        staff = CORE.split("pub fn staff_workplaces(", 1)[1].split(
            "pub fn", 1
        )[0]
        self.assertIn("(self.wheat_carrier_id, Job::Farmer)", staff)
        self.assertIn("(self.bread_carrier_id, Job::Baker)", staff)

    def test_navigation_routes_only_authenticated_couriers_to_hall(self):
        route = GLUE.split("fn guide_actors(", 1)[1].split(
            "fn ", 1
        )[0]
        self.assertIn("village.wheat_carrier_id == Some(resident_id)", route)
        self.assertIn("village.bread_carrier_id == Some(resident_id)", route)

    def test_old_unbound_transit_is_valid_on_load_but_not_reassignable(self):
        check = CORE.split("pub fn validate(", 1)[1].split("fn event(", 1)[0]
        self.assertIn("village.wheat_carrier_id", check)
        self.assertIn("village.bread_carrier_id", check)
        self.assertIn("invalid {label} courier custody binding", check)
        self.assertNotIn("if village.wheat_carrier_id.is_none()", check)
        domain = CORE.split("pub fn deliver_farm_wheat_from(", 1)[1].split(
            "pub fn craft_bread_with_carrier(", 1
        )[0]
        self.assertIn("v.wheat_carrier_id != Some(worker_id)", domain)

    def test_no_issued_or_bound_inventory_can_be_retired_by_operator(self):
        domain = CORE.split("pub fn retire_unbound_cargo(", 1)[1].split(
            "/// Reserve the actual resource budget", 1
        )[0]
        self.assertIn("v.wheat_carrier_id.is_none()", domain)
        self.assertIn("v.bread_carrier_id.is_none()", domain)
        self.assertNotIn("warehouse_credit(", domain)
        self.assertIn("zero warehouse credit", domain)
        native = GLUE.split('"retirecargo" if player.permission >= 2 =>', 1)[1].split(
            '"war" | "peace"', 1
        )[0]
        self.assertIn('Some("confirm")', native)
        self.assertIn("village.dimension != player.dimension", native)
        self.assertIn("village.barrel_transfer.is_some() || village.market_purchase.is_some()", native)

    def test_missing_public_storage_can_never_produce_phantom_pit_demand(self):
        planner = GLUE.split("fn autonomous_village_step(", 1)[1].split(
            "fn ", 1
        )[0]
        self.assertIn("let Some(stock) = physical_barrel_stock(", planner)
        self.assertIn("else { return Ok(()); };", planner)

    def test_rust_tests_cover_restart_mismatch_legacy_retirement_and_staffing(self):
        cases = CORE.split("mod same_uuid_courier_custody_tests {", 1)[1].split(
            "mod physical_civic_service_tests {", 1
        )[0]
        self.assertEqual(cases.count("#[test]"), 5)
        for title in (
            "harvest_receipt_binds_the_exact_farmer_through_restart_and_delivery",
            "baker_recipe_output_is_bound_to_the_one_uuid_linked_creator",
            "older_unbound_transit_is_loadable_but_not_auto_deliverable",
            "staffing_preserves_an_active_courier_role_and_rejects_unknown_ids",
            "registered_cargo_cannot_be_retired_without_the_bound_courier",
        ):
            self.assertIn(title, cases)


if __name__ == "__main__":
    unittest.main()
