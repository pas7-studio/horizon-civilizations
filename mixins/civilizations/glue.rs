// Horizon bridge. All world logic lives in civilizations_core; this module
// translates server hooks, commands, inventory changes, and native NPC spawns.
use crate::game::{
    civilizations_core::{farm_plan, hut_plan, town_hall_plan, inspect_farm, inspect_hut, inspect_town_hall, is_ground,
        BuildInspection, DailyPhase, FarmBlock, FarmDecision, FarmStage, Good, HomeSlot, Job, VisualBuildJob,
        World, inspect_home, hall_board_positions, HOUSING_PLOT_OFFSETS, MAX_HOUSING_SURVEY_RESULTS,
        MAX_SIM_DAYS},
    native_mobs::NativeSpawn,
    native_server::{NativeCommand, NativeServerPlayer},
    Game,
};
use horizon_client::text::{self, Text};
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
        "civ hall <name:word>", "civ warehouse <name:word>",
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
        "civ approvehouse <name:word> <slot:integer> <y:integer>",
        "civ autonomy <name:word> <mode:word>",
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
        let dx = mob.pos[0] - f64::from(village.center[0]);
        let dz = mob.pos[2] - f64::from(village.center[2]);
        (dx * dx + dz * dz < 96.0 * 96.0).then_some((resident.id, mob_id))
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
            } else if phase.on_shift(resident.job) && resident.job == Job::Farmer {
                if let Some(farm) = &village.farm {
                    [f64::from(farm.origin[0]) + 0.5,
                     f64::from(farm.origin[1]),
                     f64::from(farm.origin[2] - 3) + 0.5]
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
                } else if let Some(job) = &village.visual_job {
                    [f64::from(job.origin[0]) + 0.5,
                     f64::from(job.origin[1]),
                     f64::from(job.origin[2] - 4) + 0.5]
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
            let dz = mob.pos[2] - pos[2];
            if dx * dx + dz * dz > 3.0 * 3.0 {
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
                        let dx = actor.pos[0] - f64::from(job.origin[0]);
                        let dz = actor.pos[2] - f64::from(job.origin[2]);
                        dx * dx + dz * dz <= 9.0 * 9.0
                    })
                }));
            if !near { return Ok(()); }
            let [x, y, z] = position;
            let cmd = format!(
                "execute in {dimension} if block {x} {y} {z} {old_block} run setblock {x} {y} {z} {new_block} replace"
            );
            if !game.native_run_command(&cmd) {
                log::warn!("[civilizations] civic hall block command refused: {cmd}");
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
                let dx = mob.pos[0] - f64::from(job.origin[0]);
                let dz = mob.pos[2] - f64::from(job.origin[2]);
                dx * dx + dz * dz <= 9.0 * 9.0
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
        let dx = mob.pos[0] - f64::from(farm.origin[0]);
        let dz = mob.pos[2] - f64::from(farm.origin[2]);
        dx * dx + dz * dz <= 8.0 * 8.0
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
        resident.name, resident.job.name(), village.name, village.morale,
        phase.name(), duty, home, council, invitation, village.name
    );
    drop(guard);
    // Do not consume a vanilla interaction unless the information was sent.
    if game.native_tell(player.uuid, Text::literal(message).with_color(text::GOLD)) {
        *claimed = true;
    }
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
        reply(game, "Civilizations: /civ list | info [name] | roster <name> | schedule <name> | homes <name> | survey <name> | council <name> | forecast <name> | plots <name> | development <name> | hall <name> | warehouse <name> | buyplot <name> | buyhouse <name> | commission <name> | construction <name> | farming <name> | harvests <name> | join <name> | title | promote | history | buy/sell <good> <1..64> | (OP) found <name>, actors <name>, reconcile <name>, day [days], construct <name>, approvehouse <name> <slot> <y>, autonomy <name> on/off, farm <name>, recoverfarm <name>, war/peace <from> <to>. Goods: wheat, bread, wood (oak logs), stone (cobble), iron.");
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
        let (hall_wood, hall_stone) = village.hall_materials_missing();
        let (hut_wood, hut_stone) = (
            crate::game::civilizations_core::HUT_WOOD_LOGS
                .saturating_sub(village.warehouse_quantity(Good::Wood)),
            crate::game::civilizations_core::HUT_COBBLESTONE
                .saturating_sub(village.warehouse_quantity(Good::Stone)),
        );
        let goods = [Good::Wheat, Good::Bread, Good::Wood, Good::Stone, Good::Iron];
        let inventory = goods.iter().map(|good| format!(
            "{}:{}", good.as_str(), village.warehouse_quantity(*good)
        )).collect::<Vec<_>>().join(" / ");
        if action == "warehouse" {
            reply(game, format!(
                "{} PHYSICAL warehouse [{}]. Current hall needs {} logs, {} cobblestone; next hut needs {} logs, {} cobblestone. Player /civ sell <good> <amount> delivers actual inventory. Abstract daily-production stock is NOT this warehouse.",
                village.name, inventory, hall_wood, hall_stone, hut_wood, hut_stone
            ));
        } else {
            let building = village.hall_job.as_ref().map(|job| format!(
                "{}/{} blocks verified at {:?}, blocked={}",
                job.cursor, town_hall_plan(job.origin).len(), job.origin, job.blocked
            )).unwrap_or_else(|| "waiting for funded construction".to_owned());
            let state = if village.hall_complete {
                "COMPLETED: mayor desk, resource barrels and 2 bulletin signs"
            } else { "UNDER CONSTRUCTION / UNFUNDED" };
            reply(game, format!(
                "{} civic center: {}. Mayor: {} (village merchant). {}. Physical goods: [{}]. Need {} logs and {} stone to fund hall. Signs refresh when a complete hall is loaded.",
                village.name, state, mayor, building, inventory, hall_wood, hall_stone
            ));
        }
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
                "{} proposes huts: {} physical spaces missing, {} abstract houses awaiting build. {}. OP must visit a plot and run /civ approvehouse {} <slot> <y>. Suggestions expire if terrain changes; all blocks are rechecked.",
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
            "{} visual harvest evidence: {} recorded harvests, {} kept in bounded log. {}. No physical stock has been credited.",
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
            format!("{} has no farm; OP: select empty 3x3 natural patch 12..80 blocks away, /civ farm {}", settlement.name, settlement.name)
        };
        reply(game, message);
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
                "Commissioned your own 5x5 hut at {:?} for {} civic coins, 8 village wood and 6 stone. A UUID-linked builder will physically build during work hours. /civ construction {}",
                site, price, village.name
            ))
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
            Ok(format!("Founded {name} (#{id}). Use /civ actors {name} to summon its 10 villagers."))
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
        "found" | "day" | "war" | "peace" | "actors" | "construct" | "approvehouse" | "autonomy" | "farm" | "recoverfarm" | "reconcile" => Err("requires server operator level 2".into()),
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
    let need_hut = has_builder && village.hall_complete && village.visual_job.is_none()
        && village.pending_visual_buildings > 0 && village.physical_housing().2 > 0;
    if !need_field && !need_hut { return Ok(()); }

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

/// Refresh two actual native Minecraft sign tile entities with readable
/// information. Only the mod's expected signs in a COMPLETED hall qualify.
/// No change to stock, players or civic JSON is made by this operation.
fn refresh_hall_boards(game: &mut Game<'_>, world: &World) {
    let dimension = game.dim.id().to_owned();
    let mut commands = Vec::new();
    for village in world.settlements.values()
        .filter(|v| v.dimension == dimension && v.hall_complete)
    {
        let mayor = village.residents.iter()
            .find(|r| r.alive && r.job == Job::Merchant)
            .map(|r| r.name.as_str()).unwrap_or("NONE");
        let (_, _, homeless) = village.physical_housing();
        let (wood_missing, stone_missing) = if village.hall_complete {
            let next = if village.visual_job.is_some() || village.pending_visual_buildings > 0 {
                (crate::game::civilizations_core::HUT_WOOD_LOGS,
                 crate::game::civilizations_core::HUT_COBBLESTONE)
            } else { (0, 0) };
            (next.0.saturating_sub(village.warehouse_quantity(Good::Wood)),
             next.1.saturating_sub(village.warehouse_quantity(Good::Stone)))
        } else { village.hall_materials_missing() };
        let [mayor_sign, needs_sign] = hall_board_positions(village.center);
        for (pos, lines) in [
            (mayor_sign, [
                "TOWN HALL".to_owned(), village.name.clone(),
                format!("MAYOR {mayor}"), format!("POP {}", village.population()),
            ]),
            (needs_sign, [
                "VILLAGE NEEDS".to_owned(),
                format!("WOOD -{wood_missing}"),
                format!("STONE -{stone_missing}"),
                format!("NO HOME {homeless}"),
            ]),
        ] {
            if world_block(game, &dimension, pos).as_deref() != Some("minecraft:oak_sign") {
                continue;
            }
            // All printed inputs originate from names constrained to ASCII
            // letters/digits/underscore and bounded integer counters.
            let json_lines: Vec<String> = lines.iter().map(|line| {
                format!("'{}'", serde_json::json!({"text": line}).to_string())
            }).collect();
            let [x, y, z] = pos;
            commands.push(format!(
                "execute in {dimension} if block {x} {y} {z} minecraft:oak_sign run data merge block {x} {y} {z} {{front_text:{{messages:[{}]}}}}",
                json_lines.join(",")
            ));
        }
    }
    for command in commands {
        if !game.native_run_command(&command) {
            log::warn!("[civilizations] town hall sign refresh unsupported: {command}");
        }
    }
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
    if state.physical_ticks % 8 == 0 {
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
