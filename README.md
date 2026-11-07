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

Not available yet: physical harvesting/mining/farming production, generalized building templates,
full trade UI, courts/dynasties, physical war, crash-safe item ledger.
Only **operator-approved** starter huts are physically constructed; normal housing and all warfare
remain abstract. Do not use this build on an important world until native build and E2E pass.

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

Packaging produces `target/civilizations-0.1.2.hnmod` (source-only; the
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
3. Put `civilizations-0.1.2.hnmod` in that instance's `horizon-native-mods/`
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
| `/civ construction NAME` | All | Physical hut state, verified block progress, pending count |
| `/civ join NAME` | All | Join a settlement |
| `/civ title`, `/civ promote` | All | View rank/coins; promote if eligible |
| `/civ sell GOOD N`, `/civ buy GOOD N` | All | Trade with joined settlement |
| `/civ history` | All | Recent world events |
| `/civ found NAME` | Operators lvl 2 | Create settlement at current position |
| `/civ actors NAME` | Operators lvl 2 | Spawn named vanilla villager actors once |
| `/civ construct NAME` | Operators lvl 2 | Approve a safe natural site at your feet for a physical 5x5 hut; requires a pending simulated house |
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
NPCs do not physically farm, mine or transfer stock. Normal economy and war remain abstract.
Construction stops when a plot becomes obstructed, but there is no recovery UI for a blocked job yet. Villager identity uses a tagged visible name, not a stable UUID; duplicate names are ignored and unloaded actors are not forced into memory. Selling/buying inventory items and saving mod-world JSON are not one atomic engine operation; use only a disposable test world until transaction recovery is implemented and tested.

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
