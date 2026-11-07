"""Unreleased native gather fairness and post-command receipt source contracts.

These text-level contracts cannot replace Rust tests, native Horizon compilation
or actual authoritative Minecraft block/actor observation.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


def section(source, signature, closing):
    return source.split(signature, 1)[1].split(closing, 1)[0]


class NativeGatherFairQueueAndCrashReceiptContracts(unittest.TestCase):
    def test_fair_round_robin_is_ticked_once_per_gather_turn(self):
        helper = section(CORE, "pub fn gather_turn_index(", "\n}")
        self.assertIn("if eligible_count == 0 { return None; }", helper)
        self.assertIn("(physical_ticks / 8)", helper)
        self.assertIn("% eligible_count as u64", helper)

    def test_native_adapter_collects_all_eligible_loaded_villages(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("let eligible: Vec<_>", runner)
        self.assertIn('filter(|v| v.dimension == dimension)', runner)
        self.assertIn('filter(|(_, job)| !job.blocked && game.world.loaded(', runner)
        self.assertIn("gather_turn_index(state.physical_ticks, eligible.len())", runner)
        self.assertIn("eligible[slot].clone()", runner)
        self.assertNotIn(".find(|(_, job)| !job.blocked", runner)

    def test_native_worker_shift_matches_the_actual_job_type(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("if job.good == Good::Wood { Job::Lumberjack } else { Job::Miner }", runner)
        self.assertIn("on_shift(role)", runner)

    def test_receipt_is_serde_default_and_does_not_break_old_saves(self):
        job = section(CORE, "pub struct GatherJob {", "\n}")
        self.assertIn("#[serde(default)]\n    pub native_edit_confirmed: bool", job)
        self.assertIn("native_edit_confirmed: false", CORE)

    def test_issued_without_durable_native_proof_is_quarantined_before_air(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        issued = runner.split("if job.issued {", 1)[1]
        self.assertIn("if !job.native_edit_confirmed {", issued)
        self.assertIn("block_gather(id)?", issued)
        self.assertLess(issued.index("if !job.native_edit_confirmed"),
                        issued.index('if name == "minecraft:air"'))

    def test_adapter_checks_native_command_acceptance_and_fresh_air_together(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("let accepted = game.native_run_command(&cmd);", runner)
        self.assertIn("let now_air = world_block(game, &dimension, job.position)", runner)
        self.assertIn('== Some("minecraft:air")', runner)
        self.assertIn("if accepted && now_air {", runner)
        self.assertIn("mark_gather_native_edit_confirmed(id)?", runner)
        self.assertIn("block_gather(id)?", runner)

    def test_native_ack_is_saved_after_command_and_before_pickup(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        native = runner.index("let accepted = game.native_run_command(&cmd)")
        observed = runner.index("let now_air = world_block(", native)
        ack = runner.index("mark_gather_native_edit_confirmed(id)?", observed)
        durable = runner.index("save_autonomous(state, issued)?", ack)
        self.assertLess(native, observed)
        self.assertLess(observed, ack)
        self.assertLess(ack, durable)
        self.assertLess(runner.index("mark_gather_issued(id)?"), native)

    def test_core_enforces_native_ack_before_pickup_and_delivery(self):
        ack = section(CORE, "pub fn mark_gather_native_edit_confirmed(", "\n    }")
        pickup = section(CORE, "pub fn witness_gather_pickup(", "\n    }")
        confirm = section(CORE, "pub fn confirm_gather(", "\n    }")
        self.assertIn("!job.issued || job.blocked || job.worker_id.is_none()", ack)
        self.assertIn("job.native_edit_confirmed = true", ack)
        self.assertIn("!job.native_edit_confirmed", pickup)
        self.assertIn("!job.native_edit_confirmed", confirm)
        self.assertNotIn("warehouse_credit(", ack)

    def test_legacy_witnessed_receipt_loads_but_cannot_mint_wood(self):
        validation = section(CORE, "pub fn validate(", "\n    }")
        self.assertIn("if gather.native_edit_confirmed", validation)
        self.assertIn("if gather.pickup_witnessed", validation)
        self.assertNotIn("if gather.pickup_witnessed && !gather.native_edit_confirmed", validation)
        self.assertIn("mod legacy_witnessed_gather_migration_tests", CORE)

    def test_native_ack_never_blindly_reissues_or_gives_items(self):
        runner = section(GLUE, "fn gather_one_step(", "\n}")
        self.assertEqual(runner.count("game.native_run_command(&cmd)"), 1)
        self.assertNotIn("native_give_items(", runner)
        self.assertNotIn("native_take_items(", runner)
        self.assertIn("return save_autonomous(state, before);", runner)
        self.assertIn("save_autonomous(state, issued)?;", runner)
        self.assertIn("native edit confirmed={}", GLUE)
        self.assertIn("job.native_edit_confirmed, job.pickup_witnessed", GLUE)


if __name__ == "__main__":
    unittest.main()
