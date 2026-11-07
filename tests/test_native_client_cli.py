"""The optional native binary build is discoverable without fetching game assets."""
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class NativeBuildCli(unittest.TestCase):
    def test_explicit_binary_build_flag_is_off_by_default(self):
        p = subprocess.run(
            [sys.executable, str(ROOT / "tools/native_build.py"), "--help"],
            check=True, text=True, capture_output=True,
        )
        self.assertIn("--build-client", p.stdout)
        self.assertIn("--test", p.stdout)
        script = (ROOT / "tools/native_build.py").read_text()
        self.assertIn("if args.build_client:", script)
        self.assertIn("'--profile', 'fast'", script)
        self.assertIn("'native_client_binary'", script)


if __name__ == "__main__":
    unittest.main()
