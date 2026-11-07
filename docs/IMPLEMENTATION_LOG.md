# Implementation log — Horizon Civilizations

## 2026-11-07, v0.1.0 — original source prototype

- Created an independent MIT Rust mod, no Horizon engine fork and no Millénaire source/assets copied.
- Domain: settlement founding, 10 residents, jobs, stock, day simulation, titles/reputation, abstract war, state JSON.
- Horizon adapter: commands, ticks, world close/save, on-demand vanilla villager spawning.
- Native `.hnmod` source package, initial CI and static validations.
- Untested: real Rust compiler, native mixins transform, Horizon E2E.

## 2026-11-07, v0.1.1 — reliability and NPC routing

- Added semantic validation on load: monotonic IDs, unique names, existing references, symmetric wars, upper bounds.
- Capped abstract housing demand by population, rather than endlessly constructing houses.
- Added deterministic role worksite offsets and NPC identity labels carrying settlement/resident IDs.
- Read loaded actors by their stored custom names and direct them toward worksite targets via Horizon native navigation.
- Preserved recognition of v0.1.0 actor labels; ambiguous matches are skipped instead of controlling both.
- Added `/civ roster <name>` for loaded-actor/profession inspection.
- On automatic-day save failure, revert memory and stop simulation rather than continuing on an unsaved state.
- Added Rust domain test cases (written, not executed here), repeatable source-package Python tests, and an isolated native compile script/workflow.
- Added a Windows PowerShell installer for an explicitly selected disposable Horizon instance.
- Local results: Python packaging/contract tests PASS, archive SHA-256 reproducible; Rust compile + game E2E NOT RUN.

## Current blocking risks before calling it a playable release

1. Validate the mod through `cargo check`, `cargo test` and the actual mixin compiler on pinned Horizon revision.
2. Prove actors exist, reattach after relog and route to workstations in a real level.
3. Upgrade identity to authoritative UUIDs, so renamed impostor villagers cannot be mistaken for actors.
4. Add job-driven material gathering and real incremental block building; neither is present in this version.
5. Introduce crash-recoverable journal or game-engine transaction for player inventory versus mod JSON changes.
6. Test multiworld/multiplayer and fail-closed recovery from corrupted or interrupted writes.

## 2026-11-07, v0.1.2 — physical starter hut prototype (source)

- Implemented deterministic small hut blueprint, optional persisted build job and completed-site tracking, semantic invariants and domain tests for plan/progress/legacy JSON.
- Added operator-approved empty-plot preflight and block-by-block placement gated by the actual named builder's presence.
- Moved world-dependent navigation/build work from global `native_server_tick` to `native_level_tick_post`, matching Horizon's dimension-specific hook.
- Added block-state-conditional console commands with observed-state cursor commits, replay reconciliation and fail-closed obstruction behavior.
- Improved Windows save replacement with recoverable `.swap`/`.bak`; skip save on a quarantined world and reject unexpected pending swap on load.
- Confirmed relevant native APIs and level hook exist in the pinned Horizon SHA through its public source.
- Ran offline Python 4/4 package/structure tests and checked deterministic `.hnmod` bytes. Rust compiler unavailable, native build, world edits and gameplay NOT tested.

## 2026-11-07 — build recovery and multi-settlement fairness

- Confirmed pinned Horizon native compile PASS (CI run 42375553368) and Core CI PASS (42375993049); no game was launched by CI.
- Refactored previously ad hoc physical block checks to a side-effect-free Rust inspection contract: unloaded chunks, rollback rewind, obstruction, previously placed blocks and completed jobs.
- Changed construction selection to rotate among loaded jobs so the first stalled village does not block every other settlement.
- Added scenario-driven Rust tests and explicit in-game acceptance requirements. Physical building remains experimental until the real-world E2E is captured.

## 2026-11-07 — UUID-bound NPCs / migration

- Replaced actor custom-name ownership with persisted Horizon UUID bindings; verification uses loaded UUID lookup.
- Saved identity after each spawn with a precommitted one-shot reservation; partial creation fails closed.
- Added an explicit operator-only legacy reconciliation command; malformed, repeated or misowned UUIDs are rejected.
- Added native-mod on/off guards to command and simulation ticks and Rust identity invariants/tests.
- Pending: native CI and manual game-world spawn/restart/reconciliation smoke tests.

## 2026-11-07 — first player-facing NPC conversation

- Integrated the authoritative server entity-interaction hook for UUID-bound villagers.
- Right-click displays real resident/profession/settlement/morale context and join/trade commands; ordinary vanilla villagers remain untouched.
- Updated build/package/installer version to 0.1.3, with source-level safety tests and in-game acceptance cases.
- Pending: native CI and real right-click gameplay, multiplayer and relog E2E.

## 2026-11-07 — v0.1.4 experimental visual wheat fields

- Implemented a three-by-three irrigated plot with 8 farmland/wheat sites and center water.
- Saved preparation/growing/harvesting/replanting phase, cursor, and visual cycle counter in optional legacy-compatible settlement data.
- Added farmer worksite navigation, actual crop-age inspection, loaded chunk gates and one conditional authoritative block command per tick.
- Added pure Rust tests for plot blueprint, obstructions, age gating, crash rewind, phase transitions, save compatibility and economy isolation.
- Deliberately no warehouse wheat credit or Minecraft item drops; a cross-store transaction journal is required before economic linkage.
- Pending GitHub Core/native checks and real-world E2E, including vanilla crop hydration/growth.

- Added fail-closed operator `/civ recoverfarm NAME`, with world-block preflight before clearing a blocked farm; simple unblock without checks is forbidden.

## 2026-11-07 — v0.1.5 harvest evidence

- Created HarvestReceipt journal on FarmPlot: monotonic sequence, world day, eight crops; bounded 32 entries.
- Receipt and Harvesting->Replanting transition mutate the same persisted JSON object.
- Added /civ harvests NAME inspection and core tests for duplicate completion, bounded rollover, validation and legacy JSON.
- No economy credits or player item drops from receipts; two-store recovery remains future work.

## 2026-11-07 — fail-closed world lifecycle

- Refused incoming world activation when outgoing world.json persistence fails; old state remains resident and commands/ticks halt with an error.
- Shutdown now retains in-memory world on save failure instead of clearing the singleton.
- Stale swap check moved before writes to backup/temp files.
- Requires native CI and a disk-failure world-switch test with a disposable save; process crash cannot preserve volatile memory.

## 2026-11-07 — world-authoritative time, v0.1.6

- Removed artificial server_ticks % 24000 daily progression that reset each time the mod reloaded.
- Observed Horizon's authoritative day_time and saved last_host_day for deterministic once-per-host-day economic progression.
- First-time anchor yields no production, backwards clock cannot double pay, catch-up limited to 30 days on a huge forward jump.
- Added four Rust tests for bootstrap/legacy saves, restarts, rewinds, bounded jumps and i64 extremes.
- Pending pinned Horizon native compile and disposable-world time/sleep/relaunch E2E.

## 2026-11-07 — v0.1.7 deterministic NPC routines (CI disabled)

- Added a four-phase Horizon-clock routine in pure Rust with deterministic
  per-resident waypoint offsets and unit tests for day boundaries, rewinds,
  guard shifts, waypoint bounds and save compatibility.
- Existing loaded UUID-bound actors receive phase-specific navigation
  targets; farm and hut construction are limited to the work window.
- Introduced read-only `/civ schedule NAME` and manual E2E cases.
- No new persistence schema fields, native mixin hook points or assets.
- User disabled GitHub Actions. Rust compile/Clippy/native Horizon and
  actual gameplay are not verified; keep this version experimental.

## 2026-11-07 — v0.1.8 physical home assignments (CI disabled)

- Created stacked draft work on top of v0.1.7 NPC day/night routines.
- Only completed `Settlement.visual_sites` count toward physical homes;
  abstract housing and unfinished jobs are excluded.
- Added pure Rust stable-by-ID slots (3 indoor positions per hut) and
  a fail-closed six-block physical validation before navigation.
- Night home routing falls back to commons if any floor/door/interior block
  is absent, altered or unloaded. No chunk loading, world edits, or new
  serialized fields.
- Added read-only `/civ homes NAME` and resident home information, four
  Rust unit tests and six Python source-contract tests.
- No CI, local Rust compile, Horizon native check or gameplay E2E has run.

## 2026-11-07 — v0.1.9 safe housing site proposals (CI disabled)

- Stacked a new feature branch on v0.1.8; no changes to pinned Horizon
  hooks or world JSON schema.
- Added bounded 20-slot deterministic candidate coordinates from each
  settlement center, with strict domain and same-dimension cross-plot
  collision checks (completed huts, farms and active projects).
- Enhanced both suggested and manual construction to share that same
  safety policy and 12..80 block distance restriction.
- Read-only `/civ survey NAME` inspects at most five nearby Y levels per
  candidate using the existing loaded-world 5x5 natural/air preflight,
  returning up to five live candidates without saving a reservation.
- OP-only `/civ approvehouse NAME SLOT Y` requires the exact slot/Y,
  checks dimension and operator proximity and repeats live-world preflight,
  then persists one job for the existing UUID-bound builder to implement.
- Added five pure Rust regression tests and seven source-contract Python
  tests, plus acceptance plan items 67–75.
- No GitHub Actions, Rust compilation, Python test execution, native Horizon
  integration or real gameplay E2E performed; all remain local gates.

## 2026-11-07 — v0.1.10 civic council and three-day outlook (CI disabled)

- Stacked source-only civic planning on top of physical housing, daily
  routines and safe operator-approved building proposals.
- Added pure deterministic prioritization for hunger, wars, low morale,
  unbound or unsummoned residents, lack of real physical huts, and future
  simulated-housing material shortage.
- Forecast reuses the existing daily abstract economy on a **clone** for
  three days (maximum seven for direct callers); no duplicate production
  model or simulated trade/war execution.
- Added read-only `/civ council NAME` and `/civ forecast NAME`, eight
  Rust regression cases, and six Python structural source contracts.
- Preserved schema=1 and the six existing native hooks. No new stock
  credits, actions, land claims, combat decisions, or world-save writes.
- No GitHub Actions or actual Rust/Python/native/game executions. Only
  structural source audits can be claimed until local testing.

- Added a local-only PowerShell validator for formatting, Clippy, Cargo,
  Python, packaging and optional pinned native client compilation.
- Fixed the native test filter from `civilizations_core::tests` (which misses
  newer sibling test modules) to `civilizations_core::` (all nested core tests).
- Exposed the top council concern in UUID-bound resident right-click text.
- Extended Python contracts to guard the dialogue and local validation path.
