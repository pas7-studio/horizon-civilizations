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
