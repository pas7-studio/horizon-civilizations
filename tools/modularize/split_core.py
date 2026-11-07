"""One-time migration: split mixins/civilizations/core.rs into core/ modules.

Kept outside the repository on purpose; the result is plain Rust source.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

BUNDLE = Path(sys.argv[1])
ROOT = BUNDLE / "core"
SOURCE = BUNDLE / "core.rs"


STR_RE = re.compile(r'"(?:\\.|[^"\\])*"')
CHAR_RE = re.compile(r"'(?:\\.|[^'\\])'")
RAW_RE = re.compile(r'r#*"(?:\\.|[^"\\])*"#*')


def blank_literals(line: str) -> str:
    """Drop comments and string/char literals so brace counting stays honest."""
    line = RAW_RE.sub('""', line)
    line = STR_RE.sub('""', line)
    line = CHAR_RE.sub("''", line)
    cut = line.find("//")
    if cut >= 0:
        line = line[:cut]
    return line


def depth_delta(s: str) -> int:
    line = blank_literals(s)
    return line.count("{") - line.count("}")


def parse_items(lines: list[str]) -> list[tuple[int, int, str]]:
    out = []
    depth = 0
    start = None
    header = ""
    pending: list[int] = []
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if depth == 0 and start is None:
            if stripped.startswith("//") or stripped.startswith("#["):
                pending.append(idx)
                continue
            if not stripped:
                continue
            start = pending[0] if pending else idx
            pending = []
            header = stripped
        if start is not None:
            depth += depth_delta(line)
            if depth == 0 and stripped.endswith(("}", ";")):
                out.append((start, idx, header))
                start = None
    if start is not None:
        raise SystemExit(f"unterminated item at line {start + 1}: {header}")
    return out


TYPE_RE = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(struct|enum|fn|const|type|mod|trait)\s+([A-Za-z_][A-Za-z0-9_]*)")
IMPL_RE = re.compile(
    r"^impl(?:<[^>]*>)?\s+(?:[A-Za-z_][A-Za-z0-9_:<>]*\s+for\s+)?([A-Za-z_][A-Za-z0-9_]*)")
IMPL_OF_RE = re.compile(r"^impl(?:<[^>]*>)?\s+.*?\bfor\s+([A-Za-z_][A-Za-z0-9_]*)")

TARGETS = {}
for name, module in {
    "Good": "goods", "Job": "jobs", "PostKind": "jobs", "WorkPost": "jobs",
    "native_worker_near": "jobs", "native_worker_reaches_block": "jobs",
    "DailyPhase": "jobs", "Rank": "jobs",
    "Resident": "people", "entity_uuid_hex": "people", "Player": "people",
    "HomeSlot": "people", "inspect_home": "people",
    "PlayerParcel": "housing", "near_plot": "housing", "HutBlock": "housing",
    "hut_plan": "housing", "VisualBuildJob": "housing", "BuildInspection": "housing",
    "is_ground": "housing", "inspect_hut": "housing",
    "town_hall_plan": "buildings", "bakery_plan": "buildings",
    "hall_storage_positions": "buildings", "inspect_town_hall": "buildings",
    "inspect_bakery": "buildings",
    "GatherJob": "gather", "OakStem": "gather", "short_oak_stem_height": "gather",
    "inspect_oak_stem": "gather", "is_rooted_oak_harvest_origin": "gather",
    "BarrelDirection": "barrel", "BarrelPhase": "barrel", "BarrelTransfer": "barrel",
    "MarketPhase": "barrel", "MarketPurchase": "barrel", "BakeryWorkshop": "bakery",
    "SignColor": "signs", "SignLine": "signs", "SignSpec": "signs", "sign_fit": "signs",
    "sl": "signs", "sign_head": "signs", "sign_blank": "signs", "sign_bar": "signs",
    "HallBoardInput": "signs", "render_hall_boards": "signs",
    "render_build_progress": "signs", "site_board_position": "signs",
    "render_hut_ready": "signs", "render_bakery_ready": "signs",
    "hall_board_positions": "signs", "legacy_hall_board_positions": "signs",
    "hall_all_board_positions": "signs",
    "FarmStage": "farm", "HarvestReceipt": "farm", "FarmPlot": "farm", "FarmStep": "farm",
    "farm_plan": "farm", "FarmBlock": "farm", "FarmDecision": "farm", "inspect_farm": "farm",
    "CivicNeed": "civic", "CivicPriority": "civic", "AbstractDayForecast": "civic",
    "village_chunk_footprint": "chunks", "gather_turn_index": "gather",
    "MenuEntry": "dialogue", "resident_wants": "dialogue", "job_trades": "dialogue",
    "sale_for_held": "dialogue", "job_icon": "dialogue", "menu_text": "dialogue",
    "DialogueInput": "dialogue", "dialogue_menu": "dialogue",
    "TalkPage": "dialogue", "build_dialog": "dialogue", "Dialog": "dialogue",
    "dialog_text": "dialogue", "dialog_snbt": "dialogue", "greeting": "dialogue",
    "command_for": "dialogue", "back_button": "dialogue", "DlgButton": "dialogue",
    "DlgSlider": "dialogue", "snbt_string": "dialogue", "snbt_text": "dialogue",
    "parse_sale_args": "dialogue", "good_label": "goods", "Dialog": "dialogue",
    "DialogLine": "dialogue", "DialogChoice": "dialogue",
    "TRADE_INPUT": "dialogue", "MenuPage": "dialogue", "MenuAction": "dialogue",
    "OfferedStack": "dialogue", "settle_offer": "dialogue", "page_title": "dialogue",
    "page_entries": "dialogue", "filler": "dialogue", "entry": "dialogue",
    "Settlement": "settlement", "physical_construction_shortfall": "settlement",
    "physical_bread_target": "settlement", "needs_real_bread": "settlement",
    "autonomous_by_default": "settlement",
}.items():
    TARGETS[name] = module

IMPL_TARGETS = {
    "Good": "goods", "Job": "jobs", "PostKind": "jobs", "DailyPhase": "jobs",
    "Rank": "jobs", "Player": "people", "HomeSlot": "people", "GatherJob": "gather",
    "BarrelTransfer": "barrel", "SignColor": "signs", "SignSpec": "signs",
    "CivicNeed": "civic", "Settlement": "settlement", "TalkPage": "dialogue", "Dialog": "dialogue",
}

WORLD_FILES = {
    "state": [
        "validate", "event", "observe_host_day", "advance", "bind_actor", "by_name",
        "join", "promote", "reserve_actor_recovery",
    ],
    "founding": ["found"],
    "housing": [
        "check_visual_house_site", "check_visual_house_site_for", "proposed_hut",
        "prepare_village_housing", "set_autonomy", "buy_plot", "buy_finished_house",
        "commission_house", "start_visual_house", "reserve_hut_supplies",
        "next_visual_job", "finish_visual_house",
    ],
    "hall": ["reserve_town_hall", "finish_town_hall"],
    "bakery": [
        "deliver_farm_wheat", "start_bakery", "reserve_bakery_supplies", "finish_bakery",
        "craft_bread", "deliver_bakery_bread",
    ],
    "barrel": [
        "plan_barrel_transfer", "issue_barrel_transfer", "confirm_barrel_transfer",
        "block_barrel_transfer", "cancel_planned_barrel_transfer",
        "reconcile_blocked_barrel_transfer", "begin_market_purchase",
        "complete_market_purchase", "block_market_purchase",
    ],
    "gather": [
        "gather_site_unclaimed", "plan_gather", "plan_short_oak_stem", "assign_gather_worker",
        "mark_gather_issued", "mark_gather_native_edit_confirmed", "witness_gather_pickup", "block_gather",
        "discard_blocked_gather", "abandon_blocked_issued_gather", "confirm_gather",
    ],
    "posts": ["add_post", "post_origin", "set_post_worker"],
    "farm": ["start_farm", "recover_farm", "advance_farm", "next_farm", "advance_quarry"],
    "trade": ["sell", "buy"],
    "protection": ["is_civic_sign", "player_may_break"],
    "diplomacy": ["relation", "declare_war", "make_peace"],
}


def item_name(header: str) -> str:
    if header.startswith("impl"):
        m = IMPL_OF_RE.match(header)
        if m:
            return m.group(1)
        return IMPL_RE.match(header).group(1)
    m = TYPE_RE.match(header)
    if m:
        return m.group(2)
    raise SystemExit(f"unknown item header: {header}")


def btree_import(code: str) -> str:
    names = []
    for name in ("BTreeMap", "BTreeSet"):
        if re.search(rf"use [^;]*\b{name}\b", code):
            continue
        if any(re.search(rf"(?<!::)\b{name}\b", line) and not line.lstrip().startswith("use ")
               for line in code.splitlines()):
            names.append(name)
    return f"use std::collections::{{{', '.join(names)}}};\n" if names else ""


def imports_for(text: str) -> str:
    lines = []
    if re.search(r"\bSerialize\b|\bDeserialize\b", text):
        lines.append("use serde::{Deserialize, Serialize};")
    btree = btree_import(text)
    if btree:
        lines.append(btree.rstrip("\n"))
    return "".join(x + "\n" for x in lines)


def main() -> None:
    lines = SOURCE.read_text(encoding="utf-8").splitlines(keepends=True)
    items = parse_items(lines)
    buckets: dict[str, list[str]] = {}
    world_members: list[tuple[int, int, str]] = []
    world_offset = None
    tests: dict[str, str] = {}

    for start, end, header in items:
        text = "".join(lines[start:end + 1])
        if header.startswith("use "):
            continue
        if header.startswith("mod ") and header.endswith("{"):
            name = TYPE_RE.match(header).group(2)
            if name == "tests" or name.endswith("_tests"):
                first = next(i for i in range(start, end + 1)
                             if lines[i].lstrip().startswith("mod "))
                if name == "tests":
                    name = "domain_tests"
                tests[name] = "".join(lines[first + 1:end])
                continue
            module = TARGETS[name]
            buckets.setdefault(module, []).append(text)
            continue
        name = item_name(header)
        if name == "World" or header.startswith("impl World"):
            if header.startswith("pub struct World") or header.startswith("impl Default for World"):
                continue
            if world_offset is not None:
                raise SystemExit("more than one impl World block")
            world_members.extend(parse_items(lines[start + 1:end]))
            world_offset = start + 1
            continue
        if header.startswith("impl Default for World"):
            continue
        if header.startswith("impl"):
            module = IMPL_TARGETS.get(name)
            if module is None:
                raise SystemExit(f"impl without target: {header}")
        elif TYPE_RE.match(header).group(1) == "const":
            module = TARGETS.get(name, "constants")
        else:
            module = TARGETS.get(name)
        if module is None:
            raise SystemExit(f"item without target: {header}")
        buckets.setdefault(module, []).append(text)

    assert world_offset is not None
    member_text: dict[str, list[str]] = {}
    for start, end, header in world_members:
        name = item_name(header)
        for module, names in WORLD_FILES.items():
            if name in names:
                break
        else:
            raise SystemExit(f"unmapped World method: {header}")
        member_text.setdefault(module, []).append(
            "".join(lines[world_offset + start: world_offset + end + 1]))

    ROOT.mkdir(parents=True, exist_ok=True)
    (ROOT / "tests").mkdir(parents=True, exist_ok=True)
    (ROOT / "world").mkdir(parents=True, exist_ok=True)

    for module, chunks in sorted(buckets.items()):
        body = "\n".join(chunks)
        text = ("// Horizon Civilizations domain module, re-exported by core/mod.rs.\n"
                "use super::*;\n" + imports_for(body) + "\n" + body)
        (ROOT / f"{module}.rs").write_text(text, encoding="utf-8")

    for module, chunks in sorted(member_text.items()):
        body = "impl World {\n" + "\n".join(chunks) + "}\n"
        text = ("// Inherent impl block of World, split by domain (same crate/module root).\n"
                "use super::super::*;\n" + imports_for(body) + "\n" + body)
        (ROOT / "world" / f"{module}.rs").write_text(text, encoding="utf-8")

    for name, body in sorted(tests.items()):
        body = body.replace("use super::*;", "use super::super::*;", 1)
        extra = ""
        code = "\n".join(blank_literals(line) for line in body.splitlines())
        extra = btree_import(code)
        (ROOT / "tests" / f"{name}.rs").write_text(extra + body, encoding="utf-8")

    modules = sorted(buckets)
    root = [
        "// Civilization domain model. Deliberately independent from Minecraft/Horizon APIs.\n",
        "// Authored as a normal Rust module tree; tools/mixin_sources.py flattens it into\n",
        "// the single self-contained payload the Horizon mixin tool requires.\n",
        "\n",
    ]
    root += [f"mod {m};\n" for m in modules]
    root += ["mod world;\n", "\n#[cfg(test)]\nmod tests;\n\n"]
    root += [f"pub use {m}::*;\n" for m in modules]
    root += ["pub use world::*;\n"]
    (ROOT / "mod.rs").write_text("".join(root), encoding="utf-8")

    struct_text = "".join("".join(lines[s:e + 1]) for s, e, h in items if h.startswith("pub struct World {"))
    default_text = "".join("".join(lines[s:e + 1]) for s, e, h in items if h.startswith("impl Default for World"))
    world = [
        "// World state owner plus one inherent impl file per domain.\n",
        "use super::*;\n",
        "use serde::{Deserialize, Serialize};\n",
        btree_import(struct_text + default_text),
        "\n",
        struct_text,
        "\n",
        default_text,
        "\n\n",
    ]
    world += [f"mod {m};\n" for m in sorted(member_text)]
    (ROOT / "world" / "mod.rs").write_text("".join(world), encoding="utf-8")

    tests_mod = [
        "// Unit tests moved out of the crate root: `use super::super::*` still reaches\n",
        "// every production item through core/mod.rs.\n",
    ]
    tests_mod += [f"mod {n};\n" for n in sorted(tests)]
    (ROOT / "tests.rs").write_text("".join(tests_mod), encoding="utf-8")

    # Cross-module private helpers become crate-visible inside the module tree.
    swaps = [
        (ROOT / "settlement.rs",
         "    fn take(&mut self, good: Good, amount: u32) -> bool {",
         "    pub(crate) fn take(&mut self, good: Good, amount: u32) -> bool {"),
        (ROOT / "settlement.rs",
         "    fn day(&mut self) -> Vec<String> {",
         "    pub(crate) fn day(&mut self) -> Vec<String> {"),
        (ROOT / "world" / "state.rs",
         "    fn event(&mut self, message: String) {",
         "    pub(crate) fn event(&mut self, message: String) {"),
    ]
    for path, old, new in swaps:
        text = path.read_text(encoding="utf-8")
        if old not in text:
            raise SystemExit(f"fixup not found in {path}: {old}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    # Modules that never reach outside themselves keep no glob import.
    for module in ("civic", "constants", "goods", "housing"):
        path = ROOT / f"{module}.rs"
        path.write_text(path.read_text(encoding="utf-8").replace("use super::*;\n", "", 1),
                        encoding="utf-8")

    SOURCE.unlink()
    print(f"modules: {modules}")
    print(f"world impl files: {sorted(member_text)}")
    print(f"test modules: {len(tests)}")


main()
