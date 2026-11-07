"""Source contracts for fail-closed lifecycle saves.

These checks do not replace a real filesystem fault-injection playtest.
"""
from pathlib import Path
import unittest

GLUE = (Path(__file__).resolve().parents[1] /
        "mixins" / "civilizations" / "glue.rs").read_text()

class WorldSwitchSafetyContracts(unittest.TestCase):
    def test_world_switch_never_discards_world_after_failed_save(self):
        start = GLUE.index("fn active(path: PathBuf)")
        end = GLUE.index("\nfn save(active:", start)
        code = GLUE[start:end]
        self.assertIn("if let Err(error) = save(old)", code)
        self.assertIn("return locked;", code)
        self.assertIn("state.problem = Some(", code)
        self.assertLess(code.index("return locked;"), code.rindex("*locked = Some(load(path));"))

    def test_shutdown_retains_in_memory_world_on_save_failure(self):
        start = GLUE.index("pub fn closed()")
        end = GLUE.index("\npub fn register(", start)
        code = GLUE[start:end]
        self.assertIn("if let Err(error) = save(state)", code)
        self.assertIn("return;", code)
        self.assertLess(code.index("return;"), code.index("*guard = None;"))

    def test_stale_swap_guard_precedes_file_mutations(self):
        start = GLUE.index("fn save(active:")
        end = GLUE.index("\npub fn closed()", start)
        code = GLUE[start:end]
        self.assertLess(code.index("if swap.exists()"),
                        code.index('fs::File::create(&temp)'))
        self.assertLess(code.index("if swap.exists()"),
                        code.index("fs::copy(&dest"))
