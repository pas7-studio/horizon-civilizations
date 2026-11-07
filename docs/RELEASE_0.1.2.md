# Horizon Civilizations v0.1.2 - source prototype

Date: 2026-11-07

## New since v0.1.1

- Experimental *physical* 5x5 hut, placed a block at a time with a loaded builder NPC in range.
- `/civ construct NAME` is operator-only and checks a flat natural patch with no structures on it; `/civ construction NAME` reports progress.
- Building state persists with an optional job cursor and completed sites, compatible with previous world JSON.
- Placement is run on Horizon's dimension-specific `native_level_tick_post` hook, not the global tick.
- Existing placed steps are verified again before progressing. Unknown blocks stop the job rather than being overwritten.
- Recoverable `.swap` stage added for Windows world-file replacement; ambiguous interrupted writes fail closed.

## Verification

PASS: Python static checks, 4 packaging tests, reproducible `.hnmod` archive, source API inspection against pinned Horizon SHA.

NOT RUN: Rust format/compiler, Rust unit tests, horizon-mixins apply, Horizon native `cargo check`, launcher, Minecraft visual gameplay tests.

**Do not call this version player-ready.** No real in-world structure has been confirmed yet. Do not install on an important world.

## Known issues

- Construction cannot be resumed after the system flags an obstructed site without editing/restoring the save or future recovery tooling.
- Villagers keep vanilla brain/AI, so worksite navigation can be overridden.
- Only one physical hut shape exists; production, mining, crops, combat, titles and normal houses still use abstract simulation.
- Existing cross-boundary trades between player inventory and JSON state are not crash-atomic.
- After a partial `/civ actors` summon the one-shot flag avoids automatic duplicates but requires future manual reconciliation.
- A whole-world checkpoint cannot be atomically committed with the mod's separate JSON file. A physical building is verified step-by-step, but stronger backup/recovery integration remains work in progress.

## Run it

See `README.md` and `docs/TEST_PLAN.md`. Use a disposable Horizon instance and **first run the native compile gate** on a machine with Rust 1.95 and Git:

```sh
cargo fmt --all
cargo test
cargo clippy --all-targets
python3 tools/native_build.py --test
```
