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
  Rust regression cases, and eight Python structural source contracts.
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

## 2026-11-07 — default-on autonomous village P0 correction (v0.1.11)

- Identified manual-only actors, physical hut and farm commands as the
  primary reason the mod did not feel like a living settlement.
- Autonomy is ON on new villages and old saves (serde default true), with
  a per-village operator pause for new site planning.
- Added a bounded native level-tick starter pipeline: safe preflight of
  ten natural/air spawn positions, one-shot persisted UUID-bound villager
  spawn, already-paid abstract house backlog, and real 3x3 farm / 5x5
  hut site scheduling using loaded-world inspections and existing worker AI.
- Added 5x5 player land ownership in world JSON, overlapping-mod-construction
  exclusions and paid private hut commissions on owned parcels.
- Player civic coin purchases and commissions transfer to a persistent
  village treasury. Private homes are excluded from public NPC housing.
- Exposed read-only in-game /civ development NAME and /civ plots NAME.
- Existing building and farm conditional writes continue to refuse foreign
  blocks, so the planner does not get a terrain-clearing privilege.
- Added Rust domain and Python structural contracts; local Rust/native
  compilation and actual playable E2E remain pending, GitHub Actions OFF.

- Added real completed-hut purchase: 35 civic coins to settlement
  treasury, UUID-owned private land+building, native full-block
  verification, and public NPC housing eviction/recalculation.
- Fixed abstract public housing target to exclude sold/commissioned
  private houses, permitting material-paid replacement over time.
- Added additional Rust ownership/economy regression and Python source
  assertions, plus manual gameplay cases 99–102.

## 2026-11-07 — v0.1.12 first civic hall + receipt-backed physical warehouse

- Stacked a new source milestone on default-on village autonomy PR #19,
  without touching the active main binary or enabling GitHub Actions.
- Real central 5x5 town hall building blueprint includes mayor's lectern,
  two warehouse barrels and two bulletin signs; the existing merchant
  resident is routed to this civic center on duty.
- Added independent, persistent hall building job with domain/native
  cursors, blocked protection, completion and world-load validation.
- Added distinct physical warehouse `BTreeMap<Good,u32>`. Abstract
  simulated stock cannot pay for physical blocks. Hall requires 24 logs
  and 25 cobblestone; public/private physical huts require 20 logs and
  25 cobblestone, reserved once before first block placement.
- Actual native player-item sales enter physical warehouse; buys debit
  physical ledger. Confirmed 8-crop physical harvest receipt now credits
  precisely 8 village warehouse wheat while abstract stock stays unchanged.
- Added durable plan/issued/observed gather ticket, UUID-linked native
  lumberjack/miner navigation, conditional log/stone extraction and
  exactly-once world-JSON credit after successful observation.
- Added two real oak sign text update attempts with name, mayor,
  population, stock shortages and missing homes; native NBT command
  support is unverified.
- Fixed two legacy test fixtures that previously fabricated completed
  huts without physically reserved supplies; added nine domain tests
  and eleven Python source contracts for the new subsystem.
- Crucial follow-up: real vanilla barrel container contents are NOT
  transactionally connected; these are physical service markers for a
  separate authoritative save ledger. Native/gameplay tests NOT RUN.

## 2026-11-07 — v0.1.13 real native barrel storage and worker couriers

- Discovered and inspected the pinned upstream Horizon API 48:
  `native_container_slots`, `native_extract_item` and
  `insert_stack_through`. Replaced barrel-as-marker with actual native
  inventory reads and writes in the game bridge, without adding item NBT
  commands for inventory manipulation.
- Added strict official hall barrel positions, durable Planned/Issued/
  Blocked transfer records for bounded 1–8 item batches, and native slot
  count reconciliation before retiring the incoming item receipt.
- Added builder escrow only for physically extracted real barrel materials;
  corrected the navigation/reservation deadlock so workers fetch supplies
  at the hall BEFORE walking to the plot.
- Lumberjack/miner returns and farmer wheat receipts now require nearby
  live UUID-linked actors to reach the hall before goods enter the
  steward's delivery queue. The steward inserts them into real barrels.
- Player buy after hall creation precharges coins and saves a market
  order before one verified native barrel extraction/player inventory grant;
  any ambiguity becomes quarantined rather than automatically repeated.
  Trades require proximity to the actual village center.
- Added Horizon API 233 native block inventory-opening hook for the two
  village-owned barrels; ordinary players cannot loot them without trade.
  This adds a seventh native injection, with packaging validations updated.
- Rewired signs and /civ hall/warehouse status to inspect real native
  barrel stock, pending deliveries, builder escrow and market journals.
- Added structural source contracts and Rust transaction regression
  cases. No cargo, native build, Python test execution or live game E2E
  was available in this remote environment; GitHub Actions remain OFF.
- Known pending: physical hopper/external mod access, vanilla block-break
  hooks, atomicity across game chunks and world.json, manual quarantine
  recovery, actual courier held-item animations, quarry/lumberyard/workshop
  archetypes and town self-founding.

## 2026-11-07 — v0.1.14 actual bakery recipe + public inventory safety

- Branched from v0.1.13 real native barrel implementation, preserving
  existing user-disabled GitHub Actions and keeping changes out of main.
- Created unique bakery blueprint with crafting table and smoker, one
  bakery per settlement, persisted cursor/funding state and actual native
  conditional block-build execution by the existing UUID-bound builder.
- Required exact 24 oak logs and 33 cobblestone extracted from real hall
  barrels into separate bakery escrow before any physical bakery block work.
- Extended native Planned→Issued→Observed barrel transfers for bakery
  construction and wheat purchases; Bakers move to hall for actual wheat.
- Implemented on-site Baker 3-native-wheat→1-bread recipe using fully
  observed real workstation blocks and loaded UUID-bound actor proximity.
  Finished bread is carried as a durable receipt to the town hall, then
  steward deposits the corresponding actual vanilla ItemStack into barrel.
- Default-on village planner now automatically reserves one protected
  loaded plot for bakery and corrects prior v0.1.13 resource shortage
  accounting to count actual native barrel slots and pending/held goods.
- Added native API 334 to block vanilla hopper movement to/from public
  barrels, and native API 14 to deny unauthorized breaking of public
  structures or privately bought plots. Owners retain bought-plot editing.
- Added read-only /civ bakery detailed status, Rust regression tests and
  Python structural contracts; 9 native injections (previously 7).
- This is still **source-only**; native compiler, full test suite,
  in-world pathfinding, container/hopper/ownership hooks and persistence
  crash E2E have not been verified locally. No GitHub Actions enabled.

## 2026-11-07 — unreleased hardening stacked on v0.1.14 (Draft)

- Fixed misplaced `match t.direction` in `World::validate()`: bakery
  journal validation now has access to the actual `t` record and both
  branches are checked within its `if let` scope. This is a source-level
  compile blocker missed by the earlier static checks; not yet built.
- Added pure physical construction shortfall calculation that sums **both**
  currently outstanding bakery and hut bills, subtracts their independent
  saved material escrows, and then applies live native-barrel stock plus
  confirmed incoming queue once. Avoids starvation of an unfunded hut when
  a bakery is pending, and avoids considering one escrow as free stock.
- Native journals (barrel transfers or market purchases) and missing/unloaded
  barrel block entities prevent fresh construction material survey and
  gathering decisions until the journal is reconciled. Blocked bakery jobs
  cannot continue extracting more raw materials.
- The `/civ warehouse` read-only summary reports an honest unknown demand
  while native journals are unsettled. Added six core Rust regression cases,
  five new source-contract assertions; adjusted legacy bakery contracts.
- Changes were written to GitHub; Rust/Python executable suites, Horizon
  native compilation and actual Minecraft gameplay were **not run**. No CI
  activation, no merge, no claim of playable binary.

## 2026-11-08 — OP warehouse recovery + unified hall display (unreleased source)

- Added an authenticated OP-only, read-only native barrel inspection command:
  `/civ barrelcheck NAME`. It reports the specific saved native transaction
  and actual observed official barrel quantity. Missing/unloaded containers
  are reported as unknown rather than accepted as empty.
- Added strict OP-only manual reconciliation commands. `cancel` is allowed
  for unissued Planned intents when a fresh native count equals `before`.
  `confirm` is allowed only for Blocked receipts when native count equals
  `expected_after`; this changes only the persistent ledger and never
  calls a native inventory mutation. Ambiguous counts remain quarantined.
  Native market-purchase journals are explicitly outside this mechanism.
- Hall bulletin shortage values use the combined bakery+hut actual material
  calculation. An unresolved native receipt causes the sign to display
  `STOCK CHECK / PENDING TX` rather than a potentially false shortage.
- Added four Rust domain and six Python source tests; no Rust/Python suites
  or live Horizon/Minecraft E2E were executed. Do not use on real worlds
  until compile, migration, operator recovery and crash fault tests pass.

## 2026-11-08 — First-play diagnostic milestone (unreleased source)

- Added `/civ diagnose NAME` (ordinary users, read-only) to report actual
  loaded UUID-linked roles, current daytime shift, civic hall construction,
  native warehouse availability, bakery escrow/batches, planned gathering
  and the first saved-state blocker. No native item transfer, block mutation
  or persistent world change from the command.
- Combined supply deficit in this command uses the same native barrel
  snapshot and dual-project helper as warehouse planning, rather than the
  abstract economy. Missing/unloaded vanilla block entities are reported
  as unavailable, not as empty.
- Fixed PLAYTEST quickstart guidance to use the newest stacked branch instead
  of obsolete v0.1.9 checkout and to distinguish old native CI from this
  unbuilt v0.1.14 source stack.
- Added six Python source contracts and first-play acceptance tests 175–181.
  Local executable Rust/Python tests, native compile and in-game verification
  remain unperformed here. No GitHub Actions or merge.

- Added physical bread-restocking backpressure at both native wheat pickup
  and baking workbench. The pure helper uses 2 bread per living resident
  (maximum 64), counts verified native barrel items + confirmed inbound
  receipts + bakery bread-in-transit, not abstract daily stock.
  Missing/unloaded barrel inventories pause bread production. Full bread
  reserves do NOT starve the steward's other goods deliveries.
- Three Rust domain cases and five Python source contracts authored for the
  food buffer. No native build or game execution performed here.

## 2026-11-08 — natural oak and write-ahead gather safety (unreleased source)

- Replaced single-adjacent-leaf matching with pure native oak rooted-tree
  inspection: real soil, at least three contiguous logs, upper oak canopy,
  loaded block checks and conservative bounded height. This selects an
  actual first trunk block at ground level; whole-tree felling/replanting
  remain a later feature. Other native/player-placed trees outside mod
  claims cannot be distinguished from naturally grown trees by block
  states alone. Avoid production worlds.
- Added shared `World::gather_site_unclaimed` policy used both at plan
  creation and at actual issue, preventing a new player-purchased plot or
  pending structure from being mined after the original plan.
- Changed native wood/stone resource sequence to persist `Issued` BEFORE
  native `setblock`, with fail-closed no-command-on-save-error. A crash
  after issue can block without credit; the same source is never blindly
  retried. Unissued failed plans can be discarded automatically; issued
  blocked receipts are quarantined.
- Added OP-only `gathercheck` and `discardgather NAME intact|empty`
  for manual, evidence-matched abandonment with ZERO stock credit, rather
  than any unobserved grant or automatic replay. Other village planner
  work continues while a source job remains quarantined.
- Strengthened persisted gather bounds and worker stand checks on load.
  Nine new Rust tests and nine new Python source contracts authored.
- No Rust/Python suite, formatter, Clippy, native game compile or in-world
  execution was possible here. GitHub Actions remain OFF, main unchanged.
