// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

pub(super) fn quarantined_native_purchase(state: &mut ActiveWorld, village_id: u64, reason: &str) -> String {
    let marked = state.world.as_mut().unwrap().block_market_purchase(village_id);
    let outcome = marked.and_then(|()| save(state));
    if let Err(error) = outcome {
        state.problem = Some(format!("market purchase recovery halted: {error}"));
    }
    format!(
        "Native warehouse purchase QUARANTINED ({reason}). Coins already reserved; no automatic retry or refund. OP must inspect the saved order and exact player/barrel inventory before recovery."
    )
}

/// Market purchases read and extract one entire plain slot from one of the
/// two loaded native town-hall barrels. Save the buyer's coin debit and
/// issued purchase journal FIRST. A native/restart ambiguity can lose a
/// purchase, but never silently grants a second identical item batch.
pub(super) fn buy_from_real_barrel(
    game: &mut Game<'_>, state: &mut ActiveWorld, actor: &str,
    village_id: u64, good: Good, count: u32, position: [f64; 3],
    already_saved: &mut bool,
) -> Result<String, String> {
    let v = state.world.as_ref().unwrap().settlements
        .get(&village_id).ok_or("unknown village")?.clone();
    if game.dim.id() != v.dimension { return Err("visit the village dimension to trade".into()); }
    if !position.iter().all(|x| x.is_finite()) {
        return Err("invalid trading location".into());
    }
    let dx = position[0] - f64::from(v.center[0]);
    let dz = position[2] - f64::from(v.center[2]);
    if dx * dx + dz * dz > 12.0 * 12.0
        || (position[1] - f64::from(v.center[1])).abs() > 8.0 {
        return Err("trade within 12 blocks of the actual village hall".into());
    }
    if v.barrel_transfer.is_some() || v.market_purchase.is_some() {
        return Err("warehouse transfer or previous purchase still pending; /civ warehouse shows it".into());
    }
    if game.native_room_for(good.item()) < count {
        return Err("not enough empty native player inventory space".into());
    }
    // A single plain stack is required: native extraction is one bounded
    // API call with a stable slot ID, never an uncontrolled multi-slot sweep.
    let mut selected = None;
    for barrel in hall_storage_positions(v.center) {
        if world_block(game, &v.dimension, barrel).as_deref() != Some("minecraft:barrel") {
            continue;
        }
        let Some(slots) = game.native_container_slots(&v.dimension, barrel, Direction::Up)
        else { continue; };
        if let Some(slot) = slots.iter().find_map(|entry| {
            let stack = entry.stack.as_ref()?;
            (entry.can_take && stack.item.as_ref() == good.item() && stack.count >= count
                && stack.damage == 0 && stack.extra.is_none())
                .then_some((entry.slot, stack.count))
        }) {
            let Some(before) = plain_barrel_count(game, &v.dimension, barrel, good) else {
                continue;
            };
            selected = Some((barrel, slot.0, slot.1, before));
            break;
        }
    }
    let Some((barrel, slot, _in_slot, before)) = selected else {
        return Err(format!(
            "no intact native barrel stack with {count} {} available; smaller purchases can work across split stacks",
            good.as_str()
        ));
    };

    state.world.as_mut().unwrap().begin_market_purchase(
        actor, village_id, good, count, barrel, slot, before
    )?;
    if let Err(error) = save(state) {
        state.problem = Some(format!("market reservation save refused: {error}"));
        return Err(format!("native warehouse purchase not started: {error}"));
    }
    *already_saved = true; // outer command handler must NEVER undo this debit
    let taken = game.native_extract_item(&v.dimension, barrel, Direction::Up, slot, count);
    let exact = taken.as_ref().is_some_and(|stack| {
        stack.item.as_ref() == good.item() && stack.count == count
            && stack.damage == 0 && stack.extra.is_none()
    });
    if !exact {
        return Ok(quarantined_native_purchase(state, village_id,
            "native barrel returned a missing or wrong item stack"));
    }
    if plain_barrel_count(game, &v.dimension, barrel, good)
        != before.checked_sub(count) {
        return Ok(quarantined_native_purchase(state, village_id,
            "native barrel stock did not decrease by exact purchased quantity"));
    }
    let received = game.native_give_items(good.item(), count);
    if received != count {
        return Ok(quarantined_native_purchase(state, village_id,
            "player inventory did not accept the full paid stack"));
    }
    let finished = state.world.as_mut().unwrap().complete_market_purchase(village_id, actor);
    if let Err(error) = finished.and_then(|()| save(state)) {
        state.problem = Some(format!("physical market purchase final audit save failed: {error}"));
        return Ok(format!(
            "Item delivery completed, but market audit persistence FAILED: {error}. World halted to prevent duplicate grants."
        ));
    }
    let price = u64::from(good.price()) * u64::from(count);
    Ok(format!(
        "Bought {count} real {} from the town hall barrel for {price} civic coins. Native inventory and village treasury updated.",
        good.as_str()
    ))
}
