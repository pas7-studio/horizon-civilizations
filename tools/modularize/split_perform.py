"""One-time refactor: split the 1100-line `perform` dispatcher by role.

`perform` keeps argument/state setup and the save/rollback tail; the read-only
report blocks move to glue/reports.rs and the mutating match arms move to
glue/actions.rs. No command text, message or condition is rewritten.
"""
from __future__ import annotations

import sys
from pathlib import Path

GLUE = Path(sys.argv[1])
COMMANDS = GLUE / "commands.rs"

text = COMMANDS.read_text(encoding="utf-8")

report_start_marker = '    if action == "help" {\n'
mutate_comment = "    // Mutating actions use a cloned state for ordinary error rollback.\n"
result_marker = "    let result: Result<String, String> = match action {\n"
tail_marker = "    match result {\n"

report_start = text.index(report_start_marker)
mutate_at = text.index(mutate_comment)
result_at = text.index(result_marker, mutate_at)
tail_at = text.index(tail_marker, result_at)

reports_body = text[report_start:mutate_at]
match_body = text[result_at:tail_at]

assert match_body.endswith("    };\n"), repr(match_body[-40:])
match_body_inner = match_body[len(result_marker):-len("    };\n")]

# Every `return;` in the report blocks ends the command, so `report` reports
# "handled" instead of returning from `perform`.
assert "return true" not in reports_body
reports_body = reports_body.replace("return;", "return true;")

# The mutating arms keep `already_saved` as a local flag of `mutate`.
assert "&actor" in match_body_inner
match_body_inner = match_body_inner.replace("&actor", "actor")

reports = (
    "// Horizon adapter: read-only /civ report commands, split out of `perform`.\n"
    "use super::*;\n\n"
    "/// Prints one /civ report. Returns false when no report matched, so\n"
    "/// `perform` can fall through to the mutating commands.\n"
    "pub(super) fn report(\n"
    "    game: &mut Game<'_>,\n"
    "    player: &NativeServerPlayer,\n"
    "    world: &World,\n"
    "    action: &str,\n"
    "    actor: &str,\n"
    "    args: &mut std::str::SplitWhitespace<'_>,\n"
    ") -> bool {\n"
    + reports_body
    + "    false\n"
    "}\n"
)
(GLUE / "reports.rs").write_text(reports, encoding="utf-8")

actions = (
    "// Horizon adapter: mutating /civ commands, split out of `perform`.\n"
    "use super::*;\n\n"
    "/// Applies one mutating /civ command to the live world. The world stays\n"
    "/// untouched on error; `perform` decides whether to save and how to roll back.\n"
    "pub(super) fn mutate(\n"
    "    game: &mut Game<'_>,\n"
    "    player: &NativeServerPlayer,\n"
    "    state: &mut ActiveWorld,\n"
    "    previous: &World,\n"
    "    action: &str,\n"
    "    actor: &str,\n"
    "    args: &mut std::str::SplitWhitespace<'_>,\n"
    ") -> (Result<String, String>, bool) {\n"
    "    let mut already_saved = false;\n"
    "    let result: Result<String, String> = match action {\n"
    + match_body_inner
    + "    };\n"
    "    (result, already_saved)\n"
    "}\n"
)
(GLUE / "actions.rs").write_text(actions, encoding="utf-8")

head = text[:report_start]
new_perform = (
    report_start_marker.replace("if action == \"help\" {", "if report(game, player, world, action, &actor, &mut args) {")
    + "        return;\n"
    "    }\n"
    "    let previous = world.clone();\n"
    "    let (result, already_saved) = mutate(game, player, state, &previous, action, &actor, &mut args);\n"
)
tail = text[tail_at:]
COMMANDS.write_text(head + new_perform + tail, encoding="utf-8")
print("reports:", len(reports.splitlines()), "actions:", len(actions.splitlines()),
      "commands:", len((head + new_perform + tail).splitlines()))
