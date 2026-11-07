"""Verify filtered native Rust tests cannot silently match zero cases.

Uses a fake Cargo process and has no native toolchain or network dependency.
"""
import importlib.util
from pathlib import Path
from subprocess import CompletedProcess
from unittest import TestCase, mock

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "tools" / "native_build.py"


def load_native_builder():
    spec = importlib.util.spec_from_file_location("civilizations_native_test_runner", MODULE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class NativeTestSelectionContracts(TestCase):
    def test_nonempty_native_suite_is_listed_and_then_executed(self):
        builder = load_native_builder()
        listed = CompletedProcess(
            args=[], returncode=0,
            stdout=("game::civilizations_glue::interrupted_first_save_tests::a: test\n"
                    "game::civilizations_glue::interrupted_first_save_tests::b: test\n"
                    "2 tests, 0 benchmarks\n"),
        )
        with mock.patch.object(builder.subprocess, "run", return_value=listed) as listing:
            with mock.patch.object(builder, "run") as executor:
                evidence = builder.run_filtered_native_tests(
                    Path("staged/Cargo.toml"), "civilizations_glue::",
                )
        self.assertEqual(evidence, {
            "filter": "civilizations_glue::", "discovered": 2, "status": "PASS",
        })
        self.assertIn("--list", listing.call_args.args[0])
        self.assertEqual(executor.call_args.args[-1], "civilizations_glue::")

    def test_zero_matched_tests_cannot_be_reported_as_pass(self):
        builder = load_native_builder()
        listed = CompletedProcess(
            args=[], returncode=0, stdout="0 tests, 0 benchmarks\n",
        )
        with mock.patch.object(builder.subprocess, "run", return_value=listed):
            with mock.patch.object(builder, "run") as executor:
                with self.assertRaisesRegex(RuntimeError, "matched ZERO tests"):
                    builder.run_filtered_native_tests(
                        Path("staged/Cargo.toml"), "civilizations_glue::",
                    )
        executor.assert_not_called()

    def test_enumeration_failure_stops_before_test_execution(self):
        builder = load_native_builder()
        failed = CompletedProcess(
            args=[], returncode=101,
            stdout="error: could not compile injected native bridge\n",
        )
        with mock.patch.object(builder.subprocess, "run", return_value=failed):
            with mock.patch.object(builder, "run") as executor:
                with self.assertRaisesRegex(RuntimeError, "enumeration failed"):
                    builder.run_filtered_native_tests(
                        Path("staged/Cargo.toml"), "civilizations_core::",
                    )
        executor.assert_not_called()
