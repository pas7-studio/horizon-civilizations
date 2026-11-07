"""Source contracts for fail-closed lifecycle saves.

These checks do not replace a real filesystem fault-injection playtest.
"""
from pathlib import Path
import unittest

from civil_sources import core_source, glue_source
GLUE = glue_source()

class WorldSwitchSafetyContracts(unittest.TestCase):
    def test_world_switch_never_discards_world_after_failed_save(self):
        start = GLUE.index("fn active(path: PathBuf)")
        end = GLUE.index("fn save(active:", start)
        code = GLUE[start:end]
        self.assertIn("if let Err(error) = save(old)", code)
        self.assertIn("return locked;", code)
        self.assertIn("state.problem = Some(", code)
        self.assertLess(code.index("return locked;"), code.rindex("*locked = Some(load(path));"))

    def test_shutdown_retains_in_memory_world_on_save_failure(self):
        code = GLUE.split("pub fn closed()", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("if let Err(error) = save(state)", code)
        self.assertIn("return;", code)
        self.assertLess(code.index("return;"), code.index("*guard = None;"))

    def test_orphaned_first_save_tmp_blocks_empty_world_creation(self):
        start = GLUE.index("fn load(path: PathBuf)")
        end = GLUE.index("fn active(path:", start)
        code = GLUE[start:end]
        self.assertIn('["world.json.swap", "world.json.tmp"]', code)
        self.assertIn("world: None, problem: Some(message)", code)
        self.assertLess(code.index("for remnant in"), code.index("Ok(World::default())"))

    def test_stale_temp_is_rejected_before_writing(self):
        start = GLUE.index("fn save(active:")
        end = GLUE.index("\npub fn closed()", start)
        code = GLUE[start:end]
        self.assertIn("if swap.exists() || temp.exists()", code)
        self.assertLess(code.index("if swap.exists() || temp.exists()"),
                        code.index("fs::File::create(&temp)"))
        self.assertIn("pending world.json.swap or world.json.tmp", code)

    def test_native_rust_regressions_cover_first_and_later_save_tmp(self):
        native = GLUE.split("mod interrupted_first_save_tests", 1)[1]
        self.assertEqual(native.count("#[test]"), 3)
        self.assertIn("interrupted_first_write_cannot_look_like_a_new_empty_world", native)
        self.assertIn("a_valid_primary_does_not_silently_discard_an_abandoned_tmp", native)
        self.assertIn("stale_tmp_is_never_truncated_by_a_later_save", native)

    def test_stale_swap_guard_precedes_file_mutations(self):
        start = GLUE.index("fn save(active:")
        end = GLUE.index("\npub fn closed()", start)
        code = GLUE[start:end]
        self.assertLess(code.index("if swap.exists()"),
                        code.index('fs::File::create(&temp)'))
        self.assertLess(code.index("if swap.exists()"),
                        code.index("fs::copy(&dest"))
