# First in-game acceptance checklist

1. **Build**: against exact Horizon commit, mixin `plan/apply`, `cargo check` and actual launch; collect logs.
2. **Foundation**: `/civ found Ashford` once, duplicate name and <192-block village rejected; data created in right world folder.
3. **Spawn**: `/civ actors Ashford` near origin visibly summons ten named villagers; repeated command does not duplicate.
4. **Survival**: daylight/time progresses, `/civ info` indicates resources; mobs remain after relog and reload (no duplication). Check `/civ roster NAME` before and after relog; loaded actors must be recognized again.
5. **Worksites**: observe loaded actors walking to job-specific positions, while verifying physical block/items do not magically change from simulation-only production.
6. **Economy**: available goods stay nonnegative; player buy/sell inventory counts and balances are conserved.
7. **Fault injection**: corrupted JSON must be rejected without overwrite; disk write errors must show error; restore from `.bak` manually.
8. **Isolation**: a second saved world never shows the first world's settlement; saving and switching worlds preserves both.
9. **Multiplayer**: unauthenticated/guest clients cannot use op commands; item transactions only affect acting player.
10. **Strategy**: two villages >=192 blocks apart can declare war and sign peace with symmetric state; war is abstract.
11. **Performance**: twenty villages with ten records each; server tick budget and world JSON size measured.
12. **Upstream**: repeat 1-10 on a newer Horizon revision before repinning.

**Stop-ship criteria:** native compilation failures; lost/duplicated items or
NPCs; save corruption; unauthorised commands; unbounded server tick stalls.

## v0.1.2 mandatory physical construction scenarios

13. **Consent**: without an operator, `/civ construct` rejected. On a non-flat or obstructed site no job should start and no blocks change.
14. **Natural plot**: after `/civ day 1`, stand in a clear 5x5 spot 12..80 blocks from village; `/civ construct Ashford` queues a job without immediately placing blocks.
15. **Builder gating**: without a loaded linked builder NPC, nothing gets placed. With a builder close, the cobblestone foundation then walls/roof should materialize incrementally.
16. **Crash/reload**: restart at cursor 1, 25 and near completion; cursor must never skip an absent physical block. A mismatched player-placed block must block the job rather than be overwritten.
17. **Pausing**: unload chunks or leave the dimension; no more work until the selected level's tick resumes.
18. **Windows file protocol**: interrupt before swap creation, between primary->swap and tmp->primary, and after new primary; validate explicit recovery behavior, backups, and no silent corrupted save replacement.
19. **Resource accounting**: abstract house costs wood and stone once. Materializing it must not charge resources a second time or increment `houses` again.

**These checks have not yet been executed against Horizon.**

## Post-native-build construction scenarios

20. **Multi-settlement scheduling:** approve hut jobs for two villages. With both loaded, both cursors must advance; with A unloaded or blocked, B must continue. No inactive dimension may receive edits.
21. **Saved cursor ahead of blocks:** restore a saved chunk missing one previously verified natural foundation block; cursor must rewind to it rather than skip work.
22. **Foreign block:** insert a chest into an already completed position. The job must block without overwriting it or continuing elsewhere.
23. **Write-before-cursor crash:** commit a physical block but not JSON cursor. The next tick must detect it and commit progress once, with no duplicate write.

These now have pure Rust state-machine tests, not in-game proofs; keep valuable worlds out of this experiment.

## UUID identity and disable-state tests

24. Spawn ten villagers and check every resident has a unique nonzero saved entity UUID. Kill/reload and confirm UUID lookup does not silently bind a different villager with the same name.
25. Use the v0.1.2 save schema with missing UUID fields. Before `/civ reconcile NAME`, `/civ roster` must show zero UUID-linked actors. Reconcile from the loaded village as OP and verify ten links saved; repeat is idempotent. Duplicate names / modified impostors must be skipped or manually reviewed.
26. Force partial spawning, restart and verify actors never auto-spawn a second batch. An unbound physical actor can be recovered only with the explicit operator command.
27. Disable `civilizations` using Horizon's native-mod switch: server and level ticks must not advance civilization state or issue block commands while disabled. Re-enable and verify state continuity.

28. **NPC right-click:** click a UUID-bound resident, get identity/profession/settlement/morale and join/trade instructions in the acting player's chat; claimed interaction blocks vanilla trade menu for this bound villager only.
29. **Vanilla safety:** right-click vanilla villagers, unbound legacy villagers, and a renamed impostor: they must retain vanilla interaction, never expose another village's identity or consume the event. Repeat while mod disabled.
30. **Reload and multiplayer:** reopen world, interact with an NPC whose UUID is saved; confirm same identity. Guest A's right-click must not write into Guest B's chat. Check with both hands and an item held.

## Physical wheat farming trial (v0.1.4)

31. **Approval:** as OP, stand on a 3×3 unmodified grassy/dirt/stone patch, 12–80 blocks from an existing village; `/civ farm NAME`. Reject non-natural ground, foreign blocks, overlapping structures, another farm, invalid distance/dimension, or unloaded chunk. Do not change blocks during command handling.
32. **Farmer gate:** the eight farmland blocks, one water source and eight crops only appear as an actual UUID-linked farmer approaches within eight blocks. Without farmer or with mod disabled: no edits. New soil may not overwrite chests/foreign blocks.
33. **Natural growth:** observe real blockstate `minecraft:wheat[age=7]` for all eight crops; the farm must remain in `growing` until all mature. The farmer may then remove each crop without item drops and replant each crop. `/civ farming NAME` should report a completed cycle. Check crop/water persistence across chunk unload/reload.
34. **Crash:** restart at partial soil placement, mid-harvest and mid-replant. Cursor must rewind after a block/chunk rollback without minting wheat, bread, coins or player drops.
35. **Obstruction:** replace a farm soil/water/crop block with a foreign block. It must fail closed and never overwrite that block or continue other work on this plot.
36. **Separate economy:** compare wheat stock and player inventory before/after a physical crop cycle with auto-day ticks paused in a controlled test. Crop cycles **must not** directly increase JSON wheat, and no farm harvest item must be dropped.
37. **Multiple worlds/villages:** fields must not edit other dimensions or other plots; only already-loaded plots may advance.

38. **Operator recovery:** obstruct a crop and confirm `blocked=true`, clear it back to the expected wheat/air state, then `/civ recoverfarm NAME`; recovery must refuse if the obstruction remains or any required chunk is unloaded, and must not credit warehouse stock.

43. **Harvest receipt:** after all eight mature crops have been observed removed, exactly one receipt is stored, with sequential ID, world day and crop count. The command /civ harvests NAME must show it, and a repeated stage completion must not append another receipt.
44. **Persistence and bounded history:** reopen world and confirm receipt identity; after more than 32 completed harvests only latest 32 remain while lifetime sequence increases. Legacy v0.1.4 farms load with no fabricated receipts.
45. **Corrupt audit:** invalid crop count, duplicate/non-increasing sequence, future observed world day and oversized history must reject the world save. No crops, money, stock or player items may be minted by audit creation.

## World-switch persistence regression

46. Force a read-only/full disk so outgoing world A cannot save, then ask Horizon to open world B. Civilizations must retain A in memory and refuse mutations/physical edits in B; do not replace A with an empty B world. Restore disk access and reattempt saving/closing.
47. Force a shutdown save failure while A is dirty. State must remain resident until the process exits; the mod must log a recoverable error instead of discarding it, and a later close after disk access is restored must succeed.
48. Pre-create a stale world.json.swap and attempt an ordinary save. Verify neither world.json.bak nor world.json.tmp are touched and the world is blocked until manual recovery of the swap. Test with copy-back of a valid backup in a disposable instance.

49. **Time bootstrap:** create a pre-v0.1.6 world with a valid save but no last_host_day, load it on an advanced Horizon day, confirm zero phantom production and a new persisted anchor.
50. **Day crossing and reload:** keep world open across a real Horizon day boundary (or sleep/skip-night), verify one abstract economy day. Close/reopen on the same host day and verify no second credit.
51. **Time rewinds:** use /time set to an earlier day, re-open, and verify no replay until the previous maximum observed day is exceeded.
52. **Large time jumps:** advance the host day by 10,000 and confirm at most 30 simulation days and one persistent cursor update; next tick at the same host day must be a no-op.
53. **World split:** overworld and Nether hooks must not each count an economic day; verify the shared Horizon world clock is used once.

## NPC routine E2E (v0.1.7, manual local only)

54. With 10 UUID-linked Ashford residents and loaded chunks, `/time set 0` should route non-guards toward commons near the center. `/civ schedule Ashford` reports dawn.
55. `/time set 2000` should route workers toward job positions. Farmers with an approved field and builders with an active hut move to work sites; guards remain on watch.
56. `/time set 11000` and `/time set 13000`: non-guards return toward near-center evening/night waypoints. The mod must not generate inventory or stock from movement.
57. At night, keep a farmer within 8 blocks of a ripe field and a builder within 9 blocks of an unfinished hut. Neither crop/block edits nor building progress should happen until tick 2000..10999.
58. Restart at night and re-open during another time phase. Schedule recomputes without extra `world.json` fields or duplicate resident UUIDs.
59. With unloaded chunks, other dimensions or mod disabled, scheduling never force-loads, navigates foreign villagers or places blocks. Repeat with two nearby villages, time rewinds and large jumps.
