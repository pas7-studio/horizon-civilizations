# Unreleased: 3D native worker proximity and real work-stand enforcement

Current branch: `feat/three-dimensional-native-worker-guards-v0115`, stacked
on Draft PR #27. GitHub Actions stay OFF; `main` remains unchanged.
**Not compiled or tested in a real Minecraft world.** Source manifest stays
at 0.1.14 pending native build evidence.

- [x] Pure finite-checked `native_worker_near` checks X, Y and Z; workers
  above/below the hall can no longer trigger wood/stone/wheat deliveries,
  bakery production or physical construction from the same X/Z point.
- [x] `native_worker_reaches_block` compares 3D eye position with actual
  block center, 4.5-block reach, before any mining command or Issued save.
  The NPC must also be within 2.5 horizontal / 2 vertical blocks of the
  **saved** natural source work stand.
- [x] Native source work stand gets a fresh loaded observation immediately
  before issuing: feet air, head air, real walkable floor. Any new wall,
  unobserved chunk or missing ground blocks the operation with no credit.
- [x] Live actor binding rejects residents >48 Y blocks away from their
  village even if X/Z matches. Navigation wakes up for vertical mismatch.
- [x] Applied 3D guard to the real native builders, farmer, baker,
  warehouse steward, wheat and mined-material receipts.
- [x] Three Rust numerical regression cases and 11 Python source contract
  tests **written** (not executed); source consistency checks only.
- [ ] Full Rust/Python tests, native Horizon mixin/client compile and
  first Minecraft gameplay E2E STILL REQUIRED. No claim of playable build.
- [ ] Height-aware 3D proximity is not line-of-sight/pathfinder proof.
  Native replay safety from prior PRs remains essential.
- [ ] True item-rendered couriers, saplings/replanting and real lumberyard/
  quarry buildings remain future work.

Refer to TEST_PLAN 212–222 for a disposable-world vertical exploit matrix.

---

# Unreleased source: short-oak stem harvesting with per-log receipts

Branch `feat/verified-oak-stem-harvest-v0115`, stacked after Draft PR #26.
**No new playable release.** Manifest is still 0.1.14, GitHub Actions are OFF,
and these changes have not passed native compile or Minecraft E2E.

- [x] Verified naturally soil-rooted, leaf-bearing **3–4-log oak stems** now
  produce a persisted multi-segment job, not merely one removed root block.
  Tall/uncertain trees are deliberately skipped; no magical reach/scaffolding.
- [x] Each log is separately conditional-edited with write-ahead Issued state,
  later verified as air and credited **once** to the physical delivery queue
  only after a UUID-bound lumberjack has returned to the town hall.
  A following segment is created as a fresh UNISSUED gather only after
  that confirmation; no multi-log batch credit.
- [x] Before each upper segment, re-read the entire stem: previous native
  blocks must be air, remaining blocks must be actual oak logs, leaf crown
  must remain and no block may be unloaded. Revalidate purchased land.
- [x] On planning save failure, roll back BOTH gather_job and oak_stem using
  a full original world snapshot. On uncertain Issued crash/refusal,
  quarantine the current segment and remaining stem; OP abandonment
  destroys the task with ZERO credit. Legacy single-log saves migrate
  through serde default without minting any continuation.
- [x] /civ diagnose and /civ gathercheck report stem segment N / total.
- [x] Eight Rust domain regression cases and nine Python source contracts
  authored; adjusted prior diagnosis/forestry source tests for new routing.
- [ ] Cargo/rustfmt/Clippy/full Python/native pinned Horizon/game E2E
  unverified. No toolchain/network from working container.
- [ ] Sapling collection/consumption, actual planting and growth,
  fully visible lumberyard and quarry structures, and item-rendered
  haulers are still NOT implemented. No free tree regeneration.

Follow disposable-world TEST_PLAN cases 200–211. Do not merge to main.

---

# Unreleased native forestry & resource-journal hardening

Branch `feat/natural-forestry-gather-journal-v0115`, stacked after Draft PR #25.
No verified release or compiled client: manifest stays **0.1.14**, `main`
unchanged, GitHub Actions remain deliberately disabled.

- [x] Lumberjack now selects a loaded, soil-rooted, vertically continuous
  native oak trunk with a leaf-bearing upper crown (rather than a random log
  beside a leaf). Only the lowest rooted log is a safe first extraction;
  **whole-tree felling and replanting are NOT implemented yet**.
- [x] Before mining/harvesting, native adapter rechecks source provenance
  and current town, player-parcel, farm, bakery and public-building protection.
  A player buying a plot after the plan blocks subsequent NPC destruction.
- [x] Physical gather uses write-ahead `Issued` persistence **before** the
  conditional native Minecraft edit. If save fails, no edit occurs. An
  uncertain issued receipt is never auto-replayed or discarded.
- [x] OP-only read-only `/civ gathercheck NAME` inspects the saved receipt
  and live source block. OP-only `/civ discardgather NAME intact|empty`
  abandons a blocked issued receipt only after matching freshly observed
  native block evidence. It always grants ZERO resources.
- [x] Saved gather records validate world bounds, worker-stand distance and
  mining radius. Added 9 Rust regression tests and 9 Python source contracts,
  **authored only**.
- [ ] Rust/Python execution, pinned Horizon native compile and actual
  Minecraft NPC E2E have **not run**. Prior static tests are not compiler
  evidence. Do not merge or label playable.
- [ ] A proper physical lumberyard, tree replanting, quarry building,
  multi-log tree felling, hand-held courier items and paths remain incomplete.

See TEST_PLAN 189–199 and PLAYTEST for the explicit acceptance gate.

---

# Current unreleased source: first-play settlement diagnostics

Branch `feat/first-play-diagnostics-v0115`, stacked on Draft PR #24 and all earlier
physical hall/bakery/warehouse work. The package remains **0.1.14 source-only**:
there is no successful native build or real gameplay proof for this branch.
GitHub Actions remain deliberately disabled.

- [x] New read-only `/civ diagnose NAME` for ordinary citizens/OPs, with
  live UUID-bound actor count and loaded jobs, actual native hall barrel
  availability, workshop status, current worker shift and ordered potential
  blockers. Includes aggregate bakery + hut real-resource requirements.
- [x] No new world mutation path, saved state, external item transfer or
  worldgen side effects introduced by this diagnostic.
- [x] Bounded real-bread restocking: baker stops requesting wheat or crafting
  once native barrel bread plus confirmed pending delivery reaches two loaves
  per living resident (max 64). Unavailable barrels pause production; full
  bread buffer does not block unrelated warehouse deposits.
- [x] Three Rust domain tests and eleven Python source contracts written;
  executable and native gameplay verification still pending.
- [ ] Local Rust/Python suites, pinned native Horizon compilation, Windows
  executable and disposable game E2E required. Use latest branch; an old
  `main` executable does NOT include this diagnostic or warehouse changes.
- [ ] Physical lumberyard/quarry and courier transport remain future work,
  not implemented by this PR.

See TEST_PLAN 175–181 and PLAYTEST quickstart.

---
# Unreleased source: operator-assisted warehouse recovery and accurate hall signs

Branch `feat/operator-warehouse-recovery-v0115` (stacked on PR #23).
No new verified release: package still **0.1.14**, GitHub Actions remain OFF.

- [x] Added OP-only read-only `/civ barrelcheck <village>`. Shows native
  physical barrel item count next to saved `before`, `expected_after`,
  direction, phase and source position. Does not change items or save state.
- [x] Added OP-only `/civ resolvebarrel <village> cancel <observed_count>`
  for **Planned only** with unchanged actual native count. It cannot cancel
  Issued or Blocked receipts, because they may have changed Minecraft items.
- [x] Added OP-only `/civ resolvebarrel <village> confirm <observed_count>`
  for **Blocked only** when freshly observed native after-count equals the
  saved expected value. Reuses the one-time domain receipt reconciliation,
  never repeats Minecraft item insertion/extraction. An accounting failure
  restores the blocked journal. A market order always blocks recovery.
- [x] Native signs now use the same combined construction-demand function as
  actual village material gathering and operator warehouse diagnostics.
  When any native journal is unresolved, they display `STOCK CHECK` and
  `PENDING TX` instead of unverified stock shortfalls.
- [x] Four Rust domain regression cases and six Python source contracts
  **written**; game/native verification and Rust/Python execution NOT RUN.
- [ ] Manual comparison of logs, official Minecraft barrels and save backup
  is still mandatory; a matching item count alone cannot prove item identity
  or cross-file crash atomicity. Never auto-confirm or auto-refund.
- [ ] Native Horizon build and real game-world tests before enabling normal
  multiplayer usage or calling the mod playable.

See TEST_PLAN 167–174 for a disposable-world verification matrix.

---

# Current unreleased source stabilization — physical logistics hardening

Branch `feat/physical-logistics-hardening-v0115`, stacked on Draft PR #22 (bakery).
No new package release is claimed: manifest remains **0.1.14** until a native
compile and in-game checks pass. GitHub Actions stay **disabled**.

- [x] Corrected a Rust validation scope error: bakery barrel transfer checks
  now run inside `if let Some(t) = &village.barrel_transfer`. The source
  error had slipped past string-based static assertions.
- [x] Resource planning uses `physical_construction_shortfall`: independently
  subtract bakery and hut escrow from each project's full bill, then count
  actual native hall barrel stock and the already-confirmed steward delivery
  queue once. A simultaneous bakery and hut need **44 logs / 58 cobble**
  before subtracting stock/escrows. Never fund construction from abstract stock.
- [x] Any unresolved native barrel transfer or market purchase pauses new
  physical gathering demand; unloaded barrels also stop planning. Blocked
  bakery construction no longer orders more materials from the barrels.
- [x] `/civ warehouse NAME` now displays combined pending construction
  deficit or **unknown** while the journal is unsettled/containers unloaded,
  instead of implying a single hut deficit is authoritative.
- [x] Added six Rust domain regression tests and five Python source
  contracts; all tests are **authored, not executed** here.
- [ ] **Release blockers:** local `cargo fmt --all --check`, `cargo test`,
  Python unittest, Clippy, pinned Horizon native build, Windows executable,
  and real in-game NPC/stock/crash tests. No new GitHub Actions enabled.
- [ ] Manual quarantine recovery and physical lumber yard/quarry/couriers
  remain unimplemented; don't present this as full gameplay acceptance.

See TEST_PLAN cases 159–166 and IMPLEMENTATION_LOG for regression details.

---

# Current source branch — v0.1.14 physical bakery and protected public economy

Branch `feat/physical-bakery-economy-v0114`, stacked on #21.
**Current verification: source-only.** The primary `main` binary remains
older; no successful local Rust or native game compile has been run here.
GitHub Actions are disabled by the user and must stay OFF.

- [x] Self-developing villages reserve one actual loaded, unclaimed 5×5
  bakery plot when their completed town hall and UUID-bound Baker/Builder
  are present. Bakery uses its own construction state and blueprint:
  native crafting table, smoker, walls, doorway and cobblestone foundation.
- [x] Bakery construction requires 24 real logs and 33 real cobblestone,
  physically withdrawn from the public hall barrels into bakery-specific
  escrow. The abstract daily stock cannot substitute for these.
- [x] Baker walks to actual hall barrels, extracts **three real wheat**,
  works only near the intact loaded bakery workbench, consumes three
  wheat for **one bread receipt** and returns to hall. Mayor/steward
  deposits real bread in native barrel slots, from which it is tradable.
- [x] Fixed old endless gathering loop: source material shortages now
  count actual native barrel inventory + inbound deliveries + relevant
  physical building escrow. Unloaded barrels fail closed, not as zero.
- [x] Native API 334 blocks hopper transfers into/out of public hall
  barrels. Native API 14 protects player-block-breaking of recorded
  public buildings and other people's private purchased plots; owners
  retain parcel editing and OP bypass. 9 native injections total.
- [x] /civ bakery shows real workshop progress, on-worker materials,
  native integrity and completed recipe batches.
- [ ] Native Horizon compile, full Rust/Python tests, gameplay E2E,
  exact hopper/item-stack conservation under crash, real pathfinding
  and server chunk-save reconciliation: **NOT EXECUTED**.
- [ ] Vanilla fire/explosion, mod automation, player-placed trees on
  unclaimed ground and cross-file crash-atomicity remain unsafe/unproven.
  Full GUI and safe manual recovery workflow are not implemented.
- [ ] More production structures, roads and NPC couriers with visible
  hand-held items, spontaneous village worldgen, village-family economy
  and multi-village trade remain future work.

See TEST_PLAN cases 139–158, PLAYTEST and GitHub PR.
Do not present this Draft as a compiled playable binary.

---

# Historical v0.1.13 source milestone (2026-11-07)

Branch: `feat/native-barrel-inventory-v0113`, stacked after physical town hall
and autonomous village commits. **Not compiled or tested in game.**

- The two actual town hall `minecraft:barrel` block inventories are now read
  and mutated using inspected pinned Horizon native API 48 (slots, extract,
  insert). Real native contents, not the old JSON ledger, determine available
  post-hall construction materials.
- The saved `warehouse` map is now an **incoming delivery queue**, not a
  second copy of what's inside a real barrel. A separate build escrow holds
  goods physically removed by the builder. Saved journals reconcile all
  single movement batches without blind reissue.
- NPC lumberjacks/miners and farmers return to the hall before delivery
  credit; the steward transfers received resources into real vanilla barrel
  slots, and the builder walks to the hall before reserving real items.
- Post-hall trade purchases read real barrel slots and precharge coins.
  A native API 233 block-container hook denies non-OP access to the
  village's two public storage barrels; private containers are untouched.
- No atomicity exists between world.json and Minecraft container/chunk
  persistence. Ambiguous transfers become blocked, never automatically
  repeated. No admin recovery automation is enabled.
- **NOT VERIFIED:** Rust compilation, Python tests, transformed pinned
  Horizon native build, in-game barrel permission hook, actual villager
  navigation, live container transfer, NBT signs, crash windows.
- **NOT COMPLETED:** universal player land anti-grief, courier animations,
  real production workshops/recipes, auto-founded villages, safe recovery UI
  for blocked transfers, player-facing physical delivery UI and full
  economy monetary balance.

Manual checks: [TEST_PLAN](TEST_PLAN.md) cases 120–138.
Actions disabled intentionally. Main binary does not have this feature.

---

# Current stacked feature source status — v0.1.12 (2026-11-07)

**Most recent source branch:**
`feat/physical-town-hall-warehouse-v0112`, stacked on the autonomous
NPC implementation branch. This fixes the design where simulated virtual
materials could make the village look productive without actually gathering,
storing or spending Minecraft-derived resources.

- Implemented in source: a native block-by-block central civic hall with a
  named mayor/warehouse steward, two barrel markers, two oak signs and a
  lectern. Hall construction has its own persistent blocked/cursor state,
  and must reserve 24 **actually received** logs and 25 cobblestone.
- Implemented in source: resource-backed physical warehouse ledger, fed by
  player inventory trades and carefully confirmed physical wheat/wood/stone
  harvests. Each physical hut reserves 20 logs +25 cobblestone. The old
  abstract daily stock cannot pay those real block bills.
- Implemented in source: two native sign data-update commands to display
  mayor/headcount and remaining material/housing demand. This depends
  on Minecraft NBT command support and is **NOT VERIFIED IN GAME**.
- Implemented in source: lumberjack/miner scan loaded natural sources,
  claim a safe NPC approach spot, issue a conditional block change and
  confirm observed deletion before a single warehouse credit. Player
  land/known structures are excluded. Block provenance is imperfect
  outside registered claims.
- **NOT IMPLEMENTED**: true transactionally synced vanilla barrel inventory;
  a player cannot put items directly into a barrel and expect the mod
  ledger to update, or retrieve stored stock by opening it. This remains
  a high-priority engine integration.
- **NOT TESTED**: current code with Rust compiler, full native Horizon
  transform/build, Windows game launch, physical signs, NPC pathfinding,
  mining/tree harvesting, container contents, save/restart or gameplay E2E.
  GitHub Actions are intentionally disabled; local only.

See [PLAYTEST](PLAYTEST.md) and [TEST_PLAN](TEST_PLAN.md) cases 103–119.
No claim of a released playable milestone until actual local acceptance.

---

# Current feature-branch status — v0.1.11 (2026-11-07)

**Correction to the original prototype:** the initial and main implementations
did not create a living civilization automatically. The **latest stacked
source-only feature branch** is
`feat/autonomous-physical-village-v0111`; it has not been compiled or
playtested, so it cannot yet be described as working in an actual game.

- Source authored: villages default to autonomy ON, physically spawn and
  UUID-bind their own residents when safe chunks are loaded, convert
  previously paid abstract houses to real work projects, and select safe
  physical wheat fields and starter huts without OP site commands.
- Source authored: 5x5 purchased player parcels, commissionable private huts,
  sale of existing fully verified public huts, civic treasury and public
  housing-demand recalculation. These are **mod-level** protection rules,
  not vanilla multiplayer land protection.
- Native Rust executable, built-world activity, pathfinding, block edits,
  farm maturity, nighttime behavior, restart and crash recovery:
  **ALL UNVERIFIED for v0.1.11**.
- GitHub Actions are intentionally disabled by user request. No runs have
  been scheduled for this release.
- The default `main` branch is an older version, and source changes in
  the stacked Draft PRs are not visible in its existing Windows artifact.

Go to [PLAYTEST](PLAYTEST.md) for the
`feat/autonomous-physical-village-v0111` local Windows build, then
cases 85–102 in [TEST_PLAN](TEST_PLAN.md). Never use an irreplaceable save
for an unverified native mod.

---

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

## Fail-closed world switch hardening (feature branch)

The outgoing world must persist successfully before replacing the singleton with an incoming world. A failed save retains existing state in memory and reports a blocking problem. Shutdown write failure also keeps the state resident for retry while the process exists. Stale world.json.swap is examined before any backup/tmp mutation. This protects against an avoidable lost-state path; it cannot prevent data loss if the operating system/process terminates before disk errors are fixed. Native build/gameplay E2E still required.

## Host day counter migration (v0.1.6 feature branch)

Authoritative daily simulation now follows the persistent Horizon `day_time` clock. A saved `last_host_day` anchor protects against repeat application across reloads and backwards clock changes; old worlds initialize the anchor without backpay. Large clock jumps are limited to 30 simulation days to prevent runaway stock. Manual `/civ day` remains explicit. Native compile, real world time progression and sleep E2E pending.
