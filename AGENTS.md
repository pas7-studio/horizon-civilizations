# Local agent handoff: Horizon Civilizations

Do not fork upstream Horizon for this project. It is a native mod with a pure
Rust core. Work in a separate repo. NEVER copy GPL-3.0 Millenaire Java files,
texture packs or any Minecraft resources into this MIT repository.

## Current state

- This is **prototype source**, not validated as running in Horizon.
- `mixins/civilizations/core.rs` is a standalone Cargo lib with unit tests.
- `mixins/civilizations/glue.rs` is a Horizon game adapter using native hooks.
- The pinned Horizon SHA is in `compatibility.json`.
- `tools/package.py` builds a SOURCE-ONLY `.hnmod`; it does not compile Rust.

## Local execution priorities

1. Install Rust 1.95.0 and run `cargo fmt --all && cargo test && cargo clippy --all-targets`.
   Fix every compilation/borrow/type issue found. Do not silently change game
   mechanics while fixing syntactic errors.
2. Run `python tools/package.py && python tools/verify_static.py`.
3. Create a scratch Horizon instance with a legal prepared Minecraft Java game.
   Build the native `.hnmod` with the launcher. If a mixin hook target is
   incompatible, update the adapter/pin and record the exact compiler error.
4. Run every numbered scenario in `docs/TEST_PLAN.md`, saving logs. Do not call
   the build playable until at least founding, NPC summon, trade, save/reload,
   day progression and multi-world isolation pass.
5. Prioritize a real resident ID -> entity UUID reconciliation mechanism. The
   current one-shot `actors_spawned` flag protects against automatic duplicates
   but not all partial failures.
6. v0.1.2 includes **experimental approved hut construction**. Verify every
   construction case before calling it playable. Generalized templates and
   real farming/mob goal overrides are not yet implemented. All other houses
   must remain explicitly marked as abstract simulation.
7. Add cross-inventory crash journal before making live trading public.
8. Preserve upstream updates by only using versioned hook points where possible.

## Provenance

API integration patterns are based on official Horizon docs and its MIT SMP
native mod. The Millenaire source was reviewed for abstract design ideas only.
All game resources come from the player's legitimate Horizon/Minecraft instance.

## Publish when ready

If the `pas7-studio/horizon-civilizations` repository does not exist, create it
as a **new repository** (not a fork of Horizon). Then add this project as its
first commit and push only after running local validation; keep world saves,
Minecraft assets, `target/`, and credentials out of Git.
