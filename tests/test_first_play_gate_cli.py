"""Tests the release-gate harness with fake executables only.

These tests DO NOT compile the native Rust game or prove gameplay.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

GATE = Path(__file__).resolve().parents[1] / "tools" / "first_play_gate.py"
BRANCH = "feat/guarded-partial-villager-recovery-v0115"


class LocalReleaseGateHarnessTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        (self.repo / "tools").mkdir()
        (self.repo / "tests").mkdir()
        (self.repo / "tests" / "test_smoke.py").write_text(
            "import unittest\nclass Smoke(unittest.TestCase):\n"
            "    def test_mock(self): self.assertTrue(True)\n")
        (self.repo / "mixins" / "civilizations").mkdir(parents=True)
        (self.repo / "mixins" / "civilizations" / "core.rs").write_text("// fake\n")
        (self.repo / ".gitignore").write_text("/target/\n")
        for file in ("package.py", "verify_static.py"):
            (self.repo / "tools" / file).write_text("print('mock only')\n")
        (self.repo / "compatibility.json").write_text(json.dumps({
            "horizon_revision": "a" * 40,
        }))
        (self.repo / "tools" / "native_build.py").write_text(
            "import json, os, sys\n"
            "from pathlib import Path\n"
            "repo = Path(__file__).resolve().parents[1]\n"
            "client = repo / 'target' / 'native-check' / 'modded' / 'target' / 'fast' / "
            "('horizon-client.exe' if os.name == 'nt' else 'horizon-client')\n"
            "build = '--build-client' in sys.argv\n"
            "if build and not os.environ.get('MOCK_MISSING_CLIENT'):\n"
            "    client.parent.mkdir(parents=True, exist_ok=True)\n"
            "    client.write_bytes(b'X' * 2048)\n"
            "revision = 'wrong' if os.environ.get('MOCK_WRONG_REVISION') else '" +
                "a" * 40 + "'\n"
            "print(json.dumps({'cargo_check': 'PASS', 'horizon_revision': revision, "
            "'modded_horizon': str(repo / 'target' / 'native-check' / 'modded'), "
            "'native_client_binary': str(client) if build else 'NOT BUILT'}))\n"
        )
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self._exe("rustc", '#!/bin/sh\necho "rustc 1.95.0 (mock)"\n')
        self._exe("rustfmt", '#!/bin/sh\necho "rustfmt 1.95.0 (mock)"\n')
        self._exe("cargo", '#!/bin/sh\n'
                  'if [ "$1" = "test" ] && [ "${FAIL_TEST:-}" = "yes" ]; then exit 9; fi\n'
                  'if [ "$1" = "fmt" ] && [ "${FAIL_FORMAT:-}" = "yes" ]; then exit 8; fi\n'
                  'echo "cargo 1.95.0 (mock)"\n')
        self.git("init", "-q")
        self.git("checkout", "-qb", BRANCH)
        self.git("-c", "user.email=test@invalid.local", "-c", "user.name=Test",
                 "add", ".")
        self.git("-c", "user.email=test@invalid.local", "-c", "user.name=Test",
                 "commit", "-qm", "initial")
        self.env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ.get("PATH", ""))

    def _exe(self, name: str, text: str) -> None:
        if os.name == "nt":
            # Windows can't run POSIX shebang mocks. Exercise the same CLI
            # through harmless .cmd stubs resolved by PATHEXT.
            f = self.bin / (name + ".cmd")
            if name == "cargo":
                text = ('@echo off\n'
                        'if "%1"=="test" if "%FAIL_TEST%"=="yes" exit /b 9\n'
                        'if "%1"=="fmt" if "%FAIL_FORMAT%"=="yes" exit /b 8\n'
                        'echo cargo 1.95.0 (mock)\n')
            elif name == "rustc":
                version = "1.94.2" if "1.94.2" in text else "1.95.0"
                text = f'@echo off\necho rustc {version} (mock)\n'
            else:
                text = '@echo off\necho rustfmt 1.95.0 (mock)\n'
        else:
            f = self.bin / name
        f.write_text(text)
        f.chmod(0o755)

    def git(self, *args: str) -> None:
        subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True)

    def run_gate(self, *args: str, extra_env: dict | None = None):
        env = dict(self.env)
        env.update(extra_env or {})
        return subprocess.run([sys.executable, str(GATE), "--repo", str(self.repo), *args],
                              text=True, capture_output=True, env=env)

    def latest_report(self):
        reports = sorted((self.repo / "target" / "first-play-gate").glob("*/report.json"))
        return json.loads(reports[-1].read_text())

    def test_dry_run_does_not_write_or_execute(self):
        result = self.run_gate("--dry-run", "--build-client")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("pinned-horizon-native-build", result.stdout)
        self.assertFalse((self.repo / "target").exists())

    def test_source_gate_passes_in_mock_clean_repo(self):
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        data = self.latest_report()
        self.assertEqual(data["gate"], "SOURCE_CHECK_PASS")
        self.assertEqual(len(data["steps"]), 7)
        self.assertEqual(data["gameplay_e2e"], "NOT_TESTED")
        self.assertTrue(data["clean_end"])

    def test_compiler_failure_is_reported_before_formatting(self):
        result = self.run_gate(extra_env={"FAIL_TEST": "yes", "FAIL_FORMAT": "yes"})
        self.assertEqual(result.returncode, 1)
        data = self.latest_report()
        self.assertEqual([step["name"] for step in data["steps"]], ["cargo-test"])
        self.assertEqual(data["steps"][0]["exit_code"], 9)
        self.assertEqual(data["gate"], "BLOCKED")

    def test_format_remains_mandatory_after_compilation(self):
        result = self.run_gate(extra_env={"FAIL_FORMAT": "yes"})
        self.assertEqual(result.returncode, 1)
        data = self.latest_report()
        self.assertEqual([step["name"] for step in data["steps"]],
                         ["cargo-test", "clippy", "cargo-fmt"])
        self.assertEqual(data["steps"][-1]["exit_code"], 8)
        self.assertEqual(data["gate"], "BLOCKED")

    def test_mock_native_build_marks_only_ready_for_gameplay_not_playable(self):
        result = self.run_gate("--build-client")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        data = self.latest_report()
        self.assertEqual(data["gate"], "READY_FOR_GAMEPLAY_E2E")
        self.assertEqual(len(data["steps"]), 8)
        self.assertEqual(data["gameplay_e2e"], "NOT_TESTED")
        self.assertTrue(data["native_build_verified"])
        self.assertGreater(data["native_client_bytes"], 1024)
        self.assertEqual(len(data["native_client_sha256"]), 64)
        self.assertTrue(Path(data["native_client_path"]).is_file())

    def test_mock_native_exit_zero_without_binary_is_blocked(self):
        result = self.run_gate("--build-client", extra_env={"MOCK_MISSING_CLIENT": "yes"})
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        data = self.latest_report()
        self.assertEqual(data["gate"], "BLOCKED")
        self.assertFalse(data["native_build_verified"])
        self.assertIn("missing or too small", data["blocking_reason"])
        self.assertEqual(data["steps"][-1]["status"], "FAIL")
        self.assertIsNone(data["native_client_sha256"])

    def test_native_revision_mismatch_cannot_mark_client_ready(self):
        result = self.run_gate("--build-client", extra_env={"MOCK_WRONG_REVISION": "yes"})
        self.assertEqual(result.returncode, 1)
        data = self.latest_report()
        self.assertEqual(data["gate"], "BLOCKED")
        self.assertIn("wrong pinned Horizon revision", data["blocking_reason"])

    def test_native_check_does_not_require_client_binary(self):
        result = self.run_gate("--native-check")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        data = self.latest_report()
        self.assertEqual(data["gate"], "SOURCE_AND_NATIVE_CHECK_PASS")
        self.assertTrue(data["native_build_verified"])
        self.assertIsNone(data["native_client_path"])
        self.assertEqual(data["gameplay_e2e"], "NOT_TESTED")

    def test_first_test_failure_captures_logs_and_blocks(self):
        result = self.run_gate(extra_env={"FAIL_TEST": "yes"})
        self.assertEqual(result.returncode, 1)
        data = self.latest_report()
        self.assertEqual(data["gate"], "BLOCKED")
        self.assertEqual(data["steps"][-1]["name"], "cargo-test")
        self.assertEqual(data["steps"][-1]["exit_code"], 9)
        self.assertTrue((self.repo / data["steps"][-1]["log"]).is_file())

    def test_dirty_checkout_refused_but_can_diagnose_without_certifying(self):
        (self.repo / "README.md").write_text("dirty\n")
        blocked = self.run_gate()
        self.assertEqual(blocked.returncode, 1)
        self.assertIn("dirty source checkout", self.latest_report()["blocking_reason"])
        allowed = self.run_gate("--allow-dirty")
        self.assertEqual(allowed.returncode, 1)
        self.assertEqual(self.latest_report()["gate"], "BLOCKED")
        self.assertIn("diagnostic-only", self.latest_report()["blocking_reason"])

    def test_wrong_branch_or_sha_is_refused(self):
        result = self.run_gate("--expected-branch", "main")
        self.assertEqual(result.returncode, 1)
        self.assertIn("wrong branch", self.latest_report()["blocking_reason"])
        result = self.run_gate("--expected-sha", "0" * 40)
        self.assertEqual(result.returncode, 1)
        self.assertIn("wrong SHA", self.latest_report()["blocking_reason"])

    def test_wrong_rust_version_blocks_first_stage(self):
        self._exe("rustc", '#!/bin/sh\necho "rustc 1.94.2 (mock)"\n')
        result = self.run_gate()
        self.assertEqual(result.returncode, 1)
        self.assertIn("expected Rust 1.95", self.latest_report()["blocking_reason"])
        self.assertEqual(self.latest_report()["steps"], [])


if __name__ == "__main__":
    unittest.main()
