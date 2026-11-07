"""Finish the glue split: shared visibility, one import surface, no dead imports."""
from __future__ import annotations

import re
import sys
from pathlib import Path

GLUE = Path(sys.argv[1])
ROOT = GLUE / "mod.rs"
root_text = ROOT.read_text(encoding="utf-8")
test_modules = set(re.findall(r"#\[cfg\(test\)\]\nmod ([a-z_0-9]+);", root_text))
modules = sorted(p.stem for p in GLUE.glob("*.rs")
                 if p.stem != "mod" and p.stem not in test_modules)

# 1. Children see siblings through glue/mod.rs, not per-file globs.
for path in GLUE.glob("*.rs"):
    if path.stem == "mod":
        continue
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"use super::[a-z_]+::\*;\n", "", text)
    path.write_text(text, encoding="utf-8")

# 2. One private glob per sibling module at the root. `hooks`, `interact` and
#    `protect` are re-exported explicitly, so their globs would be dead.
text = ROOT.read_text(encoding="utf-8")
if "mod actions;" not in text:
    text = text.replace("mod actors;", "mod actions;\nmod actors;", 1)
if "mod reports;" not in text:
    text = text.replace("mod shop;", "mod reports;\nmod shop;", 1)
text = re.sub(r"^use [a-z_]+::\*;\n", "", text, flags=re.MULTILINE)
shared = [m for m in modules if m not in {"hooks", "protect"}]
for late in ("actions", "reports"):
    if late not in shared:
        shared.append(late)
shared = sorted(shared)
anchor = "\npub use "
index = text.index(anchor)
text = text[:index] + "\n" + "".join(f"use {m}::*;\n" for m in shared) + text[index:]
# `hall_board_positions` is only used by core unit tests; importing it here
# would be a dead import in the native build.
text = text.replace("World, inspect_home, hall_board_positions, HOUSING_PLOT_OFFSETS,",
                    "World, inspect_home, HOUSING_PLOT_OFFSETS,")
ROOT.write_text(text, encoding="utf-8")

# 3. Shared helpers are crate-visible inside the adapter module tree.
for path in GLUE.glob("*.rs"):
    if path.stem == "mod":
        continue
    lines = path.read_text(encoding="utf-8").splitlines()
    for i, line in enumerate(lines):
        if re.match(r"^(fn |struct |enum |static |const |type )", line):
            lines[i] = "pub(super) " + line
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")

# 4. Struct fields are shared state too: siblings construct these values.
for path in GLUE.glob("*.rs"):
    if path.stem == "mod":
        continue
    lines = path.read_text(encoding="utf-8").splitlines()
    in_struct = False
    out = []
    for line in lines:
        if re.match(r"^(pub(\([^)]*\))? )?struct [A-Za-z_]", line):
            in_struct = True
        elif in_struct and line == "}":
            in_struct = False
        elif in_struct:
            match = re.match(r"^    ([a-z_][a-z0-9_]*): ", line)
            if match:
                line = "    pub(super) " + line.lstrip()
        out.append(line)
    path.write_text("\n".join(out) + "\n", encoding="utf-8")

# 5. `actor` is a `&str` parameter now, not a `&String`.
reports = GLUE / "reports.rs"
text = reports.read_text(encoding="utf-8")
text = text.replace(".get(&actor)", ".get(actor)")
reports.write_text(text, encoding="utf-8")
print("finalized:", ", ".join(modules))
