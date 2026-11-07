// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// Find reachable-looking air next to a candidate real resource. This is
/// NOT a pathfinding guarantee: the native AI may refuse its route.
pub(super) fn gather_stand(game: &Game<'_>, dimension: &str, pos: [i32; 3]) -> Option<[i32; 3]> {
    for dy in [0_i32, 1, -1] {
        for (dx, dz) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
            let (Some(x), Some(y), Some(z)) = (
                pos[0].checked_add(dx), pos[1].checked_add(dy), pos[2].checked_add(dz)
            ) else { continue; };
            let Some(head) = y.checked_add(1) else { continue; };
            let Some(floor) = y.checked_sub(1) else { continue; };
            if world_block(game, dimension, [x, y, z]).as_deref() == Some("minecraft:air")
                && world_block(game, dimension, [x, head, z]).as_deref() == Some("minecraft:air")
                && world_block(game, dimension, [x, floor, z])
                    .is_some_and(|block| is_ground(&block)) {
                return Some([x, y, z]);
            }
        }
    }
    None
}

/// A saved work stand was clear when planned, but it can become a wall,
/// cave or player-built obstruction before the villager arrives. Reobserve
/// both standing blocks and actual floor at the native issue boundary.
pub(super) fn gather_stand_still_clear(game: &Game<'_>, dimension: &str, stand: [i32; 3]) -> bool {
    let [x, y, z] = stand;
    let (Some(head), Some(floor)) = (y.checked_add(1), y.checked_sub(1))
    else { return false; };
    world_block(game, dimension, stand).as_deref() == Some("minecraft:air")
        && world_block(game, dimension, [x, head, z]).as_deref() == Some("minecraft:air")
        && world_block(game, dimension, [x, floor, z])
            .is_some_and(|block| is_ground(&block))
}

/// Local material source recognition. Naturalness is conservative but not
/// provable from block state alone: player protection claims and the structure
/// exclusion remain mandatory; no guaranteed anti-grief for other mods.
pub(super) fn seems_natural_resource(game: &Game<'_>, dimension: &str, point: [i32; 3], good: Good) -> bool {
    match good {
        Good::Wood => is_rooted_oak_harvest_origin(point, |pos|
            world_block(game, dimension, pos)),
        Good::Stone => world_block(game, dimension, point).as_deref() == Some("minecraft:stone"),
        // Only plain natural sand and dirt: a placed block inside a build is
        // still excluded by gather_site_unclaimed in the domain.
        Good::Sand => world_block(game, dimension, point).as_deref() == Some("minecraft:sand"),
        Good::Dirt => world_block(game, dimension, point).as_deref() == Some("minecraft:dirt"),
        _ => false,
    }
}

/// A resource may be credited ONLY after: (1) plan saved; (2) Issued
/// journal persisted; (3) native conditional block edit attempted; (4) later
/// observation verifies air and a UUID-linked worker returns to the hall.
/// If the process crashes after Issued but before the actual edit, the
/// resulting uncertainty is quarantined; no automatic native retry or credit.
pub(super) fn gather_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dimension = game.dim.id().to_owned();
    // Rotate among all loaded nonblocked village jobs. Previously .find()
    // always chose the lowest village ID: an assigned worker unable to
    // reach the hall could starve EVERY later settlement indefinitely.
    // The tick advances by eight between gather runs, making this a
    // deterministic fair round-robin, with no new world/save fields.
    let eligible: Vec<_> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension)
        .filter_map(|v| v.gather_job.as_ref().map(|job| (v.id, job.clone())))
        .filter(|(_, job)| !job.blocked && game.world.loaded(job.position[0], job.position[2]))
        .collect();
    let Some(slot) = gather_turn_index(state.physical_ticks, eligible.len()) else {
        return Ok(());
    };
    let (id, job) = eligible[slot].clone();
    let role = if job.good == Good::Wood { Job::Lumberjack } else { Job::Miner };
    if !DailyPhase::at(game.world.day_time).on_shift(role) {
        return Ok(());
    }
    let observed = world_block(game, &dimension, job.position);
    let Some(name) = observed else { return Ok(()); };
    let before = state.world.as_ref().unwrap().clone();
    if job.issued {
        // If the server stopped after Issued was saved but before the
        // command's native result AND fresh observed air were durably
        // acknowledged, the block may have been removed by anyone.
        // Quarantine even if it is now air; never credit such a crash.
        if !job.native_edit_confirmed {
            state.world.as_mut().unwrap().block_gather(id)?;
            return save_autonomous(state, before);
        }
        // Legacy journals lack a stable worker ID. Since any same-role NPC
        // might stand near the hall, such a receipt is NOT sufficient proof
        // of a physically returned courier. Quarantine, never mint goods.
        let Some(assigned) = job.worker_id else {
            state.world.as_mut().unwrap().block_gather(id)?;
            return save_autonomous(state, before);
        };
        if name == "minecraft:air" {
            let world = state.world.as_ref().unwrap();
            let v = &world.settlements[&id];
            let role = if job.good == Good::Wood { Job::Lumberjack } else { Job::Miner };
            let linked = loaded_actors(game, world, id);
            if !job.pickup_witnessed {
                // Seeing air alone is not proof of resource custody. Check
                // that the EXACT assigned UUID-bound worker is still at the
                // loaded source stand, in actual 3D reach, after the edit.
                // Save this witness before navigation sends them home.
                let witnessed = v.residents.iter()
                    .filter(|r| r.alive && r.job == role && r.id == assigned)
                    .any(|r| linked.iter().any(|(known, mob)| *known == r.id
                        && game.native_mob(&dimension, *mob).is_some_and(|actor|
                            native_worker_near(actor.pos, job.stand, 2.5, 2.0)
                                && native_worker_reaches_block(actor.pos, job.position, 4.5))));
                if !witnessed { return Ok(()); }
                state.world.as_mut().unwrap().witness_gather_pickup(id, assigned)?;
                return save_autonomous(state, before);
            }
            let delivered = v.residents.iter()
                .filter(|r| r.alive && r.job == role && r.id == assigned)
                .any(|r| linked.iter().any(|(known, mob)| *known == r.id
                    && game.native_mob(&dimension, *mob).is_some_and(|actor| {
                        native_worker_near(actor.pos, v.center, 8.0, 4.0)
                    })));
            if !delivered {
                // No resource is credited simply because a log disappeared:
                // the named worker must return to the loaded receiving area.
                return Ok(());
            }
            state.world.as_mut().unwrap().confirm_gather(id)?;
        } else {
            // Commands can be refused or a plot may be modified. Never
            // issue twice: block and replan a different natural block.
            state.world.as_mut().unwrap().block_gather(id)?;
        }
        return save_autonomous(state, before);
    }
    let saved_stem = state.world.as_ref().unwrap().settlements[&id].oak_stem.as_ref();
    let valid_resource = if let Some(stem) = saved_stem {
        // All previous trunk blocks must actually be air and the remaining
        // segments remain vanilla oak, with an intact upper leaf crown.
        // No arbitrary floating logs become a fresh source of free wood.
        inspect_oak_stem(stem, |pos| world_block(game, &dimension, pos))
    } else {
        seems_natural_resource(game, &dimension, job.position, job.good)
    };
    if name != job.source_block()
        || !valid_resource
        || !gather_stand_still_clear(game, &dimension, job.stand)
        || !state.world.as_ref().unwrap().gather_site_unclaimed(id, job.position, job.stand) {
        // Physical resource or land ownership may change after a saved plan.
        // Recheck tree/stone provenance and every registered parcel/worksite
        // immediately before authorizing a Minecraft world mutation.
        state.world.as_mut().unwrap().block_gather(id)?;
        return save_autonomous(state, before);
    }
    let world = state.world.as_ref().unwrap();
    let v = &world.settlements[&id];
    let profession = job.good.job().unwrap_or(Job::Miner);
    let camp_kind = PostKind::for_good(job.good);
    // A camp already bound to one resident is serviced by that resident only:
    // the name on the camp sign and the worker who delivers stay the same.
    let camp_worker = v.posts.iter().find(|post| post.kind == camp_kind)
        .and_then(|post| post.worker_id);
    let actors = loaded_actors(game, world, id);
    let selected = v.residents.iter()
        .filter(|r| r.alive && r.job == profession
            && job.worker_id.is_none_or(|assigned| assigned == r.id)
            && camp_worker.is_none_or(|bound| bound == r.id))
        .find_map(|r| actors.iter().find_map(|(rid, mob)| {
            if *rid != r.id { return None; }
            game.native_mob(&dimension, *mob).and_then(|actor| {
                (native_worker_near(actor.pos, job.stand, 2.5, 2.0)
                    && native_worker_reaches_block(actor.pos, job.position, 4.5))
                .then_some(r.id)
            })
        }));
    let Some(selected_worker) = selected else { return Ok(()); };
    // The camp keeps the same resident across restarts: delivery credit is
    // checked against that ID at the hall, never against whoever is nearby.
    if state.world.as_mut().unwrap()
        .set_post_worker(id, camp_kind, selected_worker).is_err()
    {
        log::warn!("[civilizations] camp worker binding skipped for settlement {id}");
    }
    if job.worker_id.is_none() {
        // Save this exact resident ID BEFORE we persist Issued or send the
        // native edit. A different miner at the hall cannot claim delivery.
        state.world.as_mut().unwrap().assign_gather_worker(id, selected_worker)?;
        return save_autonomous(state, before);
    }
    let [x, y, z] = job.position;
    let cmd = format!(
        "execute in {dimension} if block {x} {y} {z} {} run setblock {x} {y} {z} minecraft:air replace",
        job.source_block()
    );
    // Persist Issued *before* executing the conditional Minecraft edit.
    // A crash after changing the block cannot revive a Planned job that
    // would be reissued. If persistence fails, no native command is sent.
    state.world.as_mut().unwrap().mark_gather_issued(id)?;
    save_autonomous(state, before)?;
    // Native command success alone is not enough: a conditional command
    // can be accepted without modifying the expected source. Re-read the
    // exact block immediately while the current dimension/chunk is loaded.
    let accepted = game.native_run_command(&cmd);
    let now_air = world_block(game, &dimension, job.position).as_deref()
        == Some("minecraft:air");
    let issued = state.world.as_ref().unwrap().clone();
    if accepted && now_air {
        // If this save fails after a native edit, the durable journal stays
        // Issued-but-unconfirmed. On restart it is quarantined, never
        // mistakenly treated as a valid paid resource.
        state.world.as_mut().unwrap().mark_gather_native_edit_confirmed(id)?;
    } else {
        // Refused, no-op or unloaded command is unsafe to retry.
        // Keep the uncertainty for OP inspection with ZERO auto-credit.
        state.world.as_mut().unwrap().block_gather(id)?;
    }
    save_autonomous(state, issued)?;
    Ok(())
}
