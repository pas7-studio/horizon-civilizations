"""Source-only contracts for showing receipt-backed goods in native NPC dialogs.

These contracts are authored for the user's later local gate. They do
not prove an item is equipped to a villager or that gameplay was run.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def method(source: str, start: str, next_start: str) -> str:
    return source.split(start, 1)[1].split(next_start, 1)[0]


class VerifiedCourierDialogSourceContracts(unittest.TestCase):
    def test_dialog_cargo_is_a_read_only_projection_of_existing_receipts(self):
        cargo = method(CORE, "pub fn resident_verified_cargo(", "/// Build one conversation page.")
        self.assertIn("village.gather_job", cargo)
        self.assertIn("village.wheat_carrier_id", cargo)
        self.assertIn("village.bread_carrier_id", cargo)
        for forbidden in ("warehouse_credit(", "native_run_command(", "native_give_items(",
                          "native_take_items(", "minecraft:item_display"):
            self.assertNotIn(forbidden, cargo)

    def test_unlinked_or_dead_mob_never_advertises_goods(self):
        cargo = method(CORE, "pub fn resident_verified_cargo(", "/// Build one conversation page.")
        self.assertIn("!resident.alive || resident.entity_uuid.is_none()", cargo)
        self.assertIn("return None", cargo)

    def test_gathered_items_only_show_after_pickup_witness(self):
        cargo = method(CORE, "pub fn resident_verified_cargo(", "/// Build one conversation page.")
        self.assertIn("job.worker_id == Some(resident.id)", cargo)
        self.assertIn("job.good.job() == Some(resident.job)", cargo)
        self.assertIn("job.native_edit_confirmed", cargo)
        self.assertIn("job.pickup_witnessed", cargo)
        self.assertIn("!job.blocked", cargo)
        self.assertIn("Some((job.good, 1))", cargo)

    def test_wheat_cargo_is_specific_to_farmer_uuid(self):
        cargo = method(CORE, "pub fn resident_verified_cargo(", "/// Build one conversation page.")
        self.assertIn("resident.job == Job::Farmer", cargo)
        self.assertIn("village.wheat_carrier_id == Some(resident.id)", cargo)
        self.assertIn("village.wheat_in_transit > 0", cargo)
        self.assertIn("Some((Good::Wheat, village.wheat_in_transit))", cargo)

    def test_bread_cargo_is_specific_to_baker_uuid(self):
        cargo = method(CORE, "pub fn resident_verified_cargo(", "/// Build one conversation page.")
        self.assertIn("resident.job == Job::Baker", cargo)
        self.assertIn("village.bread_carrier_id == Some(resident.id)", cargo)
        self.assertIn("village.bakery.as_ref()", cargo)
        self.assertIn("Some((Good::Bread, count))", cargo)

    def test_native_dialog_renders_actual_good_icon_only_for_verified_receipt(self):
        talk = method(CORE, "pub fn build_dialog(", "/// Turn the")
        self.assertIn("resident_verified_cargo(village, resident)", talk)
        self.assertIn("portrait: cargo.map_or(job_icon(resident.job)", talk)
        self.assertIn("good.item()", talk)
        self.assertIn("receipt {} x{}", talk)
        self.assertIn("Verified delivery record:", talk)
        self.assertIn("en route to the town hall", talk)

    def test_original_game_native_dialog_item_renderer_is_reused(self):
        snbt = method(CORE, "pub fn dialog_snbt(", "/// Standing site board:")
        self.assertIn('minecraft:item', snbt)
        self.assertIn("snbt_string(dialog.portrait)", snbt)
        self.assertIn("snbt_text(&dialog.portrait_caption", snbt)

    def test_dialog_does_not_write_equipment_or_spawn_item_entities(self):
        talk = method(CORE, "pub fn build_dialog(", "/// Turn the")
        for forbidden in ("weapon.mainhand", "item replace entity", "native_spawn_mob",
                          "native_run_command", "native_give_items"):
            self.assertNotIn(forbidden, talk)
        self.assertIn("let snbt = dialog_snbt(&build_dialog(page, &input))", GLUE)

    def test_unverified_legacy_bread_cannot_prevent_real_restock_need(self):
        logic = method(CORE, "pub fn needs_real_bread(", "impl Settlement {")
        self.assertIn("village.bread_carrier_id.is_some()", logic)
        self.assertIn("village.warehouse_quantity(Good::Bread)", logic)
        self.assertIn("physical_bread_target(village)", logic)
        self.assertNotIn("village.stock", logic)

    def test_cargo_comes_from_same_worker_route_not_any_profession(self):
        self.assertIn("deliver_farm_wheat_from(id, carrier)", GLUE)
        self.assertIn("deliver_bakery_bread_from(id, carrier)", GLUE)
        self.assertIn("advance_farm_with_carrier(village_id, decision.clone(), farmer_id)", GLUE)

    def test_preexisting_dialogue_input_shape_is_unchanged(self):
        inp = method(CORE, "pub struct DialogueInput<'a> {", "fn greeting(")
        self.assertNotIn("pub cargo:", inp)
        self.assertIn("pub village:", inp)
        self.assertIn("pub resident:", inp)

    def test_five_rust_regressions_cover_all_movement_states(self):
        section = method(CORE, "mod verified_dialog_cargo_tests {",
                         "mod same_uuid_courier_custody_tests {")
        self.assertEqual(section.count("#[test]"), 5)
        for name in ("gather_icon_appears_only_after_native_witness_and_not_after_block",
                     "farmer_dialogue_never_credits_or_displays_someone_elses_grain",
                     "baker_dialogue_shows_saved_bread_cargo_but_not_fake_equipment",
                     "after_delivery_no_cargo_icon_or_report_survives",
                     "old_unbound_loaf_never_hides_physical_food_shortage"):
            self.assertIn(name, section)


if __name__ == "__main__":
    unittest.main()
