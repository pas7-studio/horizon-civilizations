// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// A native barrel is the authoritative on-site storage after the civic
/// hall completes. Only plain item stacks count (never damaged/NBT-bearing
/// equipment that a trader might have placed in the container).
pub(super) fn plain_barrel_count(game: &Game<'_>, dimension: &str, pos: [i32; 3], good: Good) -> Option<u32> {
    if world_block(game, dimension, pos).as_deref() != Some("minecraft:barrel") {
        return None;
    }
    let slots = game.native_container_slots(dimension, pos, Direction::Up)?;
    slots.iter().filter_map(|entry| entry.stack.as_ref())
        .filter(|stack| stack.item.as_ref() == good.item() && stack.damage == 0 && stack.extra.is_none())
        .try_fold(0_u32, |total, stack| total.checked_add(stack.count))
}

/// Reads exactly the two village hall barrels, never a nearby player chest.
pub(super) fn physical_barrel_stock(
    game: &Game<'_>, dimension: &str, center: [i32; 3]
) -> Option<Vec<(Good, u32)>> {
    let mut values = Vec::new();
    for good in [Good::Wheat, Good::Bread, Good::Wood, Good::Stone, Good::Iron,
                 Good::Sand, Good::Dirt] {
        let mut count = 0_u32;
        for pos in hall_storage_positions(center) {
            count = count.checked_add(plain_barrel_count(game, dimension, pos, good)?)?;
        }
        values.push((good, count));
    }
    Some(values)
}

/// A transfer has to be serviced by a real, UUID-linked NPC in the same
/// loaded village. The merchant deposits; builders collect their materials.
pub(super) fn storage_worker_near(game: &mut Game<'_>, world: &World, id: u64, role: Job) -> bool {
    let v = &world.settlements[&id];
    let linked = loaded_actors(game, world, id);
    v.residents.iter().filter(|r| r.alive && r.job == role).any(|r| {
        linked.iter().any(|(known, mob)| *known == r.id
            && game.native_mob(&v.dimension, *mob).is_some_and(|actor| {
                native_worker_near(actor.pos, v.center, 8.0, 4.0)
            }))
    })
}

/// Select one exact plain native slot from either hall barrel. This
/// helper only persists the planned intent. Native extraction happens on a
/// *later* tick, after a fresh slot read and a durable Issued record.
pub(super) fn request_workshop_pickup(
    game: &Game<'_>, state: &mut ActiveWorld, id: u64,
    good: Good, amount: u32, direction: BarrelDirection,
) -> Result<bool, String> {
    if amount == 0 { return Ok(false); }
    let village = &state.world.as_ref().unwrap().settlements[&id];
    let dimension = village.dimension.clone();
    let center = village.center;
    for pos in hall_storage_positions(center) {
        if world_block(game, &dimension, pos).as_deref() != Some("minecraft:barrel") {
            continue;
        }
        let Some(slots) = game.native_container_slots(&dimension, pos, Direction::Up)
        else { continue; };
        let item = slots.iter().find_map(|entry| {
            let stack = entry.stack.as_ref()?;
            (entry.can_take && stack.item.as_ref() == good.item()
                && stack.damage == 0 && stack.extra.is_none()
                && stack.count > 0).then_some((entry.slot, stack.count))
        });
        let Some((slot, available)) = item else { continue; };
        let Some(before) = plain_barrel_count(game, &dimension, pos, good)
        else { continue; };
        let count = amount.min(available).min(8);
        let old = state.world.as_ref().unwrap().clone();
        state.world.as_mut().unwrap().plan_barrel_transfer(
            id, pos, good, direction, before, count, Some(slot),
        )?;
        save_autonomous(state, old)?;
        return Ok(true);
    }
    Ok(false)
}

/// Never re-issue a native operation marked Issued in saved world.json.
/// An uncertain barrel result becomes Blocked rather than being replayed.
/// The first attempt saves Issued *before* mutating a native inventory.
pub(super) fn barrel_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dimension = game.dim.id().to_owned();
    let ids: Vec<u64> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension && v.hall_complete)
        .map(|v| v.id).collect();
    for id in ids {
        let village = state.world.as_ref().unwrap().settlements[&id].clone();
        // Never move village items behind a precharged player purchase.
        // Issued market orders require explicit local reconciliation.
        if village.market_purchase.is_some() { continue; }
        if let Some(transfer) = village.barrel_transfer {
            if transfer.phase == BarrelPhase::Blocked { continue; }
            let Some(actual) = plain_barrel_count(game, &dimension, transfer.position, transfer.good)
            else { continue; }; // unloaded, missing block entity: no mutation
            let before = state.world.as_ref().unwrap().clone();
            if transfer.phase == BarrelPhase::Issued {
                if transfer.expected_after() == Some(actual) {
                    state.world.as_mut().unwrap().confirm_barrel_transfer(id, actual)?;
                } else {
                    state.world.as_mut().unwrap().block_barrel_transfer(id)?;
                    log::warn!(
                        "[civilizations] {} barrel transfer discrepancy: expected {:?}, observed {}, requires inspection",
                        village.name, transfer.expected_after(), actual
                    );
                }
                save_autonomous(state, before)?;
                return Ok(());
            }
            if actual != transfer.before {
                state.world.as_mut().unwrap().block_barrel_transfer(id)?;
                save_autonomous(state, before)?;
                return Ok(());
            }
            // Aggregate counts alone do not identify the reserved slot.
            // An operator/hopper may replace the exact source stack between
            // planning and issuing. Never take a foreign item even when the
            // total number of logs/stone in the barrel is unchanged.
            if transfer.direction != BarrelDirection::ToBarrel {
                let valid_slot = transfer.slot.is_some_and(|selected| {
                    game.native_container_slots(
                        &dimension, transfer.position, Direction::Up
                    ).is_some_and(|slots| slots.iter().any(|entry| {
                        entry.slot == selected && entry.can_take
                            && entry.stack.as_ref().is_some_and(|stack| {
                                stack.item.as_ref() == transfer.good.item()
                                    && stack.damage == 0 && stack.extra.is_none()
                                    && stack.count >= transfer.count
                            })
                    }))
                });
                if !valid_slot {
                    state.world.as_mut().unwrap().block_barrel_transfer(id)?;
                    save_autonomous(state, before)?;
                    return Ok(());
                }
            }
            // Mark issued and durably save BEFORE the external inventory edit.
            state.world.as_mut().unwrap().issue_barrel_transfer(id)?;
            save_autonomous(state, before)?;
            let success = match transfer.direction {
                BarrelDirection::ToBarrel => {
                    let item = ItemStack {
                        item: transfer.good.item().into(),
                        count: transfer.count, damage: 0, extra: None,
                    };
                    game.insert_stack_through(transfer.position, Direction::Up, item).is_none()
                }
                BarrelDirection::ToBuilder
                | BarrelDirection::ToBakeryBuilder
                | BarrelDirection::ToBakerIngredients => {
                    let Some(slot) = transfer.slot else {
                        return Err("native barrel extraction has no recorded slot".into());
                    };
                    let taken = game.native_extract_item(
                        &dimension, transfer.position, Direction::Up, slot, transfer.count
                    );
                    taken.is_some_and(|stack| {
                        stack.item.as_ref() == transfer.good.item() && stack.count == transfer.count
                            && stack.damage == 0 && stack.extra.is_none()
                    })
                }
            };
            if !success {
                let before = state.world.as_ref().unwrap().clone();
                state.world.as_mut().unwrap().block_barrel_transfer(id)?;
                save_autonomous(state, before)?;
                log::warn!("[civilizations] {} failed native barrel transfer; no blind retry", village.name);
            }
            return Ok(());
        }

        // First fund the public bakery from physically present native
        // oak logs and cobblestone. A real builder must stand at the hall,
        // and every extracted item becomes bakery-specific saved escrow.
        let bakery_builder = village.bakery.as_ref().is_some_and(|b|
            b.building.as_ref().is_some_and(|job| !job.blocked) && !b.supplies_reserved)
            && DailyPhase::at(game.world.day_time).on_shift(Job::Builder)
            && storage_worker_near(game, state.world.as_ref().unwrap(), id, Job::Builder);
        if bakery_builder {
            let bakery = village.bakery.as_ref().unwrap();
            let wood = bakery.construction_escrow.get(&Good::Wood).copied().unwrap_or(0);
            let stone = bakery.construction_escrow.get(&Good::Stone).copied().unwrap_or(0);
            if wood >= crate::game::civilizations_core::BAKERY_WOOD_LOGS
                && stone >= crate::game::civilizations_core::BAKERY_COBBLESTONE {
                let before = state.world.as_ref().unwrap().clone();
                if state.world.as_mut().unwrap().reserve_bakery_supplies(id)? {
                    save_autonomous(state, before)?;
                    return Ok(());
                }
            }
            for (good, required) in [
                (Good::Wood, crate::game::civilizations_core::BAKERY_WOOD_LOGS),
                (Good::Stone, crate::game::civilizations_core::BAKERY_COBBLESTONE),
            ] {
                let waiting = required.saturating_sub(
                    bakery.construction_escrow.get(&good).copied().unwrap_or(0));
                if request_workshop_pickup(
                    game, state, id, good, waiting, BarrelDirection::ToBakeryBuilder,
                )? {
                    return Ok(());
                }
            }
        }

        // A completed real bakery needs THREE wheat actually removed from
        // vanilla barrels per bread. Baker must be at the receiving area;
        // no abstract simulated inventory or virtual daily production.
        let bakery_needs_wheat = village.bakery.as_ref().is_some_and(|b|
            b.complete
                && b.wheat_escrow < crate::game::civilizations_core::BREAD_WHEAT_RECIPE
                && b.bread_in_transit == 0)
            && DailyPhase::at(game.world.day_time).on_shift(Job::Baker)
            && storage_worker_near(game, state.world.as_ref().unwrap(), id, Job::Baker);
        if bakery_needs_wheat {
            // Do not order wheat once native bread stock + confirmed incoming
            // deliveries cover the buffer; continue servicing other warehouse
            // goods in the same village even when baking is not required.
            if let Some(stored) = physical_barrel_stock(game, &dimension, village.center) {
                let physical_bread = stored.iter().find(|(good, _)| *good == Good::Bread)
                    .map(|(_, count)| *count).unwrap_or(0);
                if crate::game::civilizations_core::needs_real_bread(&village, physical_bread) {
                    let missing = crate::game::civilizations_core::BREAD_WHEAT_RECIPE
                        - village.bakery.as_ref().unwrap().wheat_escrow;
                    if request_workshop_pickup(
                        game, state, id, Good::Wheat, missing, BarrelDirection::ToBakerIngredients,
                    )? {
                        return Ok(());
                    }
                }
            }
        }

        let can_collect = village.visual_job.is_some()
            && !village.hut_supplies_reserved
            && DailyPhase::at(game.world.day_time).on_shift(Job::Builder)
            && storage_worker_near(game, state.world.as_ref().unwrap(), id, Job::Builder);
        if can_collect {
            // A builder must finish collecting their bill at the town hall
            // and have it durably reserved BEFORE routing to the build plot.
            // Otherwise the old build_one_step near-plot check would deadlock
            // with the new hall-first worker navigation.
            let wood_ready = village.build_escrow.get(&Good::Wood).copied().unwrap_or(0)
                >= crate::game::civilizations_core::HUT_WOOD_LOGS;
            let stone_ready = village.build_escrow.get(&Good::Stone).copied().unwrap_or(0)
                >= crate::game::civilizations_core::HUT_COBBLESTONE;
            if wood_ready && stone_ready {
                let prior = state.world.as_ref().unwrap().clone();
                if state.world.as_mut().unwrap().reserve_hut_supplies(id)? {
                    save_autonomous(state, prior)?;
                    log::info!("[civilizations] {} builder collected real hut materials at the public warehouse", village.name);
                    return Ok(());
                }
            }
            for (good, target) in [
                (Good::Wood, crate::game::civilizations_core::HUT_WOOD_LOGS),
                (Good::Stone, crate::game::civilizations_core::HUT_COBBLESTONE),
            ] {
                let in_hand = village.build_escrow.get(&good).copied().unwrap_or(0);
                let missing = target.saturating_sub(in_hand);
                if missing == 0 { continue; }
                for position in hall_storage_positions(village.center) {
                    if world_block(game, &dimension, position).as_deref() != Some("minecraft:barrel") {
                        continue;
                    }
                    let Some(slots) = game.native_container_slots(&dimension, position, Direction::Up)
                    else { continue; };
                    let Some((slot, available)) = slots.iter().find_map(|entry| {
                        let stack = entry.stack.as_ref()?;
                        (entry.can_take && stack.item.as_ref() == good.item()
                            && stack.damage == 0 && stack.extra.is_none()
                            && stack.count > 0).then_some((entry.slot, stack.count))
                    }) else { continue; };
                    let Some(before) = plain_barrel_count(game, &dimension, position, good) else {
                        continue;
                    };
                    let quantity = missing.min(available).min(8);
                    let prior = state.world.as_ref().unwrap().clone();
                    state.world.as_mut().unwrap().plan_barrel_transfer(
                        id, position, good, BarrelDirection::ToBuilder,
                        before, quantity, Some(slot)
                    )?;
                    save_autonomous(state, prior)?;
                    return Ok(());
                }
            }
        }

        // Delivery into actual Minecraft barrels is served by the real
        // village merchant/steward; no player-owned inventory is silently
        // consumed, and source goods came from prior verified receipts.
        if !storage_worker_near(game, state.world.as_ref().unwrap(), id, Job::Merchant) {
            continue;
        }
        for good in [Good::Wood, Good::Stone, Good::Wheat, Good::Bread, Good::Iron,
                     Good::Sand, Good::Dirt] {
            let waiting = village.warehouse_quantity(good);
            if waiting == 0 { continue; }
            let spots = hall_storage_positions(village.center);
            let preferred = if matches!(good, Good::Wood | Good::Stone) {
                spots
            } else { [spots[1], spots[0]] };
            for position in preferred {
                if world_block(game, &dimension, position).as_deref() != Some("minecraft:barrel") {
                    continue;
                }
                let Some(slots) = game.native_container_slots(&dimension, position, Direction::Up)
                else { continue; };
                let space: u32 = slots.iter().map(|entry| match &entry.stack {
                    None => 64,
                    Some(stack) if stack.item.as_ref() == good.item()
                        && stack.damage == 0 && stack.extra.is_none() =>
                        64_u32.saturating_sub(stack.count),
                    _ => 0,
                }).sum();
                let count = waiting.min(space).min(8);
                if count == 0 { continue; }
                let Some(before) = plain_barrel_count(game, &dimension, position, good) else {
                    continue;
                };
                let prior = state.world.as_ref().unwrap().clone();
                state.world.as_mut().unwrap().plan_barrel_transfer(
                    id, position, good, BarrelDirection::ToBarrel, before, count, None
                )?;
                save_autonomous(state, prior)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

/// One physical farmer delivery per village, after an actual UUID-linked
/// villager has returned from a fully confirmed crop harvest to the hall.
pub(super) fn farmer_delivery_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Farmer) { return Ok(()); }
    let dim = game.dim.id().to_owned();
    let ids: Vec<u64> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dim && v.wheat_delivery_pending() > 0)
        .map(|v| v.id).collect();
    for id in ids {
        let world = state.world.as_ref().unwrap();
        let v = &world.settlements[&id];
        let loaded = loaded_actors(game, world, id);
        let returned = v.residents.iter().filter(|r| r.alive && r.job == Job::Farmer)
            .any(|r| loaded.iter().any(|(known, mob)| *known == r.id
                && game.native_mob(&dim, *mob).is_some_and(|actor| {
                    native_worker_near(actor.pos, v.center, 8.0, 4.0)
                })));
        if !returned { continue; }
        let before = state.world.as_ref().unwrap().clone();
        if state.world.as_mut().unwrap().deliver_farm_wheat(id)? {
            save_autonomous(state, before)?;
        }
        return Ok(());
    }
    Ok(())
}

/// Bakers need real extracted native wheat and an intact public crafting
/// table. Three wheat become one bread receipt only while a UUID-bound Baker
/// is physically working at the native bakery; bread is made available only
/// after this same baker returns to the hall and the steward stocks a barrel.
pub(super) fn baker_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Baker) { return Ok(()); }
    let dimension = game.dim.id().to_owned();
    let ids: Vec<u64> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension && v.bakery.as_ref().is_some_and(|b| b.complete))
        .map(|v| v.id).collect();
    for id in ids {
        let world = state.world.as_ref().unwrap();
        let village = &world.settlements[&id];
        let b = village.bakery.as_ref().unwrap();
        if b.bread_in_transit > 0 {
            if !storage_worker_near(game, world, id, Job::Baker) { continue; }
            let old = state.world.as_ref().unwrap().clone();
            if state.world.as_mut().unwrap().deliver_bakery_bread(id)? {
                save_autonomous(state, old)?;
            }
            return Ok(());
        }
        if b.wheat_escrow < crate::game::civilizations_core::BREAD_WHEAT_RECIPE {
            continue;
        }
        // A stock already replenished by trading, another pending delivery
        // or physical barrel items must prevent another loaf even if the
        // baker happens to be carrying ingredients. Never inspect virtual
        // day-simulation food for this native decision.
        let Some(stored) = physical_barrel_stock(game, &dimension, village.center)
        else { continue; };
        let physical_bread = stored.iter().find(|(good, _)| *good == Good::Bread)
            .map(|(_, count)| *count).unwrap_or(0);
        if !crate::game::civilizations_core::needs_real_bread(village, physical_bread) {
            continue;
        }
        // Check that the whole actual workshop still exists and is loaded,
        // not merely that the old save claims it was complete yesterday.
        let intact = bakery_plan(b.origin).iter().all(|step|
            world_block(game, &dimension, step.pos).as_deref() == Some(step.block));
        if !intact { continue; }
        let actors = loaded_actors(game, world, id);
        let working = village.residents.iter().filter(|r| r.alive && r.job == Job::Baker)
            .any(|r| actors.iter().any(|(known,mob)| *known == r.id
                && game.native_mob(&dimension, *mob).is_some_and(|actor| {
                    native_worker_near(actor.pos, b.origin, 6.0, 4.0)
                })));
        if !working { continue; }
        let before = state.world.as_ref().unwrap().clone();
        if state.world.as_mut().unwrap().craft_bread(id)? {
            save_autonomous(state, before)?;
        }
        return Ok(());
    }
    Ok(())
}
