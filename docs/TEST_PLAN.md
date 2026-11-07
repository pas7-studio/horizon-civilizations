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

## Native 3D actor presence and resource work-stand acceptance (unreleased)

212. On exact latest branch, run rustfmt, cargo check/test, clippy,
     the full Python suite and pinned native Horizon client build locally.
     Record SHA and first failure; old CI/Windows binaries are not proof.
213. Place UUID-linked lumberjack/miner in the cave 15-25 blocks vertically
     below a saved natural gather site while retaining identical X/Z.
     No native block edit, Issued journal or resource credit may occur.
214. Move the worker onto the saved clear two-high natural stand at the
     correct Y. A normal 3–4-log tree harvest can proceed one log per
     return, with actual 4.5-block eye-to-source-block reach.
215. Move a worker near the source only in X/Z but to Y +10 and Y -10;
     even if their name and UUID are valid, no mining command is allowed.
216. After planning a gather but before its native Issued action, place a
     stone wall in the saved stand (or one block above it). Preflight
     must block safely, neither overwrite the foreign block nor credit wood.
217. After planning but before issue, remove/replace the worker stand's
     walkable floor or unload that chunk. Native extraction must refuse.
     Restore source/stand and replan only after safe settlement state.
218. Place the steward 15 blocks above/below the completed town hall while
     keeping X/Z identical. Warehouse barrel insertion/extraction must
     not be authorized until a true 3D presence at the hall.
219. Place the returning miner and farmer in a cave directly underneath
     the hall. Their issued physical receipts must not be credited as
     delivered until an actual UUID-linked worker returns to hall height.
220. Test bakery crafter far above/below crafting table: cannot consume
     wheat or produce bread. Test builder directly below/above an
     unfinished hut/civic hall/bakery: cannot place native blocks.
221. Confirm vertical navigation correction triggers when resident is at
     target X/Z but wrong Y, and that missing/blocked path does not
     silently teleport, complete jobs or mint resource credits.
222. Test non-finite actor positions, negative/nonfinite radii in Rust,
     dimension mismatch, two villages and short-tree upper logs. No
     cross-dimensional, off-shift or fake remote worker confirmation.

## Assigned UUID-bound gather courier, identity & crash continuity

223. Compile the exact latest stacked source locally with rustfmt, cargo,
     Clippy, complete Python unittest and native Horizon client. Record
     commit SHA, exact binary and any first failing test; old build
     artifacts are not evidence for this code.
224. In a fresh disposable world, select a natural loaded oak or exposed
     stone. Confirm initial `GatherJob.worker_id` is absent; the worker
     must be physically at the recorded work stand with 3D reach before
     assigning. The saved resident ID must be persisted **before**
     `Issued` and the first native conditional block command.
225. Make another qualified worker appear at the source/hall while the
     first saved worker is assigned. The identity must NOT change;
     only that resident may mine or claim the returned item receipt.
226. Have another same-role, UUID-linked NPC stand at the hall while
     the assigned worker is still at the tree/mine. Confirm zero wood
     or stone gets credited until the actual assigned worker returns.
227. On the assigned worker's correct 3D return to hall, confirm exactly
     one resource receipt; duplicate ticks/relog must not credit twice.
228. For a short 3-4-log oak, check each saved next segment carries the
     SAME worker_id, begins unissued and cannot reassign to a substitute.
     After the last confirmed segment, both job and tree cursor clear.
229. Crash after assignment is saved but before Issued. Reload and
     verify the same resident ID is still bound and the block untouched.
     Next valid source action must issue at most once.
230. Crash immediately after Issued, before the original block disappears,
     and then after native deletion but before return. Neither crash
     may cause blind replay or allow another worker to finish the receipt.
231. Deserialize a legacy Issued gather journal without `worker_id`:
     no worker, even at the hall, can credit it. Job must be marked
     blocked/quarantined; OP audit and evidence-matched discard supply
     ZERO material. An old unissued journal may receive a new valid
     durable assignment and continue safely.
232. Kill, unload or unbind the assigned UUID-linked NPC mid-delivery.
     Different NPCs may NOT release its saved receipt. Use OP
     `/civ gathercheck` and careful recovery on disposable backed-up
     saves. Observe that a dead resident is not newly assignable.
233. Run simultaneous village jobs across two settlements/dimensions,
     using at least two workers of the same profession where feasible.
     Worker IDs are scoped to the settlement and wrong-role/foreign
     IDs fail domain validation. Record blocks, native actor UUIDs,
     receiving area and JSON before/after. This is an identity-backed
     delivery gate, NOT proof of a rendered item in the NPC's hands.

## Source-witnessed native resource pickup / courier return (unreleased)

234. On the exact newest stacked branch, run Rust formatter, cargo check,
     Clippy, cargo test, full Python unittest and pinned Horizon native
     client compile. Record SHA and logs. Do not use a previous main build.
235. Start fresh disposable Ashford, select physically accessible native
     stone and short rooted oak. Verify planned gather has no pickup
     witness and no credit. Bind a specific UUID-linked worker at source;
     persist worker_id, then persist Issued before native edit.
236. Confirm a native conditional block edit removed the exact source
     (air observed). BEFORE any hall movement/credit, the same assigned
     worker must still be physically near the saved clear work stand and
     within 4.5-block eye-to-native-source reach.
237. At Issued + native air but no bound worker at source, the job
     remains unwitnessed, no hall routing and zero material credit.
     A second same-role villager nearby cannot substitute as a witness.
238. After witnessing, inspect durable world JSON (pickup_witnessed=true)
     and prove that the same worker is NOW routed toward the real
     town-hall receiving area. A different worker already at the hall
     must not credit the pending resource.
239. After the correct worker actually reaches the hall in X/Y/Z,
     exactly ONE wood/stone appears in the confirmed physical queue.
     Repeat update ticks and reload to prove no duplicate credit.
240. Kill/unload/change the worker identity after native edit but before
     source witness. The job waits or remains quarantined with zero
     credit; never move or grant material based only on an air block.
241. Stop/restart between Issued and source witness and again after
     witness save before hall delivery. Confirm no blind native
     block-edit retries and same durable worker, source/cursor and
     pickup state after both crashes.
242. For a 3-4-log native oak, confirm EVERY next saved log has same
     resident ID but resets issued=false and pickup_witnessed=false.
     Repeat complete source-and-hall evidence for each log separately.
243. Load a legacy saved Issued job missing pickup_witnessed: the
     field defaults false. The worker must first revisit loaded native
     source and witness air, not automatically grant stock at hall.
     Legacy unassigned issued receipts remain OP-quarantined.
244. Modify source/stand after issue, obstruct NPC route, test
     multiple villages/dimensions and OP abandon. No foreign
     block replacement, unstaged virtual inventory or silent
     recovery. Collect logs, source block/actor UUID checks and
     actual barrel item counts. This proves a receipt witness only,
     NOT a visually held Minecraft ItemStack.

## Native edit crash-window + multi-village fairness acceptance (unreleased)

245. Check out exact latest stacked branch in a Windows machine with
     Rust 1.95, Python 3.11+, Git and network. Run rustfmt, cargo
     check/test, Clippy, all Python tests and pinned native Horizon
     compile+game client. Log exact commit SHA and all first failures.
246. Create two valid settlements in the same dimension, each with
     different active verified physical gathers and loaded UUID workers.
     Stall the first village's worker far from its return hall. Confirm
     the second village continues its own log/stone work within multiple
     8-tick cycles instead of starving on the first job.
247. Use 3 settlements and a mixture of Wood/Stone with one blocked
     and one unloaded source; blocked/unloaded jobs must be skipped.
     Check loadable candidates receive roughly even opportunities as
     observed from server logs and saved receipts; no native mutations
     in unloaded chunks or wrong dimensions.
248. Plan, bind worker, persist Issued. Before command returns
     deliberately interrupt process. Reload with original block
     intact. The Issued but unconfirmed job is quarantined and cannot
     retry automatically or credit even if worker arrives at the hall.
249. Repeat interrupt immediately after native block edit but before
     native_edit_confirmed persistence. Reload with native block
     already air. No reissue and ZERO automatic material credit,
     even after the assigned worker visits source and hall. Operator
     sees uncertain receipt and can abandon after real inspection.
250. After successful native conditional command, verify a new
     authoritative read sees exactly air, then that
     native_edit_confirmed=true is committed durably BEFORE pickup
     witness. Stop/reload at this boundary and resume source witness,
     then separate 3D hall return without a second block command.
251. Induce native command refusal, conditional no-op (source changed
     between preflight and edit) and unloaded result. Each must block
     without granting item, must not blindly retry or treat command
     acceptance alone as proof the original log was removed.
252. Load an old PR #30 save with an Issued gather whose
     pickup_witnessed=true but native_edit_confirmed is missing.
     The whole settlement must LOAD, with zero new stock; the issued
     job is quarantined automatically and requires manual OP audit.
253. For a 3–4-log natural oak, verify every next saved segment resets
     native_edit_confirmed=false and pickup_witnessed=false while
     retaining the assigned resident ID. Each block independently
     requires native success, live air, source witness and hall return.
254. Repeat across world restart, different dimension, night shift,
     other same-role workers at the hall, downloaded/unloaded chunk,
     corrupted journal data, and warehouse maximum capacity.
     Document native log, block entity count, actual worker UUID and
     save contents. This is NOT proof of rendered carried ItemStacks.

## Local first-play gate harness and native/game readiness (unreleased)

255. On current branch, `py -3 tools/first_play_gate.py --dry-run
     --build-client` must show the exact commands with no writes
     to source, network, or output evidence. Check pinned revisions.
256. On clean exact branch, `py -3 tools/first_play_gate.py --build-client`
     must record SHA, Rust 1.95, Git branch, command logs, elapsed
     stage durations and PASS/failure for all source and native steps.
257. If a real Cargo/Rustfmt/Clippy/Python test fails, gate must STOP
     immediately, return nonzero, and retain report.json, report.md,
     stage log and first blocker. Do not misreport ready or playable.
258. If Git branch or expected SHA does not match, or source files
     are dirty, refuse release evidence. `--allow-dirty` may still
     collect diagnostic logs but must NEVER produce a passing gate.
259. Missing Rust/Python prerequisites must be reported, not silently
     skipped. Generated target/, Python caches and compiled files
     must not turn a clean checkout into dirty or alter source.
260. With successful native client compilation, `READY_FOR_GAMEPLAY_E2E`
     is allowed only as *ready to launch*, NOT a gameplay PASS. Then
     manually run the original physical village/multi-study equivalent
     scenarios (NPC spawn, hall, wood/stone, native barrel, bakery,
     reload and crash reconciliation) in a disposable MC world;
     record game logs and original/native item counts.
261. Execute `python -m unittest tests/test_first_play_gate_cli.py`
     (or discover) with controlled fake executable stubs. These verify
     only release-gate correctness; on Linux 7/7 PASS. Repeat on
     Windows for platform parity, then native-gate with REAL tools.

## Native executable attestation, gate correctness and first-play evidence

262. From exact current branch execute the local gate with
     `--build-client`. On true success, report.json MUST list
     pinned native Horizon revision, native check verified,
     expected client executable path, actual byte count and SHA-256.
263. Simulate/observe native build command exits 0 but compiled
     Horizon executable is missing or empty. Gate MUST become
     BLOCKED with first blocker describing absent binary; no
     READY_FOR_GAMEPLAY_E2E false positives.
264. Native build JSON with wrong horizon_revision, missing
     cargo_check=PASS, missing modded path or wrong client output
     path MUST fail even with native command exit 0.
265. `--native-check` requires pinned native Cargo check and
     valid build report but NOT a client executable. Source-only
     gate is explicitly a weaker assurance level.
266. Confirm report.md/report.json preserve exact SHA, branch,
     stage output and SHA-256 of a genuinely fresh compiled
     client. Never substitute a fabricated/hash-only artifact
     for a Rust/native compilation pass.
267. Run `py -3 -m unittest discover -s tests
     -p test_first_play_gate_cli.py -v` locally on Windows:
     the ten mock harness tests must PASS; these never certify
     actual Rust or gameplay. Then run the full local gate
     with REAL Rust 1.95 and owned Minecraft Java resources
     in a disposable instance, followed by the prior in-game
     E2E and integrity scenarios.

## Compile-first local release gate (revision of native acceptance)

268. Run `py -3 tools/first_play_gate.py --dry-run --build-client`
     on the latest branch. Verify `cargo-test` executes before
     Clippy, then strict rustfmt/cargo fmt checks and native build.
269. Simulate a Rust compile error AND an unformatted source:
     the first stage and first blocker must be cargo-test, not fmt.
270. Simulate cargo passing but cargo fmt failing: no
     SOURCE_CHECK_PASS or READY_FOR_GAMEPLAY_E2E is allowed.
271. With all real source stages passing, the previous strict native
     client executable, SHA-256 and manual game-world gates still apply.
     This ordering improvement is not native compilation evidence.

## Partial village spawn, UUID ownership and guarded recovery (unreleased)

272. Use the current stacked branch with real local `cargo test`,
     full Python unittest, native Horizon build and a fresh
     disposable world. Record commit SHA, binary hash and logs.
273. Create a village on natural loaded ground. Confirm all ten
     initial NPCs receive distinct saved UUIDs. Stop/restart;
     no duplicate batch. `/civ roster NAME` displays resident IDs.
274. Simulate native refusal halfway through the first ten actor
     spawns. `actors_spawned` remains reserved, with only successfully
     UUID-linked residents saved; no automatic duplicate respawn.
275. OP: `/civ reconcile NAME` before any recovery. If an original
     tagged or legacy villager still exists, bind it once by UUID.
     The existing NPC should remain intact and no new entity spawn.
276. On an actually missing resident, use `/civ roster NAME` to
     identify its numeric ID. Ensure the 96-block native neighborhood
     is fully loaded and no matching entity exists. Run
     `/civ recoveractor NAME RESIDENT_ID attempt` within 32 blocks
     of the center, as OP level 2, in the correct dimension.
277. The recovery command first persists `actor_recovery_issued`,
     then issues a SINGLE native villager spawn at a genuinely clear
     natural ground position, reads the real UUID, persists binding
     and removes pending marker. Verify no other resident changes.
278. Crash AFTER reservation but BEFORE native spawn. Reload
     and verify a repeated recovery call is refused, no native
     duplicate appears and original linked NPCs still work.
279. Crash AFTER entity creation but BEFORE UUID save.
     Do NOT respawn. Load the same chunk, run OP reconcile;
     the unique original named villager binds to its real UUID
     and clears the pending marker without creating another NPC.
280. Block chunks in the 96-block scan, wrong dimension,
     non-OP player, >32-block operator distance, missing
     'attempt' confirmation or occupied site. Each refusal must
     occur BEFORE saving recovery reservation or native spawning.
281. Attempt recovery on already linked, dead, unknown or
     previously reserved resident ID. No second native entity is
     created. A saved invalid pending ID must fail world validation.
282. Confirm ordinary village autonomy still starts with ten safe
     surveyed positions, and 1–9 safe positions never starts a
     new full batch. Single-target recovery can use one safe
     unoccupied natural spot after explicit OP confirmation.
     Re-run NPC builder, farm, warehouse and restarts. No game
     readiness claim from source-contract checks alone.
283. After reserving recovery, mark the resident dead before restart. The saved one-shot attempt must remain loadable and in the audit; the dead resident cannot be newly spawned or reserved again. The entire village must not fail validation.


## Shared surveyed initial native spawning (unreleased PR #34)

284. In a disposable world, obstruct the ten initial natural spawn spots
     or unload surrounding chunks. Check that automatic initialization and
     OP `/civ actors NAME` create **zero** entities and do **not** reserve
     `actors_spawned`. Restore terrain/chunks and retry the pre-spawn survey.
285. Place a living mob on an otherwise safe candidate position. Both
     startup paths must skip occupied positions and only reserve/spawn if
     ten other safe, loaded, natural-ground spots exist.
286. Before first reservation, mark one of the first ten residents dead,
     give one a UUID, or insert an already-loaded villager with its exact
     tag or legacy name. The shared preflight must fail, with no reservation
     and no duplicate or dead-resident spawn.
287. On terrain with varying Y height, execute explicit OP
     `/civ actors NAME`. All ten entities must appear at surveyed
     positions, not the old hard-coded central Y. Verify ten unique UUIDs,
     save/reload, and no repeated batch. Native tests and in-game evidence
     are required; source assertions do not prove gameplay.

288. On the current PR #34 branch, run
     `py -3 tools/first_play_gate.py --dry-run --build-client` and then
     the real non-dry-run command. It must identify the current branch
     rather than the superseded PR #33 branch. With `--expected-sha`,
     only the exact chosen head may produce a passing build report.


## Crash-safe spawn spacing, save consistency, first error reporting (unreleased PR #34)

289. On loaded flat natural terrain, the first preflight finds ten
     non-overlapping positions at least three blocks apart. Confirm
     native UUID-bound villagers do not collide with one another,
     previously loaded mobs or blocked ground. Reload without duplicates.
290. Force a save error during OP `/civ reconcile NAME` after UUID
     binding in memory. Verify the dispatcher does not silently retry
     the failed save or replace in-memory reservations with a stale
     snapshot. The world must halt until the operator repairs storage.
291. Fault-inject an error after a previously successful durable
     command save. The dispatcher must never restore an older clone
     over the committed `world.json`; it instead halts for inspection.
     Never use valuable saves when testing fault injection.
292. In the first-play CLI mock, make the Rust test stage emit
     `error[E0425]: unknown value in native actor code` and exit 101.
     `report.json`, `report.md` and console must surface that first
     diagnostic and preserve `BLOCKED`. The complete stage log remains
     available. Repeat using real Rust error logs when the actual
     Windows build is run.


## Aborted first save / temporary recovery artifacts (unreleased PR #34)

293. On a disposable world, create only `world.json.tmp` containing
     incomplete settlement JSON. With no `world.json` primary, starting
     Horizon Civilizations must **NOT** initialize an empty world. It must
     show an interrupted-save error and leave the temp bytes untouched.
294. Leave a valid `world.json` alongside stale `world.json.tmp`.
     The loader must refuse to proceed until the operator archives and
     inspects both files; it must not select one silently.
295. With stale `world.json.tmp`, attempt an ordinary world save.
     The write must fail BEFORE opening/truncating the temp file; compare
     exact original bytes. Existing `world.json.swap` protection must remain.
296. Close/reopen after a fully successful save: neither temp nor swap
     file should remain, and the identical settlement/UUID bindings should
     reload. Record Rust and real Minecraft evidence before release.


## Native Rust suite inventory and zero-match protection (unreleased PR #34)

297. Run pinned `tools/native_build.py --test` with real Rust and the
     exact Horizon revision. Both `civilizations_core::` and
     `civilizations_glue::` must enumerate at least one Rust test and
     pass their actual filtered executions; evidence must show counts.
298. Simulate a Cargo `--list` success returning `0 tests, 0 benchmarks`
     for `civilizations_glue::`. It must block native readiness **before**
     running that empty suite; an exit code of zero alone is insufficient.
299. Simulate failure of the Cargo test inventory command and ensure
     that no filtered tests are launched afterward. Preserve the
     first-failure log in the first-play report.
300. Simulate native-build exit zero with a valid-looking client binary
     but missing `civilizations_glue::` evidence, or evidence with a zero
     discovered count. Both must produce `BLOCKED`, not
     `READY_FOR_GAMEPLAY_E2E`. Only a real native successful run can
     certify the build gate, not these mocks.


## Windows launcher sequencing and read-only NPC recovery evidence (unreleased PR #34)

301. On Windows PowerShell with the official GUI Horizon Launcher, verify
     that `import-build` finishes and provides an exit code before
     `prepare` runs. No unpiped GUI invocation may be treated as success.
302. Force `import-build` to exit nonzero; `prepare` and `play` must not
     run. Force `prepare` to exit nonzero; `play` must not run.
     Capture the exit status without running a valuable Minecraft world.
303. In read-only `playtest_evidence.py`, leave `world.json.tmp`
     alongside an otherwise valid save, and then as the ONLY remaining
     first-save artifact. Both must yield FAIL without editing either file.
304. Supply a valid, UUID-unbound `actor_recovery_issued` entry for a live
     resident. The saved-state audit reports pending recovery as a
     warning and requires reconciliation before any further spawn.
     A marker for a UUID-bound or nonexistent resident is a hard FAIL.
305. A dead, unbound resident with a pending one-shot recovery tombstone
     must remain loadable and must NOT be treated as a new NPC to respawn.
     The audit reports the unresolved marker, not false corruption.
306. Confirm that the current PLAYTEST guide selects the same Draft PR #34
     source as `first_play_gate.py`. Do not reuse a passing old binary;
     native build and in-game E2E evidence remain mandatory.
