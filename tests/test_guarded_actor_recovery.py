"""Source-only safety contracts for one-shot native actor spawn recovery.

These assertions do NOT replace Rust compilation, runtime village
rehydration or a real Horizon Minecraft gameplay test.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def function(source: str, marker: str, end: str = "\n    }") -> str:
    return source.split(marker, 1)[1].split(end, 1)[0]


class GuardedPartialVillagerRecoveryContracts(unittest.TestCase):
    def test_pending_recovery_journal_is_backward_compatible(self):
        record = function(CORE, "pub struct Settlement {", "\n}")
        self.assertIn("#[serde(default)]\n    pub actor_recovery_issued: BTreeSet<u64>", record)
        self.assertIn("actor_recovery_issued: BTreeSet::new()", CORE)

    def test_durable_reservation_rejects_bad_and_repeated_id(self):
        reserve = function(CORE, "pub fn reserve_actor_recovery(")
        for fragment in (
            "!settlement.actors_spawned",
            "r.id == resident_id && r.alive && r.entity_uuid.is_none()",
            "!settlement.actor_recovery_issued.insert(resident_id)",
        ):
            self.assertIn(fragment, reserve)

    def test_uuid_binding_releases_recovery_journal_on_same_worker(self):
        bind = function(CORE, "pub fn bind_actor(")
        self.assertIn("resident.entity_uuid = Some(uuid)", bind)
        self.assertIn("settlement.actor_recovery_issued.remove(&resident_id)", bind)
        self.assertIn("entity UUID is already owned by another resident", bind)

    def test_saved_pending_id_validation_prevents_corrupt_journals(self):
        validate = function(CORE, "pub fn validate(")
        self.assertIn("!village.actor_recovery_issued.is_empty() && !village.actors_spawned", validate)
        self.assertIn("for recovery_id in &village.actor_recovery_issued", validate)
        self.assertIn("r.id == *recovery_id && r.entity_uuid.is_none()", validate)

    def test_recovery_requires_operator_confirmation_not_automatic_spawn(self):
        self.assertIn('"civ recoveractor <name:word> <resident:integer> <confirm:word>"', GLUE)
        command = function(GLUE, '"recoveractor" if player.permission >= 2 => (|| {', "\n        })(),")
        self.assertIn('args.next() != Some("attempt")', command)
        self.assertIn("village.dimension != player.dimension", command)
        self.assertIn("dx * dx + dz * dz > 32 * 32", command)
        self.assertIn("resident.entity_uuid.is_none()", command)
        self.assertIn("actor_recovery_issued.contains(&resident_id)", command)

    def test_full_loaded_recovery_scan_and_duplicate_actor_check(self):
        command = function(GLUE, '"recoveractor" if player.permission >= 2 => (|| {', "\n        })(),")
        self.assertIn("for ox in (-96_i32..=96).step_by(16)", command)
        self.assertIn("for oz in (-96_i32..=96).step_by(16)", command)
        self.assertIn("!game.world.loaded(x, z)", command)
        self.assertIn("mob.kind.id == \"minecraft:villager\"", command)
        self.assertIn("mob.custom_name.as_deref() == Some(exact_name.as_str())", command)
        self.assertIn("mob.custom_name.as_deref() == Some(legacy_name.as_str())", command)

    def test_safe_spawn_and_durable_reservation_precede_native_spawning(self):
        command = function(GLUE, '"recoveractor" if player.permission >= 2 => (|| {', "\n        })(),")
        checkpoints = [
            "safe_actor_spawn_points(game, village)",
            "reserve_actor_recovery(village.id, resident_id)?",
            "save(state)?;",
            "game.native_spawn_mob(",
            "game.native_entity_uuid(handle)",
            "bind_actor(village.id, resident_id, uuid)",
        ]
        indices = [command.index(item) for item in checkpoints]
        self.assertEqual(indices, sorted(indices))

    def test_no_automatic_respawn_after_partial_batch(self):
        native = function(GLUE, "fn auto_spawn_villagers(", "\n}")
        self.assertIn("if village.actors_spawned || !village.autonomy_enabled", native)
        self.assertIn("surveyed_initial_actor_positions(game, &village)", native)
        self.assertNotIn("game.native_spawn_mob(", native.split("save_autonomous(state, prior)?;", 1)[0])
        self.assertIn("actors_spawned = true", native)
        self.assertNotIn("reserve_actor_recovery(", native)

    def test_partial_sites_are_available_for_one_targeted_recovery(self):
        survey = function(GLUE, "fn safe_actor_spawn_points(", "\n}")
        self.assertIn("if spots.len() == 10 { return spots; }", survey)
        self.assertTrue(survey.rstrip().endswith("spots"))

    def test_failed_uuid_persistence_halts_native_actions(self):
        command = function(GLUE, '"recoveractor" if player.permission >= 2 => (|| {', "\n        })(),")
        self.assertIn('state.problem = Some(format!("native actor recovery identity halted: {e}"))', command)
        self.assertIn("Do NOT respawn.", command)

    def test_roster_displays_resident_ids_and_pending_state(self):
        roster = function(GLUE, 'if action == "roster" {', "\n    }")
        self.assertIn("r.id", roster)
        self.assertIn("actor_recovery_issued.contains(&r.id)", roster)
        self.assertIn("RECOVERY PENDING", roster)

    def test_initial_spawn_survey_rejects_dead_bound_and_short_batch(self):
        survey = function(GLUE, "fn surveyed_initial_actor_positions(", "\n}")
        self.assertIn("village.residents.len() < 10", survey)
        self.assertIn("!r.alive || r.entity_uuid.is_some()", survey)
        self.assertIn("positions.len() != 10", survey)

    def test_initial_spawn_refuses_matching_loaded_legacy_or_tagged_actor(self):
        survey = function(GLUE, "fn surveyed_initial_actor_positions(", "\n}")
        self.assertIn("let tagged = village.actor_name(resident)", survey)
        self.assertIn('let legacy = format!("{} [{}]", resident.name, village.name)', survey)
        self.assertIn('mob.kind.id == "minecraft:villager"', survey)
        self.assertIn("matching native villager already loaded", survey)

    def test_spawn_survey_excludes_occupied_native_locations(self):
        survey = function(GLUE, "fn safe_actor_spawn_points(", "\n}")
        self.assertIn("game.mobs.iter().any(|mob|", survey)
        self.assertIn("mob.alive() && dx * dx + dy * dy + dz * dz < 9.0", survey)
        self.assertLess(survey.index("game.mobs.iter().any(|mob|"), survey.index("spots.push(point)"))

    def test_op_and_auto_spawners_share_live_survey_before_reservation(self):
        op = function(GLUE, '"actors" if player.permission >= 2 => (|| {', "\n        })(),")
        auto = function(GLUE, "fn auto_spawn_villagers(", "\n}")
        self.assertIn("surveyed_initial_actor_positions(game, s)?", op)
        self.assertIn("surveyed_initial_actor_positions(game, &village)", auto)
        for body in (op, auto):
            self.assertLess(body.index("surveyed_initial_actor_positions"),
                            body.index("actors_spawned = true"))
            self.assertLess(body.index("actors_spawned = true"),
                            body.index("game.native_spawn_mob("))
        self.assertIn("for (resident, position) in s.residents.iter().take(10).zip(positions)", op)
        self.assertNotIn("[x, f64::from(s.center[1]), z]", op)

    def test_initial_spawn_positions_are_pairwise_separated(self):
        survey = function(GLUE, "fn safe_actor_spawn_points(", "\n}")
        self.assertIn("if spots.iter().any(|other: &[f64; 3]|", survey)
        self.assertIn("dx * dx + dy * dy + dz * dz < 9.0", survey)
        self.assertLess(survey.index("if spots.iter().any("),
                        survey.index("spots.push(point)"))

    def test_failed_reconcile_save_does_not_implicitly_retry(self):
        command = function(GLUE, '"reconcile" if player.permission >= 2 => (|| {',
                           "\n        })(),")
        failure = command.split("if let Err(e) = save(state)", 1)[1]
        self.assertLess(failure.index("already_saved = true;"),
                        failure.index('state.problem = Some('))
        self.assertIn("world halted", failure)

    def test_post_commit_error_halts_instead_of_rolling_back_snapshot(self):
        handler = GLUE.split("    match result {", 1)[-1]
        # Narrow to the mutating command handler, not earlier matches.
        handler = GLUE.split("    match result {\n        Ok(message) => {", 1)[1]
        handler = handler.split("\n}\n", 1)[0]
        self.assertIn("if already_saved {", handler)
        self.assertIn("state.problem = Some(format!(", handler)
        self.assertIn("state.world = Some(previous)", handler)
        self.assertLess(handler.index("if already_saved {"),
                        handler.index("state.world = Some(previous)", handler.index("Err(message) =>")))

    def test_core_regression_module_includes_seven_cases(self):
        chunk = CORE.split("mod guarded_partial_native_actor_recovery_tests", 1)[1]
        self.assertEqual(chunk.count("#[test]"), 8)


if __name__ == "__main__":
    unittest.main()
