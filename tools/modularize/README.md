# One-time modularization scripts

These scripts produced the `core/` and `glue/` module trees from the former
single `core.rs` / `glue.rs`. They are **not** part of the build, packaging or
test gates: the committed source is plain Rust.

They exist because the split has to be re-applied whenever `core.rs` or
`glue.rs` comes back on a branch based on older history (for example after a
long-lived feature branch). They move items verbatim, so review the diff; they
never rewrite function bodies except:

- `split_perform.py` turns the report `return;` into `return true;` and passes
  `actor` as `&str` instead of `&String`;
- `finalize_glue.py` adds `pub(super)` to shared helpers and struct fields.

## Re-applying on a newer base

```powershell
git switch -c refactor/modular-core-vX main
python tools/modularize/split_core.py mixins/civilizations
python tools/modularize/split_glue.py mixins/civilizations
python tools/modularize/split_perform.py mixins/civilizations/glue
python tools/modularize/finalize_glue.py mixins/civilizations/glue
```

Then:

1. Point `Cargo.toml` `[lib] path` at `mixins/civilizations/core/mod.rs` and the
   two `source =` entries in `horizon-mixin.toml` at `core/mod.rs` / `glue/mod.rs`.
2. Fix what the scripts refuse to guess: an `item without target` /
   `unmapped World method` / `unmapped names` error means new code needs a
   module decision — add it to `TARGETS`, `WORLD_FILES` or `FILES` at the top
   of the script instead of dumping it into a catch-all file.
3. Re-apply the source-contract helper: `tests/civil_sources.py` already exists,
   but tests that still read `core.rs`/`glue.rs` must use `core_source()` /
   `glue_source()` (a small regex rewrite plus manual fixes for slices that
   crossed module boundaries — prefer cutting a function body with
   `split("fn name(", 1)[1].split("\n}\n", 1)[0]`).
4. Gates: `cargo test`, `python tools/package.py`,
   `python tools/verify_static.py`, the Python suite, then the native gate in
   `tools/native_build.py` (`cargo check`, then the `civilizations_core::` and
   `civilizations_glue::` test filters).

`python tools/modularize/*.py` prints what it moved; nothing is committed for
you. Never claim playability from these scripts — they only move source.
