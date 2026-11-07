"""Source contracts for one real bread item per loaded UUID-bound resident.

These inspect integration points and do not claim a native Cargo/test or
Minecraft gameplay E2E has passed.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def body(source, marker):
    # Rust impl methods end at four-space indentation while module-level
    # functions end at column zero. Do not accidentally inspect the entire
    # impl block (which contains unrelated abstract economy operations).
    start = source.index(marker)
    line_start = source.rfind("\n", 0, start) + 1
    indent = start - line_start
    closing = "\n" + (" " * indent) + "}\n"
    return source[start:].split(closing, 1)[0]


class PhysicalResidentMealContracts(unittest.TestCase):
    def test_meal_fields_load_from_legacy_json_with_safe_defaults(self):
        self.assertIn("pub meal_day: Option<i64>", CORE)
        self.assertIn("pub meal_served: BTreeSet<u64>", CORE)
        self.assertIn("pub meal_shortage_days: u32", CORE)
        self.assertIn("pub recipient_id: Option<u64>", CORE)
        self.assertIn("meal_day: None", CORE)
        self.assertIn("recipient_id: None", CORE)
        self.assertIn("invalid saved physical resident meal journal", CORE)

    def test_one_native_loaf_is_exactly_one_saved_recipient(self):
        model = body(CORE, "fn plan_barrel_transfer_for(")
        self.assertIn("BarrelDirection::ToResidentMeal => {", model)
        self.assertIn("good != Good::Bread || count != 1", model)
        self.assertIn("recipient_id", model)
        self.assertIn("recipient_id.is_none_or", model)
        self.assertIn("resident.entity_uuid.is_some()", model)

    def test_food_is_credited_only_after_native_receipt(self):
        confirmation = body(CORE, "pub fn confirm_barrel_transfer(")
        self.assertIn("t.expected_after() != Some(after)", confirmation)
        self.assertIn("BarrelDirection::ToResidentMeal =>", confirmation)
        self.assertIn("v.meal_served.insert(recipient)", confirmation)
        self.assertLess(confirmation.index("t.expected_after() != Some(after)"),
                        confirmation.index("v.meal_served.insert(recipient)"))
        self.assertNotIn("warehouse_credit(Good::Bread", confirmation)

    def test_old_unserved_day_is_finalized_at_most_once(self):
        days = body(CORE, "pub fn begin_meal_day(")
        self.assertIn("host_day <= previous", days)
        self.assertIn("transfer.direction == BarrelDirection::ToResidentMeal", days)
        self.assertIn("v.meal_served.clear()", days)
        self.assertIn("v.meal_shortage_days = v.meal_shortage_days.saturating_add(1)", days)
        self.assertIn("v.morale = (v.morale - 2).max(-100)", days)
        self.assertNotIn("self.advance(", days)

    def test_physical_meals_are_never_the_abstract_economy(self):
        planner = body(GLUE, "fn resident_meal_one_step(")
        self.assertNotIn(".quantity(Good::Bread)", planner)
        self.assertNotIn("warehouse_debit(", planner)
        self.assertNotIn("stock.insert(", planner)
        self.assertIn("plain_barrel_count(", planner)
        self.assertIn("game.native_container_slots(", planner)

    def test_both_real_steward_and_uuid_recipient_are_required(self):
        planner = body(GLUE, "fn resident_meal_one_step(")
        self.assertIn("storage_worker_near(game, state.world.as_ref().unwrap(), id, Job::Merchant)", planner)
        self.assertIn("loaded_actors(game, state.world.as_ref().unwrap(), id)", planner)
        self.assertIn("r.entity_uuid.is_some()", planner)
        self.assertIn("native_worker_near(actor.pos, village.center, 8.0, 4.0)", planner)
        self.assertIn("plan_resident_meal(", planner)

    def test_meal_routes_real_villagers_to_the_hall(self):
        navigation = body(GLUE, "fn guide_actors(")
        self.assertIn("let meal_due = village.hall_complete", navigation)
        self.assertIn("village.meal_day == Some(game.world.day_time.div_euclid(24_000))", navigation)
        self.assertIn("!village.meal_served.contains(&resident.id)", navigation)
        self.assertIn("let pos = if meal_due {", navigation)

    def test_barrel_extraction_uses_existing_write_ahead_ledger(self):
        bridge = body(GLUE, "fn barrel_one_step(")
        self.assertIn("state.world.as_mut().unwrap().issue_barrel_transfer(id)?", bridge)
        self.assertIn("save_autonomous(state, before)?;", bridge)
        self.assertIn("| BarrelDirection::ToResidentMeal =>", bridge)
        self.assertIn("game.native_extract_item(", bridge)
        self.assertIn("state.world.as_mut().unwrap().confirm_barrel_transfer(id, actual)?", bridge)

    def test_planned_meal_is_cancelled_if_recipient_went_away(self):
        bridge = body(GLUE, "fn barrel_one_step(")
        self.assertIn("if transfer.direction == BarrelDirection::ToResidentMeal", bridge)
        self.assertIn("let still_eligible =", bridge)
        self.assertIn("cancel_unissued_meal(id)?", bridge)
        self.assertLess(bridge.index("cancel_unissued_meal(id)?"),
                        bridge.index("issue_barrel_transfer(id)?"))
        self.assertIn("pub fn cancel_unissued_meal(&mut self, id: u64)", CORE)

    def test_pending_issued_receipt_may_survive_recipient_death(self):
        validation = body(CORE, "pub fn validate(")
        self.assertIn('invalid native resident meal receipt', validation)
        self.assertIn("r.id == recipient", validation)
        self.assertIn("pending_issued_meal_stays_loadable_if_recipient_dies", CORE)

    def test_pending_external_mutations_never_get_automatic_second_try(self):
        model = body(CORE, "pub fn issue_barrel_transfer(")
        self.assertIn("transfer.phase != BarrelPhase::Planned", model)
        bridge = body(GLUE, "fn barrel_one_step(")
        self.assertIn("if transfer.phase == BarrelPhase::Blocked { continue; }", bridge)
        self.assertIn("block_barrel_transfer(id)?", bridge)

    def test_only_living_citizens_satisfy_the_daily_meal_requirement(self):
        self.assertIn("pub fn living_meals_served(&self) -> usize", CORE)
        self.assertIn("village.living_meals_served() < village.population()", CORE)
        self.assertIn("village.living_meals_served() >= village.population()", GLUE)

    def test_dining_progress_read_only_command(self):
        route = GLUE.split('if action == "meals" {', 1)[1].split(
            'if action == "hall"', 1
        )[0]
        self.assertIn("living_meals_served()", route)
        self.assertIn("meal_shortage_days", route)
        self.assertIn("physical_barrel_stock(", route)
        self.assertNotIn("state.world.as_mut()", route)
        self.assertNotIn("save(state)", route)
        self.assertIn('"civ meals <name:word>"', GLUE)

    def test_dining_updates_public_six_board_treasury_food_line(self):
        signs = body(CORE, "pub fn render_hall_boards(")
        self.assertIn('MEALS SHORT {}d', signs)
        self.assertIn('Fed {}/{}', signs)
        self.assertIn('Abstract hunger {day}d', signs)
        self.assertIn("v.meal_shortage_days > 0", signs)

    def test_native_meals_have_nine_independent_rust_regressions(self):
        rust_tests = CORE.split("mod physical_meals_tests", 1)[1]
        self.assertEqual(rust_tests.count("#[test]"), 9)
        for case in ("legacy_saves", "native_receipt_confirmation", "cannot_eat_twice",
                     "cannot_be_reissued", "morale_once", "invalid_saved_recipient",
                     "death_does_not_hide", "cancelled_unissued_meal", "recipient_dies"):
            self.assertTrue(case in rust_tests, case)


if __name__ == "__main__":
    unittest.main()
