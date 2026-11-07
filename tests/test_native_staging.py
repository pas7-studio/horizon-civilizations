"""Native source staging: pinned Git identity and safe symlinks."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

def native_module():
    spec = importlib.util.spec_from_file_location("civilizations_native_build", ROOT / "tools/native_build.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], text=True, capture_output=True, check=True).stdout.strip()

class NativeStaging(unittest.TestCase):
    def test_git_pin_and_safe_link_materialization(self):
        with tempfile.TemporaryDirectory() as folder:
            parent = Path(folder)
            repo = parent / "upstream"
            repo.mkdir()
            git(repo, "init", "-q")
            git(repo, "config", "user.name", "CI test")
            git(repo, "config", "user.email", "ci@example.invalid")
            (repo / "AGENTS.md").write_text("source-only\n")
            (repo / "CLAUDE.md").symlink_to("AGENTS.md")
            git(repo, "add", ".")
            git(repo, "commit", "-qm", "first")
            commit = git(repo, "rev-parse", "HEAD")
            stage = parent / "stage"
            native_module().prepare_pristine_source(repo, stage, commit)
            self.assertEqual(git(stage, "rev-parse", "HEAD"), commit)
            self.assertEqual((stage / "CLAUDE.md").read_text(), "source-only\n")
            self.assertFalse((stage / "CLAUDE.md").is_symlink())
            self.assertTrue((repo / "CLAUDE.md").is_symlink())
            self.assertEqual(git(repo, "status", "--porcelain"), "")

    def test_external_link_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            parent = Path(folder)
            repo = parent / "upstream"
            repo.mkdir()
            git(repo, "init", "-q")
            git(repo, "config", "user.name", "CI test")
            git(repo, "config", "user.email", "ci@example.invalid")
            secret = parent / "outside.txt"
            secret.write_text("should never be copied")
            (repo / "external").symlink_to(secret)
            git(repo, "add", ".")
            git(repo, "commit", "-qm", "first")
            with self.assertRaisesRegex(RuntimeError, "external symlink refused"):
                native_module().prepare_pristine_source(repo, parent / "stage", git(repo, "rev-parse", "HEAD"))

if __name__ == "__main__":
    unittest.main()
