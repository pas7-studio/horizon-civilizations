// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// Reads only loaded blocks in the currently selected dimension. Never loads
/// chunks and never authorizes overwriting player-made objects.
pub(super) fn world_block(game: &Game<'_>, dimension: &str, pos: [i32; 3]) -> Option<String> {
    if game.dim.id() != dimension || !game.world.loaded(pos[0], pos[2])
        || pos[1] < game.world.shape.min_y || pos[1] >= game.world.shape.max_y() {
        return None;
    }
    game.pack.assets.registry.state(game.world.block(pos[0], pos[1], pos[2]))
        .map(|state| state.name.to_string())
}

pub(super) fn check_empty_hut_site(game: &Game<'_>, dimension: &str, origin: [i32; 3]) -> Result<(), String> {
    for z in -2..=2 {
        for x in -2..=2 {
            let ground = [origin[0] + x, origin[1] - 1, origin[2] + z];
            let name = world_block(game, dimension, ground).ok_or("construction site not loaded")?;
            if !is_ground(&name) { return Err(format!("uneven or modified ground at {:?} ({name})", ground)); }
            for y in 0..=3 {
                let pos = [origin[0] + x, origin[1] + y, origin[2] + z];
                let name = world_block(game, dimension, pos).ok_or("construction site not loaded")?;
                if name != "minecraft:air" { return Err(format!("construction plot obstructed at {:?} ({name})", pos)); }
            }
        }
    }
    Ok(())
}

/// Natural growth a builder may clear to open a civic plot. Only plants and
/// trees: containers, player blocks and existing construction are never
/// touched, so a refused plot stays exactly as the player left it.
pub(super) fn is_clearable_growth(name: &str) -> bool {
    matches!(name,
        "minecraft:short_grass" | "minecraft:tall_grass" | "minecraft:fern"
        | "minecraft:large_fern" | "minecraft:leaf_litter" | "minecraft:dead_bush"
        | "minecraft:snow" | "minecraft:vine" | "minecraft:bush"
        | "minecraft:dandelion" | "minecraft:poppy" | "minecraft:blue_orchid"
        | "minecraft:allium" | "minecraft:azure_bluet" | "minecraft:oxeye_daisy"
        | "minecraft:cornflower" | "minecraft:lily_of_the_valley"
        | "minecraft:brown_mushroom" | "minecraft:red_mushroom"
        | "minecraft:sweet_berry_bush" | "minecraft:mangrove_roots")
        || name.ends_with("_sapling")
        || name.ends_with("_leaves")
        || name.ends_with("_log")
        || name.ends_with("_wood")
}

/// Remove natural growth from a bounded plot. Every cell is validated before
/// the first block changes, so a refused plot is never half cleared, and the
/// cell count is bounded so one turn cannot strip a forest.
pub(super) fn clear_growth_only(
    game: &mut Game<'_>, dimension: &str, origin: [i32; 3], half: i32, top: i32,
) -> Result<u32, String> {
    let mut doomed = Vec::new();
    for z in -half..=half {
        for x in -half..=half {
            for y in 0..=top {
                let pos = [origin[0] + x, origin[1] + y, origin[2] + z];
                let name = world_block(game, dimension, pos).ok_or("plot is not loaded")?;
                if name == "minecraft:air" { continue; }
                if is_clearable_growth(&name) {
                    doomed.push(pos);
                    continue;
                }
                // A natural one-block bump is ordinary earthwork: the builder
                // cuts it down to the plot floor, but only while real ground
                // stays underneath, so the plot never gains a hole.
                if y == 0 && is_ground(&name) {
                    let below = world_block(
                        game, dimension, [pos[0], origin[1] - 1, pos[2]],
                    ).ok_or("plot floor is not loaded")?;
                    if is_ground(&below) {
                        doomed.push(pos);
                        continue;
                    }
                }
                return Err(format!("{name} at {pos:?} is not natural growth"));
            }
        }
    }
    if doomed.len() > 120 {
        return Err("plot needs more clearing than one civic plot allows".into());
    }
    let mut cleared = 0_u32;
    for [x, y, z] in doomed {
        if game.native_run_command(&format!(
            "execute in {dimension} run setblock {x} {y} {z} minecraft:air replace"
        )) {
            cleared += 1;
        }
    }
    Ok(cleared)
}

/// Make one civic plot physically buildable. A plot that is merely overgrown
/// is opened by clearing that growth; anything else stays untouched.
pub(super) fn open_build_plot(
    game: &mut Game<'_>, dimension: &str, origin: [i32; 3], half: i32, top: i32,
    attempts: &mut u32,
    check: impl Fn(&Game<'_>) -> Result<(), String>,
) -> Result<u32, String> {
    if check(game).is_ok() { return Ok(0); }
    if *attempts == 0 { return Err("plot clearing budget spent this turn".into()); }
    *attempts -= 1;
    let cleared = clear_growth_only(game, dimension, origin, half, top)?;
    check(game).map_err(|error| format!("cleared {cleared} growth and still {error}"))?;
    Ok(cleared)
}

/// One real, resource-funded civic-hall construction step. The hall is
/// different from a numbered abstract house and never duplicates that stock.
pub(super) fn build_town_hall_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Builder) { return Ok(()); }
    let dimension = game.dim.id().to_owned();
    let next = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension)
        .filter_map(|v| v.hall_job.as_ref().map(|job| (v.id, job.clone())))
        .find(|(_, job)| !job.blocked && game.world.loaded(job.origin[0], job.origin[2]));
    let Some((village_id, job)) = next else { return Ok(()); };
    let observation = inspect_town_hall(&job, |point| world_block(game, &dimension, point));
    match observation {
        BuildInspection::Unloaded | BuildInspection::Paused => Ok(()),
        BuildInspection::InvalidCursor => Err("invalid civic hall cursor".into()),
        BuildInspection::Blocked { position, found } => {
            let v = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
            v.hall_job.as_mut().unwrap().blocked = true;
            save(state).map_err(|e| format!("persistence: {e}"))?;
            Err(format!("civic hall blocked at {position:?} by {found}"))
        }
        BuildInspection::Rewind(cursor) => {
            let v = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
            v.hall_job.as_mut().unwrap().cursor = cursor;
            save(state).map_err(|e| format!("persistence: {e}"))
        }
        BuildInspection::Advance => {
            let v = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
            v.hall_job.as_mut().unwrap().cursor += 1;
            save(state).map_err(|e| format!("persistence: {e}"))
        }
        BuildInspection::Complete => {
            state.world.as_mut().unwrap().finish_town_hall(village_id)?;
            save(state).map_err(|e| format!("persistence: {e}"))
        }
        BuildInspection::Place { position, old_block, new_block } => {
            let world = state.world.as_ref().unwrap();
            let v = &world.settlements[&village_id];
            let builders = loaded_actors(game, world, village_id);
            let near = v.residents.iter().filter(|r| r.alive && r.job == Job::Builder)
                .any(|r| builders.iter().any(|(id, mob)| {
                    *id == r.id && game.native_mob(&dimension, *mob).is_some_and(|actor| {
                        native_worker_near(actor.pos, job.origin, 9.0, 6.0)
                    })
                }));
            if !near { return Ok(()); }
            let [x, y, z] = position;
            // Plans store the block NAME because that is what a world read
            // returns; a wall sign still needs an explicit facing to place.
            let placed: &'static str = match new_block {
                "minecraft:oak_wall_sign" => "minecraft:oak_wall_sign[facing=north]",
                other => other,
            };
            let cmd = format!(
                "execute in {dimension} if block {x} {y} {z} {old_block} run setblock {x} {y} {z} {placed} replace"
            );
            if !game.native_run_command(&cmd) {
                log::warn!("[civilizations] civic hall block command refused: {cmd}");
            }
            Ok(())
        }
    }
}

/// A separate physically observed public bakery is constructed by an
/// actual UUID-bound builder. It neither advances a private hut cursor nor
/// consumes virtual stock; its full material bill was already extracted
/// from real hall barrels into bakery-specific escrow and durably saved.
pub(super) fn build_bakery_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Builder) { return Ok(()); }
    let dimension = game.dim.id().to_owned();
    let selected = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension)
        .filter_map(|v| v.bakery.as_ref().and_then(|b|
            b.building.as_ref().map(|job| (v.id, job.clone(), b.supplies_reserved))))
        .find(|(_, job, _)| !job.blocked && game.world.loaded(job.origin[0], job.origin[2]));
    let Some((id, job, funded)) = selected else { return Ok(()); };
    if job.cursor > 0 && !funded {
        return Err("unfunded bakery partial construction; inspect before resuming".into());
    }
    let inspection = inspect_bakery(&job, |point| world_block(game, &dimension, point));
    match inspection {
        BuildInspection::Unloaded | BuildInspection::Paused => Ok(()),
        BuildInspection::InvalidCursor => Err("invalid bakery construction cursor".into()),
        BuildInspection::Blocked {position, found} => {
            let old = state.world.as_ref().unwrap().clone();
            state.world.as_mut().unwrap().settlements.get_mut(&id).unwrap()
                .bakery.as_mut().unwrap().building.as_mut().unwrap().blocked = true;
            save_autonomous(state, old)?;
            Err(format!("public bakery site blocked at {position:?} by {found}"))
        }
        BuildInspection::Rewind(cursor) => {
            let old = state.world.as_ref().unwrap().clone();
            state.world.as_mut().unwrap().settlements.get_mut(&id).unwrap()
                .bakery.as_mut().unwrap().building.as_mut().unwrap().cursor = cursor;
            save_autonomous(state, old)
        }
        BuildInspection::Advance => {
            if !funded { return Err("bakery block exists without physical funding".into()); }
            let old = state.world.as_ref().unwrap().clone();
            state.world.as_mut().unwrap().settlements.get_mut(&id).unwrap()
                .bakery.as_mut().unwrap().building.as_mut().unwrap().cursor += 1;
            save_autonomous(state, old)
        }
        BuildInspection::Complete => {
            if !funded { return Err("cannot complete unfunded bakery".into()); }
            let old = state.world.as_ref().unwrap().clone();
            state.world.as_mut().unwrap().finish_bakery(id)?;
            save_autonomous(state, old)
        }
        BuildInspection::Place {position, old_block, new_block} => {
            if !funded { return Ok(()); }
            let world = state.world.as_ref().unwrap();
            let linked = loaded_actors(game, world, id);
            let v = &world.settlements[&id];
            let nearby_builder = v.residents.iter()
                .filter(|r| r.alive && r.job == Job::Builder)
                .any(|r| linked.iter().any(|(known, mob)| *known == r.id
                    && game.native_mob(&dimension, *mob).is_some_and(|actor| {
                        native_worker_near(actor.pos, job.origin, 9.0, 6.0)
                    })));
            if !nearby_builder { return Ok(()); }
            let [x,y,z] = position;
            let command = format!(
                "execute in {dimension} if block {x} {y} {z} {old_block} run setblock {x} {y} {z} {new_block} replace"
            );
            if !game.native_run_command(&command) {
                log::warn!("[civilizations] bakery conditional construction refused: {command}");
            }
            Ok(())
        }
    }
}

/// A physical builder must be nearby. The command path never edits blocks;
/// mutations occur on a later authoritative server tick.
pub(super) fn build_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    // Even a builder at the plot must not edit blocks off shift.
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Builder) { return Ok(()); }
    let dim = game.dim.id().to_string();
    // The next active build is chosen fairly, only among loaded plots in this
    // dimension. A paused first settlement cannot starve another village.
    let selected = state.world.as_ref().and_then(|world| {
        world.next_visual_job(&dim, (state.physical_ticks / 4) as usize, |pos| {
            game.world.loaded(pos[0], pos[2])
        })
    });
    let Some((village_id, job)) = selected else {
        return Ok(());
    };
    if !state.world.as_ref().unwrap().settlements[&village_id].hut_supplies_reserved
        && job.cursor > 0 {
        return Err("unfunded legacy partial hut; inspect before resuming real work".into());
    }
    // This pure function rechecks all previously reported blocks and makes
    // one conservative decision. It does not mutate the world or save.
    let inspection = inspect_hut(&job, |pos| world_block(game, &dim, pos));
    match inspection {
        BuildInspection::Unloaded | BuildInspection::Paused => Ok(()),
        BuildInspection::InvalidCursor => {
            Err(format!("invalid construction cursor for settlement {village_id}"))
        }
        BuildInspection::Blocked { position, found } => {
            let s = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
            s.visual_job.as_mut().unwrap().blocked = true;
            save(state).map_err(|e| format!("persistence: {e}"))?;
            Err(format!("construction blocked at {position:?} by {found}"))
        }
        BuildInspection::Rewind(cursor) => {
            let s = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
            s.visual_job.as_mut().unwrap().cursor = cursor;
            save(state).map_err(|e| format!("persistence: {e}"))?;
            Ok(())
        }
        BuildInspection::Advance => {
            if !state.world.as_ref().unwrap().settlements[&village_id].hut_supplies_reserved {
                return Err("observed unfunded hut block: manual inspection required".into());
            }
            let s = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
            s.visual_job.as_mut().unwrap().cursor += 1;
            save(state).map_err(|e| format!("persistence: {e}"))?;
            Ok(())
        }
        BuildInspection::Complete => {
            if !state.world.as_ref().unwrap().settlements[&village_id].hut_supplies_reserved {
                return Err("cannot close an unfunded physical hut".into());
            }
            state.world.as_mut().unwrap().finish_visual_house(village_id)?;
            save(state).map_err(|e| format!("persistence: {e}"))?;
            Ok(())
        }
        BuildInspection::Place { position, old_block, new_block } => {
            let world = state.world.as_ref().expect("active civilization world");
            let linked = loaded_actors(game, world, village_id);
            let builders: Vec<_> = world.settlements[&village_id].residents.iter()
                .filter(|r| r.alive && r.job == crate::game::civilizations_core::Job::Builder)
                .filter_map(|r| linked.iter().find(|(id, _)| *id == r.id).map(|(_, mob)| *mob))
                .collect();
            let near = builders.iter().any(|&id| game.native_mob(&dim, id).is_some_and(|mob| {
                native_worker_near(mob.pos, job.origin, 9.0, 6.0)
            }));
            if !near {
                return Ok(());
            }
            // Paying for a virtual house does NOT supply Minecraft items.
            // Debit actual warehouse goods exactly once before the first
            // physical block command and save the reservation first.
            if !state.world.as_ref().unwrap().settlements[&village_id].hut_supplies_reserved {
                let previous = state.world.as_ref().unwrap().clone();
                if !state.world.as_mut().unwrap().reserve_hut_supplies(village_id)? {
                    return Ok(()); // town hall board shows shortage
                }
                save_autonomous(state, previous)?;
            }
            let [x, y, z] = position;
            // Conditional server write: the saved cursor advances only after
            // the expected resulting block is observed on a later tick.
            let command = format!(
                "execute in {dim} if block {x} {y} {z} {old_block} run setblock {x} {y} {z} {new_block} replace"
            );
            if !game.native_run_command(&command) {
                log::warn!("[civilizations] conditional build command refused: {command}");
            }
            Ok(())
        }
    }
}

pub(super) fn world_farm_block(game: &Game<'_>, dimension: &str, pos: [i32; 3]) -> Option<FarmBlock> {
    if game.dim.id() != dimension || !game.world.loaded(pos[0], pos[2])
        || pos[1] < game.world.shape.min_y || pos[1] >= game.world.shape.max_y() {
        return None;
    }
    game.pack.assets.registry.state(game.world.block(pos[0], pos[1], pos[2]))
        .map(|state| FarmBlock {
            name: state.name.to_string(),
            age: state.properties.get("age").and_then(|value| value.parse::<u8>().ok()),
        })
}

pub(super) fn check_empty_farm_site(game: &Game<'_>, dimension: &str, origin: [i32; 3]) -> Result<(), String> {
    for z in -1..=1 {
        for x in -1..=1 {
            let ground = [origin[0] + x, origin[1] - 1, origin[2] + z];
            let name = world_block(game, dimension, ground).ok_or("farm plot is not loaded")?;
            if !is_ground(&name) {
                return Err(format!("farm ground must be natural and flat at {ground:?}: {name}"));
            }
            let above = [origin[0] + x, origin[1], origin[2] + z];
            let name = world_block(game, dimension, above).ok_or("farm plot is not loaded")?;
            if name != "minecraft:air" {
                return Err(format!("farm plot has an obstruction at {above:?}: {name}"));
            }
        }
    }
    Ok(())
}

/// Authoritative one-step physical farming, bounded to an already-loaded
/// level. The vanilla crop's age property determines maturity; neither
/// harvested items nor simulated wheat stock are minted by this phase.
pub(super) fn farm_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    // A farmer's proximity does not permit night-time crop edits.
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Farmer) { return Ok(()); }
    let dim = game.dim.id().to_owned();
    let selected = state.world.as_ref().and_then(|world| {
        world.next_farm(&dim, (state.physical_ticks / 8) as usize, |pos| {
            game.world.loaded(pos[0], pos[2])
        })
    });
    let Some((village_id, farm)) = selected else { return Ok(()); };
    let world = state.world.as_ref().expect("active civilization world");
    let linked = loaded_actors(game, world, village_id);
    let farmers: Vec<_> = world.settlements[&village_id].residents.iter()
        .filter(|resident| resident.alive && resident.job == Job::Farmer)
        .filter_map(|resident| linked.iter()
            .find(|(id, _)| *id == resident.id).map(|(_, mob)| *mob))
        .collect();
    let near = farmers.iter().any(|&id| game.native_mob(&dim, id).is_some_and(|mob| {
        native_worker_near(mob.pos, farm.origin, 8.0, 4.0)
    }));
    if !near { return Ok(()); }

    let decision = inspect_farm(&farm, |pos| world_farm_block(game, &dim, pos));
    match &decision {
        FarmDecision::Unloaded | FarmDecision::Paused => Ok(()),
        FarmDecision::InvalidCursor => Err(format!("invalid farm cursor for settlement {village_id}")),
        FarmDecision::Place { position, old_block, new_block } => {
            let [x, y, z] = *position;
            // A successful command does not advance the JSON cursor; it must
            // be observed on a future tick, avoiding command-vs-save races.
            let cmd = format!(
                "execute in {dim} if block {x} {y} {z} {old_block} run setblock {x} {y} {z} {new_block} replace"
            );
            if !game.native_run_command(&cmd) {
                log::warn!("[civilizations] conditional farm command refused: {cmd}");
            }
            Ok(())
        }
        _ => {
            let changed = state.world.as_mut().unwrap().advance_farm(village_id, decision.clone())?;
            if changed {
                save(state).map_err(|e| format!("persistence: {e}"))?;
            }
            if let FarmDecision::Blocked { position, found } = &decision {
                return Err(format!("farm blocked at {position:?} by {found}"));
            }
            Ok(())
        }
    }
}

/// A quarry needs no exposed stone: a pit starts on any natural surface near
/// the village. Bounded ring scan, first safe stand wins.
pub(super) fn quarry_start(game: &Game<'_>, dimension: &str, center: [i32; 3]) -> Option<[i32; 3]> {
    for radius in [20_i32, 24, 28, 32] {
        for (dx, dz) in [(radius, 0), (-radius, 0), (0, radius), (0, -radius),
                         (radius, radius), (-radius, radius), (radius, -radius), (-radius, -radius)] {
            let (Some(x), Some(z)) = (center[0].checked_add(dx), center[2].checked_add(dz))
            else { continue };
            for dy in [0_i32, 1, -1, 2] {
                let Some(y) = center[1].checked_add(dy) else { continue };
                let below = world_block(game, dimension, [x, y - 1, z]);
                if !below.as_deref().is_some_and(is_ground) { continue; }
                if world_block(game, dimension, [x, y, z]).as_deref() != Some("minecraft:air") {
                    continue;
                }
                if world_block(game, dimension, [x, y + 1, z]).as_deref() != Some("minecraft:air") {
                    continue;
                }
                return Some([x, y, z]);
            }
        }
    }
    None
}

/// An open-pit quarry: the bound miner walks a staircase down through real
/// ground until it reaches stone. Soil is excavation (no credit, no receipt);
/// a stone block goes through the normal receipt journal, so one cobble is
/// Remove one natural block (soil or growth) for a quarry pit. Never a player
/// block, water or a container: a pit only ever opens natural ground.
pub(super) fn dig_natural(game: &mut Game<'_>, dimension: &str, position: [i32; 3]) -> bool {
    let Some(block) = world_block(game, dimension, position) else { return false; };
    if block == "minecraft:air" { return false; }
    if !(is_ground(&block) || is_clearable_growth(&block)) { return false; }
    let [x, y, z] = position;
    game.native_run_command(&format!(
        "execute in {dimension} run setblock {x} {y} {z} minecraft:air replace"
    ))
}

/// credited only after that same worker returns to the hall.
pub(super) fn quarry_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Miner) { return Ok(()); }
    let dimension = game.dim.id().to_owned();
    let target = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension)
        .find_map(|v| v.posts.iter().find(|post| post.kind == PostKind::Quarry)
            .map(|post| (v.id, post.origin, v.quarry_step, post.worker_id)));
    let Some((id, origin, step, bound)) = target else { return Ok(()); };
    if step >= MAX_QUARRY_STEPS { return Ok(()); }
    let depth = i32::from(step);
    // Repair every opened step to two blocks of headroom and two wide: a
    // one-wide, one-high tunnel is something the villager AI will not walk
    // into, so the stone below it could never be reached.
    if state.physical_ticks % 40 == 0 {
        for k in 0..=depth {
            for dz in 0..=1 {
                dig_natural(game, &dimension, [origin[0] + k, origin[1] - k + 1, origin[2] + dz]);
            }
        }
    }
    // A planned stone receipt owns the pit until it is settled.
    if state.world.as_ref().unwrap().settlements[&id].gather_job.is_some() {
        return Ok(());
    }
    let position = [origin[0] + depth, origin[1] - depth, origin[2]];
    let Some(name) = world_block(game, &dimension, position) else { return Ok(()); };
    // A fresh pit has no bound digger yet: the village's own miner opens it.
    let world = state.world.as_ref().unwrap();
    let village = &world.settlements[&id];
    let village_name = village.name.clone();
    let Some(worker) = bound.or_else(|| village.residents.iter()
        .filter(|r| r.alive && r.job == Job::Miner).map(|r| r.id).min())
    else { return Ok(()); };
    // Only the bound miner digs this pit, and only while standing at it.
    let linked = loaded_actors(game, world, id);
    let near = village.residents.iter()
        .filter(|r| r.alive && r.id == worker && r.job == Job::Miner)
        .any(|r| linked.iter().any(|(known, mob)| *known == r.id
            && game.native_mob(&dimension, *mob).is_some_and(|actor|
                native_worker_near(actor.pos, position, 4.5, 5.0))));
    if !near {
        if state.physical_ticks % 600 == 0 {
            log::info!(
                "[civilizations] {village_name} quarry waits for miner #{worker} at {position:?}"
            );
        }
        return Ok(());
    }
    let before = state.world.as_ref().unwrap().clone();
    if bound.is_none() {
        // Bind the digger once: the pit and its stone receipts then always
        // belong to this resident.
        if state.world.as_mut().unwrap()
            .set_post_worker(id, PostKind::Quarry, worker).is_ok()
        {
            save_autonomous(state, before.clone())?;
            log::info!("[civilizations] {village_name} bound its miner to the quarry pit");
        }
    }
    if name == "minecraft:air" {
        state.world.as_mut().unwrap().advance_quarry(id)?;
        return save_autonomous(state, before);
    }
    if name == "minecraft:stone" {
        let Some(stand) = gather_stand(game, &dimension, position) else {
            // No room to stand behind the stone yet: open the headroom of the
            // step behind it and let the next tick find the stand.
            let back = depth.saturating_sub(1).max(1);
            let prev = [origin[0] + back, origin[1] - back, origin[2]];
            if let Some(head) = prev[1].checked_add(1) {
                dig_natural(game, &dimension, [prev[0], head, prev[2]]);
                dig_natural(game, &dimension, [prev[0], head, prev[2] + 1]);
            }
            if state.physical_ticks % 600 == 0 {
                log::info!(
                    "[civilizations] {village_name} quarry reached stone at {position:?} but has no safe stand"
                );
            }
            return Ok(());
        };
        match state.world.as_mut().unwrap()
            .plan_quarry_gather(id, position, stand)
        {
            Ok(()) => {
                save_autonomous(state, before)?;
                log::info!("[civilizations] {village_name} quarry reached stone at {position:?}");
            }
            Err(error) => {
                if state.physical_ticks % 600 == 0 {
                    log::info!(
                        "[civilizations] {village_name} quarry stone at {position:?} refused: {error}"
                    );
                }
            }
        }
        return Ok(());
    }
    if is_ground(&name) || is_clearable_growth(&name) {
        let [x, y, z] = position;
        // A two-wide, two-high stair the villager can actually walk down.
        dig_natural(game, &dimension, [x, y, z]);
        if let Some(head) = y.checked_add(1) {
            dig_natural(game, &dimension, [x, head, z]);
            dig_natural(game, &dimension, [x, head, z + 1]);
        }
        dig_natural(game, &dimension, [x, y, z + 1]);
        state.world.as_mut().unwrap().advance_quarry(id)?;
        return save_autonomous(state, before);
    }
    // A player block or water stops the pit: never dig through it.
    Ok(())
}
