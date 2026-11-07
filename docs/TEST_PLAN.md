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

## Physical homes E2E (v0.1.8, local only)

60. Start with Ashford's ten NPCs and abstract houses but no completed physical hut. `/civ homes Ashford` must report capacity 0, assigned 0 and ten waiting; night routing uses the commons without navigating into an imaginary house.
61. Complete a physical 5x5 hut via the builder, verify its blocks, then run `/civ homes Ashford`. Expect capacity 3 and three residents assigned to its first completed site; all others remain without physical slots.
62. Set time to 13000 in loaded chunks. Three off-duty assigned NPCs should attempt to navigate inside the hut through its 2-high doorway; guards must remain on watch. The game may still refuse a path; record the actual result and do not count source routing as successful game AI.
63. Obstruct one interior standing position with a foreign block, unload its chunk or replace its entrance/floor with a foreign block. Without editing player blocks, the affected NPC must use the commons fallback. Restore the structure, reload and verify the night target can become eligible again.
64. Complete a second hut: capacity becomes 6, the first three allocations remain stable, another three use the new hut. Re-open the world and confirm homes are reconstructed from recorded completed sites without duplicates or invented saves.
65. Verify dawn/evening gathering and daytime professions still override homes; a night home route never generates stock, items, beds, teleports, new houses, or forced chunk loads. Test dimension mismatch and mod-disabled state.
66. Change a resident's alive status only in a **disposable test fixture**, not manually in a valuable world; derived assignments may rebalance among living IDs, but dead actors must not be targeted. Inspect roster/UUID reconciliation before and after.

## Autonomous housing survey E2E (v0.1.9, manual local only)

67. Found Ashford; before any simulated house is ready, `/civ survey Ashford`
    must explain that there is no completed abstract house budget; no changes
    to `world.json`, terrain or player inventory should occur.
68. Advance the economy until one pending abstract house is ready.
    `/civ survey Ashford` must show only loaded, fully clear, flat 5×5 natural
    plots, each with stable slot and absolute X/Y/Z; max five results.
69. Try `/civ approvehouse Ashford SLOT Y` as a non-OP, from over 24 blocks
    away, across dimensions, at the wrong Y, and with invalid slot numbers.
    Every attempt must fail before saving a construction job.
70. Approve one listed plot from next to it. Check `/civ construction Ashford`
    shows exactly one reserved job, and the previously read-only survey had
    not changed that state. A second approval must fail while job active.
71. Obstruct/replace a single expected air or natural-floor block between
    `survey` and approval. The original approval must now reject the site
    without overwriting player-made blocks or advancing any cursor.
72. Complete a house, then attempt the original `/civ construct Ashford`
    on top of a completed hut, field, or active project, including another
    village's recorded structure in the **same dimension**. Each must fail.
73. Survey after all living residents have physical hut capacity, after
    running out of pending abstract houses, and at a world boundary.
    No unneeded, overflowed or out-of-bounds proposals may be approved.
74. Relaunch the save after approval but before work, after a partial build,
    and after completion. A proposal itself should never be serialized,
    while an approved construction remains durable and cursor-reconcilable.
75. Verify multiple villages and dimensions: plot conflicts must not be
    created in one dimension; coordinates from a different dimension are
    not automatically owned by the first. No forced chunk loading.

## Read-only civic council and forecast (v0.1.10, local E2E pending)

76. In a new Ashford save run `/civ council Ashford`. It should detect
    unspawned physical actors and unavailable physical homes without
    spawning villagers or editing blocks.
77. Before running `/civ day 1`, capture `/civ info Ashford`,
    `/civ history`, `/civ council Ashford` and `/civ forecast Ashford`.
    Re-running council/forecast 20 times must not change the world.json SHA,
    coins, stock, actor UUIDs, physical builds or crop receipts.
78. `/civ forecast Ashford` lists exactly three hypothetical days and
    identifies itself as **ABSTRACT**. Compare its economic outputs to a
    disposable clone advanced with `/civ day 3`; differences caused by
    actual wars or player actions must not be described as a forecast bug.
79. In a disposable fixture with no bread production, forecast starvation
    must be ranked above routine housing suggestions. Then restore adequate
    stock and ensure the hunger alert can disappear without a restart.
80. Start an abstract war between two villages. Council security urgency
    rises, but it must not declare peace, launch a raid or send guards into
    physical combat. The forecast explicitly excludes war attrition.
81. Lower morale in a disposable fixture: the council must show a separate
    stability need. Do not manually edit a valuable player's saved world.
82. Start a one-shot villager spawn with a deliberately interrupted UUID
    binding in a disposable fixture. Council recommends roster/reconciliation,
    **not** blind spawning; check no duplicate NPC is created.
83. With real huts or a pending abstract housing build, the housing recommendation
    must point to `/civ survey` only when approval is meaningful. It cannot
    reserve land, place blocks or manufacture a pending house by itself.
84. Visit the other dimension, relaunch, move the Horizon clock, and check
    that advisory commands remain read-only, stable on the same snapshot and
    do not trigger server actions or extra daily economy ticks.

## P0 autonomous village & player land ownership (v0.1.11, local E2E)

85. In a disposable natural-grass world, issue `/civ found Ashford`
    **only**, then remain near the village with the relevant chunks loaded.
    Do not run `/civ actors`, `/civ farm`, `/civ construct` or
    `/civ approvehouse`. Within several native planning ticks, the town
    should attempt to spawn ten linked villagers on natural empty ground.
    Verify UUIDs using `/civ roster Ashford` and save/reload.
86. `/civ development Ashford` must show `autonomy=ON` on a new and on a
    legacy save missing the autonomy field, and identify current physical
    work instead of showing only abstract house counters.
87. Farmers should automatically reserve and cultivate a real 3x3 irrigated
    wheat field during the daytime. Verify actual farmland, central water,
    growing wheat and that harvesting stays observation-only; no duplicated
    stock or player items are allowed.
88. A builder with a linked UUID must walk to a naturally selected 5x5
    plot and gradually build a cobble/oak hut through the existing
    conditional block pathway. At night physical edits must pause; they
    resume next shift without restarting the build.
89. Repeat after restarting during NPC spawning, partial hut work and
    farmer planting. UUIDs must never be blindly respawned; cursors and
    observed blocks must recover without overwriting foreign blocks.
90. Before automatic site selection, place foreign blocks at likely ring
    plot locations. The village must reject them and choose another safe
    site or keep trying later, **never** clear those blocks automatically.
91. Earn or use starting civic coins, join the village, stand at the center
    of a clear flat 5x5 site, and run `/civ buyplot Ashford`. A valid purchase
    costs 10 coins and increases village treasury by exactly 10; repeat
    purchase/overlapping plots and insufficient coin attempts must fail.
92. After buying the parcel, let the autonomous planner run. It must
    choose other sites for both farm and hut, while the purchased square
    remains clear and reserved for the player. Verify /civ plots Ashford.
93. On a bought **empty** parcel, run `/civ commission Ashford` with
    15 more civic coins available. The labor fee and one private hut
    project are recorded immediately, but no abstract wood/stone is
    charged. Actual native construction must wait for 20 real logs and
    25 real cobblestone in the warehouse, reserved exactly once when
    work starts. A second commission, foreign owner's commission or
    unaffordable request must fail without side effects.
94. Let the private hut complete; it must remain private (no assigned NPC
    beds), survive save/reload, and still be excluded from automatic
    construction on its owner's parcel. Player can also build their
    own vanilla structure instead of ordering a hut.
95. In a disposable claimed plot, replace a natural block with a foreign
    structure before commission. The mod must reject/stop its physical
    build rather than overwrite the player's materials.
96. Validate split dimensions, far world coordinates, chunk unloading,
    player plot ownership by UUID, multiple settlements, and paid treasury
    transfers after world restart. No cross-dimensional claim leakage.
97. In a disposable world only, toggle `/civ autonomy Ashford off` and
    verify no **new** plots are scheduled; on must restore scheduling.
    Existing saved construction is not silently deleted or refunded.
98. Run the same source on the pinned native Horizon client, collect logs
    and gameplay screenshots: successful Rust/Python tests alone do not
    demonstrate live civilization behavior.

## Completed village property sales (v0.1.11)

99. With a genuine completed public hut and at least 35 civic coins,
    stand within 12 blocks of it and run `/civ buyhouse Ashford`.
    The loaded hut must match its full native cobble/oak blueprint.
    A successful purchase transfers 35 player coins to the village
    treasury and persists a private player plot at the hut origin.
100. Run the command a second time, from another player's identity,
     or while the hut is damaged or unloaded. All failed operations
     must leave both player balance and civic treasury unchanged.
101. After purchase, /civ homes Ashford must exclude the sold hut
     and the owner's land must be protected against subsequent village
     construction/farming. NPCs with lost housing should return to
     available public beds or commons, not remain assigned to the
     sold house.
102. Advance one ordinary simulated economic day in a disposable
     world with available wood and stone. The city should work toward
     replacing private housing with a **material-paid** public house,
     rather than counting the sold structure as available village beds.

## P0 physical civic hall / material warehouse v0.1.12 (local E2E)

103. Create a disposable village on level natural ground. Run only
     `/civ found Ashford`. Wait with loaded chunks; the UUID-linked merchant
     must be identified as the starter mayor. Check `/civ hall Ashford`.
     Hall should remain **unbuilt** without real warehouse resources,
     even though the abstract legacy stock contains wood/stone.
104. With authentic oak logs and cobblestone in the player's native inventory,
     sell 24 logs and 25 stone to the joined settlement using `/civ sell`.
     Confirm inventory decremented by exactly those counts, warehouse rose
     by those counts, abstract stock did not duplicate, and save/reopen
     preserves the amounts exactly.
105. Verify the autonomous builder selects the existing central 5x5 clear
     plot, spends those real resources once and physically builds the hall
     gradually, from floor to walls/roof to barrels, signs and lectern.
     Leave and return mid-build; it must not mint/charge twice.
106. Inspect the real signs (not merely `/civ hall` text). They should
     show town name, mayor, headcount and outstanding materials. Any native
     console `data merge block` error must be captured; a source contract
     alone does not count as successful text rendering.
107. Check `/civ warehouse Ashford` and `/civ hall Ashford` while
     standing in front of the built civic hall. The two barrels are **not**
     yet synced vanilla chest inventories: do not manually use them as a
     verified storage source. Use the inventory transfer commands.
108. Before any subsequent physical hut, provide 20 more oak logs and 25
     cobblestone. With less than either amount, no native hut blocks may
     appear and no failed reservation may consume the other good.
109. With adequate stock, verify a physical hut's first native block action
     is preceded by one durable supply reservation. Restart during the
     first/last step and ensure no second debit, no foreign-block overwrite
     and correct site completion.
110. In a real loaded 3x3 irrigated wheat patch, confirm all eight mature
     crops are observed harvested. The farm receipts increase once and
     exactly eight physical warehouse wheat units are added, while
     abstract wheat and player inventory do not change. No duplicate on
     reload/replant/callback.
111. Allow lumberjack to find a leafy oak tree outside player claims.
     Verify the villager physically navigates near the source, the intended
     oak_log is conditionally replaced by air, then one warehouse oak log
     is credited after a later observation. No instant simulated '+3'.
112. Repeat with a miner and exposed natural stone; one verified stone
     removal yields one warehouse cobblestone unit. Resource harvesting
     must be bounded to loaded chunks, not repeated after restart.
113. Interrupt after gather plan save but before command, after the block
     command but before issued-state save, and after issued-state save but
     before confirmation (use disposable fault-injection fixtures). Nothing
     may be credited twice; uncertain changes fail closed.
114. Purchase a player parcel, place private oak/stone structures on it,
     then repeat village scanning. The village's miners/builders must not
     select registered property. Also test a constructed leaf-covered
     oak tree in **unclaimed** land; inspect limitations of provenance
     detection without assuming perfect anti-grief.
115. Damage a hall foundation, sign, barrel or lectern after construction
     only in a disposable world. A previously completed hall must not
     silently rewrite edited blocks. Assess need for explicit repair path.
116. Simulate two villages and distinct dimensions, unloading chunks and
     revisiting later; each ledger and cursor must remain isolated.
117. Buy a physical item back from the warehouse, verify exactly one
     native inventory transfer and warehouse debit; prevent selling
     non-owned items or buying unavailable stock.
118. Verify the merchant NPC actually travels into the hall during
     work hours; at dusk it should follow its home routine, without
     becoming a duplicate mayor/villager.
119. Record screen captures of working hall, signs, visible builder work,
     crop tending and material transfer, plus native logs. **Only those
     observations close first-playable acceptance, not source assertions.**
