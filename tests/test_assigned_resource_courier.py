"""Offline contracts: the worker who mines a native block must also return.

These source assertions are NOT a Rust compile or a Minecraft gameplay E2E.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()


def method(src: str, name: str, ending: str = "\n    }") -> str:
    return src.split(name, 1)[1].split(ending, 1)[0]


class AssignedGatherWorkerContracts(unittest.TestCase):
    def test_worker_id_defaults_for_legacy_saves(self):
        struct = method(CORE, "pub struct GatherJob {", "\n}")
        self.assertIn("#[serde(default)]\n    pub worker_id: Option<u64>", struct)
        self.assertIn("worker_id: None, issued: false,\n            native_edit_confirmed: false, pickup_witnessed: false, blocked: false", CORE)

    def test_binding_requires_living_qualified_citizen_and_single_owner(self):
        assign = method(CORE, "pub fn assign_gather_worker(")
        for fragment in (
            "resident.alive && resident.id == resident_id",
            "resident.job == required",
            "job.issued || job.blocked",
            "job.worker_id.is_some_and(|assigned| assigned != resident_id)",
            "worker_id = Some(resident_id)",
        ):
            self.assertIn(fragment, assign)

    def test_unbound_native_job_cannot_be_issued_or_credited(self):
        issued = method(CORE, "pub fn mark_gather_issued(")
        confirm = method(CORE, "pub fn confirm_gather(")
        self.assertIn("if job.worker_id.is_none()", issued)
        self.assertIn('job.worker_id.ok_or("unassigned gather cannot be credited")?', confirm)

    def test_same_worker_carries_ownership_into_every_tree_segment(self):
        confirm = method(CORE, "pub fn confirm_gather(")
        self.assertIn("let worker_id = job.worker_id", confirm)
        self.assertIn("worker_id: Some(worker_id), issued: false,\n                    native_edit_confirmed: false, pickup_witnessed: false, blocked: false", confirm)
        self.assertIn("v.warehouse_credit(good, 1)?;", confirm)

    def test_saved_identity_cannot_reference_stranger_or_wrong_job(self):
        validation = method(CORE, "pub fn validate(")
        self.assertIn("if let Some(worker_id) = gather.worker_id", validation)
        self.assertIn("r.id == worker_id && r.job == role", validation)

    def test_only_assigned_worker_is_sent_to_source_or_hall(self):
        routing = method(GLUE, "fn guide_actors(", "\n}")
        self.assertIn("job.worker_id.is_none_or(|assigned| assigned == resident_id)", routing)
        self.assertIn("if target.issued", routing)

    def test_loaded_worker_is_saved_before_native_issued_edit(self):
        runner = method(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("job.worker_id.is_none_or(|assigned| assigned == r.id)", runner)
        self.assertIn("assign_gather_worker(id, selected_worker)?", runner)
        self.assertIn("return save_autonomous(state, before);", runner)
        self.assertLess(runner.index("assign_gather_worker(id, selected_worker)?"),
                        runner.index("mark_gather_issued(id)?"))
        self.assertLess(runner.index("mark_gather_issued(id)?"),
                        runner.index("game.native_run_command(&cmd)"))

    def test_different_same_profession_actor_cannot_fake_return(self):
        runner = method(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("let Some(assigned) = job.worker_id else", runner)
        self.assertIn("r.alive && r.job == role && r.id == assigned", runner)
        self.assertIn("native_worker_near(actor.pos, v.center, 8.0, 4.0)", runner)
        self.assertIn("world.as_mut().unwrap().confirm_gather(id)?", runner)

    def test_legacy_issued_receipt_is_quarantined_without_stock_credit(self):
        runner = method(GLUE, "fn gather_one_step(", "\n}")
        start = runner.split("let Some(assigned) = job.worker_id else", 1)[1]
        stop = start.split("if name ==", 1)[0]
        self.assertIn("block_gather(id)?", stop)
        self.assertIn("return save_autonomous(state, before);", stop)
        self.assertNotIn("confirm_gather(id)?", stop)

    def test_operator_can_see_exact_assigned_resident_without_mutation(self):
        audit = GLUE.split('if action == "gathercheck" {', 1)[1].split(
            'if action == "barrelcheck" {', 1
        )[0]
        self.assertIn("job.worker_id", audit)
        self.assertNotIn("save(state)", audit)


if __name__ == "__main__":
    unittest.main()
