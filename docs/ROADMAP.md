# Civilization mod roadmap

These are separate acceptance gates. A source archive is **not** a gameplay
release until all preceding gates have evidence.

## P0 - Foundational source (in this starter package)

- [x] MIT original code, independent of Millenaire code/assets
- [x] Pure Rust domain model: villages, workers, resources, abstract houses
- [x] Simulated day ticks, background economy, persistence schema
- [x] Settlements and allegiance, initial player rank/reputation
- [x] Abstract war state and attrition (not a physical battle)
- [x] Horizon hook definitions, server commands, named villager spawns
- [x] Source-only `.hnmod` packaging and CI configuration
- [ ] Cargo tests actually run on a Rust toolchain
- [ ] Native mixin compiled against pinned Horizon commit
- [ ] Integration E2E on a real prepared Horizon world

## P1 - First true playable milestone

- [x] Basic reload reattachment by deterministic names (unloaded/ambiguous names are ignored)
- [x] UUID-based actor ownership and operator-driven reconciliation (native CI and gameplay E2E still required)
- [x] Experimental job-specific NPC walking targets (loaded chunks only)
- [ ] Real profession goals: farmers farm, miners mine, builders build
- [ ] Physical building templates and progressive block placement
- [ ] No dupes after restart or partial spawning
- [ ] Player dialogue and villager trading UX instead of chat-only commands
- [ ] Per-day offline catch-up and host-authoritative world clock
- [ ] World saves under corrupted-file, interrupted-write, and rollback tests
- [ ] Real-time UI panels and navigation hints

## P2 - Kingdom layer

- [ ] Territory boundaries and land ownership
- [ ] Courts, rulers, lawful title grants, succession
- [ ] Resource and treasury accounting with crash-safe action journal
- [ ] Caravan routing with loaded/unloaded transitions
- [ ] Multi-culture data packs and configurable production rules
- [ ] Diplomatic causes, treaties and persistent grievances

## P3 - War layer

- [ ] Garrisons, supply, recruitment, army movement
- [ ] Tactical combat near players, deterministic abstraction far away
- [ ] Sieges, raids, ceasefires, conquest and post-war recovery
- [ ] Multiplayer fairness and administration/performance limits

## Invariants

1. Simulation is authoritative; physical NPCs never independently mint stock.
2. One persistent resident ID must map to at most one physical entity.
3. A player-item exchange must never silently mint or destroy wealth on crash.
4. Remote and live warfare must use consistent casualties and consequences.
5. Upstream Horizon is unmodified; only versioned hooks are touched.
6. One failure must not overwrite damaged world data.

## 0.1.2 trial build: explicit proof gate

1. Run `cargo test`, rustfmt, clippy and the native `plan/apply` + Horizon cargo check.
2. On a disposable Minecraft world, approve an empty patch using `/civ construct Ashford` after `/civ day 1`.
3. Confirm **real builder actor** within 9 blocks, construction increments 0..plan length and actual blocks match plan.
4. Restart halfway and verify no duplicate or skipped blocks; restart after final completion and verify one permanent site.
5. Obstruct an unfinished location with a different block: project must become blocked, preserve it and never overwrite it.
6. Leave the chunk: no forced chunk load, no block edits in another dimension. Return and check continuation.
7. Test failing writes and interrupted `world.json.swap`; primary/backup recovery must not invent progress.

Production-quality construction still requires explicit plot ownership/claims, collision checks for entities,
recovery command for blocked projects, safety auditing, block-entity handling, durable cross-chunk save testing,
and either authoritative mob UUID ownership or a well-scoped identity registry.

## Post-native-build reliability work (2026-11-07)

- [x] The pinned Horizon native compile gate passed on GitHub Actions run 42375553368.
- [x] Core Rust/Clippy/package gate passed on run 42375993049.
- [x] Construction reconciliation extracted to a pure decision engine and covered by Rust tests for rollback, foreign blocks, loaded chunks and completion (new CI still required).
- [x] Physical construction opportunities rotate across loaded, unblocked villages so an unloaded first settlement cannot starve others (new CI still required).
- [ ] Real in-game E2E: launch, NPC navigation, hut construction/restart, multi-village fairness.

## Visual farmer milestone (v0.1.4, gameplay E2E pending)

- [x] Per-settlement single saved field, phase/cursor, crop-cycle counter and 3×3 non-overlap placement.
- [x] UUID-linked farmer waypoint and loaded-proximity gating for physical edits.
- [x] Real soil/water/plant command steps; reads loaded Horizon blocks, including real wheat age=7.
- [x] Pure observable phase machine: preparing → growing → harvesting → replanting → growing; safe rewind after chunk/save skew.
- [x] No generated item drops or warehouse credits from physical harvest (prevents double accounting against abstract daily stock).
- [ ] Gameplay evidence: irrigation, crop growth under vanilla random ticks, conditional harvest and replant, unloaded-chunk restart recovery, and client logs.
- [ ] Durable physical-harvest-to-ledger credit journal and full economic coupling. **Do not claim this is implemented yet.**

## Harvest audit milestone (v0.1.5)

- [x] Save a bounded monotonic audit of eight-crop visual harvests, including observed world day.
- [x] Keep receipt creation and harvest phase transition together in mod JSON, with legacy load defaults.
- [x] Read-only /civ harvests NAME for tracking observed physical work.
- [ ] Cross-store atomicity between Minecraft chunks and mod JSON; no economic stock credits until verified.
- [ ] In-game evidence: confirmed crop removal, audit persistence after restart, corruption detection and rollback.

## Host clock synchronization (v0.1.6)

- [x] Use actual Horizon `game.world.day_time` for automatic economic day progression.
- [x] Save and validate an optional last observed host day; legacy worlds bootstrap safely without historical output.
- [x] No double processing across relog/restart or when host clock moves backwards.
- [x] Bound large time jumps to 30 simulated days and skip the rest without repeated catch-up.
- [ ] Real-world E2E of sleep/skip-night, /time set, per-dimension behavior and pause/server lifetime.
