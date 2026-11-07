# Local Horizon Civilizations playtest

**Current assurance:** native `cargo check` and embedded core unit tests passed on
the pinned Horizon revision. The 3D client has **not** been launched or verified
with a Minecraft world. Building an executable is not an in-game E2E.

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

## Offline-first local compile (stacked v0.1.11, no GitHub Actions)

PR #19 is stacked on #18 → #17 → #16 → #15. Check out
`feat/autonomous-physical-village-v0111` when testing **all five
features together**, not `main` or the older GitHub Windows artifact.

From PowerShell in the local repository with Rust 1.95.0, Git, Python 3.11+
and sufficient disk/RAM, run:

```powershell
git fetch origin
git switch feat/safe-village-housing-proposals-v019
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
`feat/autonomous-physical-village-v0111`, followed by
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
NPC-built hut for another 15 civic coins and 8 wood/6 stone taken from
the settlement stock. Property ownership and treasury survive save
and reload. A private commissioned hut must not become NPC public
housing. Do not mistake this for vanilla multiplayer anti-grief.

Run numbered cases 85–98 from [TEST_PLAN](TEST_PLAN.md) before
calling this a working autonomous town. Existing guild, bakery, roads,
tree-cutting, auto-founding settlements and real item-economy logistics
are still independent work.
