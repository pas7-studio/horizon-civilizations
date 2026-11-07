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
- [x] Previous stable v0.1.6 cargo checks/tests passed on GitHub Actions (v0.1.8 source is untested)
- [x] Previous stable v0.1.6 pinned Horizon native compile passed (v0.1.8 requires local rebuild)
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

## Daily NPC routines (v0.1.7 source-only; game E2E pending)

- [x] Derive dawn/work/evening/night schedules from native Horizon clock; no save migration.
- [x] Deterministic near-center commons/night waypoints and job-specific daytime destinations.
- [x] Keep guards on watch while other professions regroup near village center.
- [x] Gate physical farm and construction edits to active work window.
- [x] Read-only `/civ schedule NAME` reports phase and on-duty headcount.
- [ ] Local Rust formatting, Clippy, native compile and first playable E2E (Actions disabled).
- [ ] Real indoor sleeping/home ownership, civilian AI and player interaction remain follow-ups.

## Completed-hut home routing (v0.1.8 source-only, local E2E pending)

- [x] Compute capacity only from the completed physical hut registry, not abstract houses.
- [x] Assign up to three standing places per completed hut deterministically by resident ID.
- [x] Verify loaded floor, indoor lane and two-high doorway before night route (full structural integrity/pathfinding still unverified).
- [x] Fall back to existing commons route for no/unsafe/unloaded homes without mutating world blocks.
- [x] Show physical housing capacity and allocation with read-only `/civ homes NAME` and NPC dialogue.
- [ ] Local Rust/Clippy/Python validation, pinned Horizon native compile and in-world pathfinding E2E.
- [ ] Actual bed ownership/sleep, door opening, permanent household claims, and shelter effects on morale.

## Next autonomous-settlement milestones (proposed order, do not mistake for implemented)

1. **Safety/claims:** a physical plot-ownership registry and collision/obstruction
   checks across every completed hut, farm and pending construction. Add a
   deliberate operator approval boundary before any automatic site selection.
2. **Community life:** persistent household ownership and real beds only after
   Horizon indoor navigation is verified; then introduce daily resident needs
   and morale effects without touching the item economy.
3. **Profession E2E:** mine, lumberjack and baker jobs with real loaded-world
   observations and bounded actions. Keep unobserved/offscreen production
   separate from physical inventory transfers.
4. **Crash-consistent economy:** durable cross-inventory/world action journal
   before trade UI, courier item delivery or harvest-to-stock credits. Avoid
   minting double resources after rollback/restart.
5. **Civilization autonomy:** villages propose construction, negotiate prices,
   build relationships, compete for land and form alliances. Player can join,
   advise, veto or oppose decisions instead of micromanaging every task.
6. **Politics and war:** territorial claims, succession and logistics first,
   actual guard combat/raids only after consistent casualty and chunk-boundary
   handling exists.

**Development policy while GitHub Actions are disabled:** add isolated,
reviewable source milestones with explicit local verification gaps. Do not call
them playable until the local Horizon client E2E proves the actual world state.

## Safe village housing proposals (v0.1.9 source-only)

- [x] Derived need: no housing proposal if all living residents already have physical spaces.
- [x] Bounded, deterministic village plot slots and read-only live terrain survey.
- [x] Verify recorded plots across **all settlements in the same dimension**
  before either manual construction or suggested-site approval.
- [x] Reject recorded farm, completed hut and active project conflicts.
- [x] Explicit near-site OP approval that rechecks the entire loaded 5x5
  natural/empty plot and creates one durable construction job.
- [x] Existing UUID-bound builder, daytime-only physical placement and
  post-crash cursor reconciliation unchanged.
- [ ] Local compiler, Rust/Python tests, native Horizon check and gameplay E2E.
- [ ] Genuine player land claims, placed sign/structure detection, plotted
  roads/paths and a veto-capable town council (not implemented).

## Needs-driven village council (v0.1.10 source-only)

- [x] Pure, stable urgency ranking from food risk, war state, low morale,
  actor identity integrity, missing physical homes and house-material shortages.
- [x] Reuse the real abstract economy daily model **on a clone** for the
  three-day council outlook; do not reimplement production or mutate saves.
- [x] Bound hypothetical forecasts to seven days even for untrusted inputs;
  default to a three-day window.
- [x] Read-only `/civ council NAME` and `/civ forecast NAME`, with
  explicit model exclusions and player/OP decision boundaries.
- [x] Preserve old saves (schema 1), deterministic tie order, and no extra
  native Horizon hooks.
- [ ] Local cargo test/fmt/Clippy and Python test suite, pinned Horizon
  native compile and first game E2E (GitHub Actions intentionally disabled).
- [ ] Real autonomous follow-through with veto/consent and persistent
  civic decision history, after safety/ownership and local E2E.

## P0 correction — actual autonomous settlements (v0.1.11, source-only)

The former design required manual commands to spawn villagers and choose
every hut/field, so it could never deliver Millénaire-style autonomous
play. That was a **product gap**, not cosmetic polish.

- [x] **Autonomy ON by default**, including legacy saves; no mandatory OP
  permission for clear, unclaimed village-owned natural land.
- [x] Automatically reserve and spawn 10 UUID-bound native villagers once
  after safe loaded spawn locations are identified.
- [x] Stage already-paid starter houses for actual construction and schedule
  physical huts/irrigated farm on loaded, clear, collision-free sites.
- [x] Existing linked builders/farmers act in daytime; physical writes remain
  conditional on observed world blocks and saved cursors.
- [x] Purchase private 5x5 land using actual player civic coins; transfer
  proceeds to the village treasury; prevent NPC build/farm on owned land.
- [x] Commission an NPC-built private hut with an explicit owner-only
  contract (coin fee and consumed existing wood/stone).
- [x] Separate public resident beds from private commissioned homes.
- [x] Expose `/civ development NAME` state for in-game debugging.
- [ ] Run local rustfmt, cargo, Python tests, pinned native compile and
  disposable-world E2E; these are **NOT EXECUTED** with Actions disabled.
- [ ] Auto-found villages through safe world generation/discovery, not just
  automatic development of an existing village.
- [ ] Functional bakery, woodcutting yard, stone quarry, trading post,
  real resources and delivery/stock transactions. Physical wheat receipts
  are currently observation-only.
- [ ] Player parcel protections against vanilla multiplayer block edits
  require a dedicated supported engine hook or interoperable claim system.
- [ ] More home archetypes, blueprints, roads, expansions and town planning.

- [x] Transfer title of already completed, **block-verified** village huts
  to a player for 35 civic coins, credit village treasury, and free the
  property from NPC public housing use.
- [x] Restock public abstract housing via existing material-consuming
  economic simulation after a private sale instead of minting free
  replacement huts.
