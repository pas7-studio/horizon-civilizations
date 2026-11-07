#!/usr/bin/env python3
"""Run the exact local Horizon Civilizations release-candidate source gate.

No GitHub Actions, server deployment, credentials, Minecraft assets or
interactive gameplay. Produces local evidence in target/first-play-gate/.
A passing native client compilation is READY_FOR_GAMEPLAY_E2E, not PLAYABLE.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_BRANCH = "chore/first-play-native-validation-v0115"


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
    steps = [Step("cargo-fmt", ["cargo", "fmt", "--all", "--check"])]
    for file in sorted((root / "mixins/civilizations").glob("*.rs")):
        steps.append(Step("rustfmt-" + file.stem,
                          ["rustfmt", "--edition", "2024", "--check", str(file)]))
    steps += [
        Step("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], 1800),
        Step("cargo-test", ["cargo", "test"], 1800),
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
    return StepResult(step.name, step.argv, "PASS" if code == 0 else "FAIL",
                      code, round(time.monotonic() - started, 3),
                      str(log_path.relative_to(root)), problem)


def write_report(root: Path, output: Path, data: dict) -> None:
    (output / "report.json").write_text(json.dumps(data, indent=2, ensure_ascii=False)
                                        + "\n", encoding="utf-8")
    lines = ["# Horizon Civilizations local build evidence", "",
             f"- Branch: `{data['branch']}`", f"- Commit: `{data['sha']}`",
             f"- Working tree clean before checks: `{data['clean_start']}`",
             f"- Working tree clean after checks: `{data['clean_end']}`",
             f"- Gate: **{data['gate']}**",
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
                  "gameplay_e2e": "NOT_TESTED", "blocking_reason": None, "steps": []}
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
