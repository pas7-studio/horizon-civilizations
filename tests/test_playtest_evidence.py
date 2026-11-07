"""Read-only evidence collector contracts: no gameplay claims."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("playtest_evidence", ROOT / "tools/playtest_evidence.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


def fixture(world_dir, *, uuid="a" * 32, extra=None):
    folder = world_dir / "horizon-civilizations"
    folder.mkdir()
    village = {
        "id": 1, "name": "Ashford", "actors_spawned": True,
        "pending_visual_buildings": 1, "visual_sites": [], "visual_job": {"cursor": 11, "blocked": False},
        "farm": {"stage": "growing", "cursor": 0, "completed_cycles": 2,
                 "harvest_sequence": 2,
                 "harvest_receipts": [{"sequence": 2, "world_day": 1, "crops": 8}]},
        "residents": [{"id": i, "alive": True, "entity_uuid": f"{i:032x}"} for i in range(1, 11)]
    }
    village["residents"][0]["entity_uuid"] = uuid
    data = {"schema": 1, "day": 2, "settlements": {"1": village}}
    if extra:
        data.update(extra)
    (folder / "world.json").write_text(json.dumps(data))
    return data


class EvidenceTests(unittest.TestCase):
    def test_valid_uuid_and_progress_reports_saved_state_only(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            report = mod.inspect_save(root, settlement="Ashford", min_linked=10)
            self.assertEqual(report["status"], "PASS")
            self.assertFalse(report["gameplay_verified"])
            self.assertEqual(report["settlements"][0]["uuid_linked"], 10)
            self.assertEqual(report["settlements"][0]["farm"]["harvest_sequence"], 2)

    def test_missing_and_corrupt_saves_fail_closed(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            self.assertEqual(mod.inspect_save(root)["status"], "FAIL")
            fixture(root)
            (root / "horizon-civilizations/world.json").write_text("{invalid")
            self.assertEqual(mod.inspect_save(root)["status"], "FAIL")

    def test_stale_swap_and_short_roster_fail(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            (root / "horizon-civilizations/world.json.swap").write_text("broken")
            report = mod.inspect_save(root, settlement="Ashford", min_linked=11)
            self.assertEqual(report["status"], "FAIL")
            self.assertEqual(len(report["failures"]), 2)

    def test_stale_first_save_tmp_blocks_false_green_report(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            pending = root / "horizon-civilizations" / "world.json.tmp"
            pending.write_bytes(b"recoverable newer state")
            report = mod.inspect_save(root, settlement="Ashford", min_linked=10)
            self.assertEqual(report["status"], "FAIL")
            self.assertTrue(any("world.json.tmp" in reason
                                for reason in report["failures"]))
            self.assertEqual(pending.read_bytes(), b"recoverable newer state")
            self.assertFalse(report["gameplay_verified"])

    def test_only_tmp_without_primary_does_not_look_like_empty_new_world(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            mod_dir = root / "horizon-civilizations"
            mod_dir.mkdir()
            (mod_dir / "world.json.tmp").write_bytes(b"first interrupted write")
            report = mod.inspect_save(root)
            self.assertEqual(report["status"], "FAIL")
            self.assertTrue(any("world.json.tmp" in reason
                                for reason in report["failures"]))
            self.assertTrue(any("world.json missing" in reason
                                for reason in report["failures"]))
            self.assertEqual((mod_dir / "world.json.tmp").read_bytes(),
                             b"first interrupted write")

    def test_invalid_and_duplicate_uuid_fail(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root, uuid="0" * 32)
            self.assertEqual(mod.inspect_save(root)["status"], "FAIL")
            village = json.loads((root / "horizon-civilizations/world.json").read_text())
            village["settlements"]["1"]["residents"][0]["entity_uuid"] = "2".zfill(32)
            (root / "horizon-civilizations/world.json").write_text(json.dumps(village))
            self.assertEqual(mod.inspect_save(root)["status"], "FAIL")

    def test_pending_actor_recovery_warns_without_fake_clean_pass(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            save = root / "horizon-civilizations/world.json"
            world = json.loads(save.read_text())
            village = world["settlements"]["1"]
            village["residents"][0]["entity_uuid"] = None
            village["actor_recovery_issued"] = [1]
            save.write_text(json.dumps(world))
            report = mod.inspect_save(root, settlement="Ashford", min_linked=9)
            self.assertEqual(report["status"], "PASS_WITH_WARNINGS")
            self.assertEqual(report["settlements"][0]["pending_actor_recoveries"], 1)
            self.assertTrue(any("reconcile before another spawn" in warning
                                for warning in report["warnings"]))

    def test_pending_recovery_marker_for_bound_or_unknown_actor_is_invalid(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            save = root / "horizon-civilizations/world.json"
            world = json.loads(save.read_text())
            world["settlements"]["1"]["actor_recovery_issued"] = [1, 999]
            save.write_text(json.dumps(world))
            report = mod.inspect_save(root, settlement="Ashford", min_linked=10)
            self.assertEqual(report["status"], "FAIL")
            self.assertTrue(any("missing or UUID-linked resident" in reason
                                for reason in report["failures"]))

    def test_dead_unbound_resident_recovery_tombstone_is_not_corruption(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            save = root / "horizon-civilizations/world.json"
            world = json.loads(save.read_text())
            village = world["settlements"]["1"]
            village["residents"][0]["alive"] = False
            village["residents"][0]["entity_uuid"] = None
            village["actor_recovery_issued"] = [1]
            save.write_text(json.dumps(world))
            report = mod.inspect_save(root, settlement="Ashford", min_linked=9)
            self.assertEqual(report["status"], "PASS_WITH_WARNINGS")
            self.assertEqual(report["settlements"][0]["pending_actor_recoveries"], 1)

    def test_no_raw_player_log_text_in_report(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            log = root / "latest.log"
            log.write_text("[civilizations] error secret-email@example.org\nplayer secret-email@example.org\n")
            report = mod.inspect_save(root, log_file=log)
            self.assertEqual(report["log_counts"]["civilizations_lines"], 1)
            self.assertNotIn("secret-email", json.dumps(report))

    def test_report_does_not_write_game_files(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            fixture(root)
            save = root / "horizon-civilizations/world.json"
            before = save.read_bytes()
            mod.inspect_save(root)
            self.assertEqual(before, save.read_bytes())


if __name__ == "__main__":
    unittest.main()
