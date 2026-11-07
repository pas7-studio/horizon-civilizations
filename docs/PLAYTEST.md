# Local Horizon Civilizations playtest

**Current assurance:** an older pinned Horizon native branch previously passed
`cargo check` and embedded Rust tests. The latest v0.1.14 stacked source in
`feat/first-play-diagnostics-v0115` has **not** passed native compile or
Minecraft E2E. Do not reuse an old binary as evidence for this stack.

## Build on your own development PC

1. Install Rust 1.95, Git, Python 3.11+, and the OS compiler required by Horizon.
2. Clone this repo into an isolated workspace (no engine fork).
3. Execute:

   ```sh
   python3 tools/native_build.py --test --build-client
   ```

4. Read the printed `native_client_binary` path. The tool makes temporary,
   pinned worktrees of Horizon and horizon-mixins and applies our bundle.
5. Prepare Minecraft resources for **the same Horizon revision** using the
   official Horizon launcher or `horizon-prepare --prepare`. These resources
   are not bundled or distributed here.
6. Launch the compiled client with `--resources /absolute/path/to/prepared/resources`
   (inspect `--help` for current options), then use a fresh disposable world.
7. Follow `docs/TEST_PLAN.md`, particularly NPC spawning, building a hut,
   pausing/unloading, relog, save rollback and the multi-village scenario.

## Manual GitHub Actions binary artifact

The workflow **Build Horizon native client (manual, no assets)** may be started
in the repository Actions tab. It compiles a Linux binary and stores an artifact
with **no Minecraft assets**. This is for Linux/WSL development; it is not a
Windows binary, and it does not establish gameplay compatibility.

## Safety

Actor identity now uses persistent UUIDs, but has not been validated in-game; crash-atomic inventory issues remain.
Do not use it in an important survival world or a multiplayer production server.
A passing Rust check is not permission to mark the gameplay E2E passed.

## Windows local installer reliability

`scripts/install-local.ps1` runs `cargo fmt --all` before `cargo fmt --all --check` so the source checkout can be installed without an avoidable formatting-only failure. It still runs Rust unit tests and only copies the source `.hnmod` into a deliberately selected test instance; the Horizon launcher must still build and trust native code.

## NPC right-click proof

After `/civ actors Ashford` and `/civ roster Ashford`, right-click a UUID-bound citizen: expect a civilization identity/job/help line instead of the regular trade menu. Right-click an ordinary unbound villager: vanilla interaction must remain. Test after world reload, NPC rename, disabled native-mod state, and in multiplayer (acting player's chat only). Legacy v0.1.2 villagers require `/civ reconcile Ashford` first.

## Wheat plot trial (v0.1.4)

Use **only a disposable world**. From a standing position on the center of flat natural 3×3 ground, 12–80 blocks from Ashford, execute `/civ farm Ashford` and `/civ farming Ashford`. Wait for UUID-bound farmers to reach the plot; verify eight tilled squares, a center water source and wheat planted above each tilled square. Let the vanilla wheat age to 7; verify harvest and replant, checking the visual cycle counter. No Minecraft item drops or extra simulated wheat should appear due to visual harvesting. Reboot the world during each phase; confirm the saved cursor reconciles and foreign blocks never get overwritten. Native compile success is not a substitute for these tests.

## One-command Windows playtest (optional)

When the Windows native compile workflow has uploaded `horizon-client.exe` (not just a source-only `.hnmod`), download the artifact and extract it locally. Install the **official Horizon Launcher** and sign in with your own Minecraft: Java Edition entitlement. Then run from a PowerShell terminal in this project directory:

```powershell
.\scripts\start-windows-playtest.ps1 -ClientExe 'C:\Games\HorizonTest\horizon-client.exe' -TrustUnsigned
```

The script imports that exact executable, prepares resources using the official launcher and launches the disposable `Civilizations-Test` creative world. It does not distribute resources, bypass sign-in or alter another save. The `-TrustUnsigned` switch is required to make trust in your own CI artifact an explicit decision. If official resources are already prepared for this installed build, `-SkipPrepare` skips preparing them again.

This launcher flow is **not yet run in this workspace**. If Horizon Launcher reports that a compiled binary is incompatible with its selected version, use the pinned compatible upstream build and a new disposable world instead of patching an existing Minecraft save.

## Previous GitHub Actions build instructions (archival; Actions currently disabled)

**Do not enable or run Actions for the current source branches.** These historical steps document an earlier artifact only. Use the local-only workflow below for v0.1.9.

### Historical steps for the older artifact

1. Open [Windows native Horizon client workflow](https://github.com/pas7-studio/horizon-civilizations/actions/workflows/build-windows-client.yml).
2. Click **Run workflow**, choose `main` as branch and confirm. This is needed when
   no automatic run appears; GitHub API-authorized updates may not emit ordinary
   push-triggered workflow events.
3. Wait for the run to finish with **success**, then open that **same run** and
   download `civilizations-windows-client-no-assets`. Do not take an older run
   unless deliberately testing that older commit.
4. Extract the ZIP to `C:\\Games\\HorizonTest\\horizon-client.exe`. The archive
   contains no Minecraft assets. Obtain them only from the official Horizon
   Launcher using your legitimately owned Minecraft Java Edition account.
5. Use `scripts/start-windows-playtest.ps1` as shown above. The launcher imports
   this trusted unsigned build and starts disposable `Civilizations-Test`.
6. In-game, execute the numbered cases in [TEST_PLAN](TEST_PLAN.md). After saving
   and quitting, collect the read-only report described in
   [PLAYTEST_EVIDENCE](PLAYTEST_EVIDENCE.md). The report is *not* a gameplay pass.

The original Windows build (Actions run 42381940020) was successful but used
commit `c796d6b1f9e1898c9c1dc2175b91626907adf190` before the v0.1.6 host-clock
merge. To test the current integration, choose the newest successful `main` run
and check its exact commit. No modded EXE should be considered production-ready
without in-world evidence of NPCs, hut, farm and restart behavior.

## NPC day/night schedule trial (v0.1.7, local only)

In a disposable creative world with ten UUID-linked Ashford NPCs, run
`/civ schedule Ashford` at `/time set 0`, `/time set 2000`,
`/time set 11000` and `/time set 13000`. Observe near-center
gathering, daytime movement toward workplaces, evening return and
nighttime regrouping; guards remain on watch. Farm/build world changes
must stop outside work hours. This does **not** mean actual sleep,
bed ownership or collision-safe navigation are implemented. Follow
manual cases 54–59 of [TEST_PLAN](TEST_PLAN.md).

## Physical homes trial (v0.1.8, local-only)

After completing the first starter hut in a disposable world, call
`/civ homes Ashford`. One real 5x5 hut provides at most three standing
destinations; any abstract house provides none. At night
(`/time set 13000`) the assigned, UUID-bound non-guards should attempt
to navigate into the hut **only when** its cobblestone floor, empty
standing cells and two-high doorway are loaded and intact. Move or
replace a block in a disposable copy to test commons fallback.
Run [TEST_PLAN](TEST_PLAN.md) steps 60–66, preserving screenshots and
logs for actual pathfinding evidence. No sleeps/beds or full house
ownership are claimed; no Actions workflows are used.

## Offline-first local compile (stacked v0.1.14, no GitHub Actions)

PR #22 is stacked on #21 → #20 → #19 → #18 → #17 → #16 → #15. Check out
`feat/physical-bakery-economy-v0114` when testing **all eight
features together**, not `main` or an older GitHub Windows artifact.

From PowerShell in the local repository with Rust 1.95.0, Git, Python 3.11+
and sufficient disk/RAM, run:

```powershell
git fetch origin
git switch feat/first-play-diagnostics-v0115
cargo fmt --all
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
py -3 -m unittest discover -s tests -p 'test_*.py' -v
py -3 tools/package.py
py -3 tools/verify_static.py
py -3 tools/native_build.py --test --build-client
```

The native source builder uses a disposable pinned Horizon worktree. The
compiled Windows executable, if successful, should be at
`target/native-check/modded/target/fast/horizon-client.exe`. Import that
exact EXE with the **official** launcher and your legitimately owned
Minecraft resources; never copy a preexisting player save as a disposable
world. The source `.hnmod` alone is not a playable executable.

In a disposable creative world, run `/civ survey Ashford`. When simulated
house inventory is available, inspect the printed numbered slots and Y
heights, then issue e.g. `/civ approvehouse Ashford 1 70` from within
24 blocks of **that exact proposed plot**. Do not assume the sample Y=70
matches your terrain. Execute cases 67–75 in [TEST_PLAN](TEST_PLAN.md).

## Village council and economic outlook (v0.1.10; local-only)

In a disposable Ashford world, run `/civ council Ashford` and
`/civ forecast Ashford`. The council should rank food, security, morale,
physical homes, building resources and UUID-binding concerns based on
the **existing** saved data. The hypothetical three-day forecast must
**not** advance the world. Confirm that 20 repeated calls leave the
`world.json` file byte-identical and have no effect on blocks or inventory.

Use [TEST_PLAN](TEST_PLAN.md) cases 76–84. For a native Windows build,
follow the local compilation instructions above on this new branch;
GitHub Actions are intentionally disabled. The forecast excludes war
attrition, player trades and physical farming, and must not be described
as an exact future simulation or a financial ledger.

### Simplified one-command Windows validation (optional)

Instead of manually repeating the Rust and Python commands above, use
`./scripts/validate-local.ps1 -Format` on a fresh checkout of
`feat/physical-bakery-economy-v0114`, followed by
`./scripts/validate-local.ps1 -BuildClient` for the pinned native
integration and executable. Omit `-Format` to refuse unformatted sources
without modifying them. `-Format` explicitly changes local source files;
all commands may create `target/` artifacts. Rust 1.95, Python 3.11+,
Git and enough available disk are required. Native compile needs external
network access for upstream Git sources; no GitHub Actions are used.

## Full autonomous-village acceptance (v0.1.11, local only)

On the latest **stacked feature branch** create an entirely new,
disposable world in a relatively flat grassy area. Run
`/civ found Ashford` once as OP, then do **not** run `/civ actors`,
`/civ construct` or `/civ farm`. Remain nearby with chunks loaded
for several game seconds and inspect:

```
/civ development Ashford
/civ roster Ashford
/civ construction Ashford
/civ farming Ashford
```

The mod should select its own unclaimed sites and have UUID-linked NPCs
construct a real starter hut and irrigated crop field. The exact game
pathfinding and world-block changes **are not validated yet**; if there
is no activity, inspect the native game logs and report which of the
four diagnostic commands remains idle.

For property testing, join the settlement if needed. On a natural,
level 5x5 patch 12..80 blocks from the village center, stand at the
center and run `/civ buyplot Ashford`. The parcel costs 10 civic
coins and is protected **against this mod's automatic builders and
farmers**. You may now build your own blocks, or while still on an
empty purchased parcel use `/civ commission Ashford` to order an
NPC-built hut for another 15 civic coins. This order queues without
charging abstract materials: actual building requires 20 physical
oak logs and 25 cobblestone from the warehouse, reserved once before
the first block. Property ownership and treasury survive save
and reload. A private commissioned hut must not become NPC public
housing. Do not mistake this for vanilla multiplayer anti-grief.

Run numbered cases 85–98 from [TEST_PLAN](TEST_PLAN.md) before
calling this a working autonomous town. Existing guild, bakery, roads,
tree-cutting, auto-founding settlements and real item-economy logistics
are still independent work.

### Buying an actual finished village hut

After linked builders complete an ordinary public starter hut, give
the buyer sufficient *existing civic coins* (35, earned by trading).
Stand within 12 blocks and run `/civ buyhouse Ashford`, then
`/civ plots Ashford` and `/civ homes Ashford`. Check the real hut,
player title, village treasury and removed public housing capacity.
Changing one owned plot's blocks in a **disposable** world must not
cause the village to reclaim or overwrite it. Run [TEST_PLAN](TEST_PLAN.md)
cases 99–102.

## First real town hall and resource acquisition test — v0.1.12

Use `feat/physical-bakery-economy-v0114`, not `main`. The previous
autonomous-source release can appear idle because it only had virtual
materials and no central functional building. In a **disposable** grassy
world, issue `/civ found Ashford`; wait for loaded NPCs and run:

```
/civ hall Ashford
/civ warehouse Ashford
/civ development Ashford
/civ roster Ashford
```

Try nearby leafy oak trees and exposed natural stone with the named
lumberjack/miner, and also deliver enough real items by the normal
inventory sale commands:

```
/civ sell wood 24
/civ sell stone 25
```

You need real oak logs and cobblestone in inventory first. Then observe
the central physical hall being built, including barrels and bulletin signs,
and inspect native game logs for rejected conditional or sign NBT commands.
Test log/stone gathering before assuming self-sufficient supply. Additional
public huts need 20 logs and 25 stone each. Details in
[TEST_PLAN](TEST_PLAN.md), cases 103–119.

This code does **not** yet make the barrels real vanilla inventory gateways.
Their block contents are not mapped to the saved warehouse ledger. Do not
assume that putting items manually into a barrel credits the settlement.
GitHub Actions are intentionally disabled; run only local validation.

## Real native barrel / NPC courier acceptance — v0.1.13

The newest full stacked implementation lives on
`feat/physical-bakery-economy-v0114`. In a **disposable** world with a
completed physical civic hall, keep the central chunks loaded. Use:

```text
/civ hall Ashford
/civ warehouse Ashford
/civ roster Ashford
/civ development Ashford
/civ sell wood 16
/civ sell stone 16
```

Watch the UUID-linked mayor/steward physically attend the hall. Sold items
should move from the saved incoming delivery queue to **visible native
barrel stacks**. A builder with a new 5x5 hut job must first go to hall,
extract 20 oak logs +25 cobblestone, then walk to the construction site.
Farmers and lumberjacks/miners should return to hall before their
confirmed crop/wood/stone deliveries become available.

To purchase from an actual barrel, stand within 12 blocks and run
`/civ buy wood 4` (requires player inventory room, sufficient civic
coins and a plain native barrel slot containing at least four logs).
Ordinary players cannot freely open public barrels. An operator can
inspect them, but **manual changes while a transaction is active can
quarantine the entire operation**. Other vanilla barrels are unaffected.

Do not use a valuable world for initial tests. Check [TEST_PLAN](TEST_PLAN.md)
cases 120–138 and preserve logs if a transaction is blocked; there is
intentionally NO blind recovery path. Native console/physical E2E is
not done. GitHub Actions remain disabled.

## Play the first real NPC production workshop — v0.1.14

Use the FULL stacked source branch:
`feat/physical-bakery-economy-v0114` (NOT the old main build).
In a disposable world, complete the physical town hall with its genuine
material supply; keep hall, field, and NPCs loaded. No special operator
permission is needed to start the bakery: the native day planner reserves
a clear separate plot when UUID-linked Bakers and Builders are present.

```text
/civ found Ashford
/civ hall Ashford
/civ bakery Ashford
/civ warehouse Ashford
/civ farming Ashford
/civ roster Ashford
```

The first hall needs 24 oak logs +25 cobble. The bakery *later* requires
24 additional oak logs +33 cobble from the native public barrels.
Builders go back to hall to collect these items before actually placing
the crafting table/smoker and the building's shell. Use
`/civ bakery Ashford` to inspect missing materials, cursor and integrity.

Once completed, bring real wheat to the hall:

```text
/civ sell wheat 12
/civ warehouse Ashford
/civ bakery Ashford
```

The steward must deposit the wheat in real vanilla barrels. Baker walks
to hall, withdraws three real wheat items, travels to the bakery, uses
its verified actual workstation to produce exactly one bread receipt,
returns to hall, and steward inserts a **real `minecraft:bread` stack**
in the town hall barrel. Test `/civ buy bread 1` nearby.

Ordinary players should NOT be able to open the public barrels without
trading, pull them using vanilla hoppers or break public structures. A
player who purchases a village plot **may** edit their own property.
Operator bypass remains for manual repairs, and other vanilla storage
must still work.

**This is source-only until tested.** GitHub Actions remain disabled.
Stop at any blocked native transaction; never repeatedly retry a crashed
order or force-merge into main. [TEST_PLAN](TEST_PLAN.md) cases 139–158.


## Latest first-play quickstart — unreleased diagnostic branch

Use **only a disposable world**, with a local Rust 1.95.x toolchain, Python
3.11+, Git, MSVC/C++ build dependencies, sufficient RAM/disk and legal Minecraft
Java resources. Old `main` builds do not contain this feature stack.

```powershell
git fetch origin
git switch feat/first-play-diagnostics-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

Do **not** run GitHub Actions. `-Format` changes formatting only in the local
checkout; inspect the diff and do not auto-push it. On success, use the exact
new client binary printed by the native builder with the official Horizon
Launcher. Source-only `.hnmod` files are **not** Windows executables.

In Minecraft creative (fresh disposable world), run:

```text
/civ found Ashford
/civ diagnose Ashford
/civ roster Ashford
/civ hall Ashford
/civ warehouse Ashford
/civ bakery Ashford
```

Keep the area loaded. Do **not** run `/civ actors`, `/civ construct` or
`/civ farm` when verifying default-on autonomy. Diagnose shows a single
read-only snapshot; NPC movement and production still require visual checks.
If the build or launcher fails, capture exact command, source SHA, error log
and the first failing test rather than assuming the game code worked.

## Physical bakery stock control (not yet game-verified)

A completed bakery no longer attempts infinite bread production in source:
it stops at two loaves per living resident, capped at 64, counting native
barrel bread plus confirmed incoming bread and bread already carried by
the baker. Player bread purchases can reopen demand. If the actual native
barrel state is unavailable, wheat extraction and crafting pause. This
does not count abstract day-simulation food. Test cases: 182–188.

## Latest physical forestry / resource journal acceptance (unreleased)

Use branch `feat/natural-forestry-gather-journal-v0115` on an isolated local
checkout. It includes prior physical hall, warehouse, bakery, safe barrel
recovery and diagnostics changes. It is **not** a game-verified binary.

```powershell
git fetch origin
git switch feat/natural-forestry-gather-journal-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

If both native build commands pass, launch the resulting exact-commit
`horizon-client.exe` with your owned Minecraft Java resources and
official launcher in a NEW disposable world. Keep loaded trees and
stone near `/civ found Ashford`; observe `/civ diagnose Ashford`,
`/civ roster Ashford`, `/civ warehouse Ashford`.
For a blocked issued natural gather, use OP `/civ gathercheck Ashford`
and only if original block evidence is directly inspected, OP
`/civ discardgather Ashford intact` or `... empty`. This explicitly
abandons any disputed resource with **zero** credit; it is NOT a replay
or a refund. Preserve full world backup and logs before intervention.

The forest predicate verifies a natural-looking rooted tree, not player
provenance. A player-grown oak outside registered parcel protections
looks identical to a world-generated oak. Do not use in important worlds.
See TEST_PLAN 189-199; true multi-log tree felling and replanting
remain separate work.

## First-play: sequential short-oak logging (unreleased source)

New stacked branch: `feat/verified-oak-stem-harvest-v0115`, **not** main.

```powershell
git fetch origin
git switch feat/verified-oak-stem-harvest-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

Do not start GitHub Actions. Build/check locally, then import the EXACT
new compiled client EXE using the official Horizon launcher and legally
owned Minecraft Java resources. Test only a disposable world.

After /civ found Ashford, use /civ diagnose Ashford and /civ warehouse
Ashford. A small 3-4 log natural oak can produce several physical logs,
**one verified log for each returning lumberjack trip**. OP
/civ gathercheck Ashford shows the step; it does not modify stock.
Do not infer Minecraft item carrying animation from the domain delivery
receipt. Unsupported tall trees are untouched. NO sapling replanting or
whole-forest renewable production is implemented yet. Follow TEST_PLAN
200–211 and back up the entire disposable save before fault injection.

Older instructions below refer to historical releases and capabilities;
their previous local/native assurance does NOT carry over to this branch.

## Latest 3D physical-worker safety trial (unreleased source)

To test all stacked changes, check out
`feat/three-dimensional-native-worker-guards-v0115` on Windows with
Rust 1.95, Git, Python 3.11+ and legal Minecraft Java resources:

```powershell
git fetch origin
git switch feat/three-dimensional-native-worker-guards-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

Do NOT enable GitHub Actions. Inspect any local source formatting changes
before keeping them. Use only the fresh executable from this exact
successful build and a disposable new world; source-only .hnmod is NOT a
compiled mod. For vertical safety proof follow TEST_PLAN cases 212-222:
a worker in a cave beneath the hall must not count as physically returned.
A worker at a tree's X/Z without the saved clear work stand and eye-level
reach may NOT mine it. These checks are implemented only in source until
a real native gameplay E2E is recorded.

## Same-worker wood/stone receipt trial — newest source, not yet game-verified

Checkout `feat/assigned-resource-courier-v0115`, not `main`:

```powershell
git fetch origin
git switch feat/assigned-resource-courier-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

Rust 1.95, Git, Python 3.11+, native OS build dependencies, official
Horizon launcher and legally owned Minecraft Java resources required.
GitHub Actions remain intentionally disabled. Use the EXACT newly built
client binary and a NEW disposable world only.

A gather has an assigned `worker_id` in the saved journal before any
block edit. The same UUID-linked resident must physically reach the
source and return to the hall, not simply any same-job villager already
standing near it. OP `/civ gathercheck NAME` shows the assigned resident.
Legacy unknown-issued journals must remain quarantined, not granted.
TEST_PLAN 223-233 documents exact crash/identity tests. This does NOT
implement a visible item model in the NPC's hands.

## Latest first-play checkpoint: source pickup before return to hall

Newest source branch (still **not compiled or gameplay-verified**):
`feat/source-pickup-witness-v0115`.

```powershell
git fetch origin
git switch feat/source-pickup-witness-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

Keep GitHub Actions OFF. Use only a newly compiled native Horizon game
client with legitimately owned Minecraft Java assets and a disposable
world. An issued log/stone gather must wait for its EXACT saved worker
to witness the now-air source from the real work stand. Only after
that durable witness does the worker navigate to the town hall, where
a separate 3D return can release one source-backed resource receipt.
`/civ gathercheck Ashford` and `/civ diagnose Ashford` now report
whether pickup was witnessed. See TEST_PLAN 234-244 and retain full
world/actor/log evidence. No claimed visible carried ItemStack.

## Latest native gather verification — crash-safe acknowledgement / fair queue

Git branch: `fix/gather-crash-proof-fair-queue-v0115` (Draft stacked
on PR #30; NOT merged, NOT playable-verified).

```powershell
git fetch origin
git switch fix/gather-crash-proof-fair-queue-v0115
.\scripts\validate-local.ps1 -Format
.\scripts\validate-local.ps1 -BuildClient
```

`-Format` changes files locally: inspect that diff. Build with
Rust 1.95 and Python 3.11+, using network access and official
Horizon/Minecraft Java assets owned by the tester. GitHub Actions
remain intentionally disabled. Only run on a disposable world.

Use `/civ diagnose NAME`, OP `/civ gathercheck NAME`, actual
`world.json`, and Minecraft block/barrel inspections to check:
write-ahead Issued, successful native block edit and immediate air,
durably confirmed native edit, source witness, then same NPC's
physical return to hall. Crash immediately after issued/block edit
but before native confirmation: ZERO auto-credit, OP quarantine.
Run TEST_PLAN 245–254 with multiple settlements. Real item cargo
models/animations and full playability still not claimed.
## One-command local native build evidence (latest branch)

```powershell
git fetch origin
git switch chore/first-play-native-validation-v0115
py -3 tools/first_play_gate.py --dry-run --build-client
py -3 tools/first_play_gate.py --build-client
```

Run from repository root on Windows 11 with Rust 1.95, Python 3.11+,
Git and network to fetch pinned Horizon if not cached. No GitHub
Actions are involved; the script never edits source or attempts
Minecraft login. No built executable is provided by this source
PR. Artifacts/logs stay in ignored `target/first-play-gate/<UTC>/`.
Review `report.md` and the first-failing step log. If the result is
`BLOCKED`, correct that exact failure and rerun after recording the
new source SHA. Only a successful real `--build-client` can reach
`READY_FOR_GAMEPLAY_E2E`, which STILL requires launching the new
binary with owned Minecraft Java assets and manual disposable-world
E2E. Follow TEST_PLAN 255–261 plus previous physical gameplay cases.


## Latest first-play gate — verify an actual native game binary

Checkout `fix/first-play-binary-evidence-v0115`:

```powershell
git fetch origin
git switch fix/first-play-binary-evidence-v0115
py -3 tools/first_play_gate.py --dry-run --build-client
py -3 tools/first_play_gate.py --build-client
```

This release gate now checks the pinned `native_build.py` JSON
and **the actual compiled Horizon executable** before it can
report READY_FOR_GAMEPLAY_E2E. It records the binary's SHA-256,
size, path and source SHA. A missing executable with exit=0,
bad pinned revision or incorrect output path becomes BLOCKED.

Read `target/first-play-gate/<UTC>/report.md`, JSON and first
failing step log. No GitHub Actions or deployment are run.
The 10/10 local Python mock harness tests are NOT real
Rust compilation, nor Minecraft gameplay tests; use a
disposable game instance with owned Java assets for E2E.

## Updated local order: diagnose actual compiler errors first

On `fix/first-play-binary-evidence-v0115`, run
`py -3 tools/first_play_gate.py --build-client` without source
formatting. Gate now executes `cargo test` before Clippy/formatting,
so the first build failure is actionable. Formatting must still PASS;
the script never silently changes Rust source. Open
`target/first-play-gate/<UTC>/report.md` for the first blocker.
The runner's 12/12 mock harness tests passed on Linux; native
compilation/Minecraft gameplay remain unverified.

## Recover only an actually missing resident after partial NPC creation

Latest source: `feat/guarded-partial-villager-recovery-v0115`.
This branch is **not compiled or gameplay-verified**.

On a networked Windows build machine:

```powershell
git fetch origin
git switch feat/guarded-partial-villager-recovery-v0115
py -3 tools/first_play_gate.py --expected-branch feat/guarded-partial-villager-recovery-v0115 --build-client
```

On a disposable Minecraft world: `/civ roster Ashford` now includes
each citizen's resident ID and their saved UUID state. If a crash
interrupted native creation, first **load the original village chunks**
and run OP `/civ reconcile Ashford` to bind existing villagers. Only
after verifying a particular villager is truly missing, standing
within 32 blocks and with all surrounding native chunks loaded,
an OP can issue (example ID; use the real roster value):

```
/civ recoveractor Ashford 1007 attempt
```

This is **one irreversible native spawn attempt per resident**,
journalled before native effects, never retried automatically.
If interrupted, inspect live entities and `/civ reconcile`;
do not blindly create a duplicate. See TEST_PLAN 272–282.
Never use a valuable game world to test this Draft feature.
