"""Source contracts for roles derived from real workplaces, not chance.

A resident may only hold a job the village can physically provide: a farmer
needs a reserved field, a baker a bakery, a lumberjack or miner a camp (or a
real material shortage). Everyone else is honestly a laborer. These checks are
not a Rust compiler or a Horizon game test.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


class WorkplaceStaffingContracts(unittest.TestCase):
    def test_laborer_is_a_real_job_with_a_place(self):
        self.assertIn("    Laborer,", CORE)
        self.assertIn('Self::Laborer => "laborer",', CORE)
        self.assertIn("Self::Laborer => [2, 0, 4],", CORE)

    def test_demand_is_derived_from_sites_that_exist(self):
        demand = CORE.split("pub fn workplace_demand(&self", 1)[1].split("\n    }", 1)[0]
        self.assertIn("if self.farm.is_some() {", demand)
        self.assertIn("if self.bakery.is_some() {", demand)
        self.assertIn("if self.hall_complete || self.hall_job.is_some() {", demand)
        self.assertIn("post.kind == PostKind::Lumber", demand)
        self.assertIn("post.kind == PostKind::Quarry", demand)
        self.assertIn("self.hall_materials_missing()", demand)
        self.assertIn("let building = !self.hall_complete || self.hall_job.is_some()", demand)
        self.assertIn("|| (self.pending_visual_buildings > 0 && self.physical_housing().2 > 0)", demand)
        self.assertIn("want(Job::Guard, 1);", demand)
        self.assertIn("want(Job::Builder, 1);", demand)
        self.assertNotIn("native_run_command", demand)
        self.assertNotIn("save(", demand)

    def test_staffing_keeps_camp_workers_and_releases_stale_bindings(self):
        body = CORE.split("pub fn staff_workplaces(&mut self", 1)[1].split("\n    }", 1)[0]
        self.assertIn("resident.job != job", body)
        self.assertIn("post.worker_id = None;", body)
        self.assertIn("job.worker_id.map(|worker|", body)
        self.assertIn("chosen.insert(worker, job);", body)
        self.assertNotIn("native_", body)
        self.assertNotIn("save(", body)

    def test_roles_are_restaffed_before_the_first_spawn(self):
        method = GLUE.split("fn autonomous_village_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("staff_village(game, state, id)?;", method)
        self.assertLess(method.index("staff_village(game, state, id)?;"),
                        method.index("auto_spawn_villagers(game, state, id)"))

    def test_name_tag_follows_the_saved_role_not_a_stale_label(self):
        helper = GLUE.split("fn rename_actor(", 1)[1].split("\n}", 1)[0]
        self.assertIn("native_entity_by_uuid(uuid)", helper)
        self.assertIn("CustomName", helper)
        self.assertIn("\"CustomNameVisible\".into()", helper)
        self.assertIn("native_merge_entity_nbt", helper)
        staff = GLUE.split("fn staff_village(", 1)[1].split("\n}", 1)[0]
        self.assertLess(staff.index("save_autonomous(state, prior)?;"),
                        staff.index("rename_actor(game, uuid"))
        self.assertNotIn("native_run_command", staff)
        self.assertNotIn("setblock", staff)

    def test_a_site_is_reserved_before_its_worker_exists(self):
        # The field and the bakery are the workplaces, so reserving one cannot
        # wait for a farmer or baker that only exists after the site.
        self.assertIn("let need_field = village.farm.is_none();", GLUE)
        self.assertIn("let need_bakery = has_builder && village.hall_complete", GLUE)
        bakery = CORE.split("pub fn start_bakery(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("if village.count(Job::Builder) == 0 {", bakery)
        self.assertNotIn("count(Job::Baker) == 0", bakery)
        farm = CORE.split("pub fn start_farm(", 1)[1].split("\n    }", 1)[0]
        self.assertNotIn("has no farmer", farm)


    def test_sand_and_dirt_pits_are_real_workplaces(self):
        for token in ("    Sand,", "    Dirt,", 'Self::Sand => "minecraft:sand"',
                      'Self::Dirt => "minecraft:dirt"', 'Self::Sand => "sand",',
                      'Self::Dirt => "dirt",'):
            self.assertIn(token, CORE)
        self.assertIn('Self::Sand => "SAND PIT",', CORE)
        self.assertIn('Self::Dirt => "DIRT PIT",', CORE)
        self.assertIn("Self::Sand | Self::Dirt => Job::Digger,", CORE)
        self.assertIn("Self::Digger => \"digger\",", CORE)
        self.assertIn("Self::Sand | Self::Dirt => Some(Job::Digger),", CORE)
        self.assertIn("Good::Sand => \"minecraft:sand\",", CORE)
        self.assertIn("pub fn for_good(good: Good) -> Self {", CORE)

    def test_pits_produce_only_through_a_real_receipt(self):
        body = CORE.split("pub fn plan_gather(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("let Some(worker_job) = good.job()", body)
        self.assertIn("v.count(worker_job) == 0", body)
        self.assertNotIn("native_run_command", body)
        self.assertNotIn("native_spawn", body)
        planner = GLUE.split("fn autonomous_village_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn("const PIT_TARGET: u32 = 32;", planner)
        self.assertIn("if !village.posts.iter().any(|post| post.kind == pit)", planner)
        self.assertIn("plan_gather(id, good, position, stand)", planner)

    def test_a_natural_bump_is_cut_down_to_the_plot_floor(self):
        body = GLUE.split("fn clear_growth_only(", 1)[1].split("\n}", 1)[0]
        self.assertIn("if y == 0 && is_ground(&name) {", body)
        self.assertIn("if is_ground(&below) {", body)
        self.assertIn("is not natural growth", body)
        # Nothing may be placed: a plot is only ever opened by removing blocks.
        self.assertNotIn("minecraft:dirt replace", body)
        self.assertNotIn("native_give", body)

    def test_a_quarry_digs_a_bounded_staircase_to_real_stone(self):
        self.assertIn("pub const MAX_QUARRY_STEPS: u8 = 24;", CORE)
        self.assertIn("pub fn advance_quarry(", CORE)
        self.assertIn("pub quarry_step: u8,", CORE)
        self.assertIn("pub fn plan_quarry_gather(", CORE)
        self.assertIn("pub fn gather_claim_ok(", CORE)
        pit = GLUE.split("fn quarry_one_step(", 1)[1].split("\n}", 1)[0]
        self.assertIn('name == "minecraft:stone"', pit)
        self.assertIn("plan_quarry_gather(id, position, stand)", pit)
        self.assertIn("advance_quarry(id)?", pit)
        self.assertIn("dig_natural(", pit)
        self.assertIn("native_worker_near(actor.pos, position, 4.5, 5.0)", pit)
        start = GLUE.split("fn quarry_start(", 1)[1].split("\n}", 1)[0]
        self.assertIn("is_ground", start)
        self.assertNotIn("native_run_command", start)

    def test_role_changes_are_announced_readably(self):
        staff = GLUE.split("fn staff_village(", 1)[1].split("\n}", 1)[0]
        self.assertIn("native_broadcast(", staff)
        self.assertIn("is now the village", staff)
        self.assertIn('if action == "status" {', GLUE)
        self.assertIn('"civ status <name:word>"', GLUE)


if __name__ == "__main__":
    unittest.main()
