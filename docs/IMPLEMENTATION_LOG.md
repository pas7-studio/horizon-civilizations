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

## 2026-11-08 — short-oak stem continuation, one physically returned log per receipt

- Added `OakStem {root, stand, height, cursor}` as an optional durable
  settlement field. Old saves deserialize it to None. Domain validation
  prevents orphaned/mismatched segment cursors, missing gather job,
  invalid height or wrong physical source.
- Native forester measures only a confirmed naturally rooted leaf-bearing
  oak of 3–4 reachable logs. Extra-tall trees stay intact until an
  appropriate scaffold/worker mechanic exists; a tree's root log alone is
  no longer the complete harvest.
- After the issued Minecraft conditional edit, native observation of air
  AND actual UUID-bound lumberjack return to the town hall credit exactly
  one oak log. Only then does the core advance to the next UNISSUED log.
  Each next log checks the saved air prefix, live oak suffix, crown and
  current land-ownership claims before any further native edit.
- The planner previously rolled back only gather_job on save failure.
  It now preserves the full previous World so the new oak_stem field
  cannot be stranded without its gather. Quarantined issued receipts
  remain manual-only and an abandonment yields zero materials.
- Updated both /civ diagnose and OP /civ gathercheck with current oak
  step N/total for actual first-play troubleshooting.
- Eight new Rust tests and nine Python structural checks authored;
  corrected stale test scopes after adding the OP gather commands.
  They were NOT executed with Rust/Python toolchains in this environment.
- Source-only; actual native compile, server tick behavior, loaded chunk
  logic, legacy save migration and game E2E remain unverified.

## 2026-11-08 — 3D worker presence, block reach and work-stand revalidation

- Found native actors near hall/farm/bakery were recognized by horizontal X/Z
  alone. This could let a UUID-linked villager below a village in a cave
  authorize wood, stone, wheat or bread production while not physically
  present. Added reusable finite-checked 3D actor proximity with vertical
  bounds; used by all physical resource work and deliveries.
- Resource extraction additionally requires 4.5 blocks of physical eye-to-
  block reach and being near the saved accessible worker stand. That stand
  is freshly re-observed (feet/head air, walkable floor) immediately
  before issuing any native Minecraft block command. A new obstacle or
  unloaded stand fails closed.
- Navigation now considers vertical mismatch, rather than indefinitely
  treating a vertically displaced NPC as arrived.
- Source-only tests: 3 Rust numerical regression examples, 11 Python
  source contracts. 17/17 initial static assertions passed before the
  stand recheck; refreshed final checks pending. No runtime tests or native
  game build were run. GitHub Actions intentionally remain off.

## 2026-11-08 — assigned, durable resource courier identity (unreleased)

- Audited native gather: one UUID-linked lumberjack could issue a physical
  block edit, but ANY other same-role worker already at the hall could
  complete `confirm_gather`. This incorrectly treated worker profession
  and proximity as evidence of an actual return.
- Introduced serde-default `GatherJob.worker_id: Option<u64>`. A new job
  starts unassigned. After live 3D source-worker observation, a single
  qualified, living village resident is assigned and saved as a separate
  durable step before `Issued` and any native edit.
- `mark_gather_issued` and `confirm_gather` reject unbound jobs.
  `gather_one_step` accepts delivery ONLY by the previously assigned
  UUID-linked resident at the real 3D hall. A legacy Issued journal
  without worker_id is blocked/quarantined and never credited.
- Bound worker ID persists into every subsequent short oak stem segment,
  across reloads. Guide routing honors assignments so same-role
  unassigned NPCs cannot take over someone else's saved task.
- Added six Rust tests and ten Python source contracts covering
  valid binding, theft prevention, old receipt handling, unknown
  foreign resident ID, dead workers and crash/restart continuation.
- Static inspection only. Rust/Python suites/native Horizon compilation
  and actual Minecraft gameplay have NOT been executed.

## 2026-11-08 — source witness gates NPC resource return

- Problem: an assigned, UUID-linked worker could be sent to the hall
  immediately after `Issued`, without observing its source already
  changed to air. A save reload or unrelated native removal could
  otherwise leave source-side work indistinguishable from a carried
  resource receipt.
- Added backward-compatible `GatherJob.pickup_witnessed` (false by
  default). After an issued native block edit, `gather_one_step`
  demands an actual loaded, linked **same resident** still at the
  saved 3D source stand, within native eye-to-block reach, while the
  source block is air. This saves the witness before route return.
- `guide_actors` sends the bound resource worker home only when
  `issued && pickup_witnessed`; otherwise they remain at the
  original source stand. Blocked resource jobs return to ordinary
  routines rather than repeatedly navigating into quarantined work.
- A second, independent same-worker 3D hall arrival is still
  necessary for one item-accounting receipt. Successor short-oak
  segments reset to unwitnessed/unissued.
- Added seven Rust tests and ten Python structural source contracts;
  older tests updated so they cannot skip witnessing.
- Source-only inspections performed; no Rust compile, Python unittest,
  native Horizon build or Minecraft E2E. No change to main/Actions.

## 2026-11-08 — fair multi-village gather and native-edit crash proof

- Found that `gather_one_step` used `.find()` on ascending settlement
  IDs and could stall all later village harvests whenever the first
  settlement's NPC was blocked returning from a quarry or tree.
  Introduced pure deterministic `gather_turn_index` and round-robin
  selection of all loaded nonblocked village gathers once per
  eight-tick native work opportunity; shift tracks actual job role.
- Found that Issued+air+same UUID worker could still be misinterpreted
  after a crash between Issued save and native command execution,
  especially if someone else removed the source block meanwhile.
  Added serde-default `GatherJob.native_edit_confirmed` false.
- After Issue is persisted and ONLY after native command returns
  success AND a freshly loaded read finds exact source air, the
  adapter persists `native_edit_confirmed`. This must precede
  `pickup_witnessed` and the final hall receipt. Issued without
  native confirmation on restart is quarantined, not replayed,
  witnessed or paid. Native command refusal/no-op is also blocked.
- Preserved loadability for old PR #30 saves which may already
  have pickup_witnessed=true with no new native_edit_confirmed.
  They load but cannot get further credit; the adapter blocks the
  ambiguous issued receipt until explicit OP decision.
- Added 8 Rust regression cases and 10 Python structural tests.
  Updated existing Rust successful gather cases to explicitly
  mark native confirmation before pickup witness, while preserving
  failure tests for ambiguous/legacy state. Adjusted old Python
  exact-string source contracts for new saved journal fields.
- Static inspection only (30/30); no Rust/Python runtime tests or
  native Horizon compilation/gameplay evidence. Draft-only.

## 2026-11-08 — working local first-play release gate with evidence

- Created cross-platform `tools/first_play_gate.py` that executes
  source check and optional pinned native Horizon check/client build in
  the exact checked-out branch. Verifies Rust 1.95, expected branch,
  optional SHA and clean Git worktree. No clones except the existing
  explicitly invoked native_build.py and no CI/web account actions.
- Every step writes a separate stdout/stderr log, report.json and
  report.md in `target/first-play-gate/<UTC>`. A failing stage exits
  nonzero; the report clearly records the first blocker and
  distinguishes compiled-client readiness from real gameplay.
- Actually executed 7 Python unittest harness cases using temporary
  fake cargo/rustc/rustfmt toolchain and local Git repositories.
  Tests passed 7/7 on Python 3.13/Linux, covering blocked and mock
  successful outcomes, clean source evidence, wrong SHA/branch/version,
  dry-run and diagnostic-only dirty mode. These are tests of the
  checker, **not** native game tests.
- Real test run discovered Python bytecode in tests directories could
  incorrectly dirty worktrees. Fixed the harness by disabling bytecode
  writes for subprocess checks and extended .gitignore for caches.
  Also records branch and SHA before Rust prerequisite checks so
  missing toolchain still yields a traceable report.
- No Rust cargo test, native Horizon compile, or Minecraft E2E run
  in this environment. Previous source work stays Draft.

## 2026-11-08 — build success must include a real native executable

- Found a false-positive readiness defect in PR #32: a native command
  returning 0 was sufficient to announce READY_FOR_GAMEPLAY_E2E,
  even if the client executable was missing. Fixed it by parsing
  the pinned native build script's final JSON evidence and checking
  Horizon revision, cargo_check, checkout output directory, and
  the exact native executable file path/size.
- Report now records client SHA-256, bytes and path; a bad/missing
  artifact or mismatched upstream revision overrides command success
  and blocks the gate, with the failed stage named.
- Actually executed ten Python unittest harness checks on Linux,
  including new false exit-zero/missing binary, wrong native revision,
  and native-only check without client binary; 10/10 PASS. Verified
  exact GitHub Python text against the locally tested script.
- Rust toolchain/native build/full Python source tests/Minecraft
  world E2E remain unavailable and NOT RUN; all stacked PRs Draft.

## 2026-11-08 — prioritize compiler failure over cosmetic formatting

- Reviewed current local build gate and found `cargo fmt --check`
  originally ran before any compilation. The accumulated prototype's
  Rust formatting could stop the first build before reporting an
  actionable type/borrow/compile error.
- Reordered stage plan to `cargo test` → `cargo clippy -D warnings`
  → strict formatting checks → full Python unittest → packaging/static
  checks → optional pinned Horizon native build/client.
- Real local harness exercise: 12/12 mock-toolchain tests PASS, including
  simulated compiler AND formatter failure where compiler must be first,
  plus simulated format-only failure that still blocks full PASS.
- Exact GitHub Python files matched the locally executed source by
  length and weighted checksum. No Rust build or Minecraft E2E run.

## 2026-11-08 — recover a missing citizen without duplicate village spawning

- Audited `auto_spawn_villagers`: it reserves `actors_spawned=true`
  before creating any NPC, correctly blocking a duplicate whole batch
  after restart, but partial native spawn leaves some worker IDs
  permanently unbound with no targeted recovery.
- Added serde-default `Settlement.actor_recovery_issued`, per-worker
  durable one-shot recovery reservation. `reserve_actor_recovery`
  requires original initial batch already reserved, matching living
  unbound resident, and rejects repeated attempts.
- `World::bind_actor` clears exactly the newly UUID-bound resident's
  pending recovery marker, retaining previously recorded UUIDs for
  every other villager.
- Added operator-only `recoveractor NAME ID attempt` with explicit
  confirmation, near-hall/dimension restriction, 96-block loaded
  region preflight, duplicate name protection, free surveyed spawn
  point and **save BEFORE native entity creation**. Ambiguous
  UUID/save failure halts world mutations rather than replaying spawn.
- Updated roster with resident ID + UUID/unbound/recovery state.
  Existing `reconcile` may rebind a unique visible original
  villager without additional spawn; automatic native spawn still
  performs its original strict ten-safe-point preflight.
- A pending spawn attempt remains a valid saved tombstone even if the resident later dies; this never permits spawning a dead NPC.
- Added 8 Rust regression tests + 12 Python static tests; manual
  E2E 272–283. Connector-side 22/22 source checks PASS.
  Cargo, native Horizon build and game E2E still not executed.


## 2026-11-08 — unify safe initial NPC spawn for OP and autonomy (PR #34)

- Discovered that the explicit OP `/civ actors` command still used ten
  fixed positions and central Y, bypassing the terrain survey used by
  autonomous spawning. This could create villagers inside blocks or near
  existing native mobs, despite a successful one-shot spawn reservation.
- Added shared `surveyed_initial_actor_positions` for both paths.
  Rejects dead/already UUID-linked initial residents, fewer than ten
  residents, matching preexisting loaded tagged/legacy villagers and fewer
  than ten unoccupied, loaded, natural-ground positions.
- Extended `safe_actor_spawn_points` to reject live-mob-occupied locations.
  Both paths finish preflight before durable `actors_spawned` reservation.
  Manual spawning now uses surveyed positions, not center-based offsets.
- Updated existing Python source contracts (16 authored); checked 17
  exact-source safety assertions against GitHub PR #34, all PASS.
- Evidence limit: **no** Rust compile, Python unittest execution,
  native Horizon client build or Minecraft E2E in this environment.
- Keep stacked PR Draft and main untouched; do not enable Actions or merge
  before real Windows build evidence and disposable-world test.

- Follow-up: corrected `tools/first_play_gate.py` default expected
  branch to the actual PR #34 checkout and aligned the Python test
  fixture. The user can now invoke `--build-client` on the correct
  branch without an explicit `--expected-branch` override. Runtime
  gate tests and actual native compilation remain unexecuted here.


## 2026-11-08 — post-save rollback safety, pairwise spawn clearance, build diagnostics

- Confirmed the mutating command handler could restore its pre-command
  clone after a subsequent `Err` despite an earlier successful durable
  save. After `already_saved`, errors now fail closed with
  `state.problem` rather than overwriting in-memory committed state.
- In `/civ reconcile`, save failure after an in-memory UUID binding
  sets the already-saved guard so the outer handler cannot attempt an
  unexpected second save. Manual disk review and restart are required.
- The ten-position native spawn survey now checks separation between
  positions already chosen during the batch survey, not only against
  existing mobs. Independent in-memory flat-ground survey produced
  ten candidates at squared distances >=9 (actual min 16).
- `tools/first_play_gate.py` now streams failed stage logs to extract
  their first actionable Rust/compiler/Python diagnostic into the
  structured report and console, keeping a bounded summary. Added
  Python mock stage for a compiler `error[E0425]` on POSIX/Windows.
- Added three guarded actor source contracts and one build-gate harness
  case; source total now 19 actor recovery, 14 build gate tests authored.
- Independently executed seven regex pattern probes and a ten-position
  flat-ground spacing simulation via local Python (both PASS). Those
  probes are **not** a cargo test, full Python unittest, native client
  build or Minecraft gameplay E2E.
- PR #34 remains Draft, main unchanged, Actions OFF. Acceptance 289–292.


## 2026-11-08 — guard orphaned `world.json.tmp` from interrupted first write

- Compared the pinned Horizon native mob creation and entity UUID APIs.
  `spawn_mob` inserts into `self.mobs` synchronously;
  `native_entity` and `native_entity_uuid` can immediately inspect the
  spawned actor. The suspected deferred-identity issue was **not** confirmed;
  no unnecessary changes to native spawn identity were made.
- Found the actual saved-world gap: `load` already rejected
  `world.json.swap`, but treated missing primary + residual
  `world.json.tmp` as a fresh empty world. Also, `save` would
  truncate an existing temp file on its next attempt.
- Fail-closed load now detects both temporary and swap artifacts before
  opening the primary or creating any default world. Save rejects either
  stale artifact BEFORE `fs::File::create(&temp)`; original bytes remain.
- Added three Rust filesystem regression cases (authored in native glue)
  and three Python source-contract cases. Acceptance tests 293–296.
- Verification limitation: no local Rust/cargo toolchain or direct
  GitHub DNS in the container. Exact-source checks run via GitHub;
  full Python unittest, Rust and Minecraft E2E **not executed**.
- Keep PR #34 Draft, no main merge or Actions changes.


## 2026-11-08 — reject silently skipped native Rust tests

- The native script previously used only the `civilizations_core::` Cargo
  filter, so tests added to `civilizations_glue::` would never execute.
- Added `run_filtered_native_tests`: enumerate tests with Cargo `--list`,
  require a nonzero inventory, then run the exact filter. Both core and
  glue suites are now mandatory and their results enter `native_mod_tests`.
- The first-play gate verifies two unique suite entries, successful
  status and integer `discovered >= 1` before accepting native client
  evidence. A fake executable alone is no longer sufficient.
- Added three standalone Python mock-Cargo enumeration tests and two
  first-play gate false-positive cases, and updated native CLI contracts.
- Verified upstream pinned `horizon-client/src/lib.rs` exposes a
  library target compatible with `cargo test --lib`.
- TEST_PLAN 297–300; no real Rust, native client or gameplay E2E run.


## 2026-11-08 — Windows GUI launcher wait and real saved-state diagnostics

- Compared the exact pinned Horizon launcher CLI and README. Its
  `import-build`, `prepare` and `play` commands are supported, but
  the launcher is a Windows GUI-subsystem executable. A bare PowerShell
  `& horizon-launcher` call may return before the process ends, leaving
  `$LASTEXITCODE` unset/stale. This could start preparation/play before
  import completed or miss a failure.
- Changed `scripts/start-windows-playtest.ps1` to pipe launcher output
  to `Out-Host`, forcing a wait while preserving argument boundaries,
  resetting the stale exit code first and requiring an actual zero exit
  status before the next stage. Added Windows launcher source regression.
- Extended `tools/playtest_evidence.py` to fail when orphaned
  `world.json.tmp` exists, matching the real native save guard.
- Inspected the core's custom `entity_uuid_hex` Serde adapter: saved
  actor UUIDs are canonical 32-character hex strings. The existing
  read-only UUID parser is correct and was not changed.
- Added one-shot `actor_recovery_issued` journal validation and
  `pending_actor_recoveries` counts to saved-state diagnostics.
  Pending legitimate markers yield warnings; duplicate, unknown or
  already UUID-bound markers fail; dead unbound tombstones remain valid.
- Expanded Python evidence tests for interrupted temp writes and
  pending/invalid/dead NPC recovery markers. Updated PLAYTEST's current
  branch and acceptance cases 301–306.
- These are **source changes only**. There is no working Rust toolchain,
  direct GitHub DNS or Windows PowerShell in the current runner; real
  Rust/Python full suite, Windows launcher and gameplay E2E not executed.
  PR Draft, main untouched, Actions not enabled.


## 2026-11-08 — autonomous NPC-founded frontier settlements (PR #35)

- Chose the largest missing first-play game loop: source had autonomous
  builders, farms, bakery, NPCs and physical goods, but **no natural
  settlement founding**; an OP needed to type `/civ found`.
- Refactored core World::found into a common founding path plus
  `World::found_autonomous`. All initial residents/starting abstract stock,
  native building invariants and 192-block spacing are shared. Only
  player-founded towns grant a Lord title and +25 civic coins to a real
  authenticated founder; NPC towns have an independent founder marker and
  NO synthetic player record, title or payout. Cap NPC-founded towns at 8;
  manual operator founding retains the overall 64-town rules.
- Added bounded `autonomous_frontier_found` to authoritative loaded
  Overworld level ticks every 2,400 ticks. It observes integrated single-
  player host plus joined server guests; requires finite/bounded player
  coordinates and skips explorers within 192 blocks of a known town.
  Eight deterministic ~88–91 block offsets are scanned using only loaded
  natural 5x5 areas with no non-air obstacles (no clearing or worldgen).
- Critically, the trial settlement must also pass the EXACT shared
  10-villager spawn-survey and unique-clear-position preflight BEFORE its
  first durable save, preventing stranded paper settlements. After save,
  existing autonomous logic owns real NPC spawning and physical work.
- Added player-facing, read-only `/civ nearby` command: sorted by horizontal
  distance in current dimension, bounded top four, coordinates/directions,
  hall status and joining hint. Existing commands still available.
- Added six Rust domain tests and eleven Python source contracts; checked
  26/26 targeted exact-source invariants, but there is NO Rust compiler or
  native Horizon client available in this execution environment.
- New PLAYTEST and acceptance 320–331. No new physical resource mint,
  external inventory effect or automatic chunk loading at founding.
- Remaining: real native Rust/Horizon build, single-player/dedicated tests,
  terrain generation and NPC navigation E2E, actual resource work,
  gameplay usability and server performance. PR remains Draft, no main
  merge/Actions enablement.

- Integration note: this PR is based on updated main and preserves
  the newest native civic menus, outpost posts, role-staffing and
  six-sign bulletin implementation rather than restoring older PR #34.


## 2026-11-08 — daily UUID-witnessed physical resident food (PR #35)

- Added to the current main-based PR #35, following natural NPC founding.
  Existing native baker produces physical bread but citizens previously
  never consumed those vanilla items; virtual day simulation cannot be
  presented as an actual resident meal.
- Extended existing `BarrelDirection` with `ToResidentMeal` and a
  backward-compatible optional durable `recipient_id` on BarrelTransfer.
  One real item per living UUID-linked resident per observed Horizon day;
  no virtual ledger debit or credit during this native item extraction.
- `World::begin_meal_day` saves one host-day anchor and served resident
  IDs, settles a prior physical shortage once per day, and pauses rollover
  while a previous Issued/Blocked native meal remains unresolved. The
  initial day does not invent historical missed meals.
- New `plan_resident_meal` enforces one loaf, native hall position,
  actual slot, unserved living recipient and no concurrent transfer or
  player order. The shared Planned -> Issued -> observed native inventory
  journal remains the only path to mark the resident fed. No duplicate
  extraction after restart.
- Native dining planner requires a steward and exact loaded UUID-bound
  resident near the hall. NPC movement routes an unserved citizen to the
  hall; a fresh actor witness is rechecked before issuing the native
  extraction. Only a never-issued plan can be canceled if the NPC leaves
  or dies; issued/block receipts remain durable and recoverable.
- A resident who dies after the bread was issued remains in the audit
  but is excluded from the living-fed count, so other residents do
  not incorrectly become satisfied.
- Added `/civ meals NAME` read-only diagnostics, Village GUI bread line,
  hall Treasury sign feed/shortage with abstract hunger explicitly
  separated. Physical shortage lowers morale, a fully served day
  recovers a small amount on next host-day rollover.
- Authored nine Rust domain regression cases and fifteen Python source
  contracts; core, glue, static contracts and status docs checked.
  **Not executed**: full Python unittest, real Rust compiler, pinned
  Horizon native client, real barrel extraction and Minecraft E2E.
- Acceptance 332–344; PR #35 remains Draft; no main merge/Actions trigger.

## 2026-11-08 — rebased onto quarry-pit main

The branch was refreshed against main d0308815 which added genuine
quarry staircase mining and plot earthwork, and removed the older
native menu-hook integration. The NPC food and frontier source was
ported onto that code rather than restoring obsolete menus. Meal
status remains available through `/civ meals` and hall signs; the
native menu is deliberately not claimed. Native compilation/E2E pending.


## 2026-11-08 — physical sand/dirt couriers and bounded dining shifts (PR #35)

- Inspected the actual post planner, movement adapter and durable gather
  journal. Found a concrete unfinished end-to-end route: Sand/Dirt are
  correctly assigned to Job::Digger in Good::job, but the core
  assign_gather_worker gate previously rejected them, core save
  validation treated their bound worker as a Miner, and two native
  gather stages plus actor navigation also selected a Miner. This could
  strand pit jobs and reject a correctly bound Digger save on reload.
- Unified all three authorization sites around the existing Good::job
  mapping. Diggers now follow the same loaded-actor, natural-source,
  saved native-edit acknowledgement, source-pickup witness and return
  receipt path as wood/stone. No free sand/dirt or new world-save fields.
- Prevented a self-sufficiency deadlock: hungry villagers previously
  routed to the hall on every tick and overrode work even when there
  was no physical bread. Unserved NPCs now request meals during Dawn
  and Evening, freeing Work and Night for existing activity.
- Authored two domain Rust regression cases covering both natural pit
  goods, wrong-worker refusal, mandatory pickup witness, serialization,
  restart, and one receipt; added eight Python source contracts.
  Extended live acceptance to TEST_PLAN 345-352.
- **Source changes only:** no cargo/rustc or native Horizon build
  available in this environment. These regressions were authored,
  not yet executed; actual pathfinding, physical item movement,
  loaded-world correctness and Minecraft gameplay remain unproven.
- Work remained on Draft PR #35. main and GitHub Actions unchanged.


## 2026-11-08 — first-hall provisioning and physical public requests (PR #35)

- Implemented a zero-persistence CivicSupplyRequest read model based on the
  ACTUAL unfunded first-hall material ledger, or on fresh native two-barrel
  snapshots after the hall is complete. No new currency or item mechanism.
- Pending Issued/Blocked external inventory changes make the request status
  UNKNOWN, not falsely empty; an already-funded hall job has no second bill.
- Completed hall requests include real bread buffering (physical stock,
  inbound delivery and baker in-transit), and wheat for up to four recipes
  only after a real bakery exists. No abstract simulated stock is used.
- The Merchant is now a permanent starter workplace, so player deliveries
  of real logs/cobblestone can fund the first hall before any barrel exists.
  A finished hall also accepts bread. The Baker accepts physical wheat.
  All use the pre-existing price, joined-player inventory removal and save.
- Added public read-only /civ requests NAME and wired native conversation
  wants to the same pure request calculation. A third dialogue sale
  argument is registered for wood+stone+bread at once.
- Replay of a villager dialogue command must revalidate a living saved
  resident UUID, loaded native villager type/dimension and <=6-block
  live horizontal / <=4-block vertical proximity before join/trade.
  The near-hall radius alone is no longer enough for such commands.
- Authored five Rust domain regressions and twelve Python source-contract
  cases. Expanded TEST_PLAN to 353-365 for the eventual local build/E2E.
- This is source-only development per the user's explicit request.
  No Cargo, Python unittest, native Horizon build, Windows launch or
  Minecraft gameplay E2E was executed. Existing async world JSON / native
  inventory crash-atomicity limitations remain; no release claim.
- Draft PR #35 updated, main untouched, GitHub Actions not enabled.


## 2026-11-08 — supplier standing and honest courier visibility (PR #35)

- Before writing new features, inspected the head of PR #35 and found
  TWO top-level Settlement definitions in core.rs; the first was
  truncated and even declared houses twice. Removed only this malformed
  duplicate in commit 1c9b92d and kept the complete save structure.
  Source inspection now finds exactly one top-level Settlement.
- Added backward-compatible Player.civic_deliveries, keyed by village
  and Good. Each accepted existing World::sell entry records physical
  sold item volume in the SAME JSON transaction as warehouse credit,
  civic coin payout and reputation update.
- Contribution total grants a bounded progression: 32 sold items =
  Helper, 128 = Supplier, 512 = Benefactor. Each newly crossed tier
  grants +4 reputation once using old/new cumulative totals; no new
  goods or coins are minted by milestones. Existing normal payment
  stays at Good::price. Every count is capped and validated on load.
- Command /civ service NAME shows only the authenticated player's
  service record in that town, tier, next milestone and saved per-good
  counts. Command /civ logistics NAME exposes existing gather receipt
  phases, assigned worker identity, returned farmer wheat, baker bread,
  warehouse-awaiting supplies, native transfer conflicts and either
  real loaded barrel contents or explicit unavailable status.
- Neither command manipulates blocks, reissues a receipt, mutates
  accounts or creates native inventory. No claim of actual held-item
  render, mule/cart storage or physical courier NPC equipment.
- Five Rust domain regression cases and twelve Python source contracts
  authored. Acceptance scenarios 366-378 describe later gameplay
  and save/crash validation. All code remains source-only with zero
  builds or executable test runs per user request.
- Existing native inventory and JSON world saves are not guaranteed
  one atomic transaction; this milestone reuses the current guarded
  sale path rather than falsely claiming that limitation is solved.
- Draft PR #35 remains unmerged; main and GitHub Actions unchanged.


## 2026-11-09 — UUID-bound crop and bakery couriers (PR #35)

- Audited the native source-to-hall path: gathered logs/stone/sand/dirt
  already pin one resident ID, but finished wheat could previously be
  delivered by ANY living Farmer at the hall, and crafted bread by ANY
  Baker. These were concrete identity/custody gaps.
- Added backward-compatible Settlement.wheat_carrier_id and
  bread_carrier_id. A finished real eight-crop harvest now commits the
  exact UUID-bound Farmer's resident ID in the SAME JSON save as its
  existing crop receipt/transfer tally. Actual real bakery crafting
  records the UUID-linked Baker who stood at an intact workshop.
- Native routes and delivery decisions now require the ORIGINAL worker's
  loaded UUID, correct job and return near the hall; another farmer or
  baker cannot claim their coworker's receipt.
- Worker staffing pins live pending Farmer/Baker couriers across job
  recalculation and restart. Unloaded/dead workers remain unresolved
  rather than being silently replaced. Transit with no saved carrier
  remains loadable for backward compatibility but is never auto-credited.
- Added OP-only /civ retirecargo NAME wheat|bread confirm near the actual
  village for explicit WRITE-OFF of unbound legacy transit; only if no
  external warehouse transaction is unresolved. Never creates items,
  rebinds a stranger, or retires a bound resident's cargo.
- Physical supply requests no longer count unverifiable unbound wheat or
  bread toward reliable inbound stock. Exposed carrier identities and
  unbound status in /civ logistics. A missing/unloaded public barrel now
  suspends sand/dirt pit demand instead of assuming empty storage.
- Added five Rust domain regression cases, twelve Python source
  contracts and E2E test scenarios 379-392.
- **No builds or executable tests run**, per user's source-only
  development request. Actual native entity navigation, item counts,
  Windows builds, Rust compile and save/crash E2E are still unverified.
- No true native held-stack rendering: carrying is a saved receipt and
  observed UUID movement, not an item physically equipped in a slot.
  World JSON and Minecraft native saves are not a single atomic commit.
- Work stayed on Draft PR #35. main and GitHub Actions unchanged.


## 2026-11-09 — receipt-backed NPC cargo portraits (PR #35)

- Inspected existing Horizon adapter. A native villager navigation API and
  a safe item-ICON widget in the existing /dialog show UI are available.
  No independently verified native equipment API suitable for safely
  attaching, recovering and de-duplicating physical held stacks was
  found. This milestone deliberately does NOT fake held items, mutate
  actual mob equipment, spawn display entities or create loot drops.
- Added pure `resident_verified_cargo(village, resident)`: a verified,
  alive, UUID-bound worker's in-transit cargo only. Native block gathers
  need Issued + post-edit acknowledgement + source pickup witness +
  unblocked exact worker binding. Farmer wheat and baker bread need
  their already-persisted same-worker carrier ID and nonzero transit.
- Reused the existing native item portrait widget in `build_dialog` to
  show the actual item ICON for a verified in-transit receipt, plus
  its exact quantity, worker and a clearly labelled receipt explanation.
  Other residents and those with unbound/blocked/unwitnessed receipts
  keep ordinary job portraits, not fake goods.
- Fixed `needs_real_bread` to exclude unbound old bakery transit from
  available bread so orphaned legacy loaves do not hide player supply
  shortages. Updated existing Rust backpressure expectations.
- Added five Rust domain/dialog regressions and twelve authored Python
  static contracts; TEST_PLAN now includes cases 393-404.
- Source-only GitHub commits, Draft PR #35; **no Cargo, Python tests,
  native build or Minecraft gameplay was executed**.
- Future genuine in-world equipped cargo needs a validated Horizon
  entity-equipment interface, item-loss/restart reconciliation and
  no-drop/no-duplication design before enabling it.


## 2026-11-09 — off-shift delivery and loaded navigation targets (PR #35)

- Found an actual scheduler deadlock: even with a verified resource
  pickup, Dawn/Evening meal routing preempted the return route.
  At Night, the gather and Farmer/Baker delivery executors were also
  gated by the work-shift test, leaving carried receipts stranded.
- In `guide_actors`, an actor with an existing, valid, UUID-bound
  `resident_verified_cargo` gets the receiving-area route BEFORE
  meal/home/work targets. Ordinary farming, mining, gathering and
  bread production still follow their work shift.
- Gather reconciliation/return of an ALREADY Issued saved block
  receipt now runs at any phase; its shift gate was moved to the
  creation of NEW native block edits. Returning wheat and crafted
  bread are similarly processed off-shift by the original saved
  Farmer/Baker identity. No time-based resource credit or teleport.
- `loaded_civic_receiving_goal` checks a bounded collection of
  alternative native approaches near the hall, read-only. Each
  destination needs loaded coordinates, stable native ground and
  two air blocks. If all are obstructed/unloaded the route is not
  issued. It cannot prove an entire path is traversable.
- Added a separate read-only `/civ routes NAME` command showing
  verified in-transit goods, exact UUID-bound loaded worker status,
  observed receipt proximity, approximate direct-line distance and
  missing/obstructed hall approaches. No movement timeout, save
  mutation, forced chunk load, route success assertion or inventory
  changes.
- Updated existing pit/meal source contract for the new courier
  priority and authored twelve additional Python static contracts;
  TEST_PLAN now runs through 417.
- This change is SOURCE-ONLY; **Rust/Cargo, Python tests, Horizon
  native builds and physical Minecraft E2E were NOT run**, following
  user's instruction. PR #35 remains Draft and main untouched.
