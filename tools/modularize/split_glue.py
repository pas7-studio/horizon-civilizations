"""One-time migration: split mixins/civilizations/glue.rs into glue/ modules."""
from __future__ import annotations

import re
import sys
from pathlib import Path

BUNDLE = Path(sys.argv[1])
ROOT = BUNDLE / "glue"
SOURCE = BUNDLE / "glue.rs"

STR_RE = re.compile(r'"(?:\\.|[^"\\])*"')
CHAR_RE = re.compile(r"'(?:\\.|[^'\\])'")
RAW_RE = re.compile(r'r#*"(?:\\.|[^"\\])*"#*')


def blank_literals(line: str) -> str:
    line = RAW_RE.sub('""', line)
    line = STR_RE.sub('""', line)
    line = CHAR_RE.sub("''", line)
    cut = line.find("//")
    return line[:cut] if cut >= 0 else line


def depth_delta(s: str) -> int:
    line = blank_literals(s)
    return line.count("{") - line.count("}")


ITEM_START = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?:fn|struct|enum|const|static|type|mod|trait|use)\b")


def parse_items(lines: list[str]) -> list[tuple[int, int, str]]:
    """Top-level items of a rustfmt-ed file: a column-0 `}` ends each item."""
    out: list[tuple[int, int, str]] = []
    pending: list[int] = []
    index = 0
    while index < len(lines):
        stripped = lines[index].rstrip("\n")
        if not ITEM_START.match(stripped):
            if stripped.startswith("//") or stripped.startswith("#["):
                pending.append(index)
            elif stripped.strip():
                raise SystemExit(f"unexpected top-level line {index + 1}: {stripped[:70]}")
            else:
                pending = []
            index += 1
            continue
        start = pending[0] if pending else index
        pending = []
        end = index
        if stripped.startswith("use "):
            while not lines[end].rstrip().endswith(";"):
                end += 1
                if end >= len(lines):
                    raise SystemExit(f"unterminated use at line {index + 1}")
        elif stripped.endswith(";") and "{" not in stripped:
            pass
        else:
            while lines[end].rstrip() != "}":
                end += 1
                if end >= len(lines):
                    raise SystemExit(f"unterminated item at line {index + 1}: {stripped[:70]}")
        out.append((start, end, stripped))
        index = end + 1
    return out


NAME_RE = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?:fn|struct|enum|const|static|type|mod|trait)\s+"
    r"([A-Za-z_][A-Za-z0-9_]*)")

FILES = {
    "store": ["ActiveWorld", "ACTIVE", "load", "active", "save", "save_autonomous", "closed"],
    "commands": ["register", "perform", "reply", "error", "parse_amount", "own_settlement"],
    "shop": ["quarantined_native_purchase", "buy_from_real_barrel"],
    "actors": ["loaded_actors", "loaded_home_target", "guide_actors", "safe_actor_spawn_points",
               "auto_spawn_villagers", "rename_actor", "surveyed_initial_actor_positions"],
    "build": ["world_block", "check_empty_hut_site", "is_clearable_growth", "clear_growth_only",
              "open_build_plot", "quarry_start", "quarry_one_step", "build_town_hall_one_step",
              "build_bakery_one_step", "build_one_step", "world_farm_block",
              "check_empty_farm_site", "farm_one_step"],
    "protect": ["protect_public_hopper", "protect_civilization_block", "protect_hall_inventory",
                "civic_sign_at", "protect_civic_sign_click", "protect_civic_sign_edit"],
    "dialogue": ["send_dialogue_chat", "DIALOG_QUEUE", "dialog_queue", "process_dialog_queue", "read_talk",
                 "show_talk", "Talk", "talk_command", "sell_to_resident"],
    "chunks": ["settlement_work_sites", "hold_village_chunks"],
    "menu": ["MenuSession", "PendingOpen", "MENU_SESSIONS", "MENU_QUEUE", "MENU_TELLS",
             "menu_sessions", "menu_queue", "menu_tells", "RenderedPage", "render_page",
             "menu_stack", "page_items", "open_page", "process_menu_queue", "menu_clicked",
             "menu_input_closed", "menu_closed", "sell_offered"],
    "interact": ["interact_entity", "join_village"],
    "logistics": ["plain_barrel_count", "physical_barrel_stock", "storage_worker_near",
                  "request_workshop_pickup", "barrel_one_step", "farmer_delivery_one_step",
                  "baker_one_step"],
    "economy": ["autonomous_village_step"],
    "boards": ["BOARD_CACHE", "sign_data_command", "sign_data_command_for", "refresh_site_boards",
               "refresh_hall_boards", "staff_village", "refresh_post_boards"],
    "gather": ["gather_stand", "gather_stand_still_clear", "seems_natural_resource",
               "gather_one_step"],
    "hooks": ["level_tick", "tick"],
}

REEXPORT = {
    "commands": ["perform", "register"],
    "store": ["closed"],
    "hooks": ["level_tick", "tick"],
    "interact": ["interact_entity"],
    "protect": ["protect_civilization_block", "protect_hall_inventory", "protect_public_hopper",
                "protect_civic_sign_click", "protect_civic_sign_edit"],
    "menu": ["menu_clicked", "menu_closed", "menu_input_closed"],
}

ROOT_ITEMS = {"MOD_ID", "DIR", "SAVE"}


def main() -> None:
    lines = SOURCE.read_text(encoding="utf-8").splitlines(keepends=True)
    items = parse_items(lines)
    header_comments: list[str] = []
    imports: list[str] = []
    buckets: dict[str, list[str]] = {}
    glue_tests: dict[str, str] = {}
    seen: set[str] = set()

    for start, end, header in items:
        text = "".join(lines[start:end + 1])
        if not buckets and not imports:
            text = __import__("re").sub(r"(?:[ \t]*//[^\n]*\n)+", "", text, count=1)
        if header.startswith("use "):
            imports.append(text)
            continue
        match = NAME_RE.match(header)
        if not match:
            raise SystemExit(f"unmapped item: {header}")
        name = match.group(1)
        if header.startswith("mod ") and header.endswith("{"):
            start_line = next(i for i in range(start, end + 1)
                              if lines[i].lstrip().startswith("mod "))
            glue_tests[name] = "".join(lines[start_line + 1:end])
            continue
        if name in ROOT_ITEMS:
            buckets.setdefault("", []).append(text)
            continue
        for module, names in FILES.items():
            if name in names:
                break
        else:
            raise SystemExit(f"unmapped item: {header}")
        buckets.setdefault(module, []).append(text)
        seen.add(name)

    buckets = {m: chunks for m, chunks in buckets.items() if chunks}
    missing = {n for names in FILES.values() for n in names} - seen
    if missing:
        # Stale entries are normal when the script is replayed on a newer base.
        print("note: mapped items absent on this base:", ", ".join(sorted(missing)))

    ROOT.mkdir(parents=True, exist_ok=True)
    for module, chunks in sorted(buckets.items()):
        if not module:
            continue
        body = "\n".join(chunks)
        text = ("// Horizon adapter: see glue/mod.rs for the module map.\n"
                "use super::*;\n\n" + body)
        (ROOT / f"{module}.rs").write_text(text, encoding="utf-8")

    root = [
        "// Horizon bridge. All world logic lives in civilizations_core; this module\n",
        "// translates server hooks, commands, inventory changes, and native NPC spawns.\n",
        "// Split into focused modules; tools/mixin_sources.py flattens it for the\n",
        "// Horizon mixin tool, which only accepts self-contained payloads.\n",
        "\n",
    ]
    root += imports
    root += ["\n"]
    root += ["".join(buckets.get("", []))]
    root += ["\n"]
    root += [f"mod {m};\n" for m in sorted(buckets) if m]
    root += ["\n"]
    for name, body in sorted(glue_tests.items()):
        (ROOT / f"{name}.rs").write_text(
            "// Horizon adapter unit test: compiled only by the native cargo test gate.\n"
            "use super::*;\n\n" + body, encoding="utf-8")
        root.append(f"#[cfg(test)]\nmod {name};\n")
    root += ["\n"]
    for module, names in sorted(REEXPORT.items()):
        present = [n for n in names if n in seen]
        if module in buckets and present:
            root.append(f"pub use {module}::{{{', '.join(present)}}};\n")
    (ROOT / "mod.rs").write_text("".join(root), encoding="utf-8")

    SOURCE.unlink()
    print("modules:", ", ".join(m for m in sorted(buckets) if m))


main()
