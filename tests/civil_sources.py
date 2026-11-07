"""Payload text exactly as the native build sees it.

The Rust sources are an ordinary module tree (`mixins/civilizations/core/mod.rs`
plus its submodules). The Horizon mixin tool requires one self-contained file
per support module, so `tools/mixin_sources.py` flattens the tree. Tests read
the flattened text instead of a single `core.rs`, so a source contract keeps
meaning the same thing after a module is moved.

`#[cfg(test)]` modules are included here because these contracts also pin Rust
unit-test expectations; the packaged payload omits them.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
TOOLS = str(ROOT / "tools")
if TOOLS not in sys.path:
    sys.path.insert(0, TOOLS)

import mixin_sources  # noqa: E402

BUNDLE = ROOT / "mixins" / "civilizations"


def core_source() -> str:
    """Domain model payload (`core/mod.rs`, `#[cfg(test)]` modules included)."""
    return mixin_sources.payload_text(BUNDLE, "core", include_tests=True)


def glue_source() -> str:
    """Horizon adapter payload (`glue.rs`)."""
    return mixin_sources.payload_text(BUNDLE, "glue", include_tests=True)


def bundle_source() -> str:
    """Every Rust payload of the bundle, for whole-mod smoke checks."""
    return core_source() + "\n" + glue_source()
