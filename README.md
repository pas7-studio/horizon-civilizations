# Horizon Civilizations (prototype source)

**Goal:** a Millenaire-like civilization system for [Horizon](https://github.com/horizon-rust/horizon):
living settlements, occupations, economy, progression, titles, political conflict.

This is an **independent native mod**, not a fork of Horizon, and contains no
Millenaire code or Minecraft assets. Original project code is MIT-licensed.

> Status: **source prototype, not gameplay-verified**. See [verification status](docs/STATUS.md).

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

Not available yet: warehouse credit from physical harvests, physical mining, generalized building templates,
full trade UI, courts/dynasties, physical war, crash-safe item ledger. The real wheat plot
and harvest receipts are observation-only, with no item drops or stock credit.
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

Packaging produces `target/civilizations-0.1.11.hnmod` (source-only; the
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
3. Put `civilizations-0.1.11.hnmod` in that instance's `horizon-native-mods/`
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
| `/civ construction NAME`, `/civ farming NAME`, `/civ harvests NAME` | All | Inspect physical build/farm progress and visual-only harvest receipts |
| `/civ join NAME` | All | Join a settlement |
| `/civ title`, `/civ promote` | All | View rank/coins; promote if eligible |
| `/civ sell GOOD N`, `/civ buy GOOD N` | All | Trade with joined settlement |
| `/civ history` | All | Recent world events |
| `/civ found NAME` | Operators lvl 2 | Create settlement at current position |
| `/civ actors NAME` | Operators lvl 2 | Spawn named vanilla villager actors once |
| `/civ construct NAME` | Operators lvl 2 | Approve a safe natural site at your feet for a physical 5x5 hut; requires a pending simulated house |
| `/civ approvehouse NAME SLOT Y` | Operators lvl 2 | Confirm one numbered housing proposal near the selected plot; real blocks rechecked |
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
  **15 additional civic coins** and consumes **8 village wood + 6 stone**
  from the existing abstract stock. The coin payments move into the
  village's persisted civic treasury, not a phantom free balance.
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
