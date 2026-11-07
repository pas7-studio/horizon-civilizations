"""Source contracts for default-on NPC autonomy and purchased player plots.

These tests do not replace native compilation or real gameplay validation.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text()
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text()


class AutonomousVillageContracts(unittest.TestCase):
    def test_village_autonomy_defaults_to_on_in_old_and_new_saves(self):
        self.assertIn("fn autonomous_by_default() -> bool { true }", CORE)
        self.assertIn('#[serde(default = "autonomous_by_default")]', CORE)
        self.assertIn("autonomy_enabled: true,", CORE)
        self.assertIn("pub fn set_autonomy(", CORE)

    def test_native_tick_autonomously_schedules_visible_work(self):
        self.assertIn("fn autonomous_village_step(game:", GLUE)
        self.assertIn("state.physical_ticks % 100 == 0", GLUE)
        self.assertIn("autonomous_village_step(game, state)", GLUE)
        self.assertIn("prepare_village_housing(id)?", GLUE)
        self.assertIn("world_block(game,", GLUE)
        self.assertIn("check_empty_hut_site(game, &dimension, site)", GLUE)
        self.assertIn("check_empty_farm_site(game, &dimension, site)", GLUE)
        self.assertIn("start_visual_house(id, site)?", GLUE)
        self.assertIn("start_farm(id, site)?", GLUE)

    def test_villager_spawning_is_one_shot_uuid_bound_and_durable(self):
        method = GLUE.split("fn auto_spawn_villagers(", 1)[1]
        method = method.split("\n}", 1)[0]
        self.assertIn("safe_actor_spawn_points(game, &village)", method)
        self.assertIn("if positions.len() != 10", method)
        self.assertIn("actors_spawned = true", method)
        self.assertLess(method.index("save_autonomous(state, prior)?"),
                        method.index("game.native_spawn_mob("))
        self.assertIn("village.actor_name(resident)", method)
        self.assertIn("game.native_entity_uuid(handle)", method)
        self.assertIn("bind_actor(village.id, resident.id, uuid)?", method)
        self.assertIn("save(state).map_err", method)
        self.assertNotIn("actors_spawned = false", method)

    def test_player_plots_are_saved_and_forbid_ordinary_auto_jobs(self):
        self.assertIn("pub struct PlayerParcel", CORE)
        self.assertIn("pub player_parcels: Vec<PlayerParcel>", CORE)
        self.assertIn("pub fn buy_plot(", CORE)
        self.assertIn("near_plot(plot.origin, origin)", CORE)
        self.assertIn('return Err("farm site is owned by a player".into())', CORE)
        self.assertIn("owner != Some(plot.owner.as_str())", CORE)
        self.assertIn("plot.owner == player_id", CORE)

    def test_commission_is_owner_verified_and_cannot_mint_resources(self):
        section = CORE.split("pub fn commission_house(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("plot.owner == player_id && !plot.commissioned", section)
        self.assertIn("p.coins < COMMISSION_PRICE_COINS", section)
        # Commission creates a physical work order without charging the old
        # virtual stock. The builder reserves REAL warehouse items later.
        self.assertNotIn("village.take(Good::Wood", section)
        self.assertNotIn("village.take(Good::Stone", section)
        self.assertIn("village.hut_supplies_reserved = false", section)
        self.assertIn("village.treasury_coins = treasury_after", section)
        self.assertIn("village.visual_job = Some(VisualBuildJob", section)

    def test_player_commission_huts_do_not_steal_npc_beds(self):
        housing = CORE.split("pub fn public_huts(", 1)[1].split("\n    }", 1)[0]
        home = CORE.split("pub fn home_slot(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("plot.commissioned", housing)
        self.assertIn("plot.commissioned", home)
        self.assertIn("self.public_huts() * PHYSICAL_HOME_CAPACITY", CORE)

    def test_plot_purchase_pays_treasury_in_single_world_save(self):
        purchase = CORE.split("pub fn buy_plot(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("p.coins < LAND_PRICE_COINS", purchase)
        self.assertIn("self.players.get_mut(player_id).unwrap().coins -= LAND_PRICE_COINS", purchase)
        self.assertIn("v.treasury_coins = treasury_after", purchase)
        self.assertIn("self.event(", purchase)
        self.assertIn("state.world = Some(previous)", GLUE)

    def test_player_commands_and_status_exposed(self):
        for name in ("buyplot", "commission", "plots", "development", "autonomy"):
            self.assertIn(name, GLUE)
        self.assertIn('if action == "development"', GLUE)
        self.assertIn('"buyplot" => (|| {', GLUE)
        self.assertIn('"commission" => (|| {', GLUE)
        self.assertIn('"autonomy" if player.permission >= 2', GLUE)
        self.assertIn("game.native_on(MOD_ID)", GLUE)

    def test_auto_work_is_only_in_loaded_chunks_and_conditional(self):
        self.assertIn("safe_actor_spawn_points(game, &village)", GLUE)
        self.assertIn("game.world.loaded(pos[0], pos[2])", GLUE)
        self.assertIn("loaded_actors(game, state.world.as_ref().unwrap(), id)", GLUE)
        self.assertIn("game.native_run_command(&command)", GLUE)
        self.assertIn("if block {x} {y} {z} {old_block}", GLUE)

    def test_purchase_complete_hut_requires_all_real_loaded_blocks(self):
        self.assertIn("pub const FINISHED_HUT_PRICE_COINS: u64 = 35;", CORE)
        buy = CORE.split("pub fn buy_finished_house(", 1)[1].split("\n    }", 1)[0]
        self.assertIn("v.visual_sites.contains(&origin)", buy)
        self.assertIn("FINISHED_HUT_PRICE_COINS", buy)
        self.assertIn("v.treasury_coins = treasury_after", buy)
        self.assertIn("commissioned: true", buy)
        adapter = GLUE.split('"buyhouse" => (|| {', 1)[1].split('\n        })(),', 1)[0]
        self.assertIn("inspect_hut(&VisualBuildJob", adapter)
        self.assertIn("BuildInspection::Complete", adapter)
        self.assertIn("buy_finished_house(&actor, village.id, origin)", adapter)
        self.assertIn("village.player_parcels", adapter)

    def test_sold_village_houses_are_removed_from_public_economy(self):
        self.assertIn("let private_houses = self.player_parcels.iter()", CORE)
        self.assertIn(".filter(|plot| plot.commissioned).count() as u32", CORE)
        self.assertIn("self.houses.saturating_sub(private_houses)", CORE)
        self.assertIn("pub fn public_huts(&self)", CORE)

    def test_package_mixin_compatibility_and_backwards_save(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        mixin = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        self.assertEqual(cargo["package"]["version"], mixin["mod"]["version"])
        self.assertEqual(mixin["mod"]["version"], cargo["package"]["version"])
        self.assertEqual(len(mixin["injection"]), 6)
        self.assertIn("pub const SCHEMA: u32 = 1;", CORE)
        self.assertIn("#[serde(default)]\n    pub player_parcels:", CORE)
        self.assertIn("#[serde(default)]\n    pub treasury_coins:", CORE)


if __name__ == "__main__":
    unittest.main()
