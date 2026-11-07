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

## 2026-11-07 — world-authoritative time, v0.1.6

- Removed artificial server_ticks % 24000 daily progression that reset each time the mod reloaded.
- Observed Horizon's authoritative day_time and saved last_host_day for deterministic once-per-host-day economic progression.
- First-time anchor yields no production, backwards clock cannot double pay, catch-up limited to 30 days on a huge forward jump.
- Added four Rust tests for bootstrap/legacy saves, restarts, rewinds, bounded jumps and i64 extremes.
- Pending pinned Horizon native compile and disposable-world time/sleep/relaunch E2E.
