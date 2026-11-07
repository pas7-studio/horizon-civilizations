# Latest assignment gate — 2026-11-08

- Work on `feat/assigned-resource-courier-v0115`, stacked on PR #28.
  Do NOT enable GitHub Actions, merge Drafts, or claim a playable game.
- `GatherJob.worker_id` is serde-default `Option<u64>`. Native worker
  is selected from UUID-linked loaded 3D source actors and durably saved
  before Issue. Same assigned resident ID is required at hall to credit
  ONE physical wood/stone receipt, retained for each oak segment.
- Old Issued records with worker_id None are quarantined and cannot
  credit the village; OP recovery only with live evidence and no grant.
  Dead/wrong-role/foreign workers cannot be newly assigned.
- Verify Rust 1.95, Clippy, all Python source tests, pinned Horizon
  native client compilation then disposable Minecraft TEST_PLAN 223-233.
  Do not infer item-held animations or autonomous courier inventory.

---

# Newest stacked source branch — 3D native worker guards

- Branch `feat/three-dimensional-native-worker-guards-v0115` now contains
  all prior short-oak, journal, warehouse and bakery work. PR is Draft,
  main unchanged, Actions OFF. 3D code is not a native/game PASS.
- `native_worker_near` in core.rs rejects vertical cave/sky mismatch;
  `native_worker_reaches_block` bounds eye-to-block 3D distance.
  Always check the saved worker stand is loaded, two-high air, supported
  on walkable native ground BEFORE issuing any world edit.
- Added Rust pure numerical tests and Python source contract tests in
  `tests/test_native_worker_3d.py`. Test cases 212-222 in
  `docs/TEST_PLAN.md` must be run in a disposable native Minecraft world.
- Handoff gate: build exact current SHA locally using Rust 1.95 + Python 3.11,
  then capture errors and in-game evidence. Do not use an older main EXE
  or auto-dispatch Actions.

---

# CURRENT SOURCE HANDOFF — verified short oak stem (2026-11-08)

- Latest source branch: `feat/verified-oak-stem-harvest-v0115`, stacked after
  PR #26. Keep Draft; **GitHub Actions OFF**, main unchanged. No native
  compile/game-world proof yet. Do not publish playable claims.
- New optional saved `Settlement.oak_stem` pairs with gather_job.
  A confirmed **3–4-log** natural rooted oak is cut segment by segment,
  with separate Issued state before each native edit, actual air
  verification and UUID-bound lumberjack return before one-log credit.
  Next log starts UNISSUED only after previous receipt is committed.
- A failed planner save must restore the WHOLE world pre-image, including
  oak_stem; an uncertain issued edit is quarantined, never auto-reissued.
  OP gathercheck/discardgather recover only with live block evidence and
  ZERO credit on abandoning uncertain segments.
- Test suite in docs/TEST_PLAN.md cases 200–211; static tests in
  tests/test_short_oak_stem.py and Rust module
  verified_short_oak_stem_tests. Run full local Rust 1.95 and Python
  toolchain tests plus pinned native Horizon compile before any merge.
- Saplings and tree replanting, forestry/quarry buildings, real hand-carried
  item entities and gameplay E2E are not done.

---

# Latest stacked local source handoff (2026-11-08)

- Work from `feat/natural-forestry-gather-journal-v0115`, not `main`.
  Stacked Draft PRs contain the latest native hall, warehouse, bakery,
  operator recovery and conservative oak/stone gathering changes.
- **GitHub Actions must stay OFF.** Do not merge or declare a playable build
  on the basis of source-only static checks or historic CI.
- First run `cargo fmt --all`, `cargo test`, Clippy, all Python tests,
  `tools/package.py` and `tools/verify_static.py` with pinned Rust 1.95.
  Then `tools/native_build.py --test --build-client` and isolate an actual
  prepared Minecraft world. Record SHA/logs for each compile/test failure.
- New `/civ diagnose NAME` shows a read-only NPC/world blocker;
  `/civ gathercheck NAME` audits the persisted native wood/stone gather;
  `/civ discardgather NAME intact|empty` is OP-only and abandons a blocked
  **Issued** harvest with ZERO resource credit only after loaded native
  evidence matches. Never blindly reissue an uncertain Minecraft edit.
- Oak harvesting presently allows only the first soil-rooted trunk block,
  not full-tree felling, replanting or a constructed lumberyard. The mod
  cannot distinguish player-grown oak from worldgen oak outside claims by
  block states alone. Preserve purchased land protection.
- Tests and local playtest instructions: `docs/TEST_PLAN.md` cases 189-199
  and `docs/PLAYTEST.md`. Keep all saves and actual Minecraft assets out of
  this MIT repository.

---

# Local agent handoff: Horizon Civilizations

Do not fork upstream Horizon for this project. It is a native mod with a pure
Rust core. Work in a separate repo. NEVER copy GPL-3.0 Millenaire Java files,
texture packs or any Minecraft resources into this MIT repository.

## Current state

- This is **prototype source**, not validated as running in Horizon.
- `mixins/civilizations/core.rs` is a standalone Cargo lib with unit tests.
- `mixins/civilizations/glue.rs` is a Horizon game adapter using native hooks.
- The pinned Horizon SHA is in `compatibility.json`.
- `tools/package.py` builds a SOURCE-ONLY `.hnmod`; it does not compile Rust.

## Local execution priorities

1. Install Rust 1.95.0 and run `cargo fmt --all && cargo test && cargo clippy --all-targets`.
   Fix every compilation/borrow/type issue found. Do not silently change game
   mechanics while fixing syntactic errors.
2. Run `python tools/package.py && python tools/verify_static.py`.
3. Create a scratch Horizon instance with a legal prepared Minecraft Java game.
   Build the native `.hnmod` with the launcher. If a mixin hook target is
   incompatible, update the adapter/pin and record the exact compiler error.
4. Run every numbered scenario in `docs/TEST_PLAN.md`, saving logs. Do not call
   the build playable until at least founding, NPC summon, trade, save/reload,
   day progression and multi-world isolation pass.
5. Prioritize a real resident ID -> entity UUID reconciliation mechanism. The
   current one-shot `actors_spawned` flag protects against automatic duplicates
   but not all partial failures.
6. v0.1.2 includes **experimental approved hut construction**. Verify every
   construction case before calling it playable. Generalized templates and
   real farming/mob goal overrides are not yet implemented. All other houses
   must remain explicitly marked as abstract simulation.
7. Add cross-inventory crash journal before making live trading public.
8. Preserve upstream updates by only using versioned hook points where possible.

## Provenance

API integration patterns are based on official Horizon docs and its MIT SMP
native mod. The Millenaire source was reviewed for abstract design ideas only.
All game resources come from the player's legitimate Horizon/Minecraft instance.

## Publish when ready

If the `pas7-studio/horizon-civilizations` repository does not exist, create it
as a **new repository** (not a fork of Horizon). Then add this project as its
first commit and push only after running local validation; keep world saves,
Minecraft assets, `target/`, and credentials out of Git.
