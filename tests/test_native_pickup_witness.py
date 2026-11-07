"""Source-level contract: physical source pickup precedes returning to hall.

Static assertions do not certify a Rust build, native pickup, or game E2E.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "mixins/civilizations/core.rs").read_text(encoding="utf-8")
GLUE = (ROOT / "mixins/civilizations/glue.rs").read_text(encoding="utf-8")


def body(source: str, signature: str, end: str = "\n    }") -> str:
    return source.split(signature, 1)[1].split(end, 1)[0]


class NativeSourcePickupWitnessContracts(unittest.TestCase):
    def test_old_saved_jobs_default_to_not_pickup_witnessed(self):
        structure = body(CORE, "pub struct GatherJob {", "\n}")
        self.assertIn("#[serde(default)]\n    pub pickup_witnessed: bool", structure)
        self.assertIn("pickup_witnessed: false, blocked: false", CORE)

    def test_issued_job_cannot_credit_without_source_witness(self):
        issue = body(CORE, "pub fn mark_gather_issued(")
        credit = body(CORE, "pub fn confirm_gather(")
        self.assertIn("job.issued = true;", issue)
        self.assertIn("!job.pickup_witnessed", credit)
        self.assertIn("v.warehouse_credit(good, 1)?;", credit)

    def test_pickup_is_issued_once_for_same_assigned_worker(self):
        fn = body(CORE, "pub fn witness_gather_pickup(")
        self.assertIn("!job.issued || job.blocked || job.pickup_witnessed", fn)
        self.assertIn("job.worker_id != Some(resident_id)", fn)
        self.assertIn("job.pickup_witnessed = true;", fn)
        self.assertNotIn("warehouse_credit(", fn)

    def test_saved_witness_must_have_issued_worker_identity(self):
        validate = body(CORE, "pub fn validate(")
        self.assertIn("if gather.pickup_witnessed &&", validate)
        self.assertIn("!gather.issued || gather.worker_id.is_none()", validate)

    def test_oak_followup_starts_with_new_unwitnessed_segment(self):
        confirm = body(CORE, "pub fn confirm_gather(")
        self.assertIn("worker_id: Some(worker_id), issued: false,", confirm)
        self.assertIn("pickup_witnessed: false, blocked: false,", confirm)

    def test_worker_not_sent_to_hall_until_witness_saved(self):
        guide = body(GLUE, "fn guide_actors(", "\n}")
        self.assertIn("if target.issued && target.pickup_witnessed {", guide)
        self.assertIn("target.stand[0]", guide)
        self.assertIn("target.stand[1]", guide)
        self.assertIn("target.stand[2]", guide)

    def test_air_without_specific_source_worker_does_not_credit(self):
        executor = body(GLUE, "fn gather_one_step(", "\n}")
        self.assertIn("if !job.pickup_witnessed {", executor)
        self.assertIn("r.alive && r.job == role && r.id == assigned", executor)
        self.assertIn("native_worker_near(actor.pos, job.stand, 2.5, 2.0)", executor)
        self.assertIn("native_worker_reaches_block(actor.pos, job.position, 4.5)", executor)
        self.assertIn("if !witnessed { return Ok(()); }", executor)

    def test_pickup_is_durable_before_hall_delivery(self):
        executor = body(GLUE, "fn gather_one_step(", "\n}")
        witness = executor.index("witness_gather_pickup(id, assigned)?")
        save = executor.index("return save_autonomous(state, before);", witness)
        delivery = executor.index("let delivered =", witness)
        confirm = executor.index("confirm_gather(id)?", witness)
        self.assertLess(witness, save)
        self.assertLess(save, delivery)
        self.assertLess(delivery, confirm)

    def test_operator_receipt_displays_pickup_witness_state(self):
        audit = GLUE.split('if action == "gathercheck" {', 1)[1].split(
            'if action == "barrelcheck" {', 1
        )[0]
        self.assertIn("job.pickup_witnessed", audit)
        self.assertNotIn("save(state)", audit)

    def test_no_virtual_items_in_pickup_phase(self):
        witness = body(CORE, "pub fn witness_gather_pickup(")
        for forbidden in ("native_give_items(", "native_run_command(",
                          "warehouse_credit(", "native_take_items("):
            self.assertNotIn(forbidden, witness)


if __name__ == "__main__":
    unittest.main()
