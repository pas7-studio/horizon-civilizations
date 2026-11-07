// Horizon adapter: mutating /civ commands, split out of `perform`.
use super::*;

/// Applies one mutating /civ command to the live world. The world stays
/// untouched on error; `perform` decides whether to save and how to roll back.
pub(super) fn mutate(
    game: &mut Game<'_>,
    player: &NativeServerPlayer,
    state: &mut ActiveWorld,
    previous: &World,
    action: &str,
    actor: &str,
    args: &mut std::str::SplitWhitespace<'_>,
) -> (Result<String, String>, bool) {
    let mut already_saved = false;
    let result: Result<String, String> = match action {
        "autonomy" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ autonomy <village> on|off")?;
            let mode = args.next().ok_or("specify on or off")?;
            let enabled = match mode {
                "on" => true, "off" => false,
                _ => return Err("use on or off".into()),
            };
            let v = previous.by_name(name).ok_or("unknown settlement")?;
            state.world.as_mut().unwrap().set_autonomy(v.id, enabled)?;
            Ok(format!(
                "{} autonomous physical development: {}. All villages start ON by default; off pauses new plans but not previously committed jobs.",
                v.name, if enabled { "ON" } else { "OFF" }
            ))
        })(),
        "buyplot" => (|| {
            let name = args.next().ok_or("usage: /civ buyplot <village>")?;
            let village = previous.by_name(name).ok_or("unknown settlement")?;
            if village.dimension != player.dimension {
                return Err("purchase the plot from within its settlement dimension".into());
            }
            if !player.position.iter().all(|x| x.is_finite()) {
                return Err("invalid player position".into());
            }
            let origin = player.position.map(|x| x.floor() as i32);
            // A land purchase is committed as a real persistent parcel; the
            // world must have a clear loaded 5x5 footprint at purchase time.
            check_empty_hut_site(game, &village.dimension, origin)?;
            let price = state.world.as_mut().unwrap().buy_plot(actor, village.id, origin)?;
            Ok(format!(
                "Bought a protected 5x5 plot at {:?} in {} for {} civic coins. Build your own blocks there or use /civ commission {} while standing on this lot. /civ plots {}",
                origin, village.name, price, village.name, village.name
            ))
        })(),
        "buyhouse" => (|| {
            let name = args.next().ok_or("usage: /civ buyhouse <village>")?;
            let village = previous.by_name(name).ok_or("unknown settlement")?;
            if village.dimension != player.dimension {
                return Err("buy a house from its settlement dimension".into());
            }
            if !player.position.iter().all(|x| x.is_finite()) {
                return Err("invalid player position".into());
            }
            let mut available: Vec<_> = village.visual_sites.iter()
                .filter(|site| !village.player_parcels.iter().any(|plot| {
                    crate::game::civilizations_core::near_plot(plot.origin, **site)
                }))
                .filter_map(|site| {
                    let dx = f64::from(site[0]) + 0.5 - player.position[0];
                    let dz = f64::from(site[2]) + 0.5 - player.position[2];
                    let dy = (f64::from(site[1]) - player.position[1]).abs();
                    (dx * dx + dz * dz <= 12.0 * 12.0 && dy <= 8.0)
                        .then_some((dx * dx + dz * dz, *site))
                }).collect();
            available.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (_, origin) = available.first().copied().ok_or(
                "stand beside an available, completed village hut within 12 blocks"
            )?;
            // A JSON registry alone does not prove a real structure exists.
            // Confirm EVERY hut block in the loaded world, no substitutions.
            let inspection = inspect_hut(&VisualBuildJob {
                origin, cursor: hut_plan(origin).len(), blocked: false
            }, |pos| world_block(game, &village.dimension, pos));
            if inspection != BuildInspection::Complete {
                return Err("hut has missing/unloaded/modified blocks; no payment taken".into());
            }
            let price = state.world.as_mut().unwrap().buy_finished_house(actor, village.id, origin)?;
            Ok(format!(
                "Purchased the physical hut at {:?} from {} for {} civic coins. It is now YOUR property and no longer public NPC housing. /civ plots {}",
                origin, village.name, price, village.name
            ))
        })(),
        "commission" => (|| {
            let name = args.next().ok_or("usage: /civ commission <village>")?;
            let village = previous.by_name(name).ok_or("unknown settlement")?;
            if village.dimension != player.dimension {
                return Err("commission construction in the settlement's own dimension".into());
            }
            if !player.position.iter().all(|x| x.is_finite()) {
                return Err("invalid player position".into());
            }
            let mut eligible = village.player_parcels.iter()
                .filter(|plot| plot.owner == actor && !plot.commissioned)
                .filter_map(|plot| {
                    let dx = f64::from(plot.origin[0]) + 0.5 - player.position[0];
                    let dz = f64::from(plot.origin[2]) + 0.5 - player.position[2];
                    let dy = (f64::from(plot.origin[1]) - player.position[1]).abs();
                    (dx * dx + dz * dz <= 6.0 * 6.0 && dy <= 6.0)
                        .then_some((dx * dx + dz * dz, plot.origin))
                }).collect::<Vec<_>>();
            eligible.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (_, site) = eligible.first().copied().ok_or(
                "stand on a bought, uncommissioned plot; use /civ plots <village>"
            )?;
            check_empty_hut_site(game, &village.dimension, site)?;
            let price = state.world.as_mut().unwrap().commission_house(actor, village.id, site)?;
            Ok(format!(
                "Commissioned your own 5x5 hut at {:?} for {} civic coins (labor fee). A UUID-bound builder will collect 20 actual oak logs and 25 actual cobblestone from the hall barrels before physically building. /civ warehouse {}",
                site, price, village.name
            ))
        })(),
        "discardgather" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ discardgather <village> intact|empty")?;
            let evidence = args.next().ok_or("confirm actual observed block: intact or empty")?;
            if args.next().is_some() {
                return Err("unexpected arguments for gather abandonment".into());
            }
            let v = previous.by_name(name).ok_or("unknown settlement")?;
            if v.dimension != player.dimension {
                return Err("inspect resource inside its own settlement dimension".into());
            }
            let job = v.gather_job.as_ref().ok_or("no gather receipt")?;
            if !job.blocked || !job.issued {
                return Err("only a blocked ISSUED gather can be manually abandoned".into());
            }
            let observed = world_block(game, &v.dimension, job.position)
                .ok_or("original native source block unavailable; load the chunk")?;
            let matches_evidence = match evidence {
                "intact" => observed == job.source_block(),
                "empty" => observed == "minecraft:air",
                _ => return Err("use intact or empty after /civ gathercheck".into()),
            };
            if !matches_evidence {
                return Err("native block changed since audit; gather receipt still quarantined".into());
            }
            state.world.as_mut().unwrap().abandon_blocked_issued_gather(v.id)?;
            Ok(format!(
                "{}: abandoned uncertain {:?} gather with {} source observed. ZERO resource credit. New physical jobs may be planned after the next tick.",
                v.name, job.good, evidence
            ))
        })(),
        "resolvebarrel" if player.permission >= 2 => (|| {
            let name = args.next().ok_or(
                "usage: /civ resolvebarrel <village> confirm|cancel <observed_count>"
            )?;
            let mode = args.next().ok_or("specify confirm or cancel")?;
            let manually_observed = args.next().ok_or(
                "type exact observed native item count after /civ barrelcheck"
            )?.parse::<u32>().map_err(|_| "invalid native item count")?;
            if args.next().is_some() {
                return Err("unexpected arguments in native recovery command".into());
            }
            let v = previous.by_name(name).ok_or("unknown village")?;
            if v.dimension != player.dimension {
                return Err("load the official village warehouse in its own dimension".into());
            }
            if v.market_purchase.is_some() {
                return Err("unresolved player market purchase; never reconcile two journals together".into());
            }
            let t = v.barrel_transfer.as_ref().ok_or("no warehouse transfer to recover")?;
            let actual = plain_barrel_count(game, &v.dimension, t.position, t.good)
                .ok_or("official native barrel is unavailable/unloaded or invalid")?;
            if actual != manually_observed {
                return Err("native barrel changed since manual inspection; inspect again".into());
            }
            match mode {
                "cancel" => {
                    if t.phase != BarrelPhase::Planned || actual != t.before {
                        return Err("only a never-issued Planned intent with unchanged native count may be canceled".into());
                    }
                    state.world.as_mut().unwrap().cancel_planned_barrel_transfer(v.id)?;
                    Ok(format!(
                        "{}: canceled an UNISSUED native barrel intent; no items changed. Check /civ warehouse {}.",
                        v.name, v.name
                    ))
                }
                "confirm" => {
                    if t.phase != BarrelPhase::Blocked
                        || t.expected_after() != Some(actual) {
                        return Err("only a Blocked intent matching exact native after-count can be confirmed".into());
                    }
                    state.world.as_mut().unwrap()
                        .reconcile_blocked_barrel_transfer(v.id, actual)?;
                    Ok(format!(
                        "{}: reconciled BLOCKED native receipt, without repeating extraction or insertion. Recheck /civ warehouse {}.",
                        v.name, v.name
                    ))
                }
                _ => Err("use confirm or cancel, with exact observed count".into()),
            }
        })(),
        "recoverfarm" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ recoverfarm <name>")?;
            let settlement = previous.by_name(name).ok_or("unknown settlement")?;
            if settlement.dimension != player.dimension {
                return Err("enter the settlement dimension".into());
            }
            let mut farm = settlement.farm.clone().ok_or("no farm to recover")?;
            if !farm.blocked {
                return Err("farm is not blocked".into());
            }
            // Preview the full previously placed prefix/current block with
            // the blocked flag cleared, against already-loaded real blocks.
            // Simply flipping the flag without this check would be unsafe.
            farm.blocked = false;
            match inspect_farm(&farm, |pos| world_farm_block(game, &settlement.dimension, pos)) {
                FarmDecision::Unloaded => return Err("load the full field first".into()),
                FarmDecision::InvalidCursor => return Err("invalid saved farm cursor".into()),
                FarmDecision::Blocked { position, found } => {
                    return Err(format!("repair foreign block at {position:?}: {found} before recovery"));
                }
                _ => {}
            }
            state.world.as_mut().unwrap().recover_farm(settlement.id)?;
            Ok(format!("Verified loaded field and resumed {name}. Farmer will continue on the next authoritative level tick."))
        })(),
        "farm" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ farm <name>")?;
            let settlement = previous.by_name(name).ok_or("unknown settlement")?;
            if settlement.dimension != player.dimension {
                return Err("enter the settlement dimension".into());
            }
            let origin = player.position.map(|value| value.floor() as i32);
            check_empty_farm_site(game, &settlement.dimension, origin)?;
            state.world.as_mut().unwrap().start_farm(settlement.id, origin)?;
            Ok(format!(
                "Approved 3x3 irrigated crop field at {origin:?}. UUID-bound farmers work when loaded and near the site. /civ farming {name}"
            ))
        })(),
        "approvehouse" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ approvehouse <name> <slot> <y>")?;
            let slot = args.next().ok_or("slot number required")?
                .parse::<usize>().map_err(|_| "invalid slot number")?;
            let y = args.next().ok_or("explicit Y level required")?
                .parse::<i32>().map_err(|_| "invalid Y level")?;
            let village = previous.by_name(name).ok_or("unknown settlement")?;
            if village.dimension != player.dimension {
                return Err("visit the settlement dimension first".into());
            }
            // Proposal is recomputed from the current saved settlement state;
            // it never holds a durable or implicit plot claim.
            let site = previous.proposed_hut(village.id, slot, y)?;
            let px = player.position[0];
            let py = player.position[1];
            let pz = player.position[2];
            if !px.is_finite() || !py.is_finite() || !pz.is_finite() {
                return Err("invalid player location".into());
            }
            let dx = f64::from(site[0]) + 0.5 - px;
            let dz = f64::from(site[2]) + 0.5 - pz;
            if dx * dx + dz * dz > 24.0 * 24.0 || (f64::from(y) - py).abs() > 8.0 {
                return Err("stand within 24 horizontal blocks and 8 vertical blocks of the chosen plot".into());
            }
            // Revalidate *all* live blocks immediately before the approval is
            // persisted. Later placement rechecks each expected old block.
            check_empty_hut_site(game, &village.dimension, site)?;
            state.world.as_mut().unwrap().start_visual_house(village.id, site)?;
            Ok(format!(
                "Approved {} proposal #{slot} at {:?}. One durable project reserved; UUID-linked builders can work during daytime when loaded. /civ construction {}",
                village.name, site, village.name
            ))
        })(),
        "post" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ post <name> lumber|quarry|sand|dirt")?;
            let kind = PostKind::parse(args.next().ok_or("kind: lumber, quarry, sand or dirt")?)
                .ok_or("kind must be lumber, quarry, sand or dirt")?;
            let s = previous.by_name(name).ok_or("unknown settlement")?;
            if s.dimension != player.dimension {
                return Err("enter the settlement dimension".into());
            }
            let site = player.position.map(|v| v.floor() as i32);
            let below = world_block(game, &s.dimension, [site[0], site[1] - 1, site[2]])
                .ok_or("camp site not loaded")?;
            if !is_ground(&below) {
                return Err(format!("stand on natural ground, found {below}"));
            }
            let at = world_block(game, &s.dimension, site).ok_or("camp site not loaded")?;
            if at != "minecraft:air" {
                return Err(format!("camp cell is blocked by {at}"));
            }
            state.world.as_mut().unwrap().add_post(s.id, kind, site)?;
            Ok(format!(
                "{} {} camp reserved at {:?}; workers gather within {} blocks. Use /civ posts {}.",
                s.name, kind.name(), site, POST_RADIUS, name
            ))
        })(),
        "construct" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ construct <name>")?;
            let s = previous.by_name(name).ok_or("unknown settlement")?;
            if s.dimension != player.dimension { return Err("enter the settlement dimension".into()); }
            let site = player.position.map(|v| v.floor() as i32);
            check_empty_hut_site(game, &s.dimension, site)?;
            state.world.as_mut().unwrap().start_visual_house(s.id, site)?;
            Ok(format!("Approved hut at {:?}. A summoned builder will place blocks while chunks remain loaded. Use /civ construction {name}.", site))
        })(),
        "found" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ found <name>")?;
            let center = player.position.map(|v| v.floor() as i32);
            let id = state.world.as_mut().unwrap().found(name, &player.dimension, center, actor)?;
            Ok(format!("Founded {name} (#{id}). Autonomous NPC spawning and town hall planning start by default. The first civic hall needs 24 real logs and 25 cobblestone; /civ hall {name} and /civ warehouse {name} show progress."))
        })(),
        "promote" => (|| {
            let rank = state.world.as_mut().unwrap().promote(actor)?;
            Ok(format!("Promoted to {}", rank.name()))
        })(),
        "join" => (|| {
            let name = args.next().ok_or("usage: /civ join <name>")?;
            let id = previous.by_name(name).ok_or("unknown settlement")?.id;
            state.world.as_mut().unwrap().join(actor, id)?;
            Ok(format!("Joined {name}. /civ title shows your balance and rank."))
        })(),
        "day" if player.permission >= 2 => (|| {
            let days = args.next().unwrap_or("1").parse::<u32>().map_err(|_| "invalid number")?;
            if days == 0 || days > MAX_SIM_DAYS { return Err("days must be 1..30".into()); }
            state.world.as_mut().unwrap().advance(days)?;
            Ok(format!("Advanced {days} days. /civ info and /civ history show the result."))
        })(),
        "war" | "peace" if player.permission >= 2 => (|| {
            let from = args.next().ok_or("first settlement required")?;
            let to = args.next().ok_or("second settlement required")?;
            let a = previous.by_name(from).ok_or("unknown first settlement")?.id;
            let b = previous.by_name(to).ok_or("unknown second settlement")?.id;
            if action == "war" { state.world.as_mut().unwrap().declare_war(a, b)?; }
            else { state.world.as_mut().unwrap().make_peace(a, b)?; }
            Ok(format!("{}: {from} / {to} (abstract strategic simulation)", action))
        })(),
        "sell" | "buy" => (|| {
            if player.permission < 2 {
                return Err("Villagers trade in person: talk to the mayor or the baker (right-click) and use their Trade menu.".into());
            }
            let good = Good::parse(args.next().ok_or("good required")?).ok_or("unknown good")?;
            let count = parse_amount(args.next())?;
            let village_id = own_settlement(&previous, actor)?;
            let village = &previous.settlements[&village_id];
            if village.dimension != player.dimension {
                return Err("trade from within the settlement dimension".into());
            }
            if !player.position.iter().all(|component| component.is_finite()) {
                return Err("invalid player trading location".into());
            }
            let dx = player.position[0] - f64::from(village.center[0]);
            let dz = player.position[2] - f64::from(village.center[2]);
            if dx * dx + dz * dz > 12.0 * 12.0
                || (player.position[1] - f64::from(village.center[1])).abs() > 8.0 {
                return Err("trade within 12 blocks of the actual village hall".into());
            }
            let item = good.item();
            if action == "sell" {
                if game.native_count_items(item) < count { return Err(format!("you need {count} {item}")); }
                let payout = state.world.as_mut().unwrap().sell(actor, village_id, good, count)?;
                let taken = game.native_take_items(item, count);
                if taken != count {
                    if taken > 0 { game.native_give_items(item, taken); }
                    return Err("inventory changed during trade".into());
                }
                if let Err(e) = save(state) {
                    let refund = game.native_give_items(item, count);
                    log::error!("[civilizations] trade save failed, restored {refund}/{count} items: {e}");
                    return Err(format!("trade not committed: {e}"));
                }
                already_saved = true;
                Ok(format!("Sold {count} {} for {payout} coins", good.as_str()))
            } else if previous.settlements[&village_id].hall_complete {
                // After the hall is built, the source of truth for a shop
                // purchase is an ACTUAL native vanilla barrel slot.
                buy_from_real_barrel(
                    game, state, actor, village_id, good, count,
                    player.position, &mut already_saved,
                )
            } else {
                if game.native_room_for(item) < count { return Err("not enough inventory space".into()); }
                let cost = state.world.as_mut().unwrap().buy(actor, village_id, good, count)?;
                let received = game.native_give_items(item, count);
                if received != count {
                    if received > 0 { game.native_take_items(item, received); }
                    return Err("inventory changed during trade".into());
                }
                if let Err(e) = save(state) {
                    let removed = game.native_take_items(item, count);
                    log::error!("[civilizations] trade save failed, removed {removed}/{count} items: {e}");
                    return Err(format!("trade not committed: {e}"));
                }
                already_saved = true;
                Ok(format!("Bought {count} {} for {cost} coins", good.as_str()))
            }
        })(),
        "reconcile" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ reconcile <name>")?;
            let village = previous.by_name(name).ok_or("unknown settlement")?;
            if !village.actors_spawned {
                return Err("spawn village actors first".into());
            }
            if village.dimension != player.dimension {
                return Err("visit the settlement dimension first".into());
            }
            let mut linked = 0usize;
            let mut ambiguous = 0usize;
            for resident in village.residents.iter().filter(|r| r.alive && r.entity_uuid.is_none()) {
                let expected = village.actor_name(resident);
                let legacy = format!("{} [{}]", resident.name, village.name);
                let mut candidates = game.mobs.iter().filter(|mob| {
                    let dx = mob.position.x - f64::from(village.center[0]);
                    let dz = mob.position.z - f64::from(village.center[2]);
                    mob.alive() && mob.kind.id == "minecraft:villager"
                        && dx * dx + dz * dz < 96.0 * 96.0
                        && (mob.custom_name.as_deref() == Some(expected.as_str())
                            || mob.custom_name.as_deref() == Some(legacy.as_str()))
                }).map(|mob| mob.id);
                let Some(entity_id) = candidates.next() else {
                    continue;
                };
                if candidates.next().is_some() {
                    ambiguous += 1;
                    continue;
                }
                let Some(handle) = game.native_entity(entity_id) else {
                    continue;
                };
                let Some(uuid) = game.native_entity_uuid(handle) else {
                    continue;
                };
                // An ambiguous/foreign UUID is never silently re-assigned.
                if state.world.as_mut().unwrap().bind_actor(village.id, resident.id, uuid).is_err() {
                    ambiguous += 1;
                    continue;
                }
                if let Err(e) = save(state) {
                    // A native UUID has already been bound in memory. Do not
                    // let the outer command handler retry this failed save,
                    // or restore an older clone over a partially committed
                    // world.json. Stop writes until disk recovery.
                    already_saved = true;
                    state.problem = Some(format!("actor identity save halted: {e}"));
                    return Ok(format!("Reconciled {linked} actors before save failure; world halted. Restore backup and restart. {e}"));
                }
                already_saved = true;
                linked += 1;
            }
            Ok(format!("Reconciled {linked} legacy actors; {ambiguous} ambiguous/conflicting skipped. /civ roster {name}. Missing actors are not spawned automatically."))
        })(),
        "recoveractor" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ recoveractor <name> <resident-id> attempt")?;
            let resident_id = args.next().ok_or("resident ID required; /civ roster <name>")?
                .parse::<u64>().map_err(|_| "invalid resident ID")?;
            if args.next() != Some("attempt") || args.next().is_some() {
                return Err("explicit OP confirmation required: /civ recoveractor <name> <resident-id> attempt".into());
            }
            let village = previous.by_name(name).ok_or("unknown settlement")?;
            if !village.actors_spawned {
                return Err("initial actor batch has not yet been reserved".into());
            }
            if village.dimension != player.dimension {
                return Err("go to this settlement dimension first".into());
            }
            let dx = i64::from(village.center[0]) - player.position[0].floor() as i64;
            let dz = i64::from(village.center[2]) - player.position[2].floor() as i64;
            if dx * dx + dz * dz > 32 * 32 {
                return Err("stand within 32 blocks of the settlement center".into());
            }
            let resident = village.residents.iter()
                .find(|r| r.id == resident_id && r.alive && r.entity_uuid.is_none())
                .ok_or("resident missing, dead or already UUID-linked; inspect /civ roster")?;
            if village.actor_recovery_issued.contains(&resident_id) {
                return Err("a recovery spawn was already issued for this resident; inspect and reconcile before any further action".into());
            }
            // All native chunks inside the normal 96-block identity
            // neighborhood must be loaded. An unseen older actor in an
            // unloaded chunk would make even an OP recovery duplicate it.
            // This is intentionally strict; return when chunks are loaded.
            for ox in (-96_i32..=96).step_by(16) {
                for oz in (-96_i32..=96).step_by(16) {
                    let x = village.center[0].checked_add(ox)
                        .ok_or("spawn recovery scan outside world bounds")?;
                    let z = village.center[2].checked_add(oz)
                        .ok_or("spawn recovery scan outside world bounds")?;
                    if !game.world.loaded(x, z) {
                        return Err("native villager recovery needs all chunks in 96-block scan loaded; no spawn was reserved".into());
                    }
                }
            }
            let exact_name = village.actor_name(resident);
            let legacy_name = format!("{} [{}]", resident.name, village.name);
            // Never spawn while a tagged old entity is visible in the
            // current loaded native dimension, even if its UUID was lost.
            if game.mobs.iter().any(|mob|
                mob.alive() && mob.kind.id == "minecraft:villager"
                    && (mob.custom_name.as_deref() == Some(exact_name.as_str())
                        || mob.custom_name.as_deref() == Some(legacy_name.as_str()))) {
                return Err("matching native actor is already loaded: use /civ reconcile, do not spawn a duplicate".into());
            }
            // Recover only at surveyed natural ground with two blocks
            // of air, never on occupied NPC coordinates.
            let position = safe_actor_spawn_points(game, village).into_iter()
                .find(|point| !game.mobs.iter().any(|mob| {
                    let dx = mob.position.x - point[0];
                    let dy = mob.position.y - point[1];
                    let dz = mob.position.z - point[2];
                    mob.alive() && dx * dx + dy * dy + dz * dz < 9.0
                })).ok_or("no safe unoccupied loaded actor spawn point")?;
            // Durable at-most-once reservation BEFORE native side effects.
            state.world.as_mut().unwrap().reserve_actor_recovery(village.id, resident_id)?;
            save(state)?;
            already_saved = true;
            let options = NativeSpawn {
                name: Some(exact_name), ..NativeSpawn::default()
            };
            let Some(entity_id) = game.native_spawn_mob(
                &village.dimension, "minecraft:villager", position, &options
            ) else {
                return Ok(format!(
                    "One recovery attempt was reserved for #{resident_id} but native spawn was refused. Do NOT retry blindly. OP: /civ roster {name} then /civ reconcile {name}."
                ));
            };
            let link = game.native_entity(entity_id)
                .and_then(|handle| game.native_entity_uuid(handle))
                .ok_or_else(|| "spawned entity UUID is unavailable".to_owned())
                .and_then(|uuid| state.world.as_mut().unwrap()
                    .bind_actor(village.id, resident_id, uuid).map(|_| ()))
                .and_then(|_| save(state));
            if let Err(e) = link {
                state.problem = Some(format!("native actor recovery identity halted: {e}"));
                log::error!("[civilizations] recovered actor without durable UUID identity: {e}");
                return Ok(format!(
                    "Recovery may have spawned resident #{resident_id}, but UUID persistence FAILED: {e}. World actions halted; inspect saved journal and native actor. Do NOT respawn."
                ));
            }
            Ok(format!(
                "Recovered one UUID-linked villager #{resident_id} in {name}. Other residents unchanged. /civ roster {name}."
            ))
        })(),
        "actors" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ actors <name>")?;
            let s = previous.by_name(name).ok_or("unknown settlement")?;
            if s.actors_spawned {
                return Ok(format!("Actors already reserved. {}/{} loaded UUID-linked; no duplicate spawns. Legacy world: /civ reconcile {name}.", loaded_actors(game, &previous, s.id).len(), s.population()));
            }
            if s.dimension != player.dimension { return Err("go to that dimension first".into()); }
            let dx = i64::from(s.center[0]) - player.position[0].floor() as i64;
            let dz = i64::from(s.center[2]) - player.position[2].floor() as i64;
            if dx * dx + dz * dz > 32 * 32 { return Err("stand within 32 blocks of the settlement center".into()); }
            // Manual spawning must use the SAME live preflight as automatic
            // spawning. Never reserve ten actors on obstructed/unloaded
            // ground, beside occupied mobs, or over tagged survivors.
            let positions = surveyed_initial_actor_positions(game, s)?;
            // Persist one-shot reservation before the first spawn. A crash
            // cannot accidentally retrigger a full batch on next login.
            state.world.as_mut().unwrap().settlements.get_mut(&s.id).unwrap().actors_spawned = true;
            save(state)?;
            already_saved = true;
            let mut spawned = 0usize;
            let mut halted = false;
            for (resident, position) in s.residents.iter().take(10).zip(positions) {
                let opts = NativeSpawn {
                    name: Some(s.actor_name(resident)),
                    ..NativeSpawn::default()
                };
                let Some(entity_id) = game.native_spawn_mob(
                    &s.dimension, "minecraft:villager", position, &opts
                ) else {
                    break;
                };
                spawned += 1;
                let maybe_uuid = game.native_entity(entity_id)
                    .and_then(|handle| game.native_entity_uuid(handle));
                let link = maybe_uuid.ok_or_else(|| "spawned actor UUID is unavailable".to_owned())
                    .and_then(|uuid| state.world.as_mut().unwrap().bind_actor(s.id, resident.id, uuid).map(|_| ()))
                    .and_then(|_| save(state));
                if let Err(e) = link {
                    state.problem = Some(format!("actor spawn identity interrupted: {e}"));
                    log::error!("[civilizations] spawned an actor without a durable identity: {e}");
                    halted = true;
                    break;
                }
            }
            if halted {
                Ok(format!("Spawn halted after {spawned}/10 actors: identity persistence failed. Restart and manually inspect/reconcile; no auto-respawn."))
            } else if spawned != 10 {
                Ok(format!("Spawned {spawned}/10 actors. Reserved one-shot spawn; use /civ roster {name}, do not rerun blindly."))
            } else {
                Ok(format!("Spawned and UUID-linked {spawned} villagers. Reopen world and verify /civ roster {name}."))
            }
        })(),
        "found" | "day" | "war" | "peace" | "actors" | "recoveractor" | "construct" | "post" | "approvehouse" | "autonomy" | "farm" | "recoverfarm" | "reconcile" | "barrelcheck" | "resolvebarrel" | "gathercheck" | "discardgather" => Err("requires server operator level 2".into()),
        _ => Err("unknown subcommand. Use /civ help".into()),
    };
    (result, already_saved)
}
