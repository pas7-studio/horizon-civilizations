// Horizon bridge. All world logic lives in civilizations_core; this module
// translates server hooks, commands, inventory changes, and native NPC spawns.
use crate::game::{
    civilizations_core::{farm_plan, hut_plan, town_hall_plan, bakery_plan, hall_storage_positions,
        BarrelDirection, BarrelPhase, inspect_farm, inspect_hut, inspect_town_hall, inspect_bakery, is_ground,
        is_rooted_oak_harvest_origin, short_oak_stem_height, inspect_oak_stem,
        native_worker_near, native_worker_reaches_block, PostKind, POST_RADIUS,
        BuildInspection, DailyPhase, FarmBlock, FarmDecision, FarmStage, Good, HomeSlot, Job, VisualBuildJob,
        World, inspect_home, hall_board_positions, HOUSING_PLOT_OFFSETS, MAX_HOUSING_SURVEY_RESULTS,
        MAX_SIM_DAYS},
    native_mobs::NativeSpawn,
    native_server::{NativeCommand, NativeServerPlayer},
    Game,
};
use horizon_client::text::{self, Text};
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

const MOD_ID: &str = "civilizations";
const DIR: &str = "horizon-civilizations";
const SAVE: &str = "world.json";

struct ActiveWorld {
    path: PathBuf,
    world: Option<World>,
    problem: Option<String>,
    physical_ticks: u64,
}

static ACTIVE: Mutex<Option<ActiveWorld>> = Mutex::new(None);

fn load(path: PathBuf) -> ActiveWorld {
    let file = path.join(SAVE);
    if path.join("world.json.swap").exists() {
        let message = "interrupted save detected (world.json.swap). Back up the folder, then restore the correct primary world.json manually".to_owned();
        return ActiveWorld { path, world: None, problem: Some(message), physical_ticks: 0 };
    }
    let result = match fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str::<World>(&text)
            .map_err(|e| format!("{}: invalid save: {e}", file.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(World::default()),
        Err(e) => Err(format!("{}: {e}", file.display())),
    }.and_then(|world| {
        world.validate().map(|()| world)
    });
    let (world, problem) = match result {
        Ok(world) => (Some(world), None),
        Err(error) => {
            log::error!("[civilizations] refusing to touch damaged data: {error}");
            (None, Some(error))
        }
    };
    ActiveWorld { path, world, problem, physical_ticks: 0 }
}

fn active(path: PathBuf) -> MutexGuard<'static, Option<ActiveWorld>> {
    let mut locked = ACTIVE.lock().unwrap_or_else(|poison| poison.into_inner());
    match locked.as_ref() {
        None => {
            *locked = Some(load(path));
        }
        Some(old) if old.path == path => {}
        Some(old) if old.world.is_none() => {
            // No in-memory state to lose; invalid on-disk world remains untouched.
            *locked = Some(load(path));
        }
        Some(old) => {
            // Retry saving even if a previous operation had failed. Never
            // discard a valid mutated world unless persistence succeeds.
            if let Err(error) = save(old) {
                log::error!(
                    "[civilizations] refusing world switch; outgoing save failed: {error}"
                );
                // All hooks for the requested new world stop on this problem.
                if let Some(state) = locked.as_mut() {
                    state.problem = Some(format!(
                        "world switch prevented; outgoing save failed: {error}"
                    ));
                }
                return locked;
            }
            *locked = Some(load(path));
        }
    }
    locked
}

fn save(active: &ActiveWorld) -> Result<(), String> {
    let world = active.world.as_ref().ok_or("world data unavailable")?;
    world.validate().map_err(|e| format!("refusing to save invalid world: {e}"))?;
    fs::create_dir_all(&active.path).map_err(|e| e.to_string())?;
    let output = serde_json::to_vec_pretty(world).map_err(|e| e.to_string())?;
    let dest = active.path.join(SAVE);
    // A stale swap marks an interrupted save. Refuse it before touching
    // either the previous backup or the next temporary file.
    let swap = active.path.join("world.json.swap");
    if swap.exists() {
        return Err("pending world.json.swap from interrupted save; inspect and recover before continuing".into());
    }
    let temp = active.path.join("world.json.tmp");
    let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&output).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    // Keep a recoverable older save. A failed backup aborts the write rather
    // than destroying the only known-good state.
    if dest.exists() {
        fs::copy(&dest, active.path.join("world.json.bak")).map_err(|e| e.to_string())?;
    }
    // Windows cannot rename a file over an existing destination. Keep a
    // recoverable swap file between the two renames; never overwrite a stale
    // swap left by a crash (that must be inspected manually).
    if dest.exists() { fs::rename(&dest, &swap).map_err(|e| e.to_string())?; }
    if let Err(e) = fs::rename(&temp, &dest) {
        if swap.exists() { let _ = fs::rename(&swap, &dest); }
        return Err(e.to_string());
    }
    // The new primary is already committed. A cleanup failure must not be
    // reported as an uncommitted trade after inventory items have moved.
    if swap.exists() {
        if let Err(e) = fs::remove_file(&swap) {
            log::warn!("[civilizations] committed world save, but stale swap needs cleanup: {e}");
        }
    }
    Ok(())
}

pub fn closed() {
    let mut guard = ACTIVE.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(state) = guard.as_ref() {
        if state.world.is_some() {
            // Keep in-memory data on write failure until the disk is fixed,
            // instead of silently discarding it on close.
            if let Err(error) = save(state) {
                log::error!(
                    "[civilizations] shutdown save failed; retaining in-memory world: {error}"
                );
                if let Some(state) = guard.as_mut() {
                    state.problem = Some(format!("shutdown save failed: {error}"));
                }
                return;
            }
        }
    }
    *guard = None;
}

pub fn register(commands: &mut Vec<NativeCommand>) {
    for usage in [
        "civ", "civ help", "civ list", "civ info", "civ info <name:word>",
        "civ roster <name:word>", "civ schedule <name:word>",
        "civ homes <name:word>", "civ survey <name:word>",
        "civ council <name:word>", "civ forecast <name:word>",
        "civ plots <name:word>", "civ development <name:word>",
        "civ hall <name:word>", "civ warehouse <name:word>", "civ bakery <name:word>",
        "civ posts <name:word>",
        "civ diagnose <name:word>",
        "civ construction <name:word>", "civ farming <name:word>",
        "civ harvests <name:word>",
        "civ join <name:word>", "civ title", "civ promote", "civ history",
        "civ sell <good:word> <amount:integer>", "civ buy <good:word> <amount:integer>",
    ] {
        commands.push(NativeCommand::new(usage));
    }
    for usage in [
        "civ found <name:word>", "civ actors <name:word>",
        "civ reconcile <name:word>",
        "civ day [days:integer]", "civ construct <name:word>",
        "civ post <name:word> <kind:word>",
        "civ approvehouse <name:word> <slot:integer> <y:integer>",
        "civ autonomy <name:word> <mode:word>",
        "civ barrelcheck <name:word>",
        "civ gathercheck <name:word>",
        "civ discardgather <name:word> <observed:word>",
        "civ resolvebarrel <name:word> <mode:word> <count:integer>",
        "civ farm <name:word>", "civ recoverfarm <name:word>",
        "civ buyplot <name:word>", "civ buyhouse <name:word>", "civ commission <name:word>",
        "civ war <from:word> <to:word>",
        "civ peace <from:word> <to:word>",
    ] {
        commands.push(NativeCommand::op(usage, 2));
    }
}

fn reply(game: &mut Game<'_>, message: impl Into<String>) {
    game.native_feedback(Text::literal(message.into()));
}

fn error(game: &mut Game<'_>, message: impl Into<String>) {
    game.native_feedback(Text::literal(message.into()).with_color(text::RED));
}

fn parse_amount(s: Option<&str>) -> Result<u32, String> {
    let n = s.ok_or("amount required")?.parse::<u32>().map_err(|_| "invalid amount")?;
    if !(1..=64).contains(&n) { return Err("amount must be 1..64".into()); }
    Ok(n)
}

fn own_settlement(world: &World, player_id: &str) -> Result<u64, String> {
    world.players.get(player_id).and_then(|p| p.settlement).ok_or("join a settlement first".into())
}

/// Find our previously named actors even after the world has been reopened.
/// Only loaded entities are inspected; missing chunks are never force-loaded.
/// The name is a soft identity, not proof against an intentionally renamed mob.
fn loaded_actors(game: &mut Game<'_>, world: &World, settlement_id: u64) -> Vec<(u64, u32)> {
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
fn loaded_home_target(game: &Game<'_>, dimension: &str, home: HomeSlot) -> Option<[f64; 3]> {
    if !inspect_home(home, |position| world_block(game, dimension, position)) {
        return None;
    }
    let [x, y, z] = home.foot();
    Some([f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5])
}

/// Profession-aware day/night routing, limited to loaded UUID-bound actors.
/// This does not implement actual sleeping or independent resource generation.
fn guide_actors(game: &mut Game<'_>, world: &World) {
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

/// Reads only loaded blocks in the currently selected dimension. Never loads
/// chunks and never authorizes overwriting player-made objects.
fn world_block(game: &Game<'_>, dimension: &str, pos: [i32; 3]) -> Option<String> {
    if game.dim.id() != dimension || !game.world.loaded(pos[0], pos[2])
        || pos[1] < game.world.shape.min_y || pos[1] >= game.world.shape.max_y() {
        return None;
    }
    game.pack.assets.registry.state(game.world.block(pos[0], pos[1], pos[2]))
        .map(|state| state.name.to_string())
}

fn check_empty_hut_site(game: &Game<'_>, dimension: &str, origin: [i32; 3]) -> Result<(), String> {
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

/// One real, resource-funded civic-hall construction step. The hall is
/// different from a numbered abstract house and never duplicates that stock.
fn build_town_hall_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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
fn build_bakery_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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
fn build_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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


fn world_farm_block(game: &Game<'_>, dimension: &str, pos: [i32; 3]) -> Option<FarmBlock> {
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

fn check_empty_farm_site(game: &Game<'_>, dimension: &str, origin: [i32; 3]) -> Result<(), String> {
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
fn farm_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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

/// Native API 334: stop hopper transfer *to or from* the two public
/// hall barrel inventories. This is not a global hopper ban and does not
/// intercept NPC's direct native container API (which is not a hopper move).
pub fn protect_public_hopper(
    game: &Game<'_>, source: [i32;3], destination: [i32;3],
    _item: &str, cancel: &mut bool,
) {
    if *cancel || !game.native_on(MOD_ID) { return; }
    let Some(dir) = game.native_world_dir() else { return; };
    let dim = game.dim.id();
    let locked = active(dir.join(DIR));
    let public = locked.as_ref().and_then(|a| a.world.as_ref())
        .is_some_and(|world| world.settlements.values()
            .filter(|v| v.dimension == dim && v.hall_complete)
            .any(|v| {
                let positions = hall_storage_positions(v.center);
                positions.contains(&source) || positions.contains(&destination)
            }));
    if public { *cancel = true; }
}

/// Native API 14: prevent ordinary players damaging registered public
/// buildings and private parcels they do not own. The owner may break
/// blocks inside their own bought 5x5 plot; OP may recover city structures.
/// Does not claim the wilderness or override third-party region plugins.
pub fn protect_civilization_block(
    game: &mut Game<'_>, position: [i32;3], cancel: &mut bool,
) {
    if *cancel || !game.native_on(MOD_ID) { return; }
    let Some(player) = game.native_acting_player() else { return; };
    if player.permission >= 2 { return; }
    let Some(dir) = game.native_world_dir() else { return; };
    let lock = active(dir.join(DIR));
    let actor = format!("{:032x}", player.uuid);
    let protected = lock.as_ref().and_then(|active| active.world.as_ref())
        .is_some_and(|world| !world.player_may_break(
            &actor, game.dim.id(), position
        ));
    drop(lock);
    if protected {
        *cancel = true;
    }
}

/// Public civic barrels are accounted stock, not free player loot. Only
/// real server operators may open them to inspect native contents; ordinary
/// villagers trade using the native inventory command path. This hook does
/// not claim arbitrary vanilla barrels or player-owned storage.
pub fn protect_hall_inventory(
    game: &mut Game<'_>, player: u128, _name: &str,
    position: [i32; 3], cancel: &mut bool,
) {
    if *cancel || !game.native_on(MOD_ID) { return; }
    let dimension = game.dim.id().to_owned();
    if world_block(game, &dimension, position).as_deref() != Some("minecraft:barrel") {
        return;
    }
    let Some(dir) = game.native_world_dir() else { return; };
    let guard = active(dir.join(DIR));
    let is_public_barrel = guard.as_ref().and_then(|state| state.world.as_ref())
        .is_some_and(|world| world.settlements.values()
            .filter(|v| v.dimension == dimension && v.hall_complete)
            .any(|v| hall_storage_positions(v.center).contains(&position)));
    drop(guard);
    if !is_public_barrel { return; }
    if game.native_player_permission(player).unwrap_or(0) >= 2 { return; }
    *cancel = true;
    let _ = game.native_tell(player, Text::literal(
        "This is the village's public warehouse. Trade with /civ sell and /civ buy; items are reserved for real NPC construction."
    ).with_color(text::GOLD));
}

/// Right-click dialogue for a UUID-bound civilization villager. Unbound or
/// vanilla villagers keep their normal Minecraft interaction.
pub fn interact_entity(game: &mut Game<'_>, entity: i32, kind: &str, claimed: &mut bool) {
    if *claimed || kind != "minecraft:villager" || !game.native_on(MOD_ID) {
        return;
    }
    let Some(player) = game.native_acting_player() else {
        return;
    };
    let Some(handle) = u32::try_from(entity).ok().and_then(|id| game.native_entity(id)) else {
        return;
    };
    let Some(uuid) = game.native_entity_uuid(handle) else {
        return;
    };
    let Some(dir) = game.native_world_dir() else {
        return;
    };
    let guard = active(dir.join(DIR));
    let Some(world) = guard.as_ref()
        .filter(|state| state.problem.is_none())
        .and_then(|state| state.world.as_ref())
    else {
        return;
    };
    let Some((village, resident)) = world.settlements.values()
        .filter(|village| village.dimension == player.dimension)
        .find_map(|village| {
            village.residents.iter()
                .find(|resident| resident.alive && resident.entity_uuid == Some(uuid))
                .map(|resident| (village, resident))
        })
    else {
        return;
    };
    let membership = world.players
        .get(&format!("{:032x}", player.uuid))
        .and_then(|p| p.settlement);
    let invitation = if membership == Some(village.id) {
        format!("You are a member of {}.", village.name)
    } else {
        format!("To join this settlement: /civ join {}", village.name)
    };
    let phase = DailyPhase::at(game.world.day_time);
    let duty = if phase.on_shift(resident.job) { "on duty" } else { "off duty" };
    let role = if resident.job == Job::Merchant {
        "mayor / warehouse steward"
    } else { resident.job.name() };
    let council = village.council_priorities().first()
        .map(|priority| format!(
            "Council: {} {}/100", priority.need.label(), priority.urgency
        ))
        .unwrap_or_else(|| "Council: stable".to_owned());
    let home = match village.home_slot(resident.id) {
        Some(slot) => format!("Hut {:?}, place {} (physical blocks checked at night)", slot.site, slot.lane + 1),
        None => "Commons (no completed physical home)".to_owned(),
    };
    let message = format!(
        "{} — {} of {} (morale {}). Routine: {} / {}. Home: {}. {}. {} View: /civ info {}. Trade: /civ buy/sell <good> <amount>.",
        resident.name, role, village.name, village.morale,
        phase.name(), duty, home, council, invitation, village.name
    );
    drop(guard);
    // Do not consume a vanilla interaction unless the information was sent.
    if game.native_tell(player.uuid, Text::literal(message).with_color(text::GOLD)) {
        *claimed = true;
    }
}

fn quarantined_native_purchase(state: &mut ActiveWorld, village_id: u64, reason: &str) -> String {
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
fn buy_from_real_barrel(
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

pub fn perform(game: &mut Game<'_>, player: &NativeServerPlayer, command: &str, ran: &mut bool) {
    let mut args = command.split_whitespace();
    if args.next() != Some("civ") { return; }
    *ran = true;
    if !game.native_on(MOD_ID) {
        error(game, "Civilizations is disabled");
        return;
    }
    let Some(world_dir) = game.native_world_dir() else { error(game, "no server world open"); return; };
    let mut guard = active(world_dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if let Some(problem) = &state.problem { error(game, format!("save error: {problem}")); return; }
    let Some(world) = state.world.as_ref() else { error(game, "world is unavailable"); return; };
    let actor = format!("{:032x}", player.uuid);
    let action = args.next().unwrap_or("help");
    if action == "help" {
        reply(game, "Civilizations: /civ list | info [name] | roster <name> | schedule <name> | homes <name> | survey <name> | council <name> | forecast <name> | plots <name> | development <name> | hall <name> | warehouse <name> | posts <name> | diagnose <name> | buyplot <name> | buyhouse <name> | commission <name> | construction <name> | farming <name> | harvests <name> | join <name> | title | promote | history | buy/sell <good> <1..64> | (OP) found <name>, actors <name>, reconcile <name>, barrelcheck <name>, resolvebarrel <name> confirm|cancel <observed_count>, gathercheck <name>, discardgather <name> intact|empty, day [days], construct <name>, post <name> lumber|quarry, approvehouse <name> <slot> <y>, autonomy <name> on/off, farm <name>, recoverfarm <name>, war/peace <from> <to>. Goods: wheat, bread, wood (oak logs), stone (cobble), iron.");
        return;
    }
    if action == "list" {
        let mut items = Vec::new();
        for s in world.settlements.values().take(12) {
            items.push(format!("#{} {} [{}] pop={} houses={}", s.id, s.name, s.culture, s.population(), s.houses));
        }
        reply(game, if items.is_empty() { "No settlements yet. OP: /civ found Ashford".into() } else { items.join(" | ") });
        return;
    }
    if action == "hall" || action == "warehouse" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ hall <name>");
            return;
        };
        let mayor = village.residents.iter()
            .find(|r| r.alive && r.job == Job::Merchant)
            .map(|r| r.name.as_str()).unwrap_or("unassigned");
        let goods = [Good::Wheat, Good::Bread, Good::Wood, Good::Stone, Good::Iron];
        let incoming = goods.iter().map(|good| format!(
            "{}:{}", good.as_str(), village.warehouse_quantity(*good)
        )).collect::<Vec<_>>().join(" / ");
        let stored = if village.hall_complete {
            physical_barrel_stock(game, &village.dimension, village.center)
        } else { None };
        let physical = stored.as_ref().map(|items| items.iter()
            .map(|(good, count)| format!("{}:{}", good.as_str(), count))
            .collect::<Vec<_>>().join(" / "))
            .unwrap_or_else(|| "unloaded / not constructed".to_owned());
        let escrow = goods.iter().map(|good| format!(
            "{}:{}", good.as_str(), village.build_escrow.get(good).copied().unwrap_or(0)
        )).collect::<Vec<_>>().join(" / ");
        let (hall_wood, hall_stone) = village.hall_materials_missing();
        let journal_pending = village.barrel_transfer.is_some()
            || village.market_purchase.is_some();
        let demand = if journal_pending { None } else {
            stored.as_ref().map(|items|
                crate::game::civilizations_core::physical_construction_shortfall(village, items)
            )
        };
        let outstanding = if journal_pending {
            "unknown (native transaction must reconcile first)".to_owned()
        } else {
            demand.map(|(wood, stone)| format!("{wood} logs / {stone} cobble"))
                .unwrap_or_else(|| "unknown (native barrels unloaded/unavailable)".to_owned())
        };
        let transfer = village.barrel_transfer.as_ref().map(|t| format!(
            "{:?} {:?} {} x{} at {:?}, before {}, phase {:?}",
            t.direction, t.good, t.good.item(), t.count, t.position,
            t.before, t.phase
        )).unwrap_or_else(|| "none".to_owned());
        let purchase = village.market_purchase.as_ref().map(|order| format!(
            "{} bought {} x{} for {} coins, source {:?}, phase {:?}; no automatic retries",
            order.buyer, order.good.as_str(), order.count, order.price,
            order.position, order.phase
        )).unwrap_or_else(|| "none".to_owned());
        if action == "warehouse" {
            reply(game, format!(
                "{} ACTUAL BARRELS [{}]. Awaiting steward delivery [{}]. Farmer carrying {} wheat. Physically extracted hut escrow [{}]. Combined pending construction deficit: {}. Durable transfer: {}. Market order: {}. Native barrels are authoritative after hall completion; abstract stock is separate. Unresolved journal operations suspend new gathering.",
                village.name, physical, incoming, village.wheat_delivery_pending(), escrow,
                outstanding, transfer, purchase
            ));
        } else {
            let building = village.hall_job.as_ref().map(|job| format!(
                "{}/{} blocks verified at {:?}, blocked={}",
                job.cursor, town_hall_plan(job.origin).len(), job.origin, job.blocked
            )).unwrap_or_else(|| "waiting for funded construction".to_owned());
            let condition = if village.hall_complete {
                "COMPLETED: native barrel warehouse, mayor desk, 2 information signs"
            } else { "UNDER CONSTRUCTION / UNFUNDED" };
            reply(game, format!(
                "{} civic center: {}. Mayor: {}. {}. Native stored items [{}], incoming [{}]. Hall funding deficit: {} logs / {} cobble. {}",
                village.name, condition, mayor, building, physical, incoming,
                hall_wood, hall_stone,
                if village.barrel_transfer.is_some() {
                    "See /civ warehouse for pending or blocked transfer"
                } else { "Warehouse transfer journal clear" }
            ));
        }
        return;
    }
    if action == "diagnose" {
        // Read-only first-play troubleshooting. No world save, no player
        // inventory access, no native mutations and no inferred offscreen NPC.
        let name = args.next().unwrap_or("");
        let Some(v) = world.by_name(name) else {
            error(game, "unknown settlement: /civ diagnose <name>");
            return;
        };
        if v.dimension != player.dimension {
            reply(game, format!(
                "{}: visit dimension {} for native actor, block and barrel checks. No cross-world state is assumed.",
                v.name, v.dimension
            ));
            return;
        }
        let linked = loaded_actors(game, world, v.id);
        let loaded_role = |role: Job| v.residents.iter().any(|resident| {
            resident.alive && resident.job == role
                && linked.iter().any(|(id, _)| *id == resident.id)
        });
        let phase = DailyPhase::at(game.world.day_time);
        let hall_status = if v.hall_complete {
            match physical_barrel_stock(game, &v.dimension, v.center) {
                Some(stock) if v.barrel_transfer.is_none() && v.market_purchase.is_none() => {
                    let (wood, stone) =
                        crate::game::civilizations_core::physical_construction_shortfall(v, &stock);
                    format!("COMPLETE; native barrel snapshot available; current construction gap {wood} logs / {stone} cobble")
                }
                Some(_) => "COMPLETE; native journals unresolved, stock needs reconciliation".to_owned(),
                None => "COMPLETE in save; real native barrels missing or unloaded".to_owned(),
            }
        } else if let Some(job) = &v.hall_job {
            format!("BUILDING {}/{} blocks, blocked={}",
                job.cursor, town_hall_plan(job.origin).len(), job.blocked)
        } else {
            let (wood, stone) = v.hall_materials_missing();
            format!("UNBUILT; confirmed incoming hall deficit {wood} logs / {stone} cobble")
        };
        let bakery_status = match &v.bakery {
            Some(b) if b.complete => format!(
                "COMPLETE; wheat escrow {}, bread in transit {}, batches {}",
                b.wheat_escrow, b.bread_in_transit, b.batches
            ),
            Some(b) => format!(
                "BUILDING; funded={}, cursor={}, blocked={}",
                b.supplies_reserved,
                b.building.as_ref().map(|job| job.cursor).unwrap_or(0),
                b.building.as_ref().is_some_and(|job| job.blocked),
            ),
            None => "NOT PLANNED".to_owned(),
        };
        let blocker = if !v.autonomy_enabled {
            "autonomy is PAUSED; OP can enable /civ autonomy NAME on".to_owned()
        } else if !v.actors_spawned {
            "villager identity reservation not started; keep center loaded".to_owned()
        } else if linked.is_empty() {
            "no UUID-bound resident is currently loaded; inspect /civ roster NAME and chunks".to_owned()
        } else if v.market_purchase.is_some() {
            "native player purchase unresolved; inspect /civ warehouse NAME (NO automatic refund)".to_owned()
        } else if v.barrel_transfer.is_some() {
            "native barrel journal pending; OP: /civ barrelcheck NAME; never blindly retry".to_owned()
        } else if v.hall_job.as_ref().is_some_and(|job| job.blocked) {
            "civic hall construction blocked by modified ground; inspect blocks".to_owned()
        } else if v.bakery.as_ref().and_then(|b| b.building.as_ref())
            .is_some_and(|job| job.blocked) {
            "bakery construction blocked by modified ground; inspect blocks".to_owned()
        } else if v.gather_job.as_ref().is_some_and(|job| job.blocked) {
            if v.gather_job.as_ref().is_some_and(|job| job.issued) {
                "issued wood/stone gather QUARANTINED; OP: /civ gathercheck NAME, then inspect before /civ discardgather NAME intact|empty".to_owned()
            } else {
                "never-issued gather candidate blocked; planner may discard without credit".to_owned()
            }
        } else if !loaded_role(Job::Builder) {
            "no loaded UUID-bound builder; required for real buildings".to_owned()
        } else if !phase.on_shift(Job::Builder) {
            "workers are off shift; wait for Minecraft daytime".to_owned()
        } else if v.hall_complete && physical_barrel_stock(game, &v.dimension, v.center).is_none() {
            "official hall barrel block entities unavailable; load and inspect both barrels".to_owned()
        } else if !v.hall_complete && v.hall_materials_missing() != (0, 0) {
            "hall lacks confirmed physical wood/stone; deliver items or let UUID-bound workers gather".to_owned()
        } else if v.hall_complete && v.bakery.is_none() && !loaded_role(Job::Baker) {
            "no loaded UUID-bound baker for first real production workshop".to_owned()
        } else {
            "no explicit saved-state blocker; check natural plots, NPC routes and native game logs".to_owned()
        };
        let gather_state = if let Some(stem) = &v.oak_stem {
            format!("oak log {}/{} at {:?}, issued={}, pickup={}, blocked={}",
                stem.cursor + 1, stem.height,
                v.gather_job.as_ref().map(|job| (job.position, job.worker_id)),
                v.gather_job.as_ref().is_some_and(|job| job.issued),
                v.gather_job.as_ref().is_some_and(|job| job.pickup_witnessed),
                v.gather_job.as_ref().is_some_and(|job| job.blocked))
        } else {
            v.gather_job.as_ref().map(|job| format!(
                "{:?} at {:?}, worker={:?}, issued={}, pickup={}, blocked={}",
                job.good, job.position, job.worker_id, job.issued,
                job.pickup_witnessed, job.blocked
            )).unwrap_or_else(|| "idle".to_owned())
        };
        reply(game, format!(
            "{} DIAG: autonomous={} | loaded UUID actors {}/{} | builder={} farmer={} lumberjack={} miner={} baker={} steward={} | shift={} | hall=[{}] | bakery=[{}] | farm={} | gather={} | blocker: {}. This is a read-only snapshot, not a gameplay pass.",
            v.name, v.autonomy_enabled, linked.len(), v.population(),
            loaded_role(Job::Builder), loaded_role(Job::Farmer),
            loaded_role(Job::Lumberjack), loaded_role(Job::Miner),
            loaded_role(Job::Baker), loaded_role(Job::Merchant),
            phase.name(), hall_status, bakery_status, v.farm.is_some(),
            gather_state, blocker
        ));
        return;
    }
    if action == "gathercheck" {
        if player.permission < 2 {
            error(game, "resource gathering audit requires server operator level 2");
            return;
        }
        let name = args.next().unwrap_or("");
        let Some(v) = world.by_name(name) else {
            error(game, "unknown settlement: /civ gathercheck <name>");
            return;
        };
        if v.dimension != player.dimension {
            error(game, "visit the settlement dimension for native block inspection");
            return;
        }
        let Some(job) = &v.gather_job else {
            reply(game, format!("{}: no pending natural wood/stone gather", v.name));
            return;
        };
        let observed = world_block(game, &v.dimension, job.position);
        let stem_progress = v.oak_stem.as_ref().map(|stem| format!(
            " Short oak stem log {}/{} rooted at {:?}; one durable delivery per segment.",
            stem.cursor + 1, stem.height, stem.root
        )).unwrap_or_default();
        reply(game, format!(
            "{} gather audit: {:?} at {:?}, worker stand {:?}, assigned resident {:?}, planned native block {}, issued={}, source pickup witnessed={}, blocked={}, live native block {:?}.{} Blocked ISSUED receipts are quarantined. Inspect world save and original block before OP /civ discardgather {} intact|empty; abandonment provides ZERO materials.",
            v.name, job.good, job.position, job.stand, job.worker_id, job.source_block(),
            job.issued, job.pickup_witnessed, job.blocked, observed, stem_progress, v.name
        ));
        return;
    }
    if action == "barrelcheck" {
        if player.permission < 2 {
            error(game, "warehouse recovery inspection requires server operator level 2");
            return;
        }
        let name = args.next().unwrap_or("");
        let Some(v) = world.by_name(name) else {
            error(game, "unknown settlement: /civ barrelcheck <name>");
            return;
        };
        if v.dimension != player.dimension {
            error(game, "enter the settlement dimension before inspecting its native barrel");
            return;
        }
        if v.market_purchase.is_some() {
            error(game, "an unresolved player purchase exists; barrel recovery is not safe");
            return;
        }
        let Some(t) = &v.barrel_transfer else {
            reply(game, format!("{}: no pending physical barrel transfer", v.name));
            return;
        };
        let observed = plain_barrel_count(game, &v.dimension, t.position, t.good);
        let expected = t.expected_after();
        reply(game, format!(
            "{} native barrel audit: {:?} {} x{} at {:?}, phase {:?}, saved before {}, expected after {:?}, current native count {:?}. Use /civ resolvebarrel {} confirm|cancel <observed_count> ONLY after manually examining real barrel inventory and logs. This command changes nothing.",
            v.name, t.direction, t.good.as_str(), t.count, t.position,
            t.phase, t.before, expected, observed, v.name
        ));
        return;
    }
    if action == "bakery" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ bakery <name>");
            return;
        };
        let report = if let Some(b) = &village.bakery {
            let construction = b.building.as_ref().map(|job| format!(
                "BUILDING {}/{} blocks, blocked={}, funded={}",
                job.cursor, bakery_plan(job.origin).len(), job.blocked,
                b.supplies_reserved
            )).unwrap_or_else(|| if b.complete {
                "COMPLETED native crafting table and smoker".to_owned()
            } else { "UNFUNDED construction".to_owned() });
            let wood = b.construction_escrow.get(&Good::Wood).copied().unwrap_or(0);
            let stone = b.construction_escrow.get(&Good::Stone).copied().unwrap_or(0);
            let still_need_wood = crate::game::civilizations_core::BAKERY_WOOD_LOGS
                .saturating_sub(wood);
            let still_need_stone = crate::game::civilizations_core::BAKERY_COBBLESTONE
                .saturating_sub(stone);
            let native_ready = if b.complete {
                bakery_plan(b.origin).iter().all(|step|
                    world_block(game, &village.dimension, step.pos).as_deref() == Some(step.block))
            } else { false };
            format!(
                "{} at {:?}. {}. Intact native blocks: {}. Materials carried for bakery: wood={}, stone={}; still needed: {} logs / {} cobblestone. Real wheat removed from hall: {}. Crafted bread on baker: {}. Completed 3-wheat-to-1-bread batches: {}.",
                village.name, b.origin, construction, native_ready, wood, stone,
                if b.supplies_reserved { 0 } else { still_need_wood },
                if b.supplies_reserved { 0 } else { still_need_stone },
                b.wheat_escrow, b.bread_in_transit, b.batches,
            )
        } else {
            format!(
                "{}: bakery not yet reserved. After the real hall and UUID-bound Baker/Builder are loaded, village autonomously chooses an unclaimed natural plot. Construction then needs {} genuine oak logs and {} genuine cobblestone.",
                village.name, crate::game::civilizations_core::BAKERY_WOOD_LOGS,
                crate::game::civilizations_core::BAKERY_COBBLESTONE
            )
        };
        reply(game, report);
        return;
    }
    if action == "development" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ development <village>");
            return;
        };
        let linked = loaded_actors(game, world, village.id);
        let (_, _, homeless) = village.physical_housing();
        let note = if !village.actors_spawned {
            "waiting for ten safe loaded natural-ground villager spawn sites"
        } else if linked.is_empty() {
            "no UUID-linked villagers loaded; return to village or inspect /civ roster, never blindly respawn"
        } else if village.farm.is_none() {
            "field not yet reserved; the loaded farmer and a clear natural 3x3 plot are required"
        } else if village.visual_job.is_none() && village.pending_visual_buildings == 0 && homeless > 0 {
            "waiting for paid abstract housing inventory; future economic days may prepare houses"
        } else {
            "workers act during daylight on loaded clear plots; follow construction/farming progress"
        };
        let job = village.visual_job.as_ref().map(|j| format!(
            "building at {:?}: {}/{} blocks, blocked={}", j.origin,
            j.cursor, hut_plan(j.origin).len(), j.blocked
        )).unwrap_or_else(|| "no current build site".to_owned());
        let field = village.farm.as_ref().map(|farm| format!(
            "field at {:?}: {:?}, blocked={}, cycles={}", farm.origin,
            farm.stage, farm.blocked, farm.completed_cycles
        )).unwrap_or_else(|| "no physical crop field".to_owned());
        reply(game, format!(
            "{} development: autonomy={} | spawn reserved={} | UUID-linked actors loaded {}/{} | {} finished PUBLIC huts, {} still without housing, {} abstract huts ready | {} | {} | {} player-owned parcels | treasury {} civic coins. Diagnostic: {}.",
            village.name,
            if village.autonomy_enabled { "ON" } else { "OFF" },
            village.actors_spawned, linked.len(), village.population(), village.public_huts(), homeless,
            village.pending_visual_buildings, job, field, village.player_parcels.len(),
            village.treasury_coins, note
        ));
        return;
    }
    if action == "plots" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ plots <name>");
            return;
        };
        let plots: Vec<String> = village.player_parcels.iter()
            .filter(|plot| plot.owner == actor).take(32)
            .map(|plot| format!(
                "{:?} ({})", plot.origin, if plot.commissioned { "private hut owned or commissioned" } else { "empty private parcel; build freely" }
            )).collect();
        reply(game, format!(
            "{} owns {} purchased plots; {} parcels registered in {}. {}. These protect against CIVILIZATIONS NPC edits only, not external players or mods.",
            actor, plots.len(), village.player_parcels.len(), village.name,
            if plots.is_empty() { "No purchased parcels".to_owned() } else { plots.join(" | ") }
        ));
        return;
    }
    if action == "council" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ council <name>");
            return;
        };
        let priorities = village.council_priorities();
        if priorities.is_empty() {
            reply(game, format!(
                "{} council, day {}: no urgent current needs under the abstract model. No decisions auto-executed.",
                village.name, world.day
            ));
        } else {
            let details: Vec<String> = priorities.iter().take(3).enumerate()
                .map(|(i, priority)| format!(
                    "#{} {} [{}/100]: {}. Next: {}",
                    i + 1, priority.need.label(), priority.urgency,
                    priority.reason, priority.next_step
                ))
                .collect();
            reply(game, format!(
                "{} council, day {}: {}. Read-only advice; no items, land claims, orders or block edits.",
                village.name, world.day, details.join(" | ")
            ));
        }
        return;
    }
    if action == "forecast" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ forecast <name>");
            return;
        };
        let output: Vec<String> = village.forecast_abstract_days(
            crate::game::civilizations_core::COUNCIL_FORECAST_DAYS
        ).iter().map(|day| format!(
            "+{}d bread {} wheat {} morale {} hunger {} houses {} pending {}",
            day.offset, day.bread, day.wheat, day.morale, day.starving_days,
            day.houses, day.pending_visual_buildings
        )).collect();
        reply(game, format!(
            "{} ABSTRACT forecast (not saved): {}. Excludes World war attrition, player trades, inventory, physical crops and real Minecraft time jumps.",
            village.name, output.join(" | ")
        ));
        return;
    }
    if action == "schedule" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ schedule <name>");
            return;
        };
        let phase = DailyPhase::at(game.world.day_time);
        let on_duty = village.residents.iter()
            .filter(|resident| resident.alive && phase.on_shift(resident.job)).count();
        reply(game, format!(
            "{} routine: {} (Horizon tick {}) | on duty {} / {} | guards watch overnight | physical farm/build edits only on work shift 2000..10999 | navigation only",
            village.name, phase.name(), game.world.day_time.rem_euclid(24_000),
            on_duty, village.population()
        ));
        return;
    }
    if action == "survey" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ survey <name>");
            return;
        };
        if village.dimension != player.dimension {
            error(game, "visit the settlement dimension to inspect live terrain");
            return;
        }
        let (capacity, _, waiting) = village.physical_housing();
        if waiting == 0 {
            reply(game, format!("{} has enough completed physical hut capacity ({capacity}) for its living residents.", village.name));
            return;
        }
        if village.pending_visual_buildings == 0 {
            reply(game, format!(
                "{} needs {} more physical spaces, but no simulated house is ready to build. Advance the economy first; no terrain is reserved.",
                village.name, waiting
            ));
            return;
        }
        if village.visual_job.is_some() {
            reply(game, format!(
                "{} already has a physical construction project. Finish or recover it before planning another.",
                village.name
            ));
            return;
        }
        let mut proposals = Vec::new();
        for slot in 1..=HOUSING_PLOT_OFFSETS.len() {
            // The X/Z slot index is immutable. Probe only a bounded +/-2 Y
            // window, using the same live terrain check as manual approval.
            for delta in [0_i32, 1, -1, 2, -2] {
                let Some(y) = village.center[1].checked_add(delta) else { continue; };
                let Ok(site) = world.proposed_hut(village.id, slot, y) else { continue; };
                if check_empty_hut_site(game, &village.dimension, site).is_ok() {
                    proposals.push(format!(
                        "#{slot} at {},{},{}",
                        site[0], site[1], site[2]
                    ));
                    break;
                }
            }
            if proposals.len() >= MAX_HOUSING_SURVEY_RESULTS { break; }
        }
        if proposals.is_empty() {
            reply(game, format!(
                "{} needs {} physical spaces; {} simulated houses await visualization, but no safe flat loaded plot was found among {} fixed candidates. Move to the area or prepare natural ground. Nothing was reserved.",
                village.name, waiting, village.pending_visual_buildings, HOUSING_PLOT_OFFSETS.len()
            ));
        } else {
            reply(game, format!(
                "{} can use these optional safe building slots: {} spaces missing, {} abstract houses awaiting build. {}. Autonomous builders select free plots by default after the funded town hall; OP /civ approvehouse {} <slot> <y> is an optional manual override. Real blocks and player plots are always rechecked.",
                village.name, waiting, village.pending_visual_buildings,
                proposals.join(" | "), village.name
            ));
        }
        return;
    }
    if action == "homes" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ homes <name>");
            return;
        };
        let (capacity, assigned, waiting) = village.physical_housing();
        let roster: Vec<String> = village.residents.iter()
            .filter(|resident| resident.alive)
            .take(10)
            .map(|resident| match village.home_slot(resident.id) {
                Some(home) => format!(
                    "{}: hut at {:?}, lane {}",
                    resident.name, home.site, home.lane + 1
                ),
                None => format!("{}: commons (no built hut slot)", resident.name),
            })
            .collect();
        reply(game, format!(
            "{} physical homes: {} completed huts, capacity {}, assigned {}, waiting {}. {}. No beds are claimed; night routes check real blocks first.",
            village.name, village.visual_sites.len(), capacity, assigned, waiting,
            roster.join(" | ")
        ));
        return;
    }
    if action == "roster" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement: /civ roster <name>"); return; };
        let linked = loaded_actors(game, world, s.id);
        let listing: Vec<_> = s.residents.iter().map(|r| format!("{}:{}", r.name, r.job.name())).collect();
        reply(game, format!("{} actors: {}/{} nearby and loaded | {}", s.name, linked.len(), s.population(), listing.join(", ")));
        return;
    }
    if action == "harvests" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement");
            return;
        };
        let Some(farm) = &village.farm else {
            reply(game, format!("{}: no physical farm", village.name));
            return;
        };
        let details: Vec<String> = farm.harvest_receipts.iter().rev().take(5)
            .map(|receipt| format!("#{} day {} ({} crops)",
                receipt.sequence, receipt.world_day, receipt.crops)).collect();
        reply(game, format!(
            "{} observed physical harvest evidence: {} recorded batches, {} kept in bounded log. {}. Confirmed crops remain with the farmer until hall delivery; steward then inserts real wheat into the public barrels. No abstract or player inventory credit.",
            village.name, farm.harvest_sequence, farm.harvest_receipts.len(),
            if details.is_empty() { "No harvest receipts".to_owned() } else { details.join(" | ") },
        ));
        return;
    }
    if action == "farming" {
        let name = args.next().unwrap_or("");
        let Some(settlement) = world.by_name(name) else {
            error(game, "unknown settlement");
            return;
        };
        let message = if let Some(farm) = &settlement.farm {
            format!(
                "{} 3x3 crop field at {:?}: {:?}, {}/{} steps, blocked={}, completed visual cycles={}, observed harvest batches={}. No farm stock is minted.",
                settlement.name, farm.origin, farm.stage, farm.cursor,
                if farm.stage == FarmStage::Preparing { farm_plan(farm.origin).len() } else { 8 },
                farm.blocked, farm.completed_cycles, farm.harvest_sequence,
            )
        } else {
            format!("{} has no physical farm yet; UUID-bound farmers automatically select a loaded natural 3x3 plot. OP /civ farm {} remains an optional manual fallback.", settlement.name, settlement.name)
        };
        reply(game, message);
        return;
    }
    if action == "posts" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return; };
        let text = if s.posts.is_empty() {
            format!("{} has no work camps yet. OP: stand at a clear spot 8..80 blocks from center and /civ post {} lumber|quarry", s.name, s.name)
        } else {
            let list: Vec<String> = s.posts.iter().map(|post| {
                let worker = post.worker_id
                    .and_then(|id| s.residents.iter().find(|r| r.id == id && r.alive))
                    .map(|r| r.name.clone()).unwrap_or_else(|| "unassigned".into());
                format!("{} at {:?} ({worker})", post.kind.name(), post.origin)
            }).collect();
            format!("{} camps: {}", s.name, list.join(" | "))
        };
        reply(game, text);
        return;
    }
    if action == "construction" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return; };
        let text = if let Some(j) = &s.visual_job {
            format!("{} physical hut: {}/{} blocks observed, blocked={}, origin {:?}",
                    s.name, j.cursor, hut_plan(j.origin).len(), j.blocked, j.origin)
        } else {
            format!("{}: {} completed physical huts, {} abstract houses awaiting construction. OP: stand on flat empty ground 12..80 blocks from center; /civ construct {}",
                    s.name, s.visual_sites.len(), s.pending_visual_buildings, s.name)
        };
        reply(game, text);
        return;
    }
    if action == "history" {
        let text = world.chronicle.iter().rev().take(6).cloned().collect::<Vec<_>>().join(" | ");
        reply(game, if text.is_empty() { "No history yet".into() } else { text });
        return;
    }
    if action == "info" {
        let s = match args.next() {
            Some(name) => world.by_name(name),
            None => world.players.get(&actor).and_then(|p| p.settlement).and_then(|id| world.settlements.get(&id)),
        };
        if let Some(s) = s {
            reply(game, format!(
                "{} #{} | day {} | people {} | houses {} ({} awaiting visual build) | morale {} | wheat {} bread {} wood {} stone {} iron {} | actors {} | wars {}",
                s.name, s.id, world.day, s.population(), s.houses, s.pending_visual_buildings,
                s.morale, s.quantity(Good::Wheat), s.quantity(Good::Bread), s.quantity(Good::Wood),
                s.quantity(Good::Stone), s.quantity(Good::Iron), if s.actors_spawned { "summoned" } else { "not summoned" }, s.active_wars.len()
            ));
        } else { error(game, "settlement not found (or join one first)"); }
        return;
    }
    if action == "title" {
        if let Some(p) = world.players.get(&actor) {
            reply(game, format!("Your rank: {} | coins: {} | reputation: {:?}", p.rank.name(), p.coins, p.reputation));
        } else { reply(game, "Visitor. Join a settlement for 25 starting coins."); }
        return;
    }
    // Mutating actions use a cloned state for ordinary error rollback.
    let previous = world.clone();
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
            let price = state.world.as_mut().unwrap().buy_plot(&actor, village.id, origin)?;
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
            let price = state.world.as_mut().unwrap().buy_finished_house(&actor, village.id, origin)?;
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
            let price = state.world.as_mut().unwrap().commission_house(&actor, village.id, site)?;
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
            let name = args.next().ok_or("usage: /civ post <name> lumber|quarry")?;
            let kind = PostKind::parse(args.next().ok_or("kind: lumber or quarry")?)
                .ok_or("kind must be lumber or quarry")?;
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
            let id = state.world.as_mut().unwrap().found(name, &player.dimension, center, &actor)?;
            Ok(format!("Founded {name} (#{id}). Autonomous NPC spawning and town hall planning start by default. The first civic hall needs 24 real logs and 25 cobblestone; /civ hall {name} and /civ warehouse {name} show progress."))
        })(),
        "promote" => (|| {
            let rank = state.world.as_mut().unwrap().promote(&actor)?;
            Ok(format!("Promoted to {}", rank.name()))
        })(),
        "join" => (|| {
            let name = args.next().ok_or("usage: /civ join <name>")?;
            let id = previous.by_name(name).ok_or("unknown settlement")?.id;
            state.world.as_mut().unwrap().join(&actor, id)?;
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
            let good = Good::parse(args.next().ok_or("good required")?).ok_or("unknown good")?;
            let count = parse_amount(args.next())?;
            let village_id = own_settlement(&previous, &actor)?;
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
                let payout = state.world.as_mut().unwrap().sell(&actor, village_id, good, count)?;
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
                    game, state, &actor, village_id, good, count,
                    player.position, &mut already_saved,
                )
            } else {
                if game.native_room_for(item) < count { return Err("not enough inventory space".into()); }
                let cost = state.world.as_mut().unwrap().buy(&actor, village_id, good, count)?;
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
                    state.problem = Some(format!("actor identity save halted: {e}"));
                    return Ok(format!("Reconciled {linked} actors before save failure; world halted. Restore backup and restart. {e}"));
                }
                already_saved = true;
                linked += 1;
            }
            Ok(format!("Reconciled {linked} legacy actors; {ambiguous} ambiguous/conflicting skipped. /civ roster {name}. Missing actors are not spawned automatically."))
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
            // Persist one-shot reservation before the first spawn. A crash
            // cannot accidentally retrigger a full batch on next login.
            state.world.as_mut().unwrap().settlements.get_mut(&s.id).unwrap().actors_spawned = true;
            save(state)?;
            already_saved = true;
            let mut spawned = 0usize;
            let mut halted = false;
            for (i, resident) in s.residents.iter().take(10).enumerate() {
                let x = f64::from(s.center[0]) + (i as f64 % 5.0) * 2.0 - 4.0 + 0.5;
                let z = f64::from(s.center[2]) + (i / 5) as f64 * 2.0 - 1.0 + 0.5;
                let opts = NativeSpawn {
                    name: Some(s.actor_name(resident)),
                    ..NativeSpawn::default()
                };
                let Some(entity_id) = game.native_spawn_mob(
                    &s.dimension, "minecraft:villager",
                    [x, f64::from(s.center[1]), z], &opts
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
        "found" | "day" | "war" | "peace" | "actors" | "construct" | "post" | "approvehouse" | "autonomy" | "farm" | "recoverfarm" | "reconcile" | "barrelcheck" | "resolvebarrel" | "gathercheck" | "discardgather" => Err("requires server operator level 2".into()),
        _ => Err("unknown subcommand. Use /civ help".into()),
    };
    match result {
        Ok(message) => {
            if already_saved {
                reply(game, message);
            } else if let Err(e) = save(state) {
                state.world = Some(previous);
                error(game, format!("failed to save world: {e}"));
            } else { reply(game, message); }
        }
        Err(message) => {
            state.world = Some(previous);
            error(game, message);
        }
    }
}

/// World-JSON operations commit before any external physical side effect.
fn save_autonomous(state: &mut ActiveWorld, prior: World) -> Result<(), String> {
    if let Err(error) = save(state) {
        state.world = Some(prior);
        return Err(format!("persistence: {error}"));
    }
    Ok(())
}

/// Try a bounded list of already-loaded, open, natural-ground positions.
/// Never summons a villager if even one of the ten positions is unsafe.
fn safe_actor_spawn_points(
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
                    spots.push([f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5]);
                    break;
                }
            }
            if spots.len() == 10 { return spots; }
        }
    }
    Vec::new()
}

/// Vanilla villagers are UUID-bound, once, after a persistent one-shot
/// reservation. A partial spawn NEVER triggers an automatic second batch.
fn auto_spawn_villagers(
    game: &mut Game<'_>, state: &mut ActiveWorld, village_id: u64
) -> Result<(), String> {
    let village = state.world.as_ref().unwrap().settlements
        .get(&village_id).ok_or("unknown settlement")?.clone();
    if village.actors_spawned || !village.autonomy_enabled { return Ok(()); }
    let positions = safe_actor_spawn_points(game, &village);
    if positions.len() != 10 { return Ok(()); } // unloaded / obstructed: retry later
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

/// A native barrel is the authoritative on-site storage after the civic
/// hall completes. Only plain item stacks count (never damaged/NBT-bearing
/// equipment that a trader might have placed in the container).
fn plain_barrel_count(game: &Game<'_>, dimension: &str, pos: [i32; 3], good: Good) -> Option<u32> {
    if world_block(game, dimension, pos).as_deref() != Some("minecraft:barrel") {
        return None;
    }
    let slots = game.native_container_slots(dimension, pos, Direction::Up)?;
    slots.iter().filter_map(|entry| entry.stack.as_ref())
        .filter(|stack| stack.item.as_ref() == good.item() && stack.damage == 0 && stack.extra.is_none())
        .try_fold(0_u32, |total, stack| total.checked_add(stack.count))
}

/// Reads exactly the two village hall barrels, never a nearby player chest.
fn physical_barrel_stock(
    game: &Game<'_>, dimension: &str, center: [i32; 3]
) -> Option<Vec<(Good, u32)>> {
    let mut values = Vec::new();
    for good in [Good::Wheat, Good::Bread, Good::Wood, Good::Stone, Good::Iron] {
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
fn storage_worker_near(game: &mut Game<'_>, world: &World, id: u64, role: Job) -> bool {
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
fn request_workshop_pickup(
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
fn barrel_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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
        for good in [Good::Wood, Good::Stone, Good::Wheat, Good::Bread, Good::Iron] {
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
fn farmer_delivery_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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
fn baker_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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

/// One bounded planning step per 100 loaded level ticks. NPCs choose free
/// natural plots *inside their settlement radius* and start actual physical
/// jobs. Paid player parcels are hard exclusions in BOTH domain paths.
fn autonomous_village_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dimension = game.dim.id().to_owned();
    let enabled: Vec<u64> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension && v.autonomy_enabled)
        .map(|v| v.id).collect();
    if enabled.is_empty() { return Ok(()); }
    let index = (state.physical_ticks / 100) as usize % enabled.len();
    let id = enabled[index];
    if !state.world.as_ref().unwrap().settlements[&id].actors_spawned {
        return auto_spawn_villagers(game, state, id);
    }
    let prior = state.world.as_ref().unwrap().clone();
    if state.world.as_mut().unwrap().prepare_village_housing(id)? {
        save_autonomous(state, prior)?;
    }
    let village = state.world.as_ref().unwrap().settlements[&id].clone();
    let actors = loaded_actors(game, state.world.as_ref().unwrap(), id);
    let has_farmer = village.residents.iter().any(|r| r.alive && r.job == Job::Farmer
        && actors.iter().any(|(known, _)| *known == r.id));
    let has_builder = village.residents.iter().any(|r| r.alive && r.job == Job::Builder
        && actors.iter().any(|(known, _)| *known == r.id));
    let has_baker = village.residents.iter().any(|r| r.alive && r.job == Job::Baker
        && actors.iter().any(|(known, _)| *known == r.id));
    // Recover from a refused/modified extraction without paying any wood or
    // stone; the next planning cycle may choose another untouched source.
    if village.gather_job.as_ref().is_some_and(|job| job.blocked && !job.issued) {
        // No Minecraft edit was dispatched. Discard this stale Planned
        // candidate without a receipt and survey a different source later.
        let before = state.world.as_ref().unwrap().clone();
        state.world.as_mut().unwrap().discard_blocked_gather(id)?;
        save_autonomous(state, before)?;
        return Ok(());
    }
    // A blocked *Issued* gather might already have changed native blocks.
    // Keep its durable receipt quarantined until OP inspection. Planning
    // other civic tasks may continue, but no further gather is scheduled
    // while the uncertain source remains recorded.

    // Resource demand MUST include real native barrel contents, saved
    // incoming deliveries and physically extracted, project-specific
    // reserves. In v0.1.13 looking at only the incoming queue caused
    // villagers to gather indefinitely even with filled real barrels.
    // An unloaded/missing barrel cannot be interpreted as zero stock:
    // pause surveying until both native block entities can be observed.
    let (wood_needed, stone_needed) = if !village.hall_complete {
        village.hall_materials_missing()
    } else {
        // A journalled mutation may already have changed the native barrel
        // without updating saved escrows/receipts. Do not count goods twice
        // or send miners to replace them until the receipt is reconciled.
        if village.barrel_transfer.is_some() || village.market_purchase.is_some() {
            return Ok(());
        }
        let Some(native_stock) = physical_barrel_stock(game, &dimension, village.center)
        else { return Ok(()); };
        crate::game::civilizations_core::physical_construction_shortfall(
            &village, &native_stock,
        )
    };

    if village.gather_job.is_none() {
        for (good, need, profession) in [
            (Good::Wood, wood_needed, Job::Lumberjack),
            (Good::Stone, stone_needed, Job::Miner),
        ] {
            if need == 0 || !village.residents.iter().any(|r| {
                r.alive && r.job == profession
                    && actors.iter().any(|(rid, _)| *rid == r.id)
            }) {
                continue;
            }
            // Inspect at most 320 of 1,681 X/Z candidates per planner
            // turn, with up to 6 nearby Y levels. Never scan thousands of
            // chunks at once; rotate to visit new places on later turns.
            // A work camp turns ad-hoc gathering into a fixed, visible post.
            // Without one, the first found natural source reserves a camp at
            // the worker stand; later scans stay inside its radius.
            let camp_kind = if good == Good::Wood { PostKind::Lumber } else { PostKind::Quarry };
            let camp = village.posts.iter().find(|post| post.kind == camp_kind).map(|post| post.origin);
            let (base_x, base_y, base_z) = {
                let base = camp.unwrap_or(village.center);
                (base[0], base[1], base[2])
            };
            let candidates = 41_usize * 41;
            let start = ((state.physical_ticks / 100) as usize * 101) % candidates;
            for step in 0..320_usize {
                let index = (start + step) % candidates;
                let dx = (index / 41) as i32 * 2 - 40;
                let dz = (index % 41) as i32 * 2 - 40;
                if camp.is_some() && (dx.abs() > POST_RADIUS || dz.abs() > POST_RADIUS) {
                    continue;
                }
                    let (Some(x), Some(z)) = (
                        base_x.checked_add(dx),
                        base_z.checked_add(dz)
                    ) else { continue; };
                    for dy in [-2_i32, -1, 0, 1, 2, 3] {
                        let Some(y) = base_y.checked_add(dy) else { continue; };
                        let position = [x, y, z];
                        if !seems_natural_resource(game, &dimension, position, good) { continue; }
                        let Some(stand) = gather_stand(game, &dimension, position) else { continue; };
                        // No camp yet: reserve this stand as the camp and let
                        // the next planner turn pick the actual resource.
                        if camp.is_none() {
                            let prior = state.world.as_ref().unwrap().clone();
                            if state.world.as_mut().unwrap().add_post(id, camp_kind, stand).is_ok() {
                                save_autonomous(state, prior)?;
                                log::info!("[civilizations] {} opened a {} camp at {:?}",
                                    village.name, camp_kind.name(), stand);
                                return Ok(());
                            }
                        }
                        // Preserve the full pre-image, not merely the
                        // former one-block GatherJob. A stem plan also
                        // mutates oak_stem; both fields must roll back if
                        // persistence fails before the physical job starts.
                        let prior = state.world.as_ref().unwrap().clone();
                        let planned = if good == Good::Wood {
                            // Each naturally rooted short oak is ONE durable
                            // job with a bounded sequence of separate actual
                            // log receipts. Taller trees remain untouched
                            // until scaffolding/real forestry is implemented.
                            let height = short_oak_stem_height(position, |pos|
                                world_block(game, &dimension, pos));
                            height.is_some_and(|height|
                                state.world.as_mut().unwrap()
                                    .plan_short_oak_stem(id, position, stand, height).is_ok())
                        } else {
                            state.world.as_mut().unwrap()
                                .plan_gather(id, good, position, stand).is_ok()
                        };
                        if planned {
                            // The entire old settlement, including the
                            // optional tree continuation, is restored on
                            // failed save. No native block command is sent.
                            save_autonomous(state, prior)?;
                            log::info!("[civilizations] {} sent a {} to {:?}", village.name,
                                profession.name(), position);
                            return Ok(());
                        }
                    }
            }
        }
    }

    // Town hall is physically assembled first from the verified warehouse
    // bill of materials; it may remain pending while players deliver goods.
    // The planner does NOT consume abstract simulation wood/stone.
    if !village.hall_complete && village.hall_job.is_none() && has_builder {
        if check_empty_hut_site(game, &dimension, village.center).is_ok() {
            let before = state.world.as_ref().unwrap().clone();
            if state.world.as_mut().unwrap().reserve_town_hall(id)? {
                save_autonomous(state, before)?;
                log::info!("[civilizations] {} started material-backed civic hall", village.name);
                return Ok(());
            }
        }
    }
    let need_field = has_farmer && village.farm.is_none();
    let need_bakery = has_builder && has_baker && village.hall_complete
        && village.bakery.is_none();
    let need_hut = has_builder && village.hall_complete && village.visual_job.is_none()
        && village.pending_visual_buildings > 0 && village.physical_housing().2 > 0;
    if !need_field && !need_hut && !need_bakery { return Ok(()); }

    for [dx, dz] in crate::game::civilizations_core::HOUSING_PLOT_OFFSETS {
        let Some(x) = village.center[0].checked_add(dx) else { continue; };
        let Some(z) = village.center[2].checked_add(dz) else { continue; };
        for dy in [0_i32, 1, -1, 2, -2] {
            let Some(y) = village.center[1].checked_add(dy) else { continue; };
            let site = [x, y, z];
            // First establish a functional field. Farms must be harvested
            // by a loaded nearby farmer; their cycles do NOT mint stock.
            if need_field && check_empty_farm_site(game, &dimension, site).is_ok() {
                let mut trial = state.world.as_ref().unwrap().clone();
                if trial.start_farm(id, site).is_ok() {
                    let prior = state.world.as_ref().unwrap().clone();
                    state.world.as_mut().unwrap().start_farm(id, site)?;
                    save_autonomous(state, prior)?;
                    log::info!("[civilizations] {} autonomously reserved crop field at {:?}", village.name, site);
                    return Ok(());
                }
            }
            // After the field and civic hall exist, build the first
            // functional workshop on the same loaded, physically empty,
            // non-player-owned plot policy as ordinary housing.
            if need_bakery && check_empty_hut_site(game, &dimension, site).is_ok() {
                let mut trial = state.world.as_ref().unwrap().clone();
                if trial.start_bakery(id, site).is_ok() {
                    let old = state.world.as_ref().unwrap().clone();
                    state.world.as_mut().unwrap().start_bakery(id, site)?;
                    save_autonomous(state, old)?;
                    log::info!("[civilizations] {} autonomously reserved real bakery at {:?}", village.name, site);
                    return Ok(());
                }
            }
            if need_hut && check_empty_hut_site(game, &dimension, site).is_ok() {
                let can_build = state.world.as_ref().unwrap()
                    .check_visual_house_site(id, site).is_ok();
                if can_build {
                    let prior = state.world.as_ref().unwrap().clone();
                    state.world.as_mut().unwrap().start_visual_house(id, site)?;
                    save_autonomous(state, prior)?;
                    log::info!("[civilizations] {} autonomously reserved real hut at {:?}", village.name, site);
                    return Ok(());
                }
            }
        }
    }
    // No clear loaded plot; never clear players' trees, fences or walls.
    Ok(())
}

/// Last text written to each hall board, so an unchanged board costs no
/// command. Keyed by dimension and position; a missing sign block always
/// forces a rewrite regardless of this cache.
static BOARD_CACHE: Mutex<BTreeMap<(String, [i32; 3]), String>> = Mutex::new(BTreeMap::new());

/// SNBT for one sign face. `rich` colors each line; the plain form is the
/// fallback for game versions whose sign NBT rejects text components.
fn sign_data_command(
    dimension: &str, pos: [i32; 3], spec: &crate::game::civilizations_core::SignSpec, rich: bool,
) -> String {
    let quote = |s: &str| format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"));
    let messages: Vec<String> = spec.lines.iter().map(|line| {
        if rich {
            format!(
                "{{text:{},color:\"{}\"{}}}",
                quote(&line.text), line.color.name(),
                if line.bold { ",bold:1b" } else { "" }
            )
        } else {
            quote(&line.text)
        }
    }).collect();
    let [x, y, z] = pos;
    format!(
        "execute in {dimension} if block {x} {y} {z} minecraft:oak_wall_sign run data merge block {x} {y} {z} {{front_text:{{messages:[{}],color:\"black\",has_glowing_text:{}b}}}}",
        messages.join(","), if spec.glow { 1 } else { 0 }
    )
}

/// Refresh the civic hall's six native wall signs from live barrel contents
/// and saved village state. Only COMPLETED halls qualify. No stock, player
/// or civic JSON is changed by this operation.
fn refresh_hall_boards(game: &mut Game<'_>, world: &World) {
    use crate::game::civilizations_core::{
        hall_all_board_positions, legacy_hall_board_positions, render_hall_boards, HallBoardInput,
    };
    let dimension = game.dim.id().to_owned();
    for village in world.settlements.values()
        .filter(|v| v.dimension == dimension && v.hall_complete)
    {
        // If a container is missing or unloaded, keep the signs unchanged.
        let Some(stored) = physical_barrel_stock(game, &dimension, village.center) else {
            continue;
        };
        // Same combined physical-demand routine as the gather planner.
        let journal_unsettled = village.barrel_transfer.is_some()
            || village.market_purchase.is_some();
        let (wood_missing, stone_missing) =
            crate::game::civilizations_core::physical_construction_shortfall(village, &stored);
        let specs = render_hall_boards(&HallBoardInput {
            village, stored: &stored, wood_missing, stone_missing, journal_unsettled,
        });
        let positions = hall_all_board_positions(village.center);
        let legacy = legacy_hall_board_positions(village.center);
        for (index, (pos, spec)) in positions.iter().zip(specs.iter()).enumerate() {
            let pos = *pos;
            let [x, y, z] = pos;
            let mut commands = Vec::new();
            let present = world_block(game, &dimension, pos).as_deref()
                == Some("minecraft:oak_wall_sign");
            if !present {
                if index < 2 {
                    // A hall completed before wall boards existed still carries
                    // standing signs on its floor: clear them first.
                    let [ox, oy, oz] = legacy[index];
                    if world_block(game, &dimension, legacy[index]).as_deref()
                        == Some("minecraft:oak_sign")
                    {
                        commands.push(format!(
                            "execute in {dimension} if block {ox} {oy} {oz} minecraft:oak_sign run setblock {ox} {oy} {oz} minecraft:air replace"
                        ));
                    }
                } else if world_block(game, &dimension, [x, y, z + 1]).as_deref()
                    != Some("minecraft:oak_planks")
                {
                    // Extra boards need the real wall behind them.
                    continue;
                }
                commands.push(format!(
                    "execute in {dimension} if block {x} {y} {z} minecraft:air run setblock {x} {y} {z} minecraft:oak_wall_sign[facing=north] replace"
                ));
            }
            let fingerprint = format!("{:?}{}", spec.lines, spec.glow);
            let key = (dimension.clone(), pos);
            if present && BOARD_CACHE.lock().unwrap().get(&key) == Some(&fingerprint) {
                continue;
            }
            for command in &commands {
                if !game.native_run_command(command) {
                    log::warn!("[civilizations] town hall sign placement unsupported: {command}");
                }
            }
            let rich = sign_data_command(&dimension, pos, spec, true);
            let ok = game.native_run_command(&rich) || {
                log::warn!("[civilizations] colored sign text refused, using plain text");
                game.native_run_command(&sign_data_command(&dimension, pos, spec, false))
            };
            if ok {
                BOARD_CACHE.lock().unwrap().insert(key, fingerprint);
            } else {
                log::warn!("[civilizations] town hall sign refresh unsupported at {pos:?}");
            }
        }
    }
}

/// Place or refresh one readable sign at each work camp. Signs are plain NBT
/// strings; the world shows them verbatim, so no component JSON is written.
fn refresh_post_boards(game: &mut Game<'_>, world: &World) {
    let dimension = game.dim.id().to_owned();
    let mut commands = Vec::new();
    for village in world.settlements.values()
        .filter(|v| v.dimension == dimension && !v.posts.is_empty())
    {
        for post in &village.posts {
            let worker = post.worker_id
                .and_then(|id| village.residents.iter().find(|r| r.id == id && r.alive))
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "NO WORKER".into());
            let lines = [
                post.kind.label().to_owned(),
                worker,
                format!("RADIUS {}", POST_RADIUS),
                post.kind.good().as_str().to_uppercase(),
            ];
            let quoted: Vec<String> = lines
                .iter()
                .map(|line| format!("'{}'", line.replace('\\', "\\\\").replace('\'', "\\'")))
                .collect();
            let [x, y, z] = post.origin;
            // The camp cell itself stays air for the worker; the sign stands
            // one block beside it when that cell is clear.
            let [sx, sy, sz] = [x + 1, y, z];
            let free = world_block(game, &dimension, [sx, sy, sz])
                .is_some_and(|block| block == "minecraft:air");
            if free {
                commands.push(format!(
                    "execute in {dimension} if block {sx} {sy} {sz} minecraft:air run setblock {sx} {sy} {sz} minecraft:oak_sign replace"
                ));
            }
            commands.push(format!(
                "execute in {dimension} if block {sx} {sy} {sz} minecraft:oak_sign run data merge block {sx} {sy} {sz} {{front_text:{{messages:[{}],color:\"black\",has_glowing_text:0b}}}}",
                quoted.join(",")
            ));
        }
    }
    for command in commands {
        if !game.native_run_command(&command) {
            log::warn!("[civilizations] camp sign refresh unsupported: {command}");
        }
    }
}

/// Find reachable-looking air next to a candidate real resource. This is
/// NOT a pathfinding guarantee: the native AI may refuse its route.
fn gather_stand(game: &Game<'_>, dimension: &str, pos: [i32; 3]) -> Option<[i32; 3]> {
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
fn gather_stand_still_clear(game: &Game<'_>, dimension: &str, stand: [i32; 3]) -> bool {
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
fn seems_natural_resource(game: &Game<'_>, dimension: &str, point: [i32; 3], good: Good) -> bool {
    match good {
        Good::Wood => is_rooted_oak_harvest_origin(point, |pos|
            world_block(game, dimension, pos)),
        Good::Stone => world_block(game, dimension, point).as_deref() == Some("minecraft:stone"),
        _ => false,
    }
}

/// A resource may be credited ONLY after: (1) plan saved; (2) Issued
/// journal persisted; (3) native conditional block edit attempted; (4) later
/// observation verifies air and a UUID-linked worker returns to the hall.
/// If the process crashes after Issued but before the actual edit, the
/// resulting uncertainty is quarantined; no automatic native retry or credit.
fn gather_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Miner) { return Ok(()); }
    let dimension = game.dim.id().to_owned();
    let target = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension)
        .filter_map(|v| v.gather_job.as_ref().map(|job| (v.id, job.clone())))
        .find(|(_, job)| !job.blocked && game.world.loaded(job.position[0], job.position[2]));
    let Some((id, job)) = target else { return Ok(()); };
    let observed = world_block(game, &dimension, job.position);
    let Some(name) = observed else { return Ok(()); };
    let before = state.world.as_ref().unwrap().clone();
    if job.issued {
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
    let profession = if job.good == Good::Wood { Job::Lumberjack } else { Job::Miner };
    let actors = loaded_actors(game, world, id);
    let selected = v.residents.iter()
        .filter(|r| r.alive && r.job == profession
            && job.worker_id.is_none_or(|assigned| assigned == r.id))
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
    if let Some(kind) = match job.good {
        Good::Wood => Some(PostKind::Lumber),
        Good::Stone => Some(PostKind::Quarry),
        _ => None,
    } {
        if state.world.as_mut().unwrap().set_post_worker(id, kind, selected_worker).is_err() {
            log::warn!("[civilizations] camp worker binding skipped for settlement {id}");
        }
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
    if !game.native_run_command(&cmd) {
        // Uncertain or refused native commands never auto-retry. Preserve
        // Blocked in the durable journal for explicit no-credit discard.
        let issued = state.world.as_ref().unwrap().clone();
        state.world.as_mut().unwrap().block_gather(id)?;
        save_autonomous(state, issued)?;
    }
    Ok(())
}

/// Runs after an authoritative level tick: the selected dimension and its
/// loaded chunks are guaranteed to be the ones inspected/modified here.
pub fn level_tick(game: &mut Game<'_>, dimension: &str) {
    if !game.native_on(MOD_ID) {
        return;
    }
    if game.dim.id() != dimension { return; }
    let Some(dir) = game.native_world_dir() else { return; };
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if state.problem.is_some() || state.world.is_none() { return; }
    state.physical_ticks = state.physical_ticks.saturating_add(1);
    // Villages start their own visible work by default, without needing
    // OP construction/field/spawn commands. The planner is bounded and
    // *never* edits a block without the existing worker-aware tick.
    if state.physical_ticks % 400 == 0 {
        if let Some(world) = state.world.as_ref() {
            refresh_hall_boards(game, world);
        }
        if let Some(world) = state.world.as_ref() {
            refresh_post_boards(game, world);
        }
    }
    if state.physical_ticks % 100 == 0 {
        if let Err(error) = autonomous_village_step(game, state) {
            log::warn!("[civilizations] autonomous planning stopped: {error}");
            // A failed identity or persistence step must never cause
            // repeated spawning or untracked world mutations.
            if error.starts_with("persistence:") || error.contains("UUID") {
                state.problem = Some(format!("automatic village action halted: {error}"));
                return;
            }
        }
    }
    // Navigation is bounded and only touches loaded actors in the current
    // dimension; it is not an offscreen tick or a physical gathering action.
    if state.physical_ticks % 20 == 0 {
        if let Some(world) = &state.world { guide_actors(game, world); }
    }
    if state.physical_ticks % 4 == 0 {
        if let Err(error) = build_town_hall_one_step(game, state) {
            log::warn!("[civilizations] civic hall construction paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
        if let Err(error) = build_bakery_one_step(game, state) {
            log::warn!("[civilizations] physical bakery construction paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
        if let Err(error) = build_one_step(game, state) {
            // Unloaded chunks are normal; a player may leave and return.
            if error.contains("unloaded") || error.contains("not loaded") {
                return;
            }
            log::warn!("[civilizations] physical construction paused: {error}");
            if error.starts_with("persistence:") {
                // A physical block may already exist. Stop mutating until
                // manual recovery, without pretending the write succeeded.
                state.problem = Some(error);
                return;
            }
        }
    }
    if state.physical_ticks % 200 == 0 {
        if let Err(error) = baker_one_step(game, state) {
            log::warn!("[civilizations] real bakery work paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
    }
    if state.physical_ticks % 8 == 0 {
        if let Err(error) = farmer_delivery_one_step(game, state) {
            log::warn!("[civilizations] returning wheat to hall failed: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
        if let Err(error) = barrel_one_step(game, state) {
            log::warn!("[civilizations] native barrel transfer paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
        if let Err(error) = gather_one_step(game, state) {
            log::warn!("[civilizations] physical gathering paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
        if let Err(error) = farm_one_step(game, state) {
            if error.contains("unloaded") || error.contains("not loaded") {
                return;
            }
            log::warn!("[civilizations] physical farm paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
            }
        }
    }
}

pub fn tick(game: &mut Game<'_>) {
    if !game.native_on(MOD_ID) {
        return;
    }
    let Some(dir) = game.native_world_dir() else { return; };
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if state.problem.is_some() || state.world.is_none() { return; }
    // The authoritative host day persists across Horizon game reloads.
    let host_day = game.world.day_time.div_euclid(24_000);
    if state.world.as_ref().unwrap().last_host_day.is_some_and(|last| host_day <= last) {
        return;
    }
    let before = state.world.as_ref().unwrap().clone();
    let result = state.world.as_mut().unwrap()
        .observe_host_day(host_day)
        .and_then(|changed| if changed { save(state) } else { Ok(()) });
    if let Err(error) = result {
        state.world = Some(before);
        state.problem = Some(format!("host day synchronization halted: {error}"));
        log::error!("[civilizations] authoritative day synchronization stopped: {error}");
    }
}
