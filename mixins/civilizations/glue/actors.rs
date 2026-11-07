// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// Find our previously named actors even after the world has been reopened.
/// Only loaded entities are inspected; missing chunks are never force-loaded.
/// The name is a soft identity, not proof against an intentionally renamed mob.
pub(super) fn loaded_actors(game: &mut Game<'_>, world: &World, settlement_id: u64) -> Vec<(u64, u32)> {
    let Some(village) = world.settlements.get(&settlement_id) else {
        return Vec::new();
    };
    if game.dim.id() != village.dimension {
        return Vec::new();
    }
    // Names can be changed or forged. A live actor is trusted ONLY if its
    // persistent UUID equals the value recorded in world.json.
    village.residents.iter().filter_map(|resident| {
        if !resident.alive {
            return None;
        }
        let uuid = resident.entity_uuid?;
        let (dimension, mob_id, kind) = game.native_entity_by_uuid(uuid)?;
        if dimension != village.dimension || kind != "minecraft:villager" {
            return None;
        }
        let mob = game.native_mob(&dimension, mob_id)?;
        native_worker_near(mob.pos, village.center, 96.0, 48.0)
            .then_some((resident.id, mob_id))
    }).collect()
}

/// Use a completed 5x5 hut only if its real floor, two-high doorway and
/// indoor standing lane are present in loaded chunks. Never edits or loads.
pub(super) fn loaded_home_target(game: &Game<'_>, dimension: &str, home: HomeSlot) -> Option<[f64; 3]> {
    if !inspect_home(home, |position| world_block(game, dimension, position)) {
        return None;
    }
    let [x, y, z] = home.foot();
    Some([f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5])
}

/// Profession-aware day/night routing, limited to loaded UUID-bound actors.
/// This does not implement actual sleeping or independent resource generation.
pub(super) fn guide_actors(game: &mut Game<'_>, world: &World) {
    let current_dim = game.dim.id().to_owned();
    let phase = DailyPhase::at(game.world.day_time);
    let mut targets = Vec::new();
    for village in world.settlements.values().filter(|v| v.dimension == current_dim && v.actors_spawned) {
        for (resident_id, mob_id) in loaded_actors(game, world, village.id) {
            let Some(resident) = village.residents.iter().find(|r| r.id == resident_id && r.alive) else { continue; };
            let offset = phase.offset(resident.job, resident_id);
            // A completed saved hut is not proof its Minecraft blocks are
            // still there. Verify before sending any resident indoors.
            let home_target = if phase == DailyPhase::Night && !phase.on_shift(resident.job) {
                village.home_slot(resident_id)
                    .and_then(|home| loaded_home_target(game, &current_dim, home))
            } else {
                None
            };
            let pos = if let Some(home) = home_target {
                home
            } else if phase.on_shift(resident.job)
                && matches!(resident.job, Job::Lumberjack | Job::Miner)
                && village.gather_job.as_ref().is_some_and(|job|
                    !job.blocked
                        && job.worker_id.is_none_or(|assigned| assigned == resident_id)
                        && ((job.good == Good::Wood && resident.job == Job::Lumberjack)
                            || (job.good == Good::Stone && resident.job == Job::Miner)))
            {
                let target = village.gather_job.as_ref().expect("gather exists");
                if target.issued && target.pickup_witnessed {
                    // The same assigned citizen stays at the real worksite
                    // until pickup has been witnessed and durably saved.
                    // Only then route them to the hall for delivery.
                    [f64::from(village.center[0]) + 0.5,
                     f64::from(village.center[1]),
                     f64::from(village.center[2] - 3) + 0.5]
                } else {
                    [f64::from(target.stand[0]) + 0.5,
                     f64::from(target.stand[1]),
                     f64::from(target.stand[2]) + 0.5]
                }
            } else if phase.on_shift(resident.job) && resident.job == Job::Farmer {
                if village.wheat_delivery_pending() > 0 {
                    // Confirmed mature wheat travels home with the actual
                    // loaded farmer before it becomes village inventory.
                    [f64::from(village.center[0]) + 0.5,
                     f64::from(village.center[1]),
                     f64::from(village.center[2] - 3) + 0.5]
                } else if let Some(farm) = &village.farm {
                    [f64::from(farm.origin[0]) + 0.5,
                     f64::from(farm.origin[1]),
                     f64::from(farm.origin[2] - 3) + 0.5]
                } else {
                    [f64::from(village.center[0] + offset[0]) + 0.5,
                     f64::from(village.center[1] + offset[1]),
                     f64::from(village.center[2] + offset[2]) + 0.5]
                }
            } else if phase.on_shift(resident.job) && resident.job == Job::Baker {
                if let Some(bakery) = &village.bakery {
                    if bakery.complete {
                        if bakery.bread_in_transit > 0 || bakery.wheat_escrow < crate::game::civilizations_core::BREAD_WHEAT_RECIPE {
                            // Baker returns to the hall to deliver bread or
                            // receive exactly three physical wheat items.
                            [f64::from(village.center[0]) + 0.5,
                             f64::from(village.center[1]),
                             f64::from(village.center[2] - 3) + 0.5]
                        } else {
                            // Actual crafting table inside the completed
                            // public bakery, not an abstract job offset.
                            [f64::from(bakery.origin[0]) + 0.5,
                             f64::from(bakery.origin[1]),
                             f64::from(bakery.origin[2] - 3) + 0.5]
                        }
                    } else {
                        [f64::from(village.center[0]) + 0.5,
                         f64::from(village.center[1]),
                         f64::from(village.center[2] - 3) + 0.5]
                    }
                } else {
                    [f64::from(village.center[0] + offset[0]) + 0.5,
                     f64::from(village.center[1] + offset[1]),
                     f64::from(village.center[2] + offset[2]) + 0.5]
                }
            } else if phase.on_shift(resident.job) && resident.job == Job::Builder {
                if let Some(job) = &village.hall_job {
                    [f64::from(job.origin[0]) + 0.5,
                     f64::from(job.origin[1]),
                     f64::from(job.origin[2] - 4) + 0.5]
                } else if village.bakery.as_ref().is_some_and(|b| b.building.is_some()) {
                    let bakery = village.bakery.as_ref().unwrap();
                    let site = bakery.building.as_ref().unwrap();
                    if !bakery.supplies_reserved {
                        [f64::from(village.center[0]) + 0.5,
                         f64::from(village.center[1]),
                         f64::from(village.center[2] - 3) + 0.5]
                    } else {
                        [f64::from(site.origin[0]) + 0.5,
                         f64::from(site.origin[1]),
                         f64::from(site.origin[2] - 4) + 0.5]
                    }
                } else if let Some(job) = &village.visual_job {
                    if village.hall_complete && !village.hut_supplies_reserved {
                        // Builders visit the actual town hall barrels first
                        // to extract their physical bill of materials.
                        [f64::from(village.center[0]) + 0.5,
                         f64::from(village.center[1]),
                         f64::from(village.center[2] - 3) + 0.5]
                    } else {
                        [f64::from(job.origin[0]) + 0.5,
                         f64::from(job.origin[1]),
                         f64::from(job.origin[2] - 4) + 0.5]
                    }
                } else {
                    [f64::from(village.center[0] + offset[0]) + 0.5,
                     f64::from(village.center[1] + offset[1]),
                     f64::from(village.center[2] + offset[2]) + 0.5]
                }
            } else if phase.on_shift(resident.job)
                && resident.job == Job::Merchant && village.hall_complete {
                // The merchant is the first civic mayor / warehouse
                // steward; they visibly attend the central hall by day.
                [f64::from(village.center[0]) + 0.5,
                 f64::from(village.center[1]),
                 f64::from(village.center[2]) + 0.5]
            } else {
                [f64::from(village.center[0] + offset[0]) + 0.5,
                 f64::from(village.center[1] + offset[1]),
                 f64::from(village.center[2] + offset[2]) + 0.5]
            };
            targets.push((mob_id, pos));
        }
    }
    for (mob_id, pos) in targets {
        if let Some(mob) = game.native_mob(&current_dim, mob_id) {
            let dx = mob.pos[0] - pos[0];
            let dy = mob.pos[1] - pos[1];
            let dz = mob.pos[2] - pos[2];
            if dx * dx + dz * dz > 3.0 * 3.0 || dy.abs() > 2.5 {
                let _ = game.native_mob_navigate(&current_dim, mob_id, pos, 2, 0.65);
            }
        }
    }
}

/// Try a bounded list of already-loaded, open, natural-ground positions.
/// Never summons a villager if even one of the ten positions is unsafe.
pub(super) fn safe_actor_spawn_points(
    game: &Game<'_>, village: &crate::game::civilizations_core::Settlement
) -> Vec<[f64; 3]> {
    let mut spots = Vec::new();
    for dz in [-6_i32, -4, -2, 0, 2, 4, 6] {
        for dx in [-6_i32, -4, -2, 0, 2, 4, 6] {
            let Some(x) = village.center[0].checked_add(dx) else { continue; };
            let Some(z) = village.center[2].checked_add(dz) else { continue; };
            for dy in [0_i32, 1, -1, 2, -2] {
                let Some(y) = village.center[1].checked_add(dy) else { continue; };
                let Some(ground_y) = y.checked_sub(1) else { continue; };
                let Some(head_y) = y.checked_add(1) else { continue; };
                if world_block(game, &village.dimension, [x, ground_y, z])
                        .is_some_and(|block| is_ground(&block))
                    && world_block(game, &village.dimension, [x, y, z]).as_deref() == Some("minecraft:air")
                    && world_block(game, &village.dimension, [x, head_y, z]).as_deref() == Some("minecraft:air") {
                    let point = [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5];
                    // Terrain may be clear while a living native entity
                    // occupies the position. Skip it for BOTH autonomous
                    // and OP spawning, before reserving the batch.
                    if game.mobs.iter().any(|mob| {
                        let dx = mob.position.x - point[0];
                        let dy = mob.position.y - point[1];
                        let dz = mob.position.z - point[2];
                        mob.alive() && dx * dx + dy * dy + dz * dz < 9.0
                    }) {
                        continue;
                    }
                    // The preflight is a batch survey: selected citizens
                    // must also be separated from EACH OTHER, not only from
                    // mobs currently present before the first spawn.
                    // Keep the same 3-block clearance used for live mobs.
                    if spots.iter().any(|other: &[f64; 3]| {
                        let dx = other[0] - point[0];
                        let dy = other[1] - point[1];
                        let dz = other[2] - point[2];
                        dx * dx + dy * dy + dz * dz < 9.0
                    }) {
                        continue;
                    }
                    spots.push(point);
                    break;
                }
            }
            if spots.len() == 10 { return spots; }
        }
    }
    // Return every available safe spot even if there are fewer than ten.
    // The INITIAL batch still requires exactly ten before it reserves
    // anything. The explicit single-resident recovery needs just one.
    spots
}

/// Refuse to reserve the first batch if a citizen is already dead/linked,
/// a matching older native actor is visible, or fewer than ten genuinely
/// clear natural-ground spawn positions can be surveyed in loaded chunks.
/// This is shared by autonomous startup and the explicit OP /civ actors path.
pub(super) fn surveyed_initial_actor_positions(
    game: &Game<'_>, village: &crate::game::civilizations_core::Settlement,
) -> Result<Vec<[f64; 3]>, String> {
    if village.residents.len() < 10 || village.residents.iter().take(10)
            .any(|r| !r.alive || r.entity_uuid.is_some()) {
        return Err("initial batch requires ten living, UUID-unbound residents".into());
    }
    if village.residents.iter().take(10).any(|resident| {
        let tagged = village.actor_name(resident);
        let legacy = format!("{} [{}]", resident.name, village.name);
        game.mobs.iter().any(|mob| {
            mob.alive() && mob.kind.id == "minecraft:villager"
                && (mob.custom_name.as_deref() == Some(tagged.as_str())
                    || mob.custom_name.as_deref() == Some(legacy.as_str()))
        })
    }) {
        return Err("matching native villager already loaded: reconcile identity before initial spawn".into());
    }
    let positions = safe_actor_spawn_points(game, village);
    if positions.len() != 10 {
        return Err("need ten loaded, natural, clear and unoccupied positions before initial spawn".into());
    }
    Ok(positions)
}

/// Vanilla villagers are UUID-bound, once, after a persistent one-shot
/// reservation. A partial spawn NEVER triggers an automatic second batch.
pub(super) fn auto_spawn_villagers(
    game: &mut Game<'_>, state: &mut ActiveWorld, village_id: u64
) -> Result<(), String> {
    let village = state.world.as_ref().unwrap().settlements
        .get(&village_id).ok_or("unknown settlement")?.clone();
    if village.actors_spawned || !village.autonomy_enabled { return Ok(()); }
    // An incomplete preflight never reserves the one-shot batch. Missing
    // chunks or occupied plots are normal: retry only the PRE-spawn survey.
    let Ok(positions) = surveyed_initial_actor_positions(game, &village)
    else { return Ok(()); };
    let prior = state.world.as_ref().unwrap().clone();
    state.world.as_mut().unwrap().settlements
        .get_mut(&village_id).unwrap().actors_spawned = true;
    save_autonomous(state, prior)?; // mandatory before first entity is created
    for (resident, position) in village.residents.iter().take(10).zip(positions) {
        let options = NativeSpawn {
            name: Some(village.actor_name(resident)),
            ..NativeSpawn::default()
        };
        let entity = game.native_spawn_mob(
            &village.dimension, "minecraft:villager", position, &options
        ).ok_or_else(|| "automatic villager spawn refused; reserved, no blind respawn".to_owned())?;
        let uuid = game.native_entity(entity)
            .and_then(|handle| game.native_entity_uuid(handle))
            .ok_or("automatic spawn UUID unavailable; inspect/reconcile manually")?;
        state.world.as_mut().unwrap().bind_actor(village.id, resident.id, uuid)?;
        // If persistence fails, halt all future mutations rather than
        // creating another NPC with a new UUID after restart.
        save(state).map_err(|e| format!("persistence: {e}"))?;
    }
    log::info!("[civilizations] auto-spawned 10 persistent villagers for {}", village.name);
    Ok(())
}

/// Rename one spawned villager so its tag carries the role it really holds.
/// Horizon stores CustomName as a plain string, so no component JSON is used.
pub(super) fn rename_actor(game: &mut Game<'_>, uuid: u128, name: &str) -> bool {
    let Some((dimension, id, kind)) = game.native_entity_by_uuid(uuid) else {
        return false;
    };
    if kind != "minecraft:villager" {
        return false;
    }
    let mut nbt = Compound::new();
    nbt.insert("CustomName".into(), Tag::String(name.into()));
    nbt.insert("CustomNameVisible".into(), Tag::Byte(1));
    game.native_merge_entity_nbt(&dimension, id, nbt)
}
