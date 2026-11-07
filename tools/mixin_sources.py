#!/usr/bin/env python3
"""Flatten the Rust module tree into one self-contained payload per mixin.

The Horizon mixin tool rejects external `mod name;` declarations: a support
module payload must be self-contained. This repository authors the domain code
as a normal Rust module tree (`core/mod.rs` plus `core/*.rs`), so the native
build needs one deterministic step that inlines every `mod name;` declaration
before the planner ever sees the payload.

`#[cfg(test)]` modules are kept in the staged payload: the native gate runs
`cargo test -p horizon-client civilizations_core:: civilizations_glue::`, and a
payload that dropped them would silently match zero tests. Python
source-contract tests read the same flattened text, so a contract keeps its
meaning after a module moves.

Nothing here compiles or validates Rust; `tools/native_build.py` and
`cargo test` remain the only real gates.
"""
from __future__ import annotations

import re
import shutil
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "mixins" / "civilizations"
MANIFEST = "horizon-mixin.toml"

MOD_DECL = re.compile(
    r"^(?P<indent>[ \t]*)(?P<vis>pub(?:\([^)]*\))?[ \t]+)?mod[ \t]+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)[ \t]*;[ \t]*$")
CFG_TEST = re.compile(r"^[ \t]*#\[cfg\(test\)\][ \t]*$")


def children_dir(path: Path) -> Path:
    """Directory rustc searches for the submodules of `path`."""
    if path.name in ("mod.rs", "lib.rs", "main.rs"):
        return path.parent
    return path.parent / path.stem


def flatten_text(path: Path, *, include_tests: bool = False,
                 _stack: tuple[Path, ...] = ()) -> str:
    path = path.resolve()
    if path in _stack:
        raise ValueError(f"module cycle: {' -> '.join(str(p) for p in _stack + (path,))}")
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    directory = children_dir(path)
    out: list[str] = []
    for line in lines:
        match = MOD_DECL.match(line.rstrip("\r\n"))
        if not match:
            out.append(line)
            continue
        previous = out[-1].strip() if out else ""
        is_test = bool(CFG_TEST.match(previous))
        if is_test and not include_tests:
            out.pop()
            continue
        name = match.group("name")
        child = directory / f"{name}.rs"
        if not child.is_file():
            child = directory / name / "mod.rs"
        if not child.is_file():
            raise ValueError(f"{path}: cannot resolve `mod {name};` under {directory}")
        body = flatten_text(child, include_tests=include_tests, _stack=_stack + (path,))
        indent = match.group("indent")
        vis = match.group("vis") or ""
        out.append(f"{indent}{vis}mod {name} {{\n")
        out.append(body)
        if not body.endswith("\n"):
            out.append("\n")
        out.append(f"{indent}}}\n")
    text = "".join(out)
    leftover = [line for line in text.splitlines() if MOD_DECL.match(line.rstrip("\r\n"))]
    if leftover:
        raise ValueError(f"{path}: unresolved module declarations: {leftover}")
    return text


def payload_text(bundle: Path = BUNDLE, module: str = "core", *,
                 include_tests: bool = False) -> str:
    """Flattened source of the module `module` declared by the bundle."""
    manifest = tomllib.loads((bundle / MANIFEST).read_text(encoding="utf-8"))
    for entry in manifest["module"]:
        if entry["id"] == module or entry["name"] == f"civilizations_{module}":
            return flatten_text(bundle / entry["source"], include_tests=include_tests)
    raise KeyError(f"{bundle}: no module {module!r}")


def stage(bundle: Path = BUNDLE, out: Path | None = None) -> Path:
    """Write a buildable copy of the bundle with flattened module payloads.

    Returns the directory that contains `civilizations/`, i.e. what the mixin
    planner takes as `--mods`.
    """
    manifest = tomllib.loads((bundle / MANIFEST).read_text(encoding="utf-8"))
    if out is None:
        out = Path(tempfile.mkdtemp(prefix="civ-bundle-"))
    target = out / bundle.name
    if target.exists():
        shutil.rmtree(target)
    target.mkdir(parents=True)
    (target / MANIFEST).write_bytes((bundle / MANIFEST).read_bytes())
    for injection in manifest["injection"]:
        helper = bundle / injection["helper"]
        (target / injection["helper"]).write_bytes(helper.read_bytes())
    for module in manifest["module"]:
        source = bundle / module["source"]
        destination = target / module["source"]
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(flatten_text(source, include_tests=True),
                               encoding="utf-8", newline="\n")
    return out


if __name__ == "__main__":
    for entry in ("core", "glue"):
        print(f"{entry}: {len(payload_text(module=entry))} bytes flattened")
