"""Source contract for the optional Windows native client CI workflow."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/build-windows-client.yml"

class NativeWindowsWorkflowTests(unittest.TestCase):
    def test_compiles_pinned_horizon_and_uploads_exe_only(self):
        config = WORKFLOW.read_text()
        self.assertIn("runs-on: windows-latest", config)
        self.assertIn("python tools/native_build.py --test --build-client", config)
        self.assertIn("horizon-client.exe", config)
        self.assertIn("ci/windows-client-smoke", config)
        self.assertIn("workflow_dispatch:", config)
        self.assertIn("if-no-files-found: error", config)
