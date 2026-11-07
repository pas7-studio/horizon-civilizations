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
