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

## Native public barrel inventory and courier acceptance (v0.1.13)

120. Use a disposable flat natural world, checkout latest stacked
     `feat/native-barrel-inventory-v0113`. Build a full real hall and verify
     its two barrel block entities exist at center (-1,+1) and (+1,+1).
121. With hall completed, /civ warehouse Ashford must show ACTUAL BARRELS
     from native container slots; it must NOT simply echo the legacy JSON
     abstract or inbound delivery queue.
122. While standing within 12 blocks, run /civ sell wood 16 from real
     player inventory. Check item count decreased once and wood appears first
     in the steward's inbound queue, then in a real vanilla barrel **only
     when the UUID-linked merchant is loaded near the hall**.
123. Interrupt a deposit after Planned is saved, after Issued is saved,
     and after native insertion but before confirmation. Recover from
     backups/controlled crash injection. Issued must NEVER be reinserted
     blindly. A mismatching observed count must become Blocked.
124. Verify mixed native storage: wood/stone prefer first barrel,
     wheat/bread/iron prefer second, with spillover when full. No damaged or
     NBT-bearing stack is consumed as an ordinary plain material.
125. Prepare a real public hut job, with 20 oak logs and 25 cobblestone
     physically present in barrels but zero available in the old virtual
     warehouse. Builder must walk to hall, extract exactly that supply,
     reserve the escrow once, then walk to and build the hut.
126. Reboot mid-builder extraction. Completed native withdrawals must
     never be reissued on reload; observed stock must match durable 1–8
     item batch transitions. Uncertain writes quarantine instead.
127. Observe a UUID-linked miner and lumberjack remove real stone/oak
     source blocks with the loaded native conditional command. They must
     walk back to village hall before receiving delivery credit; the steward
     then places receipts in barrels. Stay nearby, do not teleport workers.
128. After a physically verified eight-crop harvest, the farmer must
     carry a pending eight-wheat receipt back to hall; verify warehouse
     delivery credit exactly once and later native barrel insertion
     exactly once. No abstract or player-inventory credit.
129. Attempt to open a community hall barrel as a non-OP citizen or visitor:
     server should prevent vanilla container UI and show trade instructions.
     Check a personal barrel elsewhere is still usable. Test operator access
     for debugging, and record whether external hoppers bypass hook.
130. Run /civ buy wood 4 with sufficient coins near a completed hall.
     Confirm matching real native barrel slot decreases by four, player
     native inventory receives exactly four, and civic coin balance falls
     once with corresponding treasury increase.
131. Attempt real buy from too far away, other dimension, insufficient
     coins, no room, no full eligible plain slot, altered item NBT or while
     another native transfer is outstanding. No free items or coins.
132. Restart with a precharged native market journal in Issued/Blocked phase.
     The game must not re-extract or grant items; /civ warehouse must
     reveal the pending order for manual operator reconciliation.
133. Modify barrel contents externally using an OP or hopper during a
     pending native transfer. A count mismatch must block the operation;
     no best-effort "repair" that duplicates items may occur.
134. Break one public barrel (in a disposable world only) or unload the
     hall chunk while an inventory transfer is pending. No further
     external item movement until real slot reads become valid again.
135. Have two separate village halls in the same dimension and in
     different dimensions. Their incoming queues, actual barrel stock,
     buyer orders and build escrow remain isolated.
136. Verify sign shortage displays real barrel stock plus build escrow,
     NOT virtual abstract logs. A missing block entity must not print
     fabricated stock values.
137. Confirm village-owned real barrels do NOT open freely to ordinary
     players. This is not full vanilla block anti-grief; destructive
     edits and external-mod inventory writes need separate hooks.
138. Compile native Horizon, run all domain/structural tests, restart after
     world load, and collect logs and game screenshots of actual physical
     barrel contents, working NPC couriers, purchases and construction.
     Source inspection alone cannot close this acceptance gate.

## Physical bakery, native wheat → bread and public warehouse protection (v0.1.14)

139. Check out `feat/physical-bakery-economy-v0114`, rebuild a real
     Windows native Horizon client, and open a disposable level test world
     with the completed public hall. GitHub Actions are intentionally OFF.
140. With UUID-linked Bakers and Builders loaded, let the autonomous planner
     choose a separate 5×5 bakery plot (12–80 blocks from the town center).
     Verify it is physically empty, loaded, level and not player-owned.
141. Inspect `/civ bakery NAME`: new bakery is PLANNED, not complete,
     and cannot place even one block from virtual settlement wood/stone.
142. Deliver 24 real oak logs and 33 real cobblestone by selling actual
     player inventory. First observe receipt queue, then actual barrel slots.
143. Builder physically walks back to town hall, extracts precisely the
     24 logs/33 stone into bakery construction escrow, persists full bill,
     walks to bakery site and builds floor/walls/roof one block at a time.
144. Stop/restart while materials are partly extracted and while the
     workshop is half built. No repeated item extraction and no cursor
     advance without observing real native blocks. Wrong/player blocks
     must pause the work without overwrite.
145. Observe actual `minecraft:crafting_table` and `minecraft:smoker`
     blocks inside the finished bakery, no duplicate positions.
146. Supply at least three REAL `minecraft:wheat` items through the
     village native storage. Baker must approach the hall's actual barrel,
     extract three via a recorded transfer and travel to the workshop.
147. With baker absent, dead, unloaded, off shift or workshop invalid,
     confirmed native wheat in their escrow must NOT produce bread.
148. With UUID-bound Baker near the intact native crafting table, confirm
     3 physical wheat escrow become precisely 1 new bread-in-transit
     record, not an instant player item or virtual daily stock.
149. Baker returns to the public hall before the bread receipt enters
     the steward's inbound queue; merchant/steward puts exactly one
     native `minecraft:bread` ItemStack into the real barrel.
150. Run `/civ buy bread 1` near a completed hall with enough coins.
     Confirm 1 real native barrel bread is extracted and 1 added to
     player inventory, once; city treasury/coin debit persists.
151. Reboot with an issued wheat extraction or bread insertion. Compare
     recorded expected counts with real barrel slots. Mismatches must
     quarantine and NEVER auto-replay. No conversion without 3 wheat.
152. Build a vanilla hopper pointing into/out of a public hall barrel.
     Native API 334 must prevent all such moves; other players' hoppers
     and ordinary vanilla barrels must continue normally.
153. Try mining a public hall block, a public physical hut, a growing
     field and the bakery as an ordinary player. Native API 14 must
     stop breaking them without dropping stolen resources.
154. Purchase a 5x5 player parcel. Its authenticated owner can alter
     their own floor/walls; another non-OP player cannot. Wilderness
     and structures in other dimensions remain untouched.
155. As OP, deliberately inspect a protected hall for recovery; no
     hidden permission bypass in NPC commands, property entitlement or
     native inventory ownership.
156. Create two separate villages with Baker NPCs; ensure neither can
     withdraw wheat from the other village's public barrels, build on
     the other's plots or credit its bread receipts.
157. Unload bakery chunks or remove the real crafting table in a test
     world. Bread production must pause with no fabricated items;
     restored blocks do not retroactively double-complete batches.
158. Save game logs plus screenshot/video evidence of the actual hall,
     physical bakery under construction, completed workbench/smoker,
     wheat extraction, real bread stack and successful bought bread.
     Source-based tests do NOT close this gameplay acceptance gate.


## Physical logistics stabilization acceptance (unreleased, stacked on v0.1.14)

159. Run local `cargo fmt --all --check`, `cargo test`,
     `cargo clippy --all-targets -- -D warnings`, `python -m unittest discover -s tests`,
     `python tools/package.py`, and `python tools/verify_static.py` on
     `feat/physical-logistics-hardening-v0115`. Record exact output, commit
     SHA, installed Rust version and pinned Horizon revision. Do not infer
     compile success from source-text checks.
160. With a completed hall and both a planned bakery and a pending public hut,
     but empty barrels/receiving queue, verify planner demand is **44 oak logs
     and 58 cobblestone**. It must not stop after bakery-only 24/33.
161. Seed native hall barrels with 12 logs and 10 cobblestone. Place 5 logs
     in confirmed inbound steward queue, 6 in bakery-specific construction
     escrow and 4 in hut-specific escrow. Remaining wood demand must be
     exactly **17 logs**; no stock can be reused to satisfy two separate bills.
162. Start a native barrel transfer, stop after the journal is persisted in
     Planned/Issued phase, then resume. Physical gathering planning should
     pause until a later native observation reconciles it. Blocked phase must
     NOT produce a fresh pickup/harvest; `/civ warehouse` must show the
     unsettled transfer and an **unknown** shortfall, not invented zeros.
163. Test the same planner with one or both native hall barrel block entities
     unloaded/missing. No scavenging of neighboring player containers and
     no claim that missing stock equals zero. Restore the barrel and confirm
     the planner resumes only from a complete native snapshot.
164. Mark bakery physical construction blocked by changed blocks, while a hut
     still needs real materials. Baker's construction reserve should not
     increase and the remaining hut demand should be computed independently.
165. Test a saved `ToBakeryBuilder` or `ToBakerIngredients` journal without
     a matching real bakery. `World::validate` must refuse the save rather
     than continue with a dangling material extraction intent. Preserve a
     backup of the damaged file, do not overwrite it.
166. Before enabling non-disposable survival play, run native Windows build
     and E2E: NPC pathfinding, chunk unload/reload, two-building funding,
     actual game item counts, shift changes and crash windows. Preserve
     screenshots/world/logs; no GitHub Actions or automated merge.


## OP warehouse recovery and bulletin acceptance (unreleased source)

167. On disposable world from exact stacked latest branch, build native
     Horizon, pass `cargo fmt --all --check`, `cargo test`, Clippy,
     Python tests and package verification. Do not count source assertions
     as a native build. Confirm current save backup and pinned Horizon SHA.
168. Save a `Planned` deposit receipt with unchanged real barrel stock;
     `/civ barrelcheck Ashford` must show saved before/after and actual
     native count. Non-OP may not inspect or resolve. A loaded physical
     barrel is required; no remote unloaded-JSON-only recovery is allowed.
169. OP: `/civ resolvebarrel Ashford cancel 0` (with 0 equal to the
     observed original count, replace with actual value in the test).
     Original Planned receipt clears once, incoming goods remain pending.
     No Minecraft items are inserted or removed by cancellation.
170. After an `Issued` state, even if the current barrel count equals the
     originally observed before-count, `cancel` must refuse. A Blocked
     receipt must also refuse `cancel`; never silently refund inventory.
171. Simulate a physically completed native deposit followed by a crash
     before JSON confirmation: mark receipt `Blocked`, verify actual
     count equals expected_after; OP explicitly enters
     `/civ resolvebarrel Ashford confirm <actual_count>`. Only the saved
     warehouse receipt is committed; exactly one native inventory change
     must be observed across reload and subsequent planner ticks.
172. If actual count differs, chunk is unloaded, wrong dimension, a market
     order is present, input count is stale or receipt is not Blocked,
     `confirm` must refuse and leave world.json and native items unchanged.
     Test an accounting failure after preflight to ensure original blocked
     journal is restored, not cleared.
173. With a bakery plus unfinished public hut and empty barrels, town-hall
     NEEDS sign and `/civ warehouse Ashford` must display the same 44
     logs / 58 cobblestone. With any unsettled native receipt the sign must
     display `STOCK CHECK / PENDING TX`, not guessed shortages. After
     reconciliation and refresh, its normal shortages must return.
174. Verify no stock duplication under restart, neighboring hopper action,
     external OP barrel edits or native chunk save failure. A matching total
     alone is not proof of origin; OP must compare detailed slots and logs.
     Keep multiplayer disabled until verified.


## First-play live diagnostics (unreleased source)

175. In fresh disposable creative world on the **latest** stacked branch,
     compile the pinned native Horizon client and record exact Git commit,
     binary path and mods loaded. Run `/civ found Ashford` once; do NOT
     manually run actors/construct/farm. `/civ diagnose Ashford` must exist.
176. With all chunks loaded, compare `/civ diagnose Ashford` actual
     UUID-linked resident count and job roles with `/civ roster Ashford`.
     The diagnostic must NOT claim unloaded, unbound or renamed impostors
     are present. Time changes should accurately show worker shift.
177. Before the civic hall completes, diagnose must report the exact
     incoming real wood/stone shortage and NOT use abstract daily stock.
     Add physical oak logs/cobblestone with real inventory trades and observe
     the reported deficit shrink only after an actual receipt is saved.
178. After completing the hall, unload one public barrel chunk/entity or
     remove the barrel in a disposable save. Diagnose must report native
     barrels unavailable, not 0 goods. Reload/restore and observe the real
     combined bakery+hut building deficit using actual native item counts.
179. In a test save with a pending or blocked native barrel receipt,
     `/civ diagnose Ashford` must report native journal unresolved and
     direct the OP to `/civ barrelcheck Ashford` without moving items.
     A pending market purchase must take precedence; never auto-refund.
180. Use byte-for-byte save comparison and actual inventory snapshots to
     verify 20 diagnostic calls do not mutate world.json, blocks, barrels,
     player coins or inventory. Other autonomous ticks may naturally change
     game state, so stop simulation or isolate command execution when
     comparing before/after; never attribute unrelated ticks to diagnose.
181. Test non-OP and wrong-dimension access, an off-shift builder, a blocked
     hall construction job, an absent builder, an unfinished bakery and
     unavailable natural plots. Each should produce a meaningful blocker
     without changing the world or claiming gameplay acceptance.

## Physical bread demand and warehouse backpressure (unreleased source)

182. Build exact branch and confirm Rust/Python suite. Create settlement
     with 10 living residents and completed native hall/bakery. The target
     physical bread buffer should be 20 loaves. Record both actual barrels,
     confirmed inbound bread, and baker-carried bread separately.
183. With 20 native bread and plentiful wheat, verify baker withdraws no
     wheat and crafts no additional loaf. Other warehouse delivery of
     logs/stone/wheat must still proceed normally.
184. With 15 on shelf, 4 confirmed inbound bread, zero in transit, one
     new loaf is needed. After it is delivered, baking must pause at 20.
185. With 18 bread on shelf and 2 baker-carried finished bread receipts,
     baking must pause until those loaves are deposited. Save/reload
     cannot cause an extra batch.
186. Missing/unloaded official native barrel must pause wheat extraction
     and crafting. Restoring both native block entities resumes work,
     without treating an unavailable container as empty.
187. Trade away 5 actual bread after reaching target: the baker should
     resume bounded real-wheat restocking once the purchase is settled.
188. Test no living baker/residents, shifts, saved journals, extra
     already-extracted wheat and simultaneous hut funding. No parallel
     warehouse goods should be starved by full bread buffer.

## Native forestry, land reacquisition and crash-atomic gather (unreleased)

189. On newest stacked commit, locally run Rust fmt, clippy, cargo test,
     full Python tests and pinned Horizon native build. Record commit SHA
     and build logs. Do not treat textual assertions as a compiled mod.
190. In a disposable grassy world, position Ashford near ordinary oak trees.
     Confirm a UUID-bound lumberjack selects a soil-rooted, crown-bearing
     **LOWEST** oak trunk log with a physically accessible standing cell,
     not an unreachable canopy block. Confirm one block mined and one
     corresponding physical delivery receipt after returning to hall.
191. Test an oak decorative log with leaves positioned near it, a floating
     trunk, a trunk on cobblestone, a severed middle trunk and an unloaded
     canopy. None may be automatically harvested as natural lumber.
192. Mine an exposed natural stone resource. Confirm miner visits the
     loaded real site, native command removes only that expected stone,
     and one cobblestone appears in incoming physical warehouse receipt
     only after that UUID-bound miner returns to the hall.
193. After the gather job is planned but before worker reaches source,
     register a player-purchased parcel or civic building claim that
     overlaps source or stand. Native issued state must not be saved,
     the claimed block must remain unchanged and no resource credited.
194. Fault-inject failure when persisting `Issued` before native block
     mutation. Confirm the command was NOT sent and the physical block
     was NOT touched. Inspect unchanged world and previous saved journal.
195. Stop process immediately after `Issued` persistence but BEFORE
     native command. On resume, original block remains. Next tick may
     mark Blocked but must NEVER issue a second native edit; zero
     resource credit, and the planner must not silently discard this
     uncertain issued receipt.
196. Stop after native conditional block mutation but before observation.
     On reboot, the saved issued journal must remain and native air
     observation + physically returned UUID-linked worker must credit
     exactly once. No additional harvesting of the same source.
197. Attempt `/civ gathercheck Ashford` and `/civ discardgather Ashford
     intact|empty` as non-OP, wrong dimension, unloaded native chunk,
     on a not-blocked receipt, with mismatch between typed evidence and
     actual block, and with no pending gather. All must refuse and
     preserve stock; read-only audit must not mutate anything.
198. With a Blocked+Issued receipt, inspect native source and manually
     select the exact `intact` or `empty` mode; explicit abandonment
     clears only the job, with **zero** resource credit. Unissued blocked
     candidates may be auto discarded. Village hall, farm and bakery
     planning must continue while an issued gather is quarantined.
199. Test two settlements and multiple dimensions, replay from backed-up
     world.json, whole daytime shift, native chunk unloads, resource
     replenishment and bought claims. Record actual game logs, block
     evidence and item counts. Whole-tree felling, sapling sourcing,
     replanting, lumberyard/quarry buildings are NOT part of this PR.

## Short rooted oak stem, per-log inventory and restart acceptance

200. From the exact latest stacked branch, run local rustfmt check, cargo
     test, clippy, full Python tests and pinned native Horizon client
     compilation. Record SHA, first failure, versions and command output.
     Previous Actions artifacts from main do not include this branch.
201. In a fresh disposable world, choose a loaded ordinary rooted oak
     with 3-4 vertically connected native logs, actual leaf crown,
     unobstructed 2-high worker stand and village building demand.
     Confirm one *saved* OakStem with cursor 0 and an unissued root gather.
202. UUID-bound lumberjack arrives, original root log is removed by a
     conditional native edit. No abstract virtual wood or player item is
     created. Only after the same linked worker returns to the hall may
     incoming physical wood increase from 0 to 1 and the next log at y+1
     be planned unissued.
203. Repeat for all 3-4 logs. Every log requires a distinct persisted
     Issued state, actual changed native block, authoritative air read
     and a real linked worker return. At final delivery oak_stem and
     gather_job both disappear. Each log credits exactly one incoming
     vanilla oak log; no duplicates after normal reload.
204. Stop between previous log's native edit and the returning-worker
     receipt. On restart, pending Issued state must not create a second
     edit. When the saved job is safely observed as air and worker returns,
     the next segment must be planned once only.
205. Stop after one delivery saves the next unissued segment, before the
     worker reaches it. On restart, previous log remains counted exactly
     once, next segment unissued and its native block intact.
206. Fault-inject failure persisting the initial multi-log plan and
     verify old World is restored exactly, including both gather_job
     and oak_stem. NO future tick may reference a dangling stem.
207. Remove/replace any upper trunk log, crown leaf or source chunk
     before its next unissued edit; job must fail closed and NEVER
     overwrite a foreign block or issue a second native action.
208. Purchase a player's 5x5 parcel covering tree or worker stand
     after planning the stem (including between two logs). The next
     segment must refuse despite saved original tree evidence.
209. Force uncertain issued native operation, then OP
     /civ gathercheck Ashford. Quarantined issued stem must NOT
     auto-discard or replan; OP /civ discardgather Ashford intact|empty
     only after live matching native observation, clearing all
     remaining segments with ZERO extra wood credited.
210. Test trees taller than four trunk logs, a single decorative oak log
     with leaves, floating wood, canopy across unloaded chunk and
     old saved single-log GatherJob without oak_stem. The new planner
     refuses unsupported tall/uncertain stems, legacy jobs remain
     single-log with no imaginary continuation.
211. With multiple settlements/dimensions, compare /civ diagnose,
     /civ gathercheck, /civ warehouse and actual world blocks/barrels,
     after two or more relogs and day/night transitions. Keep complete
     crash logs/screenshots. No claim of real sapling replanting,
     full forestry building or rendered item courier at this stage.
