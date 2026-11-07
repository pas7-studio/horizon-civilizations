## Latest stacked Draft: partial NPC spawn recovery (source-only)

Branch: `feat/guarded-partial-villager-recovery-v0115`, based on
PR #33. **Not yet compiled or gameplay-verified.** `main`
unchanged and GitHub Actions disabled.

The first village NPC batch is persisted before spawning; if native
creation crashes halfway, never rerun all ten. A new OP-only
`/civ recoveractor NAME ID attempt` can attempt **one** missing
resident after `/civ reconcile NAME`, inspection of loaded
original villagers, a fully loaded 96-block scan and a safe
natural spot. A durable per-resident one-shot reservation is saved
before native spawning, with exact UUID binding and no blind retry.
`/civ roster NAME` shows IDs and pending recovery status.

On Windows with Rust 1.95, Python 3.11+ and Horizon prerequisites:

```powershell
git fetch origin
git switch feat/guarded-partial-villager-recovery-v0115
py -3 tools/first_play_gate.py --expected-branch feat/guarded-partial-villager-recovery-v0115 --build-client
```

This is NOT gameplay E2E. See [PLAYTEST](docs/PLAYTEST.md)
and [TEST_PLAN](docs/TEST_PLAN.md) for the native recovery scenario.

## Current Draft — release gate verifies the actual native client artifact

Newest stacked branch: `fix/first-play-binary-evidence-v0115`.
Compared to #32, `tools/first_play_gate.py` now **rejects a native
command returning exit code 0 if no valid Horizon client executable
was produced**. It validates the final JSON record emitted by pinned
`native_build.py`, the expected Horizon commit, `cargo_check=PASS`,
the exact modded output path, then checks that the reported executable
really exists at the expected native build location. It captures file
size and SHA-256 into report.json/report.md for evidence. Missing,
wrong-path or tiny binaries fail the gate instead of reporting
`READY_FOR_GAMEPLAY_E2E`.

The release-gate Python mock suite was actually executed on Linux:
**10/10 PASS**. These are harness tests with fake cargo and native
binary; **real Rust/native build and Minecraft gameplay remain
NOT EXECUTED**. Main untouched, Actions OFF.

```powershell
git fetch origin
git switch fix/first-play-binary-evidence-v0115
py -3 tools/first_play_gate.py --build-client
```

See [first-play evidence](docs/PLAYTEST.md) and
[acceptance plan](docs/TEST_PLAN.md).

## Current next gate: reproducible local build evidence (Draft)

The latest stacked source branch is `chore/first-play-native-validation-v0115`.
It includes all prior village, physical resource and crash-safe receipt code.
**No native Horizon compilation or real Minecraft gameplay pass has been
demonstrated for this stack.** Main remains unchanged and GitHub Actions OFF.

On a networked Windows PC with Rust 1.95, Python 3.11+, Git and compatible
Horizon/Minecraft Java resources:

```powershell
git fetch origin
git switch chore/first-play-native-validation-v0115
py -3 tools/first_play_gate.py --dry-run --build-client
py -3 tools/first_play_gate.py --build-client
```

The non-mutating gate records exact Git SHA, clean/dirty checkout, tool
versions, every first failing source/native build step and its log into
`target/first-play-gate/<UTC timestamp>/report.md` and `report.json`.
It refuses the wrong branch, Rust version, or modified source; the optional
`--allow-dirty` mode is diagnostic only, never a passing release gate.
It does not enable GitHub Actions or launch the game. Even
`READY_FOR_GAMEPLAY_E2E` means **client compiled; Minecraft still untested**.
See [PLAYTEST](docs/PLAYTEST.md) and [TEST_PLAN](docs/TEST_PLAN.md).

# Horizon Civilizations (prototype source)

**Goal:** a Millenaire-like civilization system for [Horizon](https://github.com/horizon-rust/horizon):
living settlements, occupations, economy, progression, titles, political conflict.

This is an **independent native mod**, not a fork of Horizon, and contains no
Millenaire code or Minecraft assets. Original project code is MIT-licensed.

> Status: **source prototype, not gameplay-verified**. See [verification status](docs/STATUS.md).

> **Newest stacked Draft, not a release (2026-11-08):**
> This source additionally rotates between loaded village resource
> jobs and requires a saved **native command success + observed air**
> receipt BEFORE any same-worker pickup witness or hall payment.
> An uncertain Issued operation after restart is quarantined, never
> replayed or paid, while legacy witnessed saves still load.
>
> `fix/gather-crash-proof-fair-queue-v0115` adds identity-bound wood/stone
> harvesting with a persisted worker observation at the removed block,
> followed by a separate worker return to the native hall. It includes
> all prior experimental physical hall, bakery, native-barrel and
> autonomous settlement changes. **This branch has not been compiled
> or tested in Minecraft**; main/older successful CI runs do not prove
> the latest state. GitHub Actions remain intentionally disabled.
> See [STATUS](docs/STATUS.md), [PLAYTEST](docs/PLAYTEST.md) and
> [TEST_PLAN](docs/TEST_PLAN.md) for evidence and first-game steps.
>
> To validate this exact branch on a networked Windows machine with
> Rust 1.95 and Python 3.11+:
> `git switch fix/gather-crash-proof-fair-queue-v0115`, then
> `.\scripts\validate-local.ps1 -Format` and
> `.\scripts\validate-local.ps1 -BuildClient`.
> These are local build checks, **not a gameplay pass**.

## Available in the current source

- Found settlements with a center and ten persistent residents (ten occupations)
- Spawn ten uniquely labelled physical *vanilla* villagers at a settlement center on request
- Reattach to loaded named NPCs after reopening the world; send them toward job-specific walking destinations (experimental)
- **Experimental visible construction:** an operator selects an unobstructed 5x5 natural site; an attached builder NPC must arrive before the mod places an oak-and-cobblestone hut, one verified block at a time
- Construction cursor is persisted; after a crash, previously written blocks are checked against the world before progressing (no blind replacement of unknown blocks)
- Autonomous daily abstract production (farming, wood, stone, bread)
- Population food consumption, morale, abstract housing construction
- Settlement info, player allegiance, reputation, promotion and basic coin trading
- Simple political state: bilateral hostility/peace and abstract war attrition
- Save to `<world>/horizon-civilizations/world.json`, `.bak` backup, fail-closed corrupted or inconsistent save
- Server hooks and independent Rust domain unit tests

On the original main baseline, physical mining/warehouse credit is still
incomplete. Newer **stacked Draft branches** introduce experimental real-block
harvest receipts, native barrel transfers and bakery production; none have
current native-game E2E proof. Still unavailable: real NPC-held Minecraft
ItemStack pickup, robust native pathfinding/LOS, full lumberyard/quarry,
sapling regeneration, dynasties, physical war and a proven crash-atomic
transaction across world blocks and mod JSON.
The newest stacked branch **automatically** starts native villager spawning,
physical wheat-field construction and starter-hut builds after a settlement
exists; the older main branch does not contain this. Player-owned lots are
excluded from all NPC-initiated plot selection. Do not use this build on an important world until native build and E2E pass.

## Required environment

- Horizon launcher and an account owning Minecraft Java Edition
- Rust toolchain 1.95 and Python 3.11+ for local source testing and packaging
- Horizon native build support (`.hnmod`), with a compatible pinned source checkout

On a development PC with Rust installed:

```sh
cargo fmt --all
cargo fmt --all --check
cargo clippy --all-targets
cargo test
python3 tools/package.py
python3 tools/verify_static.py
```

Packaging produces `target/civilizations-0.1.14.hnmod` (source-only; the
launcher compiles it). The template is based on the **native hook-point**
pattern used by the official Horizon SMP mod. The compatibility pin is in
`compatibility.json`; the source was inspected but not compiled in this workspace.

To compile the actual native-modded Horizon binary **on a networked build machine** (may fetch a large upstream repository and dependencies):

```sh
python3 tools/native_build.py --test
```

To also compile a real game executable (more expensive, not automatically a gameplay pass), run `python3 tools/native_build.py --test --build-client`. The binary is printed in `native_client_binary`; provide compatible Minecraft resources separately and use a disposable world. See [playtest instructions](docs/PLAYTEST.md).

This checks out isolated pinned copies of Horizon and horizon-mixins under `target/native-check/`, runs mixin plan/apply and `cargo check` for Horizon, and runs the embedded Rust domain tests. It never modifies an existing game checkout. The manual GitHub Actions workflow `Horizon native compile gate` performs the same check. Native compilation is NOT equivalent to a playable, real-world E2E test.

### Optional Windows test-instance installer

On Windows 11, after Rust and Python are installed, install the source package only
into an **explicit** disposable Horizon instance directory:

```powershell
.\scripts\install-local.ps1 -InstancePath 'C:\path\to\Horizon\test-instance'
```

This runs the local Rust tests and copies the `.hnmod`. The launcher still needs
to compile/trust the native mod before the game can run.

## Installation / first E2E (after native CI passes)

1. Install the official Horizon launcher from <https://playhorizon.dev/>.
2. Create a separate test instance and back up the world.
3. Put `civilizations-0.1.14.hnmod` in that instance's `horizon-native-mods/`
   directory. The native build requires the launcher's explicit trust action.
4. Let the launcher **build** the modded game and review build logs. If you see
   `target not found`, hook schema incompatibility, or Rust errors, stop: the
   current pin/adapter has not passed compatibility.
5. Start a test world as operator. `/civ found Ashford` creates state;
   `/civ actors Ashford` attempts ten named Minecraft villagers near the center.
   `/civ roster Ashford` shows the linked NPCs. Walk around the settlement to observe profession-specific movement.
   Reopen the world and check `/civ roster Ashford` again before attempting any resummon.
6. Try `/civ info`, `/civ list`, `/civ day 1`, `/civ title`.
   Stand near the village before spawning actors. The day advancement creates an
   abstract house pending visualization when stock and builders are available.
7. **New v0.1.2 test:** select a flat 5x5 grass/dirt/stone patch, with four air
   layers above it, 12..80 blocks away from Ashford center. Stand at its center
   and run `/civ construct Ashford`, then `/civ construction Ashford`.
   Stay nearby and observe the named builder approaching the site. The hut
   should appear gradually and the command should count verified blocks.
   Save/reopen part-way through construction; the existing blocks must NOT be
   overwritten, duplicated, or skipped. Do this only in a disposable world.
8. Try `/civ sell wheat 4` while holding four plain wheat; use `/civ buy bread 1`.
9. Save and quit, restart, `/civ info`; world records must persist.
10. Create a second settlement >=192 blocks away, `/civ war Ashford Brightwell`,
   `/civ day 4`, `/civ history`, `/civ peace Ashford Brightwell`.
11. Report results along with the installed Horizon commit and logs. See
    [verification status](docs/STATUS.md) and [test plan](docs/TEST_PLAN.md).

### Commands

| Command | Who | Purpose |
| --- | --- | --- |
| `/civ` or `/civ help` | All | Available actions |
| `/civ list`, `/civ info [name]` | All | Inspect settlements |
| `/civ roster NAME` | All | List professions and nearby recognized villagers |
| `/civ schedule NAME`, `/civ homes NAME` | All | Day/night shift and completed-hut physical allocations |
| `/civ council NAME`, `/civ forecast NAME` | All | Read-only needs and three-day abstract economic outlook |
| `/civ survey NAME` | All | Propose up to five loaded, unoccupied building sites; never reserves land |
| `/civ development NAME` | All | Inspect default-on NPC population, real farm/building activity and blockers |
| `/civ hall NAME` | All | Inspect physical town hall, mayor, construction progress and real material needs |
| `/civ warehouse NAME` | All | Inspect resource-backed village warehouse inventory, never abstract stock |
| `/civ bakery NAME` | All | Inspect physical workshop, verified recipe, missing real materials and bread deliveries |
| `/civ plots NAME` | All | See your bought land and privately owned structures |
| `/civ buyplot NAME` | Joined players | Purchase a clear 5x5 village parcel for 10 civic coins |
| `/civ buyhouse NAME` | Joined players | Purchase an intact finished public hut for 35 civic coins |
| `/civ commission NAME` | Joined plot owners | Pay 15 civic coins + village materials for an NPC-built private hut |
| `/civ construction NAME`, `/civ farming NAME`, `/civ harvests NAME` | All | Inspect physical build/farm progress and visual-only harvest receipts |
| `/civ join NAME` | All | Join a settlement |
| `/civ title`, `/civ promote` | All | View rank/coins; promote if eligible |
| `/civ sell GOOD N`, `/civ buy GOOD N` | All | Trade with joined settlement |
| `/civ history` | All | Recent world events |
| `/civ found NAME` | Operators lvl 2 | Create settlement at current position |
| `/civ actors NAME` | Operators lvl 2 | Spawn named vanilla villager actors once |
| `/civ construct NAME` | Operators lvl 2 | Approve a safe natural site at your feet for a physical 5x5 hut; requires a pending simulated house |
| `/civ approvehouse NAME SLOT Y` | Operators lvl 2 | Confirm one numbered housing proposal near the selected plot; real blocks rechecked |
| `/civ autonomy NAME on/off` | Operators lvl 2 | Optional pause/resume for new NPC site planning (ON by default) |
| `/civ farm NAME`, `/civ recoverfarm NAME` | Operators lvl 2 | Approve visual crop patch or preflight recovery |
| `/civ day [1..30]` | Operators lvl 2 | Advance abstract simulation manually |
| `/civ war A B`, `/civ peace A B` | Operators lvl 2 | Strategic-only conflict state |

Supported goods: `wheat`, `bread`, `wood` (= oak logs), `stone` (= cobblestone), `iron` (= iron ingots).

## Architecture

- `mixins/civilizations/core.rs` - pure Rust domain, built standalone via Cargo
- `mixins/civilizations/glue.rs` - game adapter, persistence, command handling
- `mixins/civilizations/{register,command,tick,lifecycle}.rs` - HEAD hook bindings
- `mixins/civilizations/horizon-mixin.toml` - native mixin manifest
- `tools/package.py` - deterministic source-only package for launcher
- `docs/ROADMAP.md` - staged acceptance and invariants

## Upstream strategy

We do **not** fork Horizon. We pin one commit (`compatibility.json`), validate the
mod against new upstream builds in a separate test instance, then repin only
after native build and E2E pass. Never use `--allow-version-mismatch` on live
worlds as the normal upgrade path.

## Important current limitations

NPC movement is **experimental**. Only the manually approved starter hut can place actual blocks;
NPC farmers can work an experimental physical wheat plot, but its harvest never
credits stock; physical mining and item transport are not implemented. Normal economy and war remain abstract.
Construction stops when a plot becomes obstructed, but there is no recovery UI for a blocked job yet. Villager identity is bound to persistent entity UUIDs in saved data, but actual
reattachment across game reloads is still awaiting in-world verification. Selling/buying inventory items and saving mod-world JSON are not one atomic engine operation; use only a disposable test world until transaction recovery is implemented and tested.

## Legal

Horizon and this new implementation use MIT for their original code; third-party
dependencies retain their licenses. Millenaire is a design reference only here.
No copied GPL Java code, assets, Minecraft resources or Millenaire cultural
content are included. Minecraft is a trademark of Mojang/Microsoft and a
legitimate Java Edition account is required by Horizon.

### Verification evidence

The pinned native Horizon compile gate has passed ([GitHub run 42375553368](https://github.com/pas7-studio/horizon-civilizations/actions/runs/42375553368)); Core Rust/Clippy/package checks passed ([run 42375993049](https://github.com/pas7-studio/horizon-civilizations/actions/runs/42375993049)). These are **build/test checks only**. Neither run launches an interactive world, and physical construction/NPC behavior still requires the disposable-world scenarios in docs/TEST_PLAN.md. Follow-up changes to scheduling and crash-recovery must pass their own CI before merge.

## UUID-bound NPC identity (2026-11-07)

The mod now persists each spawned villager's actual Horizon entity UUID as a 32-character hex string. All movement and building checks use UUID lookup, not a visible name. Existing v0.1.2 worlds have no UUID fields; an operator must visit loaded original villagers and run `/civ reconcile Ashford` exactly once to bind unique, tagged legacy NPCs. Ambiguous names, unavailable UUIDs and collisions are skipped. No automatic respawn is attempted. The migration works only in a disposable test world until the actual in-game E2E passes.

The module is checked with `game.native_on("civilizations")` before world commands and simulation ticks; native addon disable stops further simulation/writes.

## First NPC dialogue (0.1.3 experimental)

After `/civ actors Ashford` has created and persistently UUID-bound the village actors, **right-click one of those villagers** to display their identity, job, home settlement, morale, and how to join or trade. Only a known UUID-bound civilization villager claims the interaction. Ordinary Minecraft villagers and unmatched actors retain their vanilla behavior. The response is currently a chat line, not a custom GUI or a quest engine. Requires in-game validation on a disposable world.

## First living wheat field (0.1.4 experimental)

In a disposable Horizon world, after `/civ found Ashford` and `/civ actors Ashford`, select a 3×3 empty patch of natural grass/dirt/stone, 12–80 blocks from the village center. Stand on the center block and run:

```text
/civ farm Ashford
/civ farming Ashford
```

A UUID-linked farmer approaches the plot. When the field's chunks are loaded, the mod **conditionally** creates eight farmland blocks and a center irrigation water source, plants eight wheat crops, waits for **natural age=7** growth, harvests by replacing mature wheat with air **without item drops**, and replants the eight crops. It saves the phase, cursor, and completed visual cycles after observing each block change.

The visual farm **does not produce warehouse wheat, item drops, or money**. Economic wheat production remains the separately simulated daily economy until there is a tested, durable block/JSON inventory transaction journal. The plot fails closed if player blocks replace its soil, water or crop; an operator must inspect the plot and save before any recovery. Currently one 3×3 farm per settlement, no automatic new farm or additional crops.

This is experimental and has not passed a real Minecraft-world playtest. The normal native build gate checks compilation, not game-world chunk persistence.

### Recovering an obstructed farm

If `/civ farming Ashford` shows `blocked=true`, remove/restore the offending block **manually**, keep the field fully loaded, and as an OP run `/civ recoverfarm Ashford`. The mod re-inspects the physical plot and refuses recovery while any unexpected block or unloaded chunk remains. This does not roll back player edits or create stock.

## Harvest receipts (v0.1.5)

The new /civ harvests Ashford command displays an increasing harvest sequence and the five latest audit entries. Every entry records the world day and eight crops observed as harvested. The latest 32 receipts are saved; historical count remains monotonic. A phase transition and its receipt are written together to world.json, so a duplicate phase callback cannot create an extra receipt during normal execution.

This is EVIDENCE ONLY. No player items, coins or settlement wheat are credited from visual harvesting. The native world/chunk store and mod JSON are not an atomic transaction; restored backups and rolled-back chunks need explicit reconciliation before stock credit can be enabled.

## Host-authoritative civilization clock (v0.1.6)

Automatic settlement day progression now observes Horizon's persistent world day counter, rather than counting 24,000 mod uptime ticks that restart when the game closes. The optional `last_host_day` field in `world.json` is backwards-compatible. When the mod first opens an older save, it records the current host day without manufacturing historical production; on later authentic host-day increases it advances the abstract economy exactly once per observed day. Backward time changes do not replay days.

To avoid runaway stock/house generation after a command jumps the world clock forward, a single synchronization processes at most 30 days; any larger time gap is intentionally skipped. `/civ day N` remains a separate operator simulation tool and does not change the underlying Horizon clock. This does not yet simulate autonomous work while no world is open. Verify save/quit/relaunch, `/time` commands and multiplayer in a disposable world.

## Saved-world playtest evidence

After the first Windows session, use the read-only script in [playtest evidence](docs/PLAYTEST_EVIDENCE.md)
to inspect UUID bindings, physical build/farm cursors and harvest receipts. This is not
a substitute for checking actual NPCs or blocks in the running game.

## Daily NPC routines (v0.1.7 experimental)

The persistent Horizon clock now drives four deterministic movement windows:
0–1999 dawn (commons), 2000–10999 daytime work, 11000–12999 evening
gathering, and 13000–23999 near-center nighttime regrouping. Builders
and farmers work at their real approved sites during the work window;
guards stay on watch throughout the day. Use `/civ schedule Ashford` to
inspect the phase and on-duty workforce. Physical farm/construction
block edits are explicitly disabled outside the daytime work window,
even if a worker remains standing near the plot.

This is **NPC navigation and work gating only**: no real bed ownership,
sleep animations, mine production, offscreen work or extra inventory
credits are claimed. No saved fields or new Horizon hooks were added.
v0.1.7 has not been compiled or tested in an actual Horizon game yet;
GitHub Actions were disabled by the user, and local QA is planned.

## Physical hut homes for NPCs (v0.1.8 experimental)

This source milestone builds on the v0.1.7 routines. Only **completed
physical 5x5 huts** (`visual_sites`) provide sleeping/regrouping destinations.
Each hut has three deterministic indoor standing lanes. Citizens are assigned
by sorted living resident IDs; adding a new completed hut fills the previously
unassigned roster without moving the initial residents. Assignment is
recalculated when living residents change, not serialized as another source of
truth. Abstract simulation houses and unfinished building projects provide
**zero** physical home slots.

During the night, eligible non-guard villagers navigate toward their indoor
position only if the actual already-loaded Minecraft floor, two-block doorway,
and standing space match the hut template. Huts with a damaged checked floor, doorway or indoor standing lane, or with
unloaded required blocks, fall back to the commons; the mod never loads
chunks or overwrites blocks for residential movement. Guards keep their
existing on-duty patrol destination.

Use `/civ homes Ashford` to see completed huts, physical capacity, assigned
citizens and residents without physical housing. Right-click a linked resident
to see their computed home assignment. **No actual bed block, sleeping,
door-opening, family structure, rent, house ownership or pathfinding
guarantee is implemented.** Real interior navigation, damaged-house
handling and world-save E2E remain untested. GitHub Actions are intentionally
disabled; see [local playtest](docs/PLAYTEST.md).

## Autonomous housing-site proposals (v0.1.9 experimental)

Civilizations now offers **safe, read-only village-initiated construction
suggestions**, based on how many living residents lack a completed physical hut
and how many abstract houses are ready to visualize. This is an early
autonomy/decision layer, not automatic block placement or land ownership.

When an abstract house is ready (typically after simulated builder production),
enter the village dimension and run `/civ survey Ashford`. The system checks a
bounded set of 20 deterministic ring candidates around the settlement (12–80
blocks from its center), scans up to five nearby Y levels at each candidate,
and lists at most five sites that are currently **loaded**, flat, natural and
empty. It avoids all recorded physical huts, active projects and farm plots in
the same dimension. No chunks are loaded and no blocks are edited.

An operator must physically visit the plot and explicitly reserve one:
`/civ approvehouse Ashford 1 70` (the numbers are the displayed slot and Y
coordinate). The server requires OP level 2, proximity within 24 horizontal
and 8 vertical blocks, and **rechecks the current world blocks plus settlement
collisions at approval time**. Approval saves exactly one construction job. A
UUID-linked builder later places blocks conditionally during its daytime shift
through the existing crash-reconciling physical construction pipeline.

The older `/civ construct Ashford` remains available for manually chosen
spots, but now shares the same full recorded-plot collision guard. No
territorial or player ownership claim is inferred from grass/dirt/stone: **OP
approval is the required boundary**. Proposals are not persisted, do not give
NPCs permission to clear land, and may change when terrain/settlements change.
If a survey finds nothing, prepare a safe area or use explicit manual siting;
nothing is reserved.

This version is source-only and **not compiled or playtested**. GitHub Actions
remain disabled as requested; use the local commands in
[PLAYTEST.md](docs/PLAYTEST.md) before merging the stacked drafts.

## Village council: needs, priorities and economy outlook (v0.1.10 experimental)

A village can now derive a **read-only** council priority board from its existing
population, food/bread stock, three-day abstract-economy outlook, active wars,
morale, durable UUID-linked residents, physical housing slots and future
abstract-building material requirements. It ranks up to five urgent needs
deterministically, with no random prompts, new save fields or autonomous
commands. Use:

- `/civ council Ashford` — see the three highest-ranked concerns, urgency
  from 0 to 100, actual evidence and suggested next steps.
- `/civ forecast Ashford` — inspect bread, wheat, morale, starvation,
  simulated houses and pending visible houses for each of the next **three
  hypothetical abstract days**.

The forecast calls the existing `Settlement.day()` economic method on a
**clone**, so it does not recalculate output using a different formula and
cannot spend items, start construction, spawn villagers or alter the current
world save. The forecast **excludes** separate world-level war attrition,
player trading, Minecraft inventory, physical harvest credits, client play
and time rewinds. It is a bounded scenario, not a guarantee of the future.

The council does not bypass building approvals, military diplomacy or player
inventory ownership. Its housing recommendation may direct an OP to
`/civ survey Ashford`; actual land modification still needs explicit
`/civ approvehouse` after real loaded-world block checks. Every previous
trade/harvest caveat continues to apply. No gameplay E2E was performed
for this version; GitHub Actions remain deliberately disabled.

### Local validation without GitHub Actions

On Windows 11, the repository includes a local-only PowerShell validator:

```powershell
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

The first command explicitly reformats local Rust sources and then runs fmt,
Clippy, Cargo tests, all Python tests, packaging and static checks. The second
also fetches the pinned Horizon source and builds a local Windows client.
Run these from the **v0.1.10 stacked feature branch** when ready. A normal
`validate-local.ps1` run checks formatting without changing source code;
all variants may write generated files under `target/`. None trigger or
reenable GitHub Actions. Even a successful local executable build does **not**
prove in-world gameplay safety.

## v0.1.11 — autonomous towns and player-owned land (source milestone)

The foundation of this mod is **villages developing independently**. Every
new village, and every older saved village without an explicit setting,
starts with `autonomy_enabled=true`. There is no mandatory OP construction
approval for building on **free, unclaimed, empty natural terrain**.

After an initial settlement is founded, the native level tick automatically:

1. Locates ten safe loaded, air-filled spawn points on natural ground near
   its center and **one-shot summons UUID-linked villager actors**, saving the
   spawn reservation before the first entity. Partial/failed spawning never
   retriggers a duplicate batch; inspect `/civ roster NAME`.
2. Converts already-paid abstract starter houses into a bounded physical
   build backlog **without minting new houses or materials**.
3. Starts a real 3×3 irrigated wheat field when linked farmers exist, and
   a real 5×5 cobblestone/oak hut whenever a linked builder, building budget
   and missing public housing exist. Each candidate is checked against
   loaded natural ground, air, *other village plots*, and purchased player
   parcels. It never force-loads chunks, clears a player's obstacles or
   teleports workers.
4. Uses the existing native worker/loaded-world day-shift tick to place
   expected Minecraft blocks one at a time. Farmers tend real mature crops;
   physical wheat **still does not credit the abstract warehouse**.

This is *not yet* automatic worldgen of entirely new towns or advanced
crafting industries. Current physical structures are starter huts plus
working wheat fields; mining, bakeries, markets, roads and hauling still need
actual native gameplay implementation.

### Player property and construction commissions

A joined player can buy a clear **5×5** parcel 12–80 blocks from a
village center. From the plot's center, while the natural land is still
clear, run:

```
/civ buyplot Ashford
/civ plots Ashford
/civ commission Ashford
/civ development Ashford
```

- A parcel costs **10 civic coins** and is permanently recorded by the
  authenticated player's UUID in the same world JSON. NPC auto-building,
  auto-farming and ordinary village-approved structures cannot overlap it.
- After buying, the player may build **their own** vanilla structures on
  the lot. No NPC construction is required.
- If the player instead wants an NPC-built starter hut, stand on their
  *empty* parcel and use `/civ commission Ashford`. The job costs
  **15 additional civic coins** paid into the village treasury. This
  creates a queued order; **no abstract materials are charged**. When the
  builder begins, exactly **20 real oak logs and 25 real cobblestone**
  are reserved from the physical warehouse. Missing goods delay building.
- Commissioned player huts stay **private**, never counted as available
  bedrooms for village NPCs. The player can inspect the project with
  `/civ construction Ashford`.
- `/civ development Ashford` shows autonomy, loaded actors, current
  physical hut/field jobs, unbuilt housing demand, player parcels and
  treasury. OPs can optionally pause *new* planning with
  `/civ autonomy Ashford off` and resume with `... on`, but **ON is default**.

**Important boundaries:** purchased plots protect against construction
attempted **by this mod**; they are not vanilla multiplayer land claims or
a global anti-grief plugin. Another player or mod may still edit blocks unless
an independent protection mechanism is installed. The native block preflight
and conditional edits refuse replacing unfamiliar player-built blocks.
Parcels and commissions survive a normal save/restart, but the game still
needs actual local Horizon E2E to prove spawning, navigation, building,
growth, blocked construction and reload. Test only in disposable worlds.

### Buy an existing real building

To buy a **completed village-built hut**, first accumulate at least **35
civic coins** (e.g. trade with the village). Stand within 12 blocks of
that actual building and run:

```
/civ buyhouse Ashford
/civ plots Ashford
```

The native adapter verifies *every* real loaded block of the
cobblestone-and-oak hut before processing payment. The authenticated
buyer becomes its private owner, receives the 5x5 plot, and the village
treasury receives 35 coins. That hut stops being public NPC housing;
residents return to other public huts or their commons. The next
ordinary simulated economic days can replenish public housing using
normal wood/stone consumption, not free stock generation.

This purchases only a completed physical hut created by the mod. It
does not purchase arbitrary vanilla or third-party player structures.
Owners can build on their parcels using the usual game controls.

## v0.1.12 — a physical Millenaire-style civic hall & real-material construction

**New town priority:** before automatically constructing ordinary public huts,
the village creates a **real central town hall** at its founding center, using
a 5x5 version of the existing cautious cobblestone/oak blueprint. It includes
two native Minecraft barrel blocks (warehouse service points), two oak signs
(bulletin boards), a lectern (mayor desk), a clear doorway, walls and roof.
The existing UUID-linked merchant NPC acts as the **starter mayor** and is
routed to the hall during their work shift.

The native server periodically attempts to fill the **real sign tile data**
with the village name, mayor, population, shortages of wood and stone and
residents without housing. This relies on Minecraft's native `data merge
block` command and **must be checked in actual Horizon gameplay**; source
code does not prove that the pinned native host accepts sign NBT updates.

### Physical resource economy, not free blocks

A new and legacy town begins with ZERO *verified physical goods* in its
warehouse. Existing `stock` remains the separate legacy abstract economy:
its daily output of hypothetical wood/stone cannot silently pay for Minecraft
blocks. The new `warehouse` tracks only genuine inventory deliveries or
confirmed physical collection.

- **Town hall** costs **24 oak logs and 25 cobblestone**, reserved once from
  the warehouse before the first block is placed.
- Each **ordinary physical hut**, including player commissions, requires
  **20 logs and 25 cobblestone** to be reserved from the warehouse before
  its first native block edit. A previously simulated house isn't free.
- `/civ sell wood 32` or `/civ sell stone 32`, when the player has actual
  Minecraft oak logs/cobblestone, transfers those items through the native
  inventory API and credits **only the physical warehouse**, not the
  abstract stock. `/civ buy` withdraws from the same physical ledger.
  This preserves existing civic-coin trade semantics, which are not yet a
  complete cash-flow/escrow accounting implementation.
- **Farmers** put eight *physical warehouse wheat units* into the same
  serialized world transaction as a confirmed eight-crop harvest receipt,
  after all crop changes were observed. They never duplicate abstract
  wheat or items in the player inventory.
- **Lumberjacks** scout loaded leafy oak logs and **miners** scout exposed
  native stone. They receive explicit, save-first work tickets, walk toward
  safe standing positions and use conditional native block edits. Only a
  later observation of the removed block, after the issued step is durably
  saved, can credit **one** log or stone unit. A crash before issued-state
  persistence loses an uncertain delivery instead of duplicating it.
- Resource scanning rejects **recorded** player parcels, town centers,
  active huts, farms and other village structures. Block provenance cannot
  fully distinguish a placed oak tree or player-built stone from naturally
  generated blocks in unclaimed terrain; full anti-grief needs a reliable
  protection/claim integration.

### Actual village center controls

```text
/civ hall Ashford
/civ warehouse Ashford
/civ development Ashford
/civ roster Ashford
/civ sell wood 32
/civ sell stone 32
```

`/civ hall` reports whether the civic hall is complete, its mayor,
construction cursor and exact missing materials. `/civ warehouse` lists
physically backed wheat/bread/logs/cobblestone/iron and the next build deficit.
The two real wooden signs in a completed hall are intended to show an
on-site summary without repeatedly typing commands.

**Do not interpret the two barrels as an already integrated native chest
inventory.** They are currently physical service markers with an authoritative
save-backed village warehouse. Directly inserting or removing vanilla items
from those block entities is **not** yet wired into the warehouse ledger; use
the verified inventory sell/buy path. Native game verification and a proper
container transactional API remain P0. The bread-baking/stonecutting and
tree replanting processes are not yet independently physical.

### Current build and validation

The implementation is in `feat/physical-town-hall-warehouse-v0112`, stacked
after PRs #15-#19. For local Windows validation **without GitHub Actions**:

```powershell
git fetch origin
git switch feat/physical-town-hall-warehouse-v0112
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

Follow [playtest](docs/PLAYTEST.md) in a disposable world. No Rust compiler,
native Horizon build, game launch, sign NBT update, or live player-container
transaction has been tested in this environment; this is source-only work.

## v0.1.13 — actual vanilla barrel inventory and NPC material hauling (source-only)

**This supersedes v0.1.12's barrel-as-marker limitation.** On the
`feat/native-barrel-inventory-v0113` branch, after the physical town hall
has finished, its two real `minecraft:barrel` inventories become the
**authoritative on-site material storage**, through pinned Horizon native
API 48. The original abstract daily stock remains wholly separate.
No material is silently manufactured by construction.

### The material flow you can actually inspect

1. Lumberjack/miner: after conditional native block removal, their saved
   job remains pending while the UUID-linked NPC walks **back to the hall**.
   Only a nearby returning worker credits the delivery queue.
2. Farmers: eight verified harvested crops become a saved on-worker transit
   receipt. A UUID-linked farmer must return to the hall before these goods
   enter the delivery queue.
3. The merchant (starter mayor/steward) attends the central hall during
   the work shift. With this real villager nearby, each incoming item batch
   is journaled, **actually inserted into a real vanilla barrel**, verified
   by a later native inventory read, then removed from the delivery queue.
4. A house builder first walks back to the hall and **extracts actual logs
   and cobblestone from its barrels** into a project-specific escrow.
   A 5x5 hut still requires 20 oak logs and 25 cobblestone. The builder
   starts navigating to the plot ONLY after the full material reservation
   has been durably saved.
5. `/civ warehouse NAME` shows **real current barrel contents**, incoming
   deliveries, wheat carried by farmers, extracted builder escrow, and
   any pending/blocked inventory journal. Town hall sign shortages also
   read real barrel counts instead of the old JSON count.
6. `/civ buy GOOD COUNT` near a **completed** hall buys plain items from
   a real barrel slot. Civic coins are charged before the native extraction,
   with a durable order; on a crash or ambiguous result the order is
   quarantined rather than being automatically run twice. Purchases up to
   64 items require a single eligible stack of at least that size.
7. `/civ sell GOOD COUNT` near the village center takes genuine items
   from the player's native inventory and places them into the steward's
   incoming delivery queue. On complete town halls, the steward physically
   inserts them into barrels; before completion they finance the first
   civic hall (24 logs + 25 stone).

### Inventory protection and crash policy

A seventh native injection uses the inspected pinned Horizon API 233,
`native_inventory_block_start`, to deny ordinary players access to the
**two public registered hall barrels**. Operators can still open them
for troubleshooting. Other vanilla chests and private barrels are untouched.

Two persistent logs guard inventory movement:

- `barrel_transfer`: 1..8 plain native items, Planned → Issued (save
  **before** external change) → verified completion or Blocked.
- `market_purchase`: authenticated player, exact item/slot/price, coins
  precharged and saved before extracting native goods; delivery is completed
  or quarantined, **never auto-retried**.

Since Horizon's native chunk/container save and our `world.json` file
are not one atomic transaction, a crash in the narrow external-write
window may quarantine or lose a delivery rather than duplicate goods.
Blocked transfers/orders require manual operator inspection of the exact
barrel/player inventory and saved journal; no unsafe automatic repair.

**Limitations:** vanilla hoppers and other mods may still move stored goods
or disturb a saved transaction. Operators can bypass the restrictions, and
physical blocks may be broken by vanilla interactions not handled by this
hook. Natural block provenance on unclaimed land is imperfect. Goods
currently appear in native barrels, but no handheld carry-stack animation,
physical book/chest logistics GUI, roads, full item crafting, or automatic
natural-world town founding is complete. Native Horizon compilation,
in-game navigation, real barrel handling, permission hooks and save/restart
still need local E2E: source contracts are NOT proof of a working game.

```powershell
git fetch origin
git switch feat/native-barrel-inventory-v0113
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

GitHub Actions remain deliberately disabled.

## v0.1.14 — first actually productive public workshop: the bakery

**Branch:** `feat/physical-bakery-economy-v0114` (stacked after PR #21).
Villages develop by default: an existing physical hall plus loaded
UUID-bound Bakers/Builders allow one **real 5×5 bakery** to be autonomously
reserved at a natural, loaded, unclaimed site. A new world is still founded
via `/civ found NAME`; spontaneous world-generation founding is future
work. No manual construction permission is needed for this building.

The native blueprint has floor, walls, roof, doorway, a **real vanilla
crafting table** and a **smoker** as bakery workplace. Builders must fetch
**24 actual oak logs and 33 actual cobblestone** from the hall's native
barrels into a dedicated bakery material escrow before they place any
blocks. A modified/unloaded work site pauses without overwriting objects.

### Three physical wheat → one physical bread

1. A UUID-bound Baker walks to the completed town hall and takes up to
   **three real plain wheat items** from the physical barrels through the
   same saved, rechecked native inventory transaction as the builder.
2. Once three extracted wheat are saved in the Baker's work escrow, the
   NPC navigates to the separate bakery.
3. The native server verifies **every block** of the bakery blueprint,
   including the actual crafting table and smoker, and a live UUID-bound
   Baker within 6 blocks of the workstation. Only then can the persistent
   recipe consume **3 physical wheat → 1 new bread receipt** (one batch
   per bounded production tick).
4. The Baker visibly returns to the hall. The pending bread is credited
   only at this receiving point, then a UUID-bound steward inserts an
   actual `minecraft:bread` ItemStack into the hall's vanilla barrels.
5. `/civ buy bread COUNT` obtains these exact real native stacks under
   the same precharged, fail-closed market transaction rules as other goods.

No part of this cycle can use the older abstract `stock` counter or
daily bonus to craft bread. A real baker with no wheat in native barrels
cannot produce food. No hidden infinite-bread timer runs.

Inspect the process through:

```text
/civ bakery Ashford
/civ warehouse Ashford
/civ development Ashford
/civ farming Ashford
/civ roster Ashford
```

The bakery report shows build cursor, funding, actual block integrity,
physical material escrow, extracted wheat, bread still being carried and
confirmed batches. Native item slots and work locations remain the
authoritative sources.

### Protection (new native hooks)

The existing public-barrel inventory-open hook is joined by two new
pinned Horizon server hooks (9 native injections total):

- **API 334 hopper item move cancellation:** prevent vanilla hopper
  insertion/extraction to either registered public hall barrel. Other
  containers and hoppers are untouched.
- **API 14 native player block-break cancellation:** protect real public
  halls, active/completed bakeries, fields, public huts and another
  player's purchased plot. The authenticated owner may alter their own
  parcel; OP can intervene for recovery.

These controls do NOT cover explosions, fire, all automated block
replacement, external mod inventory writes, or unclaimed land outside the
mod's ownership registry. The world-JSON save and native Minecraft block/
inventory saves are not a single atomic commit. An interrupted external
item move is quarantined rather than automatically replayed; a controlled
manual recovery interface remains unfinished.

**Status:** source changes only, NOT yet successfully Rust-compiled,
Windows-native built or proven in game. Rust/Python tests are authored,
not executed here. GitHub Actions remain intentionally disabled.

```powershell
git fetch origin
git switch feat/physical-bakery-economy-v0114
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```
