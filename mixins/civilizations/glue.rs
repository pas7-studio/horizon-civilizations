// Horizon bridge. All world logic lives in civilizations_core; this module
// translates server hooks, commands, inventory changes, and native NPC spawns.
use crate::game::{
    civilizations_core::{farm_plan, hut_plan, town_hall_plan, bakery_plan, hall_storage_positions,
        BarrelDirection, BarrelPhase, inspect_farm, inspect_hut, inspect_town_hall, inspect_bakery, is_ground,
        is_rooted_oak_harvest_origin, short_oak_stem_height, inspect_oak_stem,
        native_worker_near, native_worker_reaches_block, PostKind, POST_RADIUS, MAX_QUARRY_STEPS, gather_turn_index,
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
use horizon_world::nbt::{Compound, Tag};
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
    // Either artifact signals an interrupted transaction. In particular,
    // world.json.tmp may be the ONLY persisted bytes of a new settlement
    // when the very first world save was interrupted before rename. Never
    // mistake that for an empty world and overwrite the recovery evidence.
    for remnant in ["world.json.swap", "world.json.tmp"] {
        if path.join(remnant).exists() {
            let message = format!(
                "interrupted save detected ({remnant}). Back up the world folder and inspect primary/backup/temporary save files before restarting"
            );
            return ActiveWorld { path, world: None, problem: Some(message), physical_ticks: 0 };
        }
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
    let temp = active.path.join("world.json.tmp");
    if swap.exists() || temp.exists() {
        return Err("pending world.json.swap or world.json.tmp from interrupted save; inspect and recover before continuing".into());
    }
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
        "civ", "civ help", "civ list", "civ nearby", "civ info", "civ info <name:word>",
        "civ roster <name:word>", "civ schedule <name:word>",
        "civ homes <name:word>", "civ survey <name:word>",
        "civ council <name:word>", "civ forecast <name:word>",
        "civ plots <name:word>", "civ development <name:word>",
        "civ hall <name:word>", "civ warehouse <name:word>", "civ requests <name:word>", "civ meals <name:word>", "civ bakery <name:word>",
        "civ posts <name:word>",
        "civ roles <name:word>",
        "civ status <name:word>",
        "civ chunks <name:word>",
        "civ diagnose <name:word>",
        "civ construction <name:word>", "civ farming <name:word>",
        "civ harvests <name:word>",
        "civ join <name:word>", "civ title", "civ promote", "civ history",
        "civ talk <name:word> <resident:integer>",
        "civ talk <name:word> <resident:integer> <page:word>",
        "civ talk <name:word> <resident:integer> <page:word> <a:word>",
        "civ talk <name:word> <resident:integer> <page:word> <a:word> <b:word>",
        "civ talk <name:word> <resident:integer> <page:word> <a:word> <b:word> <c:word>",
        "civ sell <good:word> <amount:integer>", "civ buy <good:word> <amount:integer>",
    ] {
        commands.push(NativeCommand::new(usage));
    }
    for usage in [
        "civ found <name:word>", "civ actors <name:word>",
        "civ reconcile <name:word>",
        "civ recoveractor <name:word> <resident:integer> <confirm:word>",
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
            // Morning/evening meals must not occupy all work hours when
            // the physical bread barrel is empty; the farm and bakery must
            // still be able to produce food for the next service window.
            let meal_due = village.hall_complete
                && matches!(phase, DailyPhase::Dawn | DailyPhase::Evening)
                && village.meal_day == Some(game.world.day_time.div_euclid(24_000))
                && !village.meal_served.contains(&resident.id);
            let pos = if meal_due {
                // Citizens actually walk to the public dining/warehouse
                // entrance before one real loaf can be taken for them.
                // Spread waiting citizens over five approach positions.
                let lane = (resident.id % 5) as i32 - 2;
                [f64::from(village.center[0] + lane) + 0.5,
                 f64::from(village.center[1]),
                 f64::from(village.center[2] - 4) + 0.5]
            } else if let Some(home) = home_target {
                home
            } else if phase.on_shift(resident.job)
                && matches!(resident.job, Job::Lumberjack | Job::Miner | Job::Digger)
                && village.gather_job.as_ref().is_some_and(|job|
                    !job.blocked
                        && job.worker_id.is_none_or(|assigned| assigned == resident_id)
                        && job.good.job() == Some(resident.job))
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

/// Natural growth a builder may clear to open a civic plot. Only plants and
/// trees: containers, player blocks and existing construction are never
/// touched, so a refused plot stays exactly as the player left it.
fn is_clearable_growth(name: &str) -> bool {
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
fn clear_growth_only(
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
fn open_build_plot(
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

/// True when a sign this mod owns is at `position` in the current dimension.
fn civic_sign_at(game: &Game<'_>, position: [i32; 3]) -> bool {
    if !game.native_on(MOD_ID) { return false; }
    let dimension = game.dim.id().to_owned();
    let is_sign = world_block(game, &dimension, position)
        .is_some_and(|block| block.ends_with("_sign"));
    if !is_sign { return false; }
    let Some(dir) = game.native_world_dir() else { return false; };
    let guard = active(dir.join(DIR));
    guard.as_ref().and_then(|state| state.world.as_ref())
        .is_some_and(|world| world.is_civic_sign(&dimension, position))
}

/// Right-clicking a village board does nothing: the editor never opens and
/// dye or ink cannot change it. Applies to everyone, operators included,
/// because the board text is rewritten only when the village state changes.
pub fn protect_civic_sign_click(game: &mut Game<'_>, position: [i32; 3], canceled: &mut bool) {
    if *canceled || !civic_sign_at(game, position) { return; }
    *canceled = true;
    if let Some(player) = game.native_acting_player() {
        let _ = game.native_action_bar(player.uuid, Text::literal(
            "Village boards show the settlement's records and cannot be edited."
        ).with_color(text::GOLD));
    }
}

/// Backstop for any path that still submits sign text: refuse the edit.
pub fn protect_civic_sign_edit(
    game: &mut Game<'_>, position: [i32; 3], cancel: &mut bool,
) {
    if *cancel || !civic_sign_at(game, position) { return; }
    *cancel = true;
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

use crate::game::civilizations_core::{
    build_dialog, dialog_snbt, good_label, parse_sale_args, resident_wants, DialogueInput, Resident,
    Settlement, TalkPage,
};

/// Dialogs waiting to be shown: (player name, inline dialog SNBT). The server
/// command that shows a dialog only runs from the server tick, never inside a
/// player's command or hook, so everything is queued and flushed there.
static DIALOG_QUEUE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn dialog_queue() -> MutexGuard<'static, Vec<(String, String)>> {
    DIALOG_QUEUE.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Called every server tick: show queued conversation windows.
pub fn process_dialog_queue(game: &mut Game<'_>) {
    let queued: Vec<(String, String)> = std::mem::take(&mut *dialog_queue());
    for (name, snbt) in queued {
        // Player names are 1..16 of letters, digits and underscore; refuse
        // anything else rather than build a command from it.
        let safe = !name.is_empty()
            && name.len() <= 16
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !safe {
            log::warn!("[civilizations] conversation not shown: unusual player name");
            continue;
        }
        if !game.native_run_command(&format!("dialog show {name} {snbt}")) {
            log::warn!("[civilizations] conversation window could not be shown to {name}");
        }
    }
}

/// What one conversation page needs, read from the live village and the
/// player's pockets. Only valid where the player's state is swapped in (a
/// command or an interaction hook), which is where it is called.
struct Talk {
    village: Settlement,
    resident: Resident,
    is_member: bool,
    wants: Vec<(Good, u32)>,
    stock_known: bool,
    wood_missing: u32,
    stone_missing: u32,
    carried: Vec<(Good, u32)>,
}

fn read_talk(game: &mut Game<'_>, uuid: u128, village_id: u64, resident_id: u64) -> Option<Talk> {
    let dir = game.native_world_dir()?;
    let (village, resident, is_member) = {
        let guard = active(dir.join(DIR));
        let world = guard.as_ref()
            .filter(|state| state.problem.is_none())
            .and_then(|state| state.world.as_ref())?;
        let village = world.settlements.get(&village_id)?.clone();
        let resident = village.residents.iter()
            .find(|r| r.id == resident_id && r.alive)?.clone();
        let member = world.players.get(&format!("{uuid:032x}"))
            .and_then(|p| p.settlement) == Some(village_id);
        (village, resident, member)
    };
    // An unfinished hall has no barrels. The saved inbound warehouse
    // provides its physical startup demand. Finished halls require a
    // fresh two-barrel snapshot and a reconciled external journal.
    let stored = if village.hall_complete {
        physical_barrel_stock(game, &village.dimension, village.center)
    } else { None };
    let requests = crate::game::civilizations_core::physical_supply_requests(
        &village, stored.as_deref()
    );
    let (wood_missing, stone_missing) = requests.as_ref().map_or((0, 0), |needs| {
        let amount = |good| needs.iter().find(|need| need.good == good)
            .map_or(0, |need| need.amount);
        (amount(Good::Wood), amount(Good::Stone))
    });
    let wants = requests.as_ref().map_or_else(Vec::new, |needs| {
        crate::game::civilizations_core::resident_wants_for_requests(resident.job, needs)
    });
    let carried = [Good::Wood, Good::Stone, Good::Bread, Good::Wheat].iter()
        .map(|good| (*good, game.native_count_items(good.item())))
        .collect();
    Some(Talk {
        village, resident, is_member, wants, stock_known: requests.is_some(),
        wood_missing, stone_missing, carried,
    })
}

/// Build one conversation page and queue it for the server tick.
fn show_talk(
    game: &mut Game<'_>, player: &NativeServerPlayer, village_id: u64, resident_id: u64,
    page: TalkPage, notice: Option<String>,
) -> bool {
    let Some(talk) = read_talk(game, player.uuid, village_id, resident_id) else {
        return false;
    };
    let phase = DailyPhase::at(game.world.day_time);
    let input = DialogueInput {
        village: &talk.village,
        resident: &talk.resident,
        is_member: talk.is_member,
        routine: phase.name().to_owned(),
        on_duty: phase.on_shift(talk.resident.job),
        wants: &talk.wants,
        stock_known: talk.stock_known,
        wood_missing: talk.wood_missing,
        stone_missing: talk.stone_missing,
        carried: &talk.carried,
        notice,
    };
    let snbt = dialog_snbt(&build_dialog(page, &input));
    dialog_queue().push((player.name.clone(), snbt));
    true
}

fn join_village(game: &Game<'_>, uuid: u128, village_id: u64) -> Result<String, String> {
    let dir = game.native_world_dir().ok_or("world folder unavailable")?;
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut()
        .filter(|state| state.problem.is_none() && state.world.is_some())
        .ok_or("civilizations is paused for this world")?;
    let previous = state.world.as_ref().unwrap().clone();
    let actor = format!("{uuid:032x}");
    state.world.as_mut().unwrap().join(&actor, village_id)?;
    if let Err(error) = save(state) {
        state.world = Some(previous);
        return Err(format!("not saved: {error}"));
    }
    let name = state.world.as_ref().unwrap().settlements[&village_id].name.clone();
    Ok(format!("You joined {name}. /civ title shows your balance and rank."))
}

/// Sell what the sliders chose. The coins, the warehouse credit and the save
/// are committed together with taking the items from the player; if any step
/// fails the items are given back and the world is restored.
fn sell_to_resident(
    game: &mut Game<'_>, player: &NativeServerPlayer, village_id: u64, resident_id: u64,
    words: &[&str],
) -> Result<String, String> {
    let talk = read_talk(game, player.uuid, village_id, resident_id)
        .ok_or("this villager is not available")?;
    if !talk.is_member {
        return Err("join the village first".into());
    }
    let sale = parse_sale_args(&talk.wants, &talk.carried, words)?;
    let dir = game.native_world_dir().ok_or("world folder unavailable")?;
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut()
        .filter(|state| state.problem.is_none() && state.world.is_some())
        .ok_or("civilizations is paused for this world")?;
    let previous = state.world.as_ref().unwrap().clone();
    let actor = format!("{:032x}", player.uuid);
    let mut total = 0_u64;
    let mut parts = Vec::new();
    for (good, count) in &sale {
        match state.world.as_mut().unwrap().sell(&actor, village_id, *good, *count) {
            Ok(payout) => {
                total += payout;
                parts.push(format!("{count} {}", good_label(*good)));
            }
            Err(error) => {
                state.world = Some(previous);
                return Err(error);
            }
        }
    }
    let mut taken: Vec<(Good, u32)> = Vec::new();
    for (good, count) in &sale {
        let got = game.native_take_items(good.item(), *count);
        taken.push((*good, got));
        if got != *count {
            for (given, n) in &taken {
                if *n > 0 { game.native_give_items(given.item(), *n); }
            }
            state.world = Some(previous);
            return Err("your inventory changed during the sale".into());
        }
    }
    if let Err(error) = save(state) {
        for (given, n) in &taken {
            if *n > 0 { game.native_give_items(given.item(), *n); }
        }
        log::error!("[civilizations] conversation sale not saved, items returned: {error}");
        state.world = Some(previous);
        return Err(format!("sale not committed: {error}"));
    }
    Ok(format!("Sold {} for {total} coins.", parts.join(", ")))
}

/// `/civ talk <village> <resident> [page] [good.amount ...]`: every button of a
/// conversation window runs this. It never trusts the window: the village,
/// the resident, the page and the amounts are all checked again here.
fn talk_command(game: &mut Game<'_>, player: &NativeServerPlayer, mut args: std::str::SplitWhitespace<'_>) {
    let (Some(village_name), Some(resident_word)) = (args.next(), args.next()) else {
        error(game, "usage: /civ talk <village> <resident> [page]");
        return;
    };
    let Ok(resident_id) = resident_word.parse::<u64>() else {
        error(game, "invalid resident");
        return;
    };
    let page_word = args.next().unwrap_or("main");
    let rest: Vec<&str> = args.collect();
    let Some(dir) = game.native_world_dir() else {
        error(game, "no server world open");
        return;
    };
    let found = {
        let guard = active(dir.join(DIR));
        guard.as_ref()
            .filter(|state| state.problem.is_none())
            .and_then(|state| state.world.as_ref())
            .and_then(|world| world.by_name(village_name))
            .and_then(|village| {
                let resident_uuid = village.residents.iter()
                    .find(|resident| resident.alive && resident.id == resident_id)?
                    .entity_uuid?;
                Some((village.id, village.dimension.clone(), village.center, resident_uuid))
            })
    };
    let Some((village_id, dimension, center, resident_uuid)) = found else {
        error(game, "unknown settlement");
        return;
    };
    let dx = player.position[0] - f64::from(center[0]);
    let dz = player.position[2] - f64::from(center[2]);
    if player.dimension != dimension || !player.position.iter().all(|v| v.is_finite())
        || !(dx * dx + dz * dz <= 64.0 * 64.0) {
        error(game, "you are too far from this village to talk to its people");
        return;
    }
    // Dialogue buttons are ordinary commands and can be replayed without
    // a right-click. Reverify the original UUID and live proximity for
    // every action, especially a physical inventory sale.
    let close_to_real_resident = game.native_entity_by_uuid(resident_uuid)
        .is_some_and(|(mob_dimension, mob_id, kind)| {
            mob_dimension == dimension && kind == "minecraft:villager"
                && game.native_mob(&mob_dimension, mob_id).is_some_and(|mob| {
                    let delta_x = player.position[0] - mob.pos[0];
                    let delta_y = player.position[1] - mob.pos[1];
                    let delta_z = player.position[2] - mob.pos[2];
                    delta_x * delta_x + delta_z * delta_z <= 6.0 * 6.0
                        && delta_y.abs() <= 4.0
                })
        });
    if !close_to_real_resident {
        error(game, "stand within 6 blocks of this loaded resident to talk or trade");
        return;
    }
    let (page, notice) = match page_word {
        "sell" => {
            let outcome = sell_to_resident(game, player, village_id, resident_id, &rest);
            (TalkPage::Trade, Some(outcome.unwrap_or_else(|reason| format!("Nothing sold: {reason}."))))
        }
        "join" => {
            let outcome = join_village(game, player.uuid, village_id);
            (TalkPage::Main, Some(outcome.unwrap_or_else(|reason| format!("Could not join: {reason}."))))
        }
        other => match TalkPage::parse(other) {
            Some(page) => (page, None),
            None => {
                error(game, "unknown conversation page");
                return;
            }
        },
    };
    if !show_talk(game, player, village_id, resident_id, page, notice) {
        error(game, "this villager is not available any more");
    }
}

/// Right-click on a villager. A village resident opens the conversation
/// window; any other villager inside village territory is blocked from
/// vanilla trading too, so no default trade screen ever appears.
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
    enum Found {
        Resident { village_id: u64, resident_id: u64 },
        Unbound { village: String },
    }
    let found = {
        let guard = active(dir.join(DIR));
        let Some(world) = guard.as_ref()
            .filter(|state| state.problem.is_none())
            .and_then(|state| state.world.as_ref())
        else {
            return;
        };
        let in_dimension = || world.settlements.values()
            .filter(|village| village.dimension == player.dimension);
        let linked = in_dimension().find_map(|village| {
            village.residents.iter()
                .find(|resident| resident.alive && resident.entity_uuid == Some(uuid))
                .map(|resident| Found::Resident { village_id: village.id, resident_id: resident.id })
        });
        match linked {
            Some(found) => found,
            // Not on any roster. If the player stands in a village's
            // territory this is still a village villager (not yet linked):
            // never let it open the default trading screen.
            None => {
                let near = in_dimension().find(|village| {
                    let dx = player.position[0] - f64::from(village.center[0]);
                    let dz = player.position[2] - f64::from(village.center[2]);
                    dx * dx + dz * dz <= 160.0 * 160.0
                });
                match near {
                    Some(village) => Found::Unbound { village: village.name.clone() },
                    None => return,
                }
            }
        }
    };
    *claimed = true;
    match found {
        Found::Resident { village_id, resident_id } => {
            if !show_talk(game, &player, village_id, resident_id, TalkPage::Main, None) {
                let _ = game.native_tell(player.uuid, Text::literal(
                    "This villager has nothing to say right now."
                ).with_color(text::GRAY));
            }
        }
        Found::Unbound { village } => {
            let _ = game.native_tell(player.uuid, Text::literal(format!(
                "This villager is not on {village}'s roster yet, so it has nothing to say. \
                 Default villager trading is turned off in village territory."
            )).with_color(text::GRAY));
        }
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
    // Conversation windows: handled before the world lock is taken because
    // the handler reads and saves the world itself.
    if args.clone().next() == Some("talk") {
        args.next();
        talk_command(game, player, args);
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
        reply(game, "Civilizations: /civ list | nearby | requests <name> | info [name] | roster <name> | schedule <name> | homes <name> | survey <name> | council <name> | forecast <name> | plots <name> | development <name> | hall <name> | warehouse <name> | meals <name> | posts <name> | roles <name> | status <name> | chunks <name> | diagnose <name> | buyplot <name> | buyhouse <name> | commission <name> | construction <name> | farming <name> | harvests <name> | join <name> | title | promote | history | buy/sell <good> <1..64> | (OP) found <name>, actors <name>, reconcile <name>, recoveractor <name> <resident-id> attempt, barrelcheck <name>, resolvebarrel <name> confirm|cancel <observed_count>, gathercheck <name>, discardgather <name> intact|empty, day [days], construct <name>, post <name> lumber|quarry|sand|dirt, approvehouse <name> <slot> <y>, autonomy <name> on/off, farm <name>, recoverfarm <name>, war/peace <from> <to>. Goods: wheat, bread, wood (oak logs), stone (cobble), iron.");
        return;
    }
    if action == "list" {
        let mut items = Vec::new();
        for s in world.settlements.values().take(12) {
            items.push(format!("#{} {} [{}] pop={} houses={}", s.id, s.name, s.culture, s.population(), s.houses));
        }
        reply(game, if items.is_empty() { "No settlements yet. Explore natural loaded Overworld terrain; NPC pioneers can found independent towns. OP /civ found NAME is optional.".into() } else { items.join(" | ") });
        return;
    }
    if action == "nearby" {
        // All information is derived from existing authoritative world.json.
        // No chunk loads, scans, inventory access, markers, or transactions.
        if !player.position.iter().all(|value| value.is_finite()) {
            error(game, "unable to determine a finite player position");
            return;
        }
        let mut villages: Vec<(f64, u64, String)> = world.settlements.values()
            .filter(|v| v.dimension == player.dimension)
            .map(|v| {
                let dx = f64::from(v.center[0]) - player.position[0];
                let dz = f64::from(v.center[2]) - player.position[2];
                let distance = dx.hypot(dz);
                let direction = format!(
                    "{}{}", if dz < 0.0 { "N" } else { "S" },
                    if dx < 0.0 { "W" } else { "E" }
                );
                let stage = if v.hall_complete { "hall built" }
                    else if v.hall_job.is_some() { "hall building" }
                    else { "starting" };
                (distance, v.id, format!(
                    "{} ({:.0}m {}, X{} Z{}, {})",
                    v.name, distance, direction, v.center[0], v.center[2], stage
                ))
            }).collect();
        villages.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let shown: Vec<_> = villages.iter().take(4)
            .map(|(_, _, description)| description.as_str()).collect();
        reply(game, if shown.is_empty() {
            "No known settlements in this dimension. Explore natural land; NPC communities can found on loaded, clear Overworld terrain.".to_owned()
        } else {
            format!(
                "Nearest settlements ({} known): {}. Visit a village or talk to a resident; /civ join NAME to become a citizen.",
                villages.len(), shown.join(" | ")
            )
        });
        return;
    }
    if action == "requests" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ requests <name>");
            return;
        };
        // Public read-only notices never load chunks or inspect a foreign
        // dimension. An unavailable native stock view is UNKNOWN, not zero.
        let stored = if village.hall_complete && village.dimension == player.dimension {
            physical_barrel_stock(game, &village.dimension, village.center)
        } else { None };
        let requests = crate::game::civilizations_core::physical_supply_requests(
            village, stored.as_deref()
        );
        let Some(requests) = requests else {
            reply(game, format!(
                "{} supply requests unavailable: native barrels are unloaded, or a native inventory journal needs reconciliation. Visit the hall and inspect /civ warehouse {}.",
                village.name, village.name
            ));
            return;
        };
        if requests.is_empty() {
            reply(game, format!(
                "{} has no current physical supply shortfall. This is a live snapshot, not a reserved order.",
                village.name
            ));
            return;
        }
        let notices = requests.iter().map(|request| format!(
            "{} x{} ({}; {} coins/item at existing market rate)",
            request.good.as_str(), request.amount, request.purpose, request.good.price()
        )).collect::<Vec<_>>();
        reply(game, format!(
            "{} asks for {}. Join and speak in person with the mayor (or a baker for wheat) to deliver items. Amounts are read-only estimates, not reserved contracts; inventory and demand are checked again on trade.",
            village.name, notices.join(" | ")
        ));
        return;
    }
    if action == "meals" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ meals <name>");
            return;
        };
        let fed = village.living_meals_served();
        let population = village.population() as usize;
        let pending = village.barrel_transfer.as_ref()
            .filter(|t| t.direction == BarrelDirection::ToResidentMeal)
            .map(|t| format!(
                "recipient {:?}, {:?}, native before={}, count={}",
                t.recipient_id, t.phase, t.before, t.count
            )).unwrap_or_else(|| "none".to_owned());
        let physical = if village.dimension == player.dimension && village.hall_complete {
            physical_barrel_stock(game, &village.dimension, village.center)
                .and_then(|items| items.into_iter()
                    .find(|(good, _)| *good == Good::Bread).map(|(_, amount)| amount))
                .map(|value| format!("{value} verified loaves"))
                .unwrap_or_else(|| "unloaded / unavailable".to_owned())
        } else { "no nearby completed native hall".to_owned() };
        reply(game, format!(
            "{} REAL meals: host day {:?}, served {}/{} living residents, shortage streak {} completed days. Bread in vanilla barrels: {}. Pending native receipt: {}. Every portion requires a UUID-bound resident at the hall, steward and exactly one extracted real loaf; virtual food does NOT count.",
            village.name, village.meal_day, fed, population,
            village.meal_shortage_days, physical, pending
        ));
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
        let goods = [Good::Wheat, Good::Bread, Good::Wood, Good::Stone, Good::Iron,
                     Good::Sand, Good::Dirt];
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
            format!("oak log {}/{} at {:?}, issued={}, native={}, pickup={}, blocked={}",
                stem.cursor + 1, stem.height,
                v.gather_job.as_ref().map(|job| (job.position, job.worker_id)),
                v.gather_job.as_ref().is_some_and(|job| job.issued),
                v.gather_job.as_ref().is_some_and(|job| job.native_edit_confirmed),
                v.gather_job.as_ref().is_some_and(|job| job.pickup_witnessed),
                v.gather_job.as_ref().is_some_and(|job| job.blocked))
        } else {
            v.gather_job.as_ref().map(|job| format!(
                "{:?} at {:?}, worker={:?}, issued={}, native={}, pickup={}, blocked={}",
                job.good, job.position, job.worker_id, job.issued,
                job.native_edit_confirmed, job.pickup_witnessed, job.blocked
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
            "{} gather audit: {:?} at {:?}, worker stand {:?}, assigned resident {:?}, planned native block {}, issued={}, native edit confirmed={}, source pickup witnessed={}, blocked={}, live native block {:?}.{} Blocked ISSUED receipts are quarantined. Inspect world save and original block before OP /civ discardgather {} intact|empty; abandonment provides ZERO materials.",
            v.name, job.good, job.position, job.stand, job.worker_id, job.source_block(),
            job.issued, job.native_edit_confirmed, job.pickup_witnessed,
            job.blocked, observed, stem_progress, v.name
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
        let listing: Vec<_> = s.residents.iter().map(|r| format!(
            "{}#{}:{} [{}]", r.name, r.id, r.job.name(),
            if s.actor_recovery_issued.contains(&r.id) { "RECOVERY PENDING" }
            else if r.entity_uuid.is_some() { "UUID saved" } else { "UNBOUND" }
        )).collect();
        reply(game, format!(
            "{} actors: {}/{} nearby and loaded | {}. OP partial spawn: /civ reconcile {}, then /civ recoveractor {} <resident-id> attempt ONLY after native inspection.",
            s.name, linked.len(), s.population(), listing.join(", "), s.name, s.name
        ));
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
    if action == "chunks" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return; };
        let sites: Vec<[i32; 3]> = settlement_work_sites(s);
        let wanted = crate::game::civilizations_core::village_chunk_footprint(
            s.center, &sites,
        );
        reply(game, format!(
            "{} holds {} chunks ({:?} + {} workplaces/corridors) around {:?} while autonomy is on ({})",
            s.name, wanted.len(), wanted.first().copied().unwrap_or([0, 0]),
            sites.len(), s.center,
            if s.autonomy_enabled { "held now" } else { "paused: released" }
        ));
        return;
    }
    if action == "status" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return; };
        let (missing_wood, missing_stone) = s.hall_materials_missing();
        let camps = if s.posts.is_empty() {
            "none".to_owned()
        } else {
            s.posts.iter().map(|post| {
                let worker = post.worker_id
                    .and_then(|id| s.residents.iter().find(|r| r.id == id && r.alive))
                    .map(|r| r.name.as_str()).unwrap_or("-");
                format!("{} {:?} ({worker})", post.kind.name(), post.origin)
            }).collect::<Vec<_>>().join(", ")
        };
        let harvest = s.gather_job.as_ref().map(|job| format!(
            "{} {:?}", job.good.as_str(),
            if job.issued { "issued" } else { "planned" }
        )).unwrap_or_else(|| "none".to_owned());
        reply(game, format!(
            "{}: pop {} (laborers {}), hall {}, homeless {}, next builds {}, missing {} logs/{} cobble, camps [{}], harvest {}",
            s.name, s.population(), s.count(Job::Laborer),
            if s.hall_complete { "complete" } else if s.hall_job.is_some() { "building" } else { "pending" },
            s.physical_housing().2, s.pending_visual_buildings,
            missing_wood, missing_stone, camps, harvest
        ));
        return;
    }
    if action == "roles" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return; };
        let seats: Vec<String> = s.workplace_demand().iter()
            .map(|(job, seats)| format!("{} {}/{}", job.name(), s.count(*job), seats))
            .collect();
        reply(game, format!(
            "{} roles (filled/workplaces): {} | laborers {}",
            s.name, seats.join(", "), s.count(Job::Laborer)
        ));
        return;
    }
    if action == "posts" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return; };
        let text = if s.posts.is_empty() {
            format!("{} has no work camps yet. OP: stand at a clear spot 8..80 blocks from center and /civ post {} lumber|quarry|sand|dirt", s.name, s.name)
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
            if player.permission < 2 {
                return Err("Villagers trade in person: talk to the mayor or the baker (right-click) and use their Trade menu.".into());
            }
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
            if already_saved {
                // A previous step committed world.json. Rolling back only
                // RAM would make subsequent saves silently erase the
                // committed identity/transaction record. Fail closed.
                log::error!("[civilizations] command failed after durable save: {message}");
                state.problem = Some(format!(
                    "command failed after durable save; inspect before restarting: {message}"
                ));
                error(game, format!(
                    "Command stopped after a durable save; world halted for inspection: {message}"
                ));
            } else {
                state.world = Some(previous);
                error(game, message);
            }
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
fn surveyed_initial_actor_positions(
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
fn auto_spawn_villagers(
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
            if transfer.direction == BarrelDirection::ToResidentMeal {
                // A citizen can die or walk away between Planned and Issued.
                // Reverify the saved recipient UUID, real mob location and
                // steward immediately before physically consuming bread.
                // Cancel only a never-issued intent, without any native edit.
                let still_eligible = transfer.recipient_id.is_some_and(|recipient| {
                    village.residents.iter().any(|r| r.id == recipient
                        && r.alive && r.entity_uuid.is_some()
                        && !village.meal_served.contains(&recipient))
                }) && storage_worker_near(
                    game, state.world.as_ref().unwrap(), id, Job::Merchant
                ) && {
                    let actors = loaded_actors(game, state.world.as_ref().unwrap(), id);
                    transfer.recipient_id.is_some_and(|recipient| {
                        actors.iter().any(|(known, mob)| *known == recipient
                            && game.native_mob(&dimension, *mob).is_some_and(|actor|
                                native_worker_near(actor.pos, village.center, 8.0, 4.0)))
                    })
                };
                if !still_eligible {
                    state.world.as_mut().unwrap().cancel_unissued_meal(id)?;
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
                | BarrelDirection::ToBakerIngredients
                | BarrelDirection::ToResidentMeal => {
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

/// A meal is served only to a genuine nearby UUID-bound resident. Every
/// portion is one actual bread item removed from a loaded civic barrel by
/// the EXISTING durable Planned -> Issued -> observed/Blocked transaction.
/// Unloaded players/NPCs and abstract food are never treated as evidence.
fn resident_meal_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dimension = game.dim.id().to_owned();
    let host_day = game.world.day_time.div_euclid(24_000);
    let ids: Vec<u64> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension && v.hall_complete
            && v.actors_spawned && v.autonomy_enabled)
        .map(|v| v.id).collect();
    for id in ids {
        let village = state.world.as_ref().unwrap().settlements[&id].clone();
        if village.meal_day != Some(host_day) {
            let old = state.world.as_ref().unwrap().clone();
            if state.world.as_mut().unwrap().begin_meal_day(id, host_day)? {
                save_autonomous(state, old)?;
                return Ok(());
            }
            continue;
        }
        // A single shared, durable transfer serializes meals with building,
        // bakery supplies, steward deposits, and real player purchases.
        if village.barrel_transfer.is_some() || village.market_purchase.is_some() {
            continue;
        }
        if village.living_meals_served() >= village.population() as usize {
            continue;
        }
        // The steward AND the actual recipient must have arrived. An NPC's
        // saved UUID plus a visible villager at the hall is the only identity
        // accepted; names alone, unloaded villagers or virtual stock do not count.
        if !storage_worker_near(game, state.world.as_ref().unwrap(), id, Job::Merchant) {
            continue;
        }
        let loaded = loaded_actors(game, state.world.as_ref().unwrap(), id);
        let recipient = village.residents.iter()
            .filter(|r| r.alive && r.entity_uuid.is_some()
                && !village.meal_served.contains(&r.id))
            .find_map(|r| loaded.iter().find_map(|(known, mob)| {
                if *known != r.id { return None; }
                game.native_mob(&dimension, *mob).and_then(|actor| {
                    native_worker_near(actor.pos, village.center, 8.0, 4.0)
                        .then_some(r.id)
                })
            }));
        let Some(resident_id) = recipient else { continue; };
        for position in hall_storage_positions(village.center) {
            if world_block(game, &dimension, position).as_deref() != Some("minecraft:barrel") {
                continue;
            }
            let Some(slots) = game.native_container_slots(&dimension, position, Direction::Up)
            else { continue; };
            let source = slots.iter().find_map(|entry| {
                let stack = entry.stack.as_ref()?;
                (entry.can_take && stack.item.as_ref() == Good::Bread.item()
                    && stack.damage == 0 && stack.extra.is_none()
                    && stack.count >= 1).then_some(entry.slot)
            });
            let Some(slot) = source else { continue; };
            let Some(before) = plain_barrel_count(game, &dimension, position, Good::Bread)
            else { continue; };
            let old = state.world.as_ref().unwrap().clone();
            state.world.as_mut().unwrap().plan_resident_meal(
                id, resident_id, host_day, position, before, slot,
            )?;
            save_autonomous(state, old)?;
            return Ok(());
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

/// Natural frontier founding: players may DISCOVER an independent town
/// without operator commands, but the game never creates one on an unloaded
/// chunk or modifies the user's immediate base. A successful write only adds
/// the social record; the existing one-shot guarded actor startup and
/// resource-backed town hall perform native work on subsequent ticks.
const FRONTIER_OFFSETS: [[i32; 2]; 8] = [
    [88, 0], [-88, 0], [0, 88], [0, -88],
    [64, 64], [-64, 64], [64, -64], [-64, -64],
];
const FRONTIER_NAMES: [&str; 8] = [
    "Oakford", "Stonevale", "Birchmere", "Redmoor",
    "Rivermark", "Pinewatch", "Ashfield", "Dawnhold",
];

fn autonomous_frontier_found(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dimension = game.dim.id();
    if dimension != "minecraft:overworld" { return Ok(()); }
    let Some(world) = state.world.as_ref() else { return Ok(()); };
    if world.settlements.len() >= crate::game::civilizations_core::MAX_SETTLEMENTS
        || world.settlements.values().filter(|s|
            s.founder == crate::game::civilizations_core::NPC_FOUNDER).count()
            >= crate::game::civilizations_core::MAX_NPC_SETTLEMENTS {
        return Ok(());
    }

    // Observe only real authoritative players in the currently ticked
    // dimension. Include the integrated single-player host, which the
    // upstream native_server_players() guest-only API intentionally omits.
    let mut explorers = Vec::new();
    if let Some(host) = game.player.as_ref() {
        explorers.push([host.position.x, host.position.y, host.position.z]);
    }
    for player in game.native_server_players() {
        if player.dimension == dimension {
            explorers.push(player.position);
        }
    }
    let first_direction = (state.physical_ticks / 2_400) as usize % FRONTIER_OFFSETS.len();
    for explorer in explorers {
        if !explorer.iter().all(|c| c.is_finite()) { continue; }
        // Keep the candidate offsets and native block reads inside
        // Minecraft's world boundary, away from arithmetic wraparound.
        if explorer[0].abs() > 29_999_000.0
            || explorer[2].abs() > 29_999_000.0
            || explorer[1].abs() > 1_900.0 {
            continue;
        }
        let origin = explorer.map(|v| v.floor() as i32);
        // Do not generate another settlement just because the player is
        // standing in their own town or has built a base around one.
        let near_existing = world.settlements.values()
            .filter(|s| s.dimension == dimension)
            .any(|v| {
                let dx = i64::from(v.center[0]) - i64::from(origin[0]);
                let dz = i64::from(v.center[2]) - i64::from(origin[2]);
                dx * dx + dz * dz < 192_i64 * 192
            });
        if near_existing { continue; }

        for index in 0..FRONTIER_OFFSETS.len() {
            let offset = FRONTIER_OFFSETS[(index + first_direction) % FRONTIER_OFFSETS.len()];
            let (Some(x), Some(z)) = (
                origin[0].checked_add(offset[0]),
                origin[2].checked_add(offset[1])
            ) else { continue; };
            for dy in [0_i32, -1, 1, -2, 2, -3, 3] {
                let Some(y) = origin[1].checked_add(dy) else { continue; };
                let candidate = [x, y, z];
                // The 5x5 site survey rejects trees, player structures,
                // non-natural ground, unloaded chunks and terrain cliffs.
                // Do not dig, level, clear or forcibly load ANY blocks.
                if check_empty_hut_site(game, dimension, candidate).is_err() {
                    continue;
                }
                let id = world.next_id;
                let name = format!(
                    "{}_{}", FRONTIER_NAMES[((id - 1) % FRONTIER_NAMES.len() as u64) as usize], id
                );
                // Recheck domain constraints against a trial, avoiding an
                // unnecessary save on name or inter-village collisions.
                let mut trial = world.clone();
                let Ok(trial_id) = trial.found_autonomous(&name, dimension, candidate)
                else { continue; };
                // Do not leave behind an independent village whose first
                // ten residents can never appear. Require the SAME full
                // loaded-chunk, no-duplicate, collision-free ten-position
                // NPC survey used for the actual native spawn reservation.
                // This first-pass survey is read-only; the later real
                // spawn path rechecks all preconditions before persistence.
                if surveyed_initial_actor_positions(
                    game, &trial.settlements[&trial_id]
                ).is_err() {
                    continue;
                }
                let before = state.world.as_ref().unwrap().clone();
                state.world.as_mut().unwrap()
                    .found_autonomous(&name, dimension, candidate)?;
                save_autonomous(state, before)?;
                log::info!(
                    "[civilizations] independent NPC village {} founded at {:?}; loaded natural plot",
                    name, candidate
                );
                game.native_broadcast(Text::literal(format!(
                    "Explorers report a new independent settlement: {} near {}, {}. Visit its town hall or use /civ info {}.",
                    name, x, z, name
                )));
                return Ok(());
            }
        }
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
    // Roles follow real workplaces. Do this before any planner decision or
    // the first spawn so residents never hold jobs the village cannot give.
    staff_village(game, state, id)?;
    if !state.world.as_ref().unwrap().settlements[&id].actors_spawned {
        return auto_spawn_villagers(game, state, id);
    }
    let prior = state.world.as_ref().unwrap().clone();
    if state.world.as_mut().unwrap().prepare_village_housing(id)? {
        save_autonomous(state, prior)?;
    }
    let village = state.world.as_ref().unwrap().settlements[&id].clone();
    let actors = loaded_actors(game, state.world.as_ref().unwrap(), id);
    let has_builder = village.residents.iter().any(|r| r.alive && r.job == Job::Builder
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
        // Stone is rarely exposed at the surface of a forest or plains
        // village. A quarry starts as a pit on natural ground, so open one
        // when cobble is short and the village has none yet.
        if stone_needed > 0 && village.posts.iter().all(|post| post.kind != PostKind::Quarry) {
            if let Some(spot) = quarry_start(game, &dimension, village.center) {
                let before = state.world.as_ref().unwrap().clone();
                if state.world.as_mut().unwrap()
                    .add_post(id, PostKind::Quarry, spot).is_ok()
                {
                    save_autonomous(state, before)?;
                    log::info!("[civilizations] {} opened a quarry pit at {:?}", village.name, spot);
                    return Ok(());
                }
            }
        }
        let mut work: Vec<(Good, u32, Job)> = vec![
            (Good::Wood, wood_needed, Job::Lumberjack),
            (Good::Stone, stone_needed, Job::Miner),
        ];
        // A sand or dirt pit the player placed is a real workplace, so it is
        // serviced on its own. Nothing builds with sand or dirt yet, so it
        // only keeps a small, explicit buffer in the hall.
        const PIT_TARGET: u32 = 32;
        for pit in [PostKind::Sand, PostKind::Dirt] {
            if !village.posts.iter().any(|post| post.kind == pit) { continue; }
            let stored = if village.hall_complete {
                physical_barrel_stock(game, &dimension, village.center)
                    .and_then(|stock| stock.iter()
                        .find(|(good, _)| *good == pit.good())
                        .map(|(_, count)| *count)).unwrap_or(0)
            } else { 0 };
            let have = village.warehouse_quantity(pit.good()).saturating_add(stored);
            if have < PIT_TARGET {
                work.push((pit.good(), PIT_TARGET - have, pit.job()));
            }
        }
        for (good, need, profession) in work {
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
            let camp_kind = PostKind::for_good(good);
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
    let mut clearing = 4_u32;
    if !village.hall_complete && village.hall_job.is_none() && has_builder {
        let hall_site = village.center;
        if open_build_plot(game, &dimension, hall_site, 2, 3, &mut clearing,
            |game: &Game<'_>| check_empty_hut_site(game, &dimension, hall_site)).is_ok() {
            let before = state.world.as_ref().unwrap().clone();
            if state.world.as_mut().unwrap().reserve_town_hall(id)? {
                save_autonomous(state, before)?;
                log::info!("[civilizations] {} started material-backed civic hall", village.name);
                return Ok(());
            }
        }
    }
    // The field and the bakery are the workplaces: reserving one is what
    // creates its worker seat, so neither waits for a farmer or baker that
    // could only exist after the site. Roles are staffed every turn.
    let need_field = village.farm.is_none();
    let need_bakery = has_builder && village.hall_complete
        && village.bakery.is_none();
    let need_hut = has_builder && village.hall_complete && village.visual_job.is_none()
        && village.pending_visual_buildings > 0 && village.physical_housing().2 > 0;
    if !need_field && !need_hut && !need_bakery { return Ok(()); }

    // The first plot the planner had to refuse, so a stalled village can say
    // why in the log instead of silently standing still.
    let mut refusal: Option<String> = None;
    for [dx, dz] in crate::game::civilizations_core::HOUSING_PLOT_OFFSETS {
        let Some(x) = village.center[0].checked_add(dx) else { continue; };
        let Some(z) = village.center[2].checked_add(dz) else { continue; };
        for dy in [0_i32, 1, -1, 2, -2] {
            let Some(y) = village.center[1].checked_add(dy) else { continue; };
            let site = [x, y, z];
            // First establish a functional field. Farms must be harvested
            // by a loaded nearby farmer; their cycles do NOT mint stock.
            if need_field {
                let mut trial = state.world.as_ref().unwrap().clone();
                if trial.start_farm(id, site).is_ok() {
                    match open_build_plot(game, &dimension, site, 1, 1, &mut clearing,
                        |game: &Game<'_>| check_empty_farm_site(game, &dimension, site)) {
                        Ok(cleared) => {
                            let prior = state.world.as_ref().unwrap().clone();
                            state.world.as_mut().unwrap().start_farm(id, site)?;
                            save_autonomous(state, prior)?;
                            log::info!("[civilizations] {} autonomously reserved crop field at {:?} ({cleared} growth cleared)",
                                village.name, site);
                            return Ok(());
                        }
                        Err(reason) => {
                            if refusal.is_none() { refusal = Some(format!("field at {site:?}: {reason}")); }
                        }
                    }
                }
            }
            // After the field and civic hall exist, build the first
            // functional workshop on the same loaded, physically open,
            // non-player-owned plot policy as ordinary housing.
            if need_bakery {
                let allowed = state.world.as_ref().unwrap()
                    .check_visual_house_site(id, site).is_ok();
                let mut trial = state.world.as_ref().unwrap().clone();
                if allowed && trial.start_bakery(id, site).is_ok() {
                    match open_build_plot(game, &dimension, site, 2, 3, &mut clearing,
                        |game: &Game<'_>| check_empty_hut_site(game, &dimension, site)) {
                        Ok(cleared) => {
                            let old = state.world.as_ref().unwrap().clone();
                            state.world.as_mut().unwrap().start_bakery(id, site)?;
                            save_autonomous(state, old)?;
                            log::info!("[civilizations] {} autonomously reserved real bakery at {:?} ({cleared} growth cleared)",
                                village.name, site);
                            return Ok(());
                        }
                        Err(reason) => {
                            if refusal.is_none() { refusal = Some(format!("bakery at {site:?}: {reason}")); }
                        }
                    }
                }
            }
            if need_hut {
                let can_build = state.world.as_ref().unwrap()
                    .check_visual_house_site(id, site).is_ok();
                if can_build {
                    match open_build_plot(game, &dimension, site, 2, 3, &mut clearing,
                        |game: &Game<'_>| check_empty_hut_site(game, &dimension, site)) {
                        Ok(cleared) => {
                            let prior = state.world.as_ref().unwrap().clone();
                            state.world.as_mut().unwrap().start_visual_house(id, site)?;
                            save_autonomous(state, prior)?;
                            log::info!("[civilizations] {} autonomously reserved real hut at {:?} ({cleared} growth cleared)",
                                village.name, site);
                            return Ok(());
                        }
                        Err(reason) => {
                            if refusal.is_none() { refusal = Some(format!("hut at {site:?}: {reason}")); }
                        }
                    }
                }
            }
        }
    }
    // No clear loaded plot; never clear players' trees, fences or walls.
    if state.physical_ticks % 1200 == 0 {
        if let Some(reason) = refusal {
            log::info!(
                "[civilizations] {} started no build and no field: {reason}",
                village.name
            );
        }
    }
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
    sign_data_command_for("minecraft:oak_wall_sign", dimension, pos, spec, rich)
}

/// Same as `sign_data_command` for any sign block (wall or standing).
fn sign_data_command_for(
    block: &str, dimension: &str, pos: [i32; 3],
    spec: &crate::game::civilizations_core::SignSpec, rich: bool,
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
        "execute in {dimension} if block {x} {y} {z} {block} run data merge block {x} {y} {z} {{front_text:{{messages:[{}],color:\"black\",has_glowing_text:{}b}}}}",
        messages.join(","), if spec.glow { 1 } else { 0 }
    )
}

/// Standing progress board beside every building under construction, which
/// becomes a short status board once the building is finished. Boards are
/// only placed on air above natural ground, and a finished board is only
/// rewritten if this session placed the matching construction board.
fn refresh_site_boards(game: &mut Game<'_>, world: &World) {
    use crate::game::civilizations_core::{
        bakery_plan, render_bakery_ready, render_build_progress, render_hut_ready,
        site_board_position, SignSpec,
    };
    const BLOCK: &str = "minecraft:oak_sign";
    let dimension = game.dim.id().to_owned();
    for village in world.settlements.values().filter(|v| v.dimension == dimension) {
        // (origin, spec, is_construction)
        let mut boards: Vec<([i32; 3], SignSpec, bool)> = Vec::new();
        if let Some(job) = village.hall_job.as_ref().filter(|_| !village.hall_complete) {
            let total = town_hall_plan(job.origin).len();
            boards.push((job.origin, render_build_progress("HALL", job.cursor, total, job.blocked), true));
        }
        if let Some(job) = village.visual_job.as_ref() {
            let total = hut_plan(job.origin).len();
            boards.push((job.origin, render_build_progress("HUT", job.cursor, total, job.blocked), true));
        }
        if let Some(bakery) = village.bakery.as_ref() {
            if let Some(job) = bakery.building.as_ref().filter(|_| !bakery.complete) {
                let total = bakery_plan(job.origin).len();
                boards.push((bakery.origin, render_build_progress("BAKERY", job.cursor, total, job.blocked), true));
            } else if bakery.complete {
                boards.push((bakery.origin, render_bakery_ready(bakery), false));
            }
        }
        for site in &village.visual_sites {
            let active = village.visual_job.as_ref().is_some_and(|job| job.origin == *site);
            if !active {
                boards.push((*site, render_hut_ready(village), false));
            }
        }
        for (origin, spec, construction) in boards {
            let pos = site_board_position(origin);
            let [x, y, z] = pos;
            let key = (dimension.clone(), pos);
            let present = world_block(game, &dimension, pos).as_deref() == Some(BLOCK);
            if !present {
                // Finished boards never create a sign: only a construction
                // board we placed may turn into one, so old villages stay clean.
                if !construction { continue; }
                let on_ground = world_block(game, &dimension, [x, y - 1, z])
                    .is_some_and(|block| is_ground(&block));
                if !on_ground
                    || world_block(game, &dimension, pos).as_deref() != Some("minecraft:air")
                { continue; }
                let place = format!(
                    "execute in {dimension} if block {x} {y} {z} minecraft:air run setblock {x} {y} {z} {BLOCK}[rotation=8] replace"
                );
                if !game.native_run_command(&place) {
                    log::warn!("[civilizations] site sign placement unsupported: {place}");
                    continue;
                }
            } else if !construction && !BOARD_CACHE.lock().unwrap().contains_key(&key) {
                // A standing sign we did not place this session: leave it alone.
                continue;
            }
            let fingerprint = format!("{:?}{}", spec.lines, spec.glow);
            if present && BOARD_CACHE.lock().unwrap().get(&key) == Some(&fingerprint) {
                continue;
            }
            let rich = sign_data_command_for(BLOCK, &dimension, pos, &spec, true);
            let ok = game.native_run_command(&rich)
                || game.native_run_command(&sign_data_command_for(BLOCK, &dimension, pos, &spec, false));
            if ok {
                BOARD_CACHE.lock().unwrap().insert(key, fingerprint);
            } else {
                log::warn!("[civilizations] site sign refresh unsupported at {pos:?}");
            }
        }
    }
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

/// Places a villager must reach while keeping the village going. The civic
/// centre itself is implicit; these are the real workplaces and homes.
fn settlement_work_sites(
    village: &crate::game::civilizations_core::Settlement,
) -> Vec<[i32; 3]> {
    let mut sites: Vec<[i32; 3]> = village.posts.iter().map(|post| post.origin).collect();
    if let Some(farm) = &village.farm { sites.push(farm.origin); }
    if let Some(bakery) = &village.bakery { sites.push(bakery.origin); }
    if let Some(job) = &village.hall_job { sites.push(job.origin); }
    if let Some(job) = &village.visual_job { sites.push(job.origin); }
    sites.extend(village.visual_sites.iter().copied().take(8));
    sites
}

/// Keep each autonomous village's own work area ticking with nobody standing
/// in it: a bounded ring of chunks around the civic centre and its camps.
/// The engine does not save these tickets, so they are re-held every few
/// seconds; a paused village releases everything it holds.
fn hold_village_chunks(game: &mut Game<'_>, world: &World) {
    let dimension = game.dim.id().to_owned();
    for village in world.settlements.values().filter(|v| v.dimension == dimension) {
        let owner = format!("{MOD_ID}:village/{}", village.id);
        let held = game.native_forced_chunks(&owner);
        if !village.autonomy_enabled {
            for chunk in held {
                game.native_force_chunk_in(&owner, &dimension, chunk, false);
            }
            continue;
        }
        let sites = settlement_work_sites(village);
        let wanted = crate::game::civilizations_core::village_chunk_footprint(
            village.center, &sites,
        );
        for chunk in held.iter().filter(|chunk| !wanted.contains(chunk)) {
            game.native_force_chunk_in(&owner, &dimension, *chunk, false);
        }
        if wanted.len() != held.len() {
            log::info!(
                "[civilizations] {} holds {} chunks of its own work area",
                village.name, wanted.len()
            );
        }
        for chunk in wanted.iter().filter(|chunk| !held.contains(chunk)) {
            game.native_force_chunk_in(&owner, &dimension, *chunk, true);
        }
    }
}

/// Rename one spawned villager so its tag carries the role it really holds.
/// Horizon stores CustomName as a plain string, so no component JSON is used.
fn rename_actor(game: &mut Game<'_>, uuid: u128, name: &str) -> bool {
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

/// Derive this village's roles from the workplaces it really has, persist one
/// change, and keep the visible name tags in step. Called before the first
/// spawn so no villager is ever labelled with a job the village cannot
/// provide. Only saved state and an entity name change here, never a block.
fn staff_village(game: &mut Game<'_>, state: &mut ActiveWorld, id: u64) -> Result<(), String> {
    let prior = state.world.as_ref().unwrap().clone();
    let changed = state
        .world
        .as_mut()
        .ok_or("world unavailable")?
        .settlements
        .get_mut(&id)
        .ok_or("unknown settlement")?
        .staff_workplaces();
    if changed.is_empty() {
        return Ok(());
    }
    // Persist the exact roster before renaming anything: a save failure must
    // not leave a tag that claims a role the save never stored.
    save_autonomous(state, prior)?;
    let world = state.world.as_ref().unwrap();
    let village = world.settlements.get(&id).ok_or("unknown settlement")?;
    for (resident_id, job) in &changed {
        let Some(resident) = village.residents.iter().find(|r| r.id == *resident_id)
        else { continue };
        log::info!(
            "[civilizations] {} now works as {} in {}",
            resident.name, job.name(), village.name
        );
        // One short system line so the player sees the village reorganise
        // itself instead of only finding different name tags later.
        game.native_broadcast(Text::literal(format!(
            "{}: {} is now the village {}", village.name, resident.name, job.name()
        )).with_color(text::GOLD));
        if let Some(uuid) = resident.entity_uuid {
            if !rename_actor(game, uuid, &village.actor_name(resident)) {
                log::info!(
                    "[civilizations] role tag for {} left unchanged; /civ roles {} is authoritative",
                    resident.name, village.name
                );
            }
        }
    }
    Ok(())
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
fn gather_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
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
    let Some(role) = job.good.job() else { return Ok(()); };
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
            let Some(role) = job.good.job() else { return Ok(()); };
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
    let profession = role;
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

/// A quarry needs no exposed stone: a pit starts on any natural surface near
/// the village. Bounded ring scan, first safe stand wins.
fn quarry_start(game: &Game<'_>, dimension: &str, center: [i32; 3]) -> Option<[i32; 3]> {
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
/// credited only after that same worker returns to the hall.
fn quarry_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    if !DailyPhase::at(game.world.day_time).on_shift(Job::Miner) { return Ok(()); }
    let dimension = game.dim.id().to_owned();
    let target = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension && v.gather_job.is_none())
        .find_map(|v| v.posts.iter().find(|post| post.kind == PostKind::Quarry)
            .map(|post| (v.id, post.origin, v.quarry_step, post.worker_id)));
    let Some((id, origin, step, Some(worker))) = target else { return Ok(()); };
    if step >= MAX_QUARRY_STEPS { return Ok(()); }
    let depth = i32::from(step);
    let position = [origin[0] + depth, origin[1] - depth, origin[2]];
    let Some(name) = world_block(game, &dimension, position) else { return Ok(()); };
    // Only the bound miner digs this pit, and only while standing at it.
    let world = state.world.as_ref().unwrap();
    let village = &world.settlements[&id];
    let village_name = village.name.clone();
    let linked = loaded_actors(game, world, id);
    let near = village.residents.iter()
        .filter(|r| r.alive && r.id == worker && r.job == Job::Miner)
        .any(|r| linked.iter().any(|(known, mob)| *known == r.id
            && game.native_mob(&dimension, *mob).is_some_and(|actor|
                native_worker_near(actor.pos, position, 4.0, 3.0))));
    if !near { return Ok(()); }
    let before = state.world.as_ref().unwrap().clone();
    if name == "minecraft:air" {
        state.world.as_mut().unwrap().advance_quarry(id)?;
        return save_autonomous(state, before);
    }
    if name == "minecraft:stone" {
        let Some(stand) = gather_stand(game, &dimension, position) else { return Ok(()); };
        if state.world.as_mut().unwrap()
            .plan_gather(id, Good::Stone, position, stand).is_ok()
        {
            save_autonomous(state, before)?;
            log::info!("[civilizations] {village_name} quarry reached stone at {position:?}");
        }
        return Ok(());
    }
    if is_ground(&name) || is_clearable_growth(&name) {
        let [x, y, z] = position;
        if game.native_run_command(&format!(
            "execute in {dimension} run setblock {x} {y} {z} minecraft:air replace"
        )) {
            state.world.as_mut().unwrap().advance_quarry(id)?;
            return save_autonomous(state, before);
        }
        return Ok(());
    }
    // A player block or water stops the pit: never dig through it.
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
    // At most one loaded-world frontier survey every 2,400 ticks.
    // Only this authoritative server-level hook may found villages.
    if state.physical_ticks % 2_400 == 0 {
        if let Err(error) = autonomous_frontier_found(game, state) {
            log::warn!("[civilizations] frontier founding paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
    }
    // Villages start their own visible work by default, without needing
    // OP construction/field/spawn commands. The planner is bounded and
    // *never* edits a block without the existing worker-aware tick.
    if state.physical_ticks % 400 == 0 {
        if let Some(world) = state.world.as_ref() {
            refresh_hall_boards(game, world);
        }
        if let Some(world) = state.world.as_ref() {
            refresh_post_boards(game, world);
            refresh_site_boards(game, world);
        }
    }
    if state.physical_ticks % 100 == 0 || state.physical_ticks == 1 {
        // A village must keep working with nobody standing in it, so its own
        // bounded work area is held loaded before any planner reads blocks.
        if let Some(world) = state.world.as_ref() {
            hold_village_chunks(game, world);
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
        if let Err(error) = resident_meal_one_step(game, state) {
            log::warn!("[civilizations] physical resident dining paused: {error}");
            if error.starts_with("persistence:") {
                state.problem = Some(error);
                return;
            }
        }
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
        if let Err(error) = quarry_one_step(game, state) {
            log::warn!("[civilizations] quarry excavation paused: {error}");
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
    // Conversation windows are shown from here: the server command that opens
    // a dialog does not run inside a player command or hook.
    process_dialog_queue(game);
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


#[cfg(test)]
mod interrupted_first_save_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    fn sandbox() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!(
            "horizon-civ-save-{}-{nanos}-{}",
            std::process::id(), NEXT_DIR.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn interrupted_first_write_cannot_look_like_a_new_empty_world() {
        let path = sandbox();
        let pending = path.join("world.json.tmp");
        fs::write(&pending, b"{ interrupted first write").unwrap();
        let loaded = load(path.clone());
        assert!(loaded.world.is_none());
        assert!(loaded.problem.as_deref().unwrap().contains("world.json.tmp"));
        assert_eq!(fs::read(&pending).unwrap(), b"{ interrupted first write");
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn a_valid_primary_does_not_silently_discard_an_abandoned_tmp() {
        let path = sandbox();
        let world = World::default();
        fs::write(path.join(SAVE), serde_json::to_vec(&world).unwrap()).unwrap();
        fs::write(path.join("world.json.tmp"), b"unfinished newer state").unwrap();
        let loaded = load(path.clone());
        assert!(loaded.world.is_none());
        assert!(loaded.problem.as_deref().unwrap().contains("world.json.tmp"));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn stale_tmp_is_never_truncated_by_a_later_save() {
        let path = sandbox();
        let pending = path.join("world.json.tmp");
        fs::write(&pending, b"recoverable pending bytes").unwrap();
        let state = ActiveWorld {
            path: path.clone(),
            world: Some(World::default()),
            problem: None,
            physical_ticks: 0,
        };
        let failure = save(&state).unwrap_err();
        assert!(failure.contains("world.json.tmp"));
        assert_eq!(fs::read(&pending).unwrap(), b"recoverable pending bytes");
        assert!(!path.join(SAVE).exists());
        fs::remove_dir_all(path).unwrap();
    }
}
