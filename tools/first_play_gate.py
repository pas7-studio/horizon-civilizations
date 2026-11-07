#!/usr/bin/env python3
"""Run the exact local Horizon Civilizations release-candidate source gate.

No GitHub Actions, server deployment, credentials, Minecraft assets or
interactive gameplay. Produces local evidence in target/first-play-gate/.
A passing native client compilation is READY_FOR_GAMEPLAY_E2E, not PLAYABLE.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_BRANCH = "feat/guarded-partial-villager-recovery-v0115"


@dataclass(frozen=True)
class Step:
    name: str
    argv: list[str]
    timeout_seconds: int = 900


@dataclass
class StepResult:
    name: str
    argv: list[str]
    status: str
    exit_code: int | None
    seconds: float
    log: str
    error: str | None = None


def planned_steps(root: Path, native_check: bool, build_client: bool) -> list[Step]:
    # Compiler errors are actionable earlier than large rustfmt diffs.
    # Keep format and Clippy strict: successful compilation alone never
    # produces PASS if formatting or lint checks fail. No format mutations.
    steps = [
        Step("cargo-test", ["cargo", "test"], 1800),
        Step("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], 1800),
        Step("cargo-fmt", ["cargo", "fmt", "--all", "--check"]),
    ]
    for file in sorted((root / "mixins/civilizations").glob("*.rs")):
        steps.append(Step("rustfmt-" + file.stem,
                          ["rustfmt", "--edition", "2024", "--check", str(file)]))
    steps += [
        Step("python-unittest", [sys.executable, "-m", "unittest", "discover",
                                 "-s", "tests", "-p", "test_*.py", "-v"], 1200),
        Step("source-package", [sys.executable, "tools/package.py"]),
        Step("static-verification", [sys.executable, "tools/verify_static.py"]),
    ]
    if native_check or build_client:
        native = [sys.executable, "tools/native_build.py", "--test"]
        if build_client:
            native.append("--build-client")
        steps.append(Step("pinned-horizon-native-build", native, 7200))
    return steps


def git_value(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True,
                                   stderr=subprocess.PIPE, timeout=20).strip()


_DIAGNOSTIC_LINE = re.compile(
    r"(?i)(?:\\berror(?:\\[[a-z0-9]+\\])?:|\\bfatal error\\b|"
    r"^\\s*(?:FAIL:|FAILED\\b|ERROR:|Traceback\\b|Caused by:|"
    r"ModuleNotFoundError:|AssertionError:|FileNotFoundError:))"
)
_ANSI_COLOR = re.compile(r"\\x1b\\[[0-9;]*m")


def first_stage_error(log_path: Path) -> str:
    """Summarize the first actionable failing line from a subprocess log.

    Only called after a stage has failed. Keep memory bounded even when a
    native compiler emits gigabytes of diagnostics; the full log remains
    available alongside report.md.
    """
    last_output = None
    with log_path.open("r", encoding="utf-8", errors="replace") as stream:
        for raw_line in stream:
            line = _ANSI_COLOR.sub("", raw_line).strip()
            if not line or line.startswith("COMMAND:"):
                continue
            last_output = line[:400]
            if _DIAGNOSTIC_LINE.search(line):
                return line[:400]
    return last_output or "nonzero exit without diagnostic output"


def run_step(root: Path, output: Path, step: Step) -> StepResult:
    log_path = output / (step.name + ".log")
    started = time.monotonic()
    code: int | None = None
    problem: str | None = None
    with log_path.open("w", encoding="utf-8", errors="replace") as logfile:
        logfile.write("COMMAND: " + json.dumps(step.argv, ensure_ascii=False) + "\n\n")
        logfile.flush()
        try:
            proc = subprocess.run(step.argv, cwd=root, stdin=subprocess.DEVNULL,
                                  stdout=logfile, stderr=subprocess.STDOUT,
                                  env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
                                  timeout=step.timeout_seconds, check=False)
            code = proc.returncode
        except subprocess.TimeoutExpired:
            problem = f"timeout after {step.timeout_seconds} seconds"
        except OSError as exc:
            problem = f"could not start: {exc}"
    if code not in (None, 0) and problem is None:
        problem = first_stage_error(log_path)
    return StepResult(step.name, step.argv, "PASS" if code == 0 else "FAIL",
                      code, round(time.monotonic() - started, 3),
                      str(log_path.relative_to(root)), problem)


def verify_native_build_evidence(root: Path, log_path: Path,
                                 build_client: bool) -> dict:
    """A 0 process exit is not proof the expected native client exists.

    Read the final JSON emitted by our pinned native_build.py, validate its
    checkout identity, and prove a nonempty exact-path binary when requested.
    This checks build artifacts only; it NEVER marks game-world E2E as tested.
    """
    raw = log_path.read_text(encoding="utf-8", errors="replace")
    start = raw.rfind("\n{")
    if start < 0:
        raise RuntimeError("native build finished without the final JSON evidence record")
    try:
        record = json.loads(raw[start + 1:])
    except (ValueError, TypeError) as exc:
        raise RuntimeError(f"invalid final native build JSON: {exc}") from exc
    if not isinstance(record, dict):
        raise RuntimeError("native build result must be a JSON object")
    pinned = json.loads((root / "compatibility.json").read_text(encoding="utf-8"))
    if record.get("horizon_revision") != pinned["horizon_revision"]:
        raise RuntimeError("native build returned wrong pinned Horizon revision")
    if record.get("cargo_check") != "PASS":
        raise RuntimeError("native build did not report a successful cargo check")
    expected_stage = (root / "target/native-check/modded").resolve()
    if not isinstance(record.get("modded_horizon"), str) or not record["modded_horizon"]:
        raise RuntimeError("native build did not report the modded checkout path")
    if Path(record["modded_horizon"]).resolve() != expected_stage:
        raise RuntimeError("native build reported an unexpected modded checkout path")
    if not build_client:
        return {"native_build_verified": True}
    expected_binary = (expected_stage / "target/fast" /
                       ("horizon-client.exe" if os.name == "nt" else "horizon-client"))
    reported = record.get("native_client_binary")
    if not isinstance(reported, str) or not reported:
        raise RuntimeError("native build returned no client executable path")
    if Path(reported).resolve() != expected_binary:
        raise RuntimeError("native build reported a client path outside the pinned output")
    if not expected_binary.is_file() or expected_binary.stat().st_size < 1024:
        raise RuntimeError("native build exited 0 but expected executable is missing or too small: "
                           + str(expected_binary))
    hasher = hashlib.sha256()
    with expected_binary.open("rb") as executable:
        for block in iter(lambda: executable.read(1024 * 1024), b""):
            hasher.update(block)
    return {"native_build_verified": True,
            "native_client_path": str(expected_binary),
            "native_client_bytes": expected_binary.stat().st_size,
            "native_client_sha256": hasher.hexdigest()}


def write_report(root: Path, output: Path, data: dict) -> None:
    (output / "report.json").write_text(json.dumps(data, indent=2, ensure_ascii=False)
                                        + "\n", encoding="utf-8")
    lines = ["# Horizon Civilizations local build evidence", "",
             f"- Branch: `{data['branch']}`", f"- Commit: `{data['sha']}`",
             f"- Working tree clean before checks: `{data['clean_start']}`",
             f"- Working tree clean after checks: `{data['clean_end']}`",
             f"- Gate: **{data['gate']}**",
             f"- Native build evidence verified: `{data['native_build_verified']}`",
             f"- Native client: `{data['native_client_path']}`",
             f"- Native client SHA256: `{data['native_client_sha256']}`",
             "- Real Minecraft gameplay E2E: **NOT TESTED**", "",
             "| Stage | Status | Log |", "| --- | --- | --- |"]
    for step in data["steps"]:
        lines.append(f"| {step['name']} | {step['status']} | `{step['log']}` |")
    if data["blocking_reason"]:
        lines += ["", "## First blocker", "", data["blocking_reason"]]
    lines += ["", "Compilation is not a gameplay pass. Do not merge or ",
              "ship before manual disposable-world evidence."]
    (output / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path,
                        default=Path(__file__).resolve().parents[1],
                        help="local repository checkout; never clones or fetches")
    parser.add_argument("--expected-branch", default=DEFAULT_BRANCH)
    parser.add_argument("--expected-sha", default=None,
                        help="optional full exact SHA for a reproducible build")
    parser.add_argument("--native-check", action="store_true",
                        help="test against pinned native Horizon upstream")
    parser.add_argument("--build-client", action="store_true",
                        help="also compile client; substantial time/disk required")
    parser.add_argument("--allow-dirty", action="store_true",
                        help="diagnostic mode only; cannot produce a clean gate PASS")
    parser.add_argument("--dry-run", action="store_true",
                        help="print steps only; perform no source checks or writes")
    args = parser.parse_args(argv)
    root = args.repo.resolve()
    steps = planned_steps(root, args.native_check, args.build_client)
    if args.dry_run:
        print(json.dumps({"repo": str(root), "expected_branch": args.expected_branch,
                          "steps": [asdict(s) for s in steps],
                          "note": "PLAN ONLY: no build or game verification"}, indent=2))
        return 0
    if not root.is_dir():
        parser.error(f"repository not found: {root}")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output = root / "target" / "first-play-gate" / stamp
    output.mkdir(parents=True, exist_ok=False)
    data: dict = {"created_utc": stamp, "branch": "UNKNOWN", "sha": "UNKNOWN",
                  "clean_start": False, "clean_end": False, "rustc": "UNKNOWN", "cargo": "UNKNOWN", "rustfmt": "UNKNOWN",
                  "python": sys.version.split()[0], "gate": "BLOCKED",
                  "gameplay_e2e": "NOT_TESTED", "blocking_reason": None,
                  "native_build_verified": False, "native_client_path": None,
                  "native_client_bytes": None, "native_client_sha256": None, "steps": []}
    try:
        if sys.version_info < (3, 11):
            raise RuntimeError("Python 3.11 or later required")
        if not shutil.which("git"):
            raise RuntimeError("missing prerequisite: git")
        data["branch"] = git_value(root, "symbolic-ref", "--quiet", "--short", "HEAD")
        data["sha"] = git_value(root, "rev-parse", "HEAD")
        repo_root = Path(git_value(root, "rev-parse", "--show-toplevel")).resolve()
        if repo_root != root:
            raise RuntimeError(f"--repo must be repository root: {repo_root}")
        data["clean_start"] = not bool(git_value(root, "status", "--porcelain"))
        for tool in ("rustc", "cargo", "rustfmt"):
            if not shutil.which(tool):
                raise RuntimeError(f"missing prerequisite: {tool}. Install Rust 1.95 and Git")
        data["rustc"] = subprocess.check_output(["rustc", "--version"], text=True,
                                                  timeout=20).strip()
        data["cargo"] = subprocess.check_output(["cargo", "--version"], text=True,
                                               timeout=20).strip()
        data["rustfmt"] = subprocess.check_output(["rustfmt", "--version"], text=True,
                                                 timeout=20).strip()
        if not data["rustc"].startswith("rustc 1.95."):
            raise RuntimeError("expected Rust 1.95.x; got " + data["rustc"])
        if args.expected_branch and data["branch"] != args.expected_branch:
            raise RuntimeError("wrong branch: expected " + args.expected_branch
                               + ", got " + data["branch"])
        if args.expected_sha and data["sha"] != args.expected_sha:
            raise RuntimeError("wrong SHA: expected " + args.expected_sha
                               + ", got " + data["sha"])
        if not data["clean_start"] and not args.allow_dirty:
            raise RuntimeError("dirty source checkout; commit/stash before testing "
                               "or use --allow-dirty for diagnostic-only mode")
        for step in steps:
            print(f">>> {step.name}: {step.argv}", flush=True)
            result = run_step(root, output, step)
            data["steps"].append(asdict(result))
            print(f"    {result.status} ({result.seconds}s): {result.log}", flush=True)
            if result.status != "PASS":
                data["blocking_reason"] = (f"{step.name} failed, exit={result.exit_code}; "
                                           f"see {result.log}: {result.error or 'nonzero return'}")
                break
            if step.name == "pinned-horizon-native-build":
                try:
                    data.update(verify_native_build_evidence(
                        root, root / result.log, args.build_client))
                except (OSError, RuntimeError, ValueError, KeyError) as exc:
                    data["steps"][-1]["status"] = "FAIL"
                    data["steps"][-1]["error"] = str(exc)
                    data["blocking_reason"] = ("native post-build verification failed: "
                                               + str(exc) + "; see " + result.log)
                    break
        current_sha = git_value(root, "rev-parse", "HEAD")
        data["clean_end"] = not bool(git_value(root, "status", "--porcelain"))
        if current_sha != data["sha"]:
            raise RuntimeError("HEAD changed while executing tests; results invalid")
        if data["blocking_reason"] is None and data["clean_start"] and not data["clean_end"]:
            data["blocking_reason"] = "source checkout changed during validation"
        if data["blocking_reason"] is None and not data["clean_start"]:
            data["blocking_reason"] = "diagnostic-only dirty checkout; cannot certify PASS"
        if data["blocking_reason"] is None:
            data["gate"] = ("READY_FOR_GAMEPLAY_E2E" if args.build_client else
                            "SOURCE_AND_NATIVE_CHECK_PASS" if args.native_check else
                            "SOURCE_CHECK_PASS")
    except (OSError, RuntimeError, subprocess.SubprocessError) as exc:
        data["blocking_reason"] = str(exc)
    finally:
        write_report(root, output, data)
        print("Gate:", data["gate"])
        print("Evidence:", output / "report.md")
        if data["blocking_reason"]:
            print("First blocker:", data["blocking_reason"])
    return 0 if data["blocking_reason"] is None else 1


if __name__ == "__main__":
    raise SystemExit(main())
