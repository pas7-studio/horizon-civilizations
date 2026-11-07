# Verification status (2026-11-07)

The sources were authored here from documented Horizon APIs and the publicly
available official SMP native mod. **No game has been launched and no Rust
compiler is available in this environment.** Packaging/static checks were run.

| Gate | Status |
| --- | --- |
| Files created | PASS |
| JSON / TOML structural checks | PASS |
| Source-only .hnmod archive | PASS |
| Rust formatting, lint, cargo unit tests | NOT RUN - toolchain unavailable |
| Horizon mixin transform on pinned upstream | NOT RUN |
| Horizon native binary build | NOT RUN |
| NPC appears in game | NOT RUN |
| Trade, save, relog, time progression E2E | NOT RUN |
| Player-ready recommendation | **NOT YET** |

Design caveats:

- Actors are actual vanilla villagers summoned on command, but their physical
  motions are **not** linked to simulated jobs yet.
- Houses are numeric civilization state, **not** blocks in the world yet.
- Wars and titles are a minimal domain demonstration; no army combats or courts.
- Cross-boundary inventory/world JSON trade is not a fully atomic transaction.
- Native mod compatibility is pinned to a particular Horizon SHA, not a promise
  of working against future Nightlies.

## 0.1.2 reliability increment (source only)

- Save load now validates settlement IDs, diplomacy, symmetrical wars, references and limits, not just JSON syntax.
- An automatic day that cannot persist reverts memory to the previous snapshot and stops advancing until the world is repaired/reopened.
- Actor names contain deterministic `(settlement ID, resident ID)` tags; loaded villagers are located again by name after world reopen, with ambiguous duplicates skipped rather than commanded. Legacy 0.1.0 actor names are recognized too.
- Profession-aware navigation targets have been added for loaded villagers. **These are walking routes, not actual farming, mining or material transfer.** No offscreen entities are spawned.
- `/civ roster NAME` reports recognized loaded villagers and occupations. `/civ actors` never blindly repeats a previously attempted spawn.
- Abstract house count now has a population-based cap; new load-validation, 20-village load and identity tests have been authored but **not run using Rust**.
- Risk remaining for this historical v0.1.1 revision: actor name is not a secure persistent entity UUID; physical build/combat not implemented; selling/buying physical items plus separate JSON cannot be guaranteed crash-atomic. Do not use with valuable world/inventory data.

## Gate commands

- `python3 -m unittest discover -s tests -p 'test_*.py' -v` — packaging/structure smoke tests; ran successfully in this environment.
- `cargo test && cargo clippy --all-targets` — **not run** (Rust toolchain unavailable here).
- `python3 tools/native_build.py --test` — **not run** (requires Git, Rust and internet on build host).
- In-game testing — **not run**; see `docs/TEST_PLAN.md`.

## v0.1.2 physical construction (code authored; gameplay not executed)

- Added a deterministic 5x5 hut plan in the pure Rust core: cobblestone foundation, oak walls/roof, front doorway, glass windows.
- Added optional save fields to maintain v0.1.1 JSON compatibility: `visual_job` with progress cursor and `visual_sites` of finished houses.
- Added `/civ construct NAME` (operator only), `/civ construction NAME`, strict site preflight and an explicit 12..80 block village radius.
- Construction uses the authoritative `native_level_tick_post` hook rather than ambiguous server tick world context. Builder NPC must be loaded and near the site.
- Each block command is conditional on the current state (natural ground for foundation; air elsewhere). No world edit happens from the player command hook. The cursor advances only after observing the intended Minecraft block on a later tick.
- Before each new action the adapter rechecks all completed steps, to recover when the JSON save got ahead of Minecraft chunk persistence. Unexpected blocks stop the project; it does not blindly overwrite another builder's blocks.
- Save path works around Windows rename-over-existing by keeping `.bak` plus a temporary `.swap` staging file. A stale `.swap` refuses loading until manually inspected.
- Rust tests were *added* for deterministic plan, legacy saves and construction progress/recovery but **cannot run here**, because Rust toolchain and network downloads are unavailable.
- Python packaging/structure suite: **4/4 passed**; `.hnmod` reproducible. No Rust compile, mixin transform, server launch, in-game placement or load test has run. This is NOT a player-ready release.
- Critical remaining risk: engine/mod JSON and Minecraft chunk saves are independent; the recheck reduces but cannot prove atomic consistency. Native pathfinding and console command behavior need real E2E on the pinned Horizon revision.

## GitHub CI observations (2026-11-07)

The first run stopped at rustfmt; an automated formatting capture gate followed. Clippy then requested `u64::is_multiple_of(4)` and the native mixin stage rejected upstream's `CLAUDE.md` symlink. These two defects are corrected in source but must pass a new CI run. Gameplay E2E remains unverified.

## Updated GitHub CI evidence (2026-11-07)

The historical verification table above describes the original prototype, **not** the current GitHub branch. GitHub native compile + embedded Rust tests: PASS (run 42375553368). Core CI (cargo, Clippy, tests and source packaging): PASS (run 42375993049). These prove compile compatibility with the pinned Horizon, **not** a real gameplay E2E. Construction reconciliation and fair scheduling are further work being tested in a follow-up PR. In-game launch, physical progress, restart/unload and multiplayer remain unverified.

## UUID binding / mod-disabled guard (implementation branch)

Each new spawned NPC is bound to its persistent Horizon UUID, saved after each spawn. Loaded-actor navigation/building verifies UUID lookup, not a custom name. Legacy actors require operator-invoked `/civ reconcile NAME`; no auto rebinding or respawn occurs. World validation prevents duplicate/zero UUIDs and rejects malformed hex or resident IDs. Hooks exit while the mod is disabled. CI and actual in-game verification must still be checked on this branch.

## 0.1.3 dialogue hook (feature branch)

Introduced the native `native_interact_entity` hook and UUID-only NPC dialogue, with vanilla interaction fallback for unknown entities. Revisions, static packaging and Windows local installer were updated to 0.1.3. This is a text conversation seed, not a custom menu, trade UI or quest engine. Native compile and in-game validation required.

## Physical farming (0.1.4 feature branch)

A persistently saved 3×3 irrigated wheat field now has a finite preparation phase and a repeatable visual farming cycle controlled by nearby **UUID-linked farmer** NPCs. Reads use authoritative loaded chunk state and crop age=7. Conditional server commands edit blocks one at a time; a later authoritative read commits the cursor. On any unexpected block the plot halts; on missed chunk saves it safely rewinds. The Minecraft world does not provide stock or item drops to the JSON economy, so **this does not close the physical economy transaction gap**. CI and real gameplay validation of this branch remain necessary.

## v0.1.5 audit (feature branch)

Full physical harvest observation now creates a persistent increasing receipt (sequence, world day, eight crops). Saves retain 32 recent receipts and reject invalid sequences on load; legacy v0.1.4 farms start with empty evidence. The command /civ harvests NAME reports this audit. This is not an economy transaction or gameplay E2E. No warehouse stock or item drops are granted by this feature.

## Host day counter migration (v0.1.6 feature branch)

Authoritative daily simulation now follows the persistent Horizon `day_time` clock. A saved `last_host_day` anchor protects against repeat application across reloads and backwards clock changes; old worlds initialize the anchor without backpay. Large clock jumps are limited to 30 simulation days to prevent runaway stock. Manual `/civ day` remains explicit. Native compile, real world time progression and sleep E2E pending.
