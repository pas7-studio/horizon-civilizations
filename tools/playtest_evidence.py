#!/usr/bin/env python3
"""Read-only, privacy-minimal diagnostics for a disposable Civilizations world.

This validates mod save metadata, NOT the actual Minecraft chunks or whether
entities are visible. It never touches game data and never uploads anything.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

UUID_RE = re.compile(r"^[0-9a-fA-F]{32}$")


def inspect_save(world_dir: Path, *, settlement: str | None = None,
                 min_linked: int = 0, log_file: Path | None = None) -> dict:
    mod_dir = world_dir / "horizon-civilizations"
    source = mod_dir / "world.json"
    failures: list[str] = []
    warnings: list[str] = []
    for remnant in ("world.json.swap", "world.json.tmp"):
        if (mod_dir / remnant).exists():
            failures.append(
                f"stale {remnant} exists: stop and manually inspect recovery files"
            )
    if not source.is_file():
        failures.append("world.json missing: the game may not have saved the mod world")
        return {"evidence_kind": "saved-state-only", "status": "FAIL",
                "failures": failures, "warnings": warnings, "settlements": []}

    payload = source.read_bytes()
    digest = hashlib.sha256(payload).hexdigest()
    try:
        world = json.loads(payload)
    except (ValueError, UnicodeError) as exc:
        return {"evidence_kind": "saved-state-only", "status": "FAIL",
                "sha256": digest, "failures": failures + [f"invalid JSON: {type(exc).__name__}"],
                "warnings": warnings, "settlements": []}
    if not isinstance(world, dict):
        failures.append("world.json root must be an object")
        world = {}
    if world.get("schema") != 1:
        failures.append("unexpected world schema")
    settlements = world.get("settlements")
    if not isinstance(settlements, dict):
        failures.append("settlements must be an object")
        settlements = {}
    if len(settlements) > 64:
        failures.append("settlement limit exceeded")

    seen_uuids: set[str] = set()
    reports: list[dict] = []
    matched = False
    for key, village in sorted(settlements.items(), key=lambda item: item[0]):
        if not isinstance(village, dict):
            failures.append(f"settlement key {key!r} is not an object")
            continue
        name = village.get("name")
        if not isinstance(name, str):
            failures.append(f"settlement key {key!r} has invalid name")
            continue
        residents = village.get("residents")
        if not isinstance(residents, list):
            failures.append(f"settlement {name!r} has invalid resident list")
            residents = []
        actor_ids: set[str] = set()
        linked = 0
        for resident in residents:
            if not isinstance(resident, dict):
                failures.append(f"settlement {name!r} has invalid resident")
                continue
            actor_id = str(resident.get("id"))
            if actor_id in actor_ids:
                failures.append(f"settlement {name!r} has duplicate resident ID")
            actor_ids.add(actor_id)
            uuid = resident.get("entity_uuid")
            if uuid is None:
                continue
            if not isinstance(uuid, str) or not UUID_RE.fullmatch(uuid) or int(uuid, 16) == 0:
                failures.append(f"settlement {name!r} has invalid entity UUID")
                continue
            uuid = uuid.lower()
            if uuid in seen_uuids:
                failures.append("duplicate entity UUID across settlements")
            seen_uuids.add(uuid)
            linked += 1
        if linked and village.get("actors_spawned") is not True:
            failures.append(f"settlement {name!r} contains linked residents but actors_spawned is false")
        farm = village.get("farm")
        farm_report = None
        if farm is not None:
            if not isinstance(farm, dict):
                failures.append(f"settlement {name!r} has invalid farm")
            else:
                receipts = farm.get("harvest_receipts", [])
                if not isinstance(receipts, list):
                    failures.append(f"settlement {name!r} has invalid harvest receipts")
                    receipts = []
                seq = farm.get("harvest_sequence", 0)
                if len(receipts) > 32:
                    failures.append(f"settlement {name!r} has too many harvest receipts")
                if receipts and (not isinstance(receipts[-1], dict) or receipts[-1].get("sequence") != seq):
                    failures.append(f"settlement {name!r} harvest sequence does not match receipt tail")
                farm_report = {"stage": farm.get("stage"), "cursor": farm.get("cursor"),
                               "blocked": farm.get("blocked", False),
                               "completed_cycles": farm.get("completed_cycles", 0),
                               "harvest_sequence": seq, "receipt_count": len(receipts)}
        construction = village.get("visual_job")
        construction_report = None
        if construction is not None:
            if not isinstance(construction, dict):
                failures.append(f"settlement {name!r} has invalid visual job")
            else:
                construction_report = {"cursor": construction.get("cursor"),
                                       "blocked": construction.get("blocked", False)}
        report = {"name": name, "resident_count": len(residents),
                  "uuid_linked": linked, "actors_spawned": village.get("actors_spawned") is True,
                  "completed_physical_huts": len(village.get("visual_sites", []))
                  if isinstance(village.get("visual_sites"), list) else None,
                  "pending_huts": village.get("pending_visual_buildings"),
                  "construction": construction_report, "farm": farm_report}
        reports.append(report)
        if settlement is not None and name.casefold() == settlement.casefold():
            matched = True
            if linked < min_linked:
                failures.append(f"{name!r}: {linked} saved UUID links, expected at least {min_linked}")
    if settlement is not None and not matched:
        failures.append(f"requested settlement {settlement!r} not present in save")

    log_summary = None
    if log_file is not None:
        if not log_file.is_file():
            warnings.append("selected log file missing")
        else:
            # Aggregate only; never copy player names, account data or raw logs.
            count = errors = 0
            with log_file.open(encoding="utf-8", errors="replace") as stream:
                for line in stream:
                    if "[civilizations]" not in line.lower():
                        continue
                    count += 1
                    if "error" in line.lower() or "warn" in line.lower() or "failed" in line.lower():
                        errors += 1
            log_summary = {"civilizations_lines": count, "possible_error_lines": errors}
            if errors:
                warnings.append("mod log contains warning/error-like entries: inspect locally")

    if not reports:
        warnings.append("no settlements saved yet")
    return {"evidence_kind": "saved-state-only", "status": "FAIL" if failures else "PASS_WITH_WARNINGS" if warnings else "PASS",
            "sha256": digest, "schema": world.get("schema"), "world_day": world.get("day"),
            "host_day": world.get("last_host_day"), "settlements": reports,
            "log_counts": log_summary, "failures": failures, "warnings": warnings,
            "gameplay_verified": False,
            "limitations": "Does not verify loaded NPC entities, world blocks, Minecraft chunks, player inventory, or crash atomicity."}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world-dir", type=Path, required=True, help="Path to the disposable Minecraft save folder")
    parser.add_argument("--settlement", default="Ashford", help="Settlement expected in the save; empty disables check")
    parser.add_argument("--min-linked", type=int, default=10, help="Minimum saved UUID bindings for that settlement")
    parser.add_argument("--log", type=Path, help="Optional local latest.log; only line counts are reported")
    parser.add_argument("--output", type=Path, help="Optional JSON report output, outside game data")
    args = parser.parse_args(argv)
    if args.min_linked < 0:
        parser.error("--min-linked cannot be negative")
    report = inspect_save(args.world_dir, settlement=args.settlement or None,
                          min_linked=args.min_linked, log_file=args.log)
    serialized = json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    print(serialized, end="")
    if args.output:
        if args.output.resolve().is_relative_to(args.world_dir.resolve()):
            parser.error("--output must be outside the Minecraft save folder")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized, encoding="utf-8")
    return 1 if report["status"] == "FAIL" else 0


if __name__ == "__main__":
    sys.exit(main())
