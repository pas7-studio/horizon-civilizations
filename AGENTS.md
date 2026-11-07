# Latest source: guarded recovery of partial villager spawning

- Branch `feat/guarded-partial-villager-recovery-v0115` stacks after
  PR #33. Actions OFF and main untouched; no playable claims.
- Problem: one-time initial native spawn reservation may be only
  partially complete after a crash. DO NOT reset `actors_spawned`
  or automatically respawn the entire batch.
- Implemented `Settlement.actor_recovery_issued` per living
  unbound resident. Operator can inspect via `/civ roster NAME`,
  first `/civ reconcile NAME`, then deliberately
  `/civ recoveractor NAME ID attempt` ONLY after 96-block
  loaded scan, no exact/legacy tagged actor, safe open site.
- Record one-shot reservation BEFORE native spawn; a failed or
  interrupted UUID persistence must not reissue the spawn. Exact
  UUID binding clears only that ID's marker.
- 7 Rust cases, 12 Python static source tests, TEST_PLAN 272–282.
  No Rust/native Horizon compile or Minecraft E2E in this container.

---

# Current source branch — native binary evidence gate (2026-11-08)

- Latest stacked Draft `fix/first-play-binary-evidence-v0115`,
  based on PR #32. Keep GitHub Actions OFF and `main` untouched.
- `tools/first_play_gate.py --build-client` must reject
  native command exit=0 without a valid exact-path Horizon client.
  Validate pinned native JSON, horizon_revision, cargo_check
  and modded checkout path; store binary SHA-256 and size.
- The Python gate harness now has 10 mock tests, actually
  executed 10/10 PASS on Linux. They are **NOT** proof
  of a true Cargo build/native game E2E.
- Continue from the first actual compile failure. No release
  until real native Horizon client and disposable-world E2E.

---

# First-play evidence gate — current newest stacked source

- Branch `chore/first-play-native-validation-v0115`, stacked after
  PR #31. Keep all Drafts, Actions OFF, main unchanged.
- Run `py -3 tools/first_play_gate.py --build-client` on networked
  Windows with exact source, Rust 1.95, Python 3.11+, Git.
- Read `target/first-play-gate/<UTC>/report.md` and failing stage
  log. Until real pinned native compile passes, NOT playable.
  Even native client compilation is NOT a Minecraft-world E2E pass.
- The release-gate runner itself was tested 7/7 with MOCK binaries
  on Linux Python 3.13; do not confuse that with Horizon build.
- Existing physical gather source logic/journals and all previous
  safety limitations remain; continue fixing the first real
  compiler/runtime failure before adding speculative features.

---

# Current stacked source — fair gather queue + native proof 2026-11-08

- Latest branch `fix/gather-crash-proof-fair-queue-v0115`, stacked
  after PR #30. No GitHub Actions, no merge to main, no claim of a
  playable/modded Horizon binary before exact native compile + E2E.
- `gather_turn_index` alternates loaded eligible settlements
  on eight-tick gather opportunities; do NOT regress to first
  `.find()` work-item selection.
- `GatherJob.native_edit_confirmed` is serde-default false. Required
  safe order: save Assigned → save Issued → execute native conditional
  edit → freshly confirm air and command accepted → save native_edit_
  confirmed → source pickup witness by same UUID worker → separate
  hall return → one material credit.
- Issued but unconfirmed means a potential crash AFTER native edit;
  quarantine even if source air. Never automatically retry command,
  infer approval from an air block, or credit the source. Old pickup
  witnesses without native confirmation must LOAD but be quarantined.
- Tests in `tests/test_gather_crash_fair_queue.py` plus 8 Rust cases;
  TEST_PLAN 245–254. Actual Rust/Python/native/game verification NOT RUN.

---

# Latest worktree — source pickup witness gate (2026-11-08)

- `feat/source-pickup-witness-v0115` stacks after PR #29. Keep all
  stacked PRs Draft; no Actions, no main merge, no playable claims
  until actual cargo, Python and native Horizon game-world E2E.
- `GatherJob.pickup_witnessed: bool` is serde-default false. New
  issued native edits do NOT route the courier home until the same
  saved UUID-bound source worker is seen at the original work stand,
  within 3D reach, with source block air. Persist this phase BEFORE
  worker moves toward the hall; only then may a separately observed
  3D hall return credit one unit.
- `witness_gather_pickup` checks issued/nonblocked and exact resident
  ID; `confirm_gather` rejects unwitnessed source. New oak segments
  keep worker ID but reset witness to false. Old saves remain loadable.
- Exact native acceptance TEST_PLAN 234-244. No actual compiled client
  or game-world E2E run here: local toolchain unavailable.

---

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
