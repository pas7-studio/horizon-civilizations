// Horizon bridge. All world logic lives in civilizations_core; this module
// translates server hooks, commands, inventory changes, and native NPC spawns.
use crate::game::{
    civilizations_core::{hut_plan, Good, World, MAX_SIM_DAYS},
    native_mobs::NativeSpawn,
    native_server::{NativeCommand, NativeServerPlayer},
    Game,
};
use horizon_client::text::{self, Text};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

const DIR: &str = "horizon-civilizations";
const SAVE: &str = "world.json";
const TICKS_PER_DAY: u64 = 24_000;

struct ActiveWorld {
    path: PathBuf,
    world: Option<World>,
    problem: Option<String>,
    server_ticks: u64,
    physical_ticks: u64,
}

static ACTIVE: Mutex<Option<ActiveWorld>> = Mutex::new(None);

fn load(path: PathBuf) -> ActiveWorld {
    let file = path.join(SAVE);
    if path.join("world.json.swap").exists() {
        let message = "interrupted save detected (world.json.swap). Back up the folder, then restore the correct primary world.json manually".to_owned();
        return ActiveWorld { path, world: None, problem: Some(message), server_ticks: 0, physical_ticks: 0 };
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
    ActiveWorld { path, world, problem, server_ticks: 0, physical_ticks: 0 }
}

fn active(path: PathBuf) -> MutexGuard<'static, Option<ActiveWorld>> {
    let mut locked = ACTIVE.lock().unwrap_or_else(|poison| poison.into_inner());
    if locked.as_ref().is_none_or(|x| x.path != path) {
        if let Some(old) = locked.as_ref() {
            if old.problem.is_none() {
                if let Err(error) = save(old) {
                    log::error!("[civilizations] outgoing world save: {error}");
                }
            }
        }
        *locked = Some(load(path));
    }
    locked
}

fn save(active: &ActiveWorld) -> Result<(), String> {
    let world = active.world.as_ref().ok_or("world data unavailable")?;
    world.validate().map_err(|e| format!("refusing to save invalid world: {e}"))?;
    fs::create_dir_all(&active.path).map_err(|e| e.to_string())?;
    let output = serde_json::to_vec_pretty(world).map_err(|e| e.to_string())?;
    let dest = active.path.join(SAVE);
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
    let swap = active.path.join("world.json.swap");
    if swap.exists() { return Err("pending world.json.swap from interrupted save; inspect and recover before continuing".into()); }
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
        if state.problem.is_none() {
            if let Err(error) = save(state) {
                log::error!("[civilizations] shutdown save failed: {error}");
            }
        }
    }
    *guard = None;
}

pub fn register(commands: &mut Vec<NativeCommand>) {
    for usage in [
        "civ", "civ help", "civ list", "civ info", "civ info <name:word>",
        "civ roster <name:word>",
        "civ construction <name:word>",
        "civ join <name:word>", "civ title", "civ promote", "civ history",
        "civ sell <good:word> <amount:integer>", "civ buy <good:word> <amount:integer>",
    ] {
        commands.push(NativeCommand::new(usage));
    }
    for usage in [
        "civ found <name:word>", "civ actors <name:word>",
        "civ day [days:integer]", "civ construct <name:word>",
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
fn loaded_actors(game: &Game<'_>, world: &World, settlement_id: u64) -> Vec<(u64, u32)> {
    let Some(village) = world.settlements.get(&settlement_id) else { return Vec::new(); };
    if game.dim.id() != village.dimension { return Vec::new(); }
    village.residents.iter().filter_map(|r| {
        let expected = village.actor_name(r);
        // 0.1.0 actors had names without explicit resident identifiers.
        let legacy = format!("{} [{}]", r.name, village.name);
        let found = game.mobs.iter().filter(|mob| {
            mob.kind.id == "minecraft:villager"
                && mob.alive()
                && (mob.custom_name.as_deref() == Some(expected.as_str())
                    || mob.custom_name.as_deref() == Some(legacy.as_str()))
        });
        let mut matches = found.take(2);
        let first = matches.next()?;
        // Duplicate identity is ambiguous: do not command either entity.
        if matches.next().is_some() { return None; }
        let dx = first.position.x - f64::from(village.center[0]);
        let dz = first.position.z - f64::from(village.center[2]);
        (dx * dx + dz * dz < 96.0 * 96.0).then_some((r.id, first.id))
    }).collect()
}

/// Profession-aware routing in loaded chunks. Economic production remains an
/// abstract daily simulation, not a claim that this path actually harvested.
fn guide_actors(game: &mut Game<'_>, world: &World) {
    let current_dim = game.dim.id().to_owned();
    let mut targets = Vec::new();
    for village in world.settlements.values().filter(|v| v.dimension == current_dim && v.actors_spawned) {
        for (resident_id, mob_id) in loaded_actors(game, world, village.id) {
            let Some(resident) = village.residents.iter().find(|r| r.id == resident_id && r.alive) else { continue; };
            let offset = resident.job.workplace(resident_id);
            let pos = if resident.job == crate::game::civilizations_core::Job::Builder {
                if let Some(job) = &village.visual_job {
                    [f64::from(job.origin[0]) + 0.5, f64::from(job.origin[1]), f64::from(job.origin[2] - 4) + 0.5]
                } else {
                    [f64::from(village.center[0] + offset[0]) + 0.5,
                     f64::from(village.center[1] + offset[1]),
                     f64::from(village.center[2] + offset[2]) + 0.5]
                }
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

fn is_ground(name: &str) -> bool {
    matches!(name, "minecraft:grass_block" | "minecraft:dirt" | "minecraft:stone")
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

/// A physical builder must be nearby. The command path never edits blocks;
/// mutations occur on a later authoritative server tick.
fn build_one_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dim = game.dim.id().to_string();
    let selected = state.world.as_ref().and_then(|world| world.settlements.values()
        .filter(|s| s.dimension == dim)
        .find_map(|s| s.visual_job.as_ref().filter(|j| !j.blocked).map(|j| (s.id, j.clone()))));
    let Some((village_id, job)) = selected else { return Ok(()); };
    let plan = hut_plan(job.origin);
    // World JSON may be saved before Minecraft flushes a chunk. A crash can
    // leave cursor ahead of the physical state; reconcile *all* earlier steps
    // before finishing or placing the next one. No blind overwrites.
    for (index, step) in plan.iter().take(job.cursor).enumerate() {
        let Some(observed) = world_block(game, &dim, step.pos) else { return Ok(()); };
        if observed == step.block { continue; }
        let safe = if step.floor { is_ground(&observed) } else { observed == "minecraft:air" };
        let s = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
        let current = s.visual_job.as_mut().unwrap();
        if safe {
            current.cursor = index;
            save(state).map_err(|e| format!("persistence: {e}"))?;
            return Ok(());
        }
        current.blocked = true;
        save(state).map_err(|e| format!("persistence: {e}"))?;
        return Err(format!("placed building block changed at {:?}: {observed}", step.pos));
    }
    if job.cursor >= plan.len() {
        state.world.as_mut().unwrap().finish_visual_house(village_id)?;
        save(state).map_err(|e| format!("persistence: {e}"))?;
        return Ok(());
    }
    let world = state.world.as_ref().expect("active civilization world");
    let builders = loaded_actors(game, world, village_id);
    let builder_ids: Vec<u32> = world.settlements[&village_id].residents.iter()
        .filter(|r| r.alive && r.job == crate::game::civilizations_core::Job::Builder)
        .filter_map(|r| builders.iter().find(|(id, _)| *id == r.id).map(|(_, mob)| *mob))
        .collect();
    let close = builder_ids.iter().any(|mob_id| game.native_mob(&dim, *mob_id).is_some_and(|mob| {
        let dx = mob.pos[0] - f64::from(job.origin[0]);
        let dz = mob.pos[2] - f64::from(job.origin[2]);
        dx * dx + dz * dz <= 9.0 * 9.0
    }));
    if !close { return Ok(()); }
    let step = &plan[job.cursor];
    let observed = world_block(game, &dim, step.pos).ok_or("build chunk unloaded")?;
    if observed == step.block {
        let s = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
        s.visual_job.as_mut().unwrap().cursor += 1;
        save(state).map_err(|e| format!("persistence: {e}"))?;
        return Ok(());
    }
    if !(if step.floor { is_ground(&observed) } else { observed == "minecraft:air" }) {
        let s = state.world.as_mut().unwrap().settlements.get_mut(&village_id).unwrap();
        s.visual_job.as_mut().unwrap().blocked = true;
        save(state).map_err(|e| format!("persistence: {e}"))?;
        return Err(format!("building blocked at {:?}: {observed}", step.pos));
    }
    let [x, y, z] = step.pos;
    let command = format!(
        "execute in {dim} if block {x} {y} {z} {observed} run setblock {x} {y} {z} {} replace",
        step.block,
    );
    // Re-check occupancy within the authoritative Minecraft command itself.
    // Cursor is not committed until the target state is observed on a later tick.
    if !game.native_run_command(&command) {
        log::warn!("[civilizations] command refused (will retry): {command}");
    }
    Ok(())
}

pub fn perform(game: &mut Game<'_>, player: &NativeServerPlayer, command: &str, ran: &mut bool) {
    let mut args = command.split_whitespace();
    if args.next() != Some("civ") { return; }
    *ran = true;
    let Some(world_dir) = game.native_world_dir() else { error(game, "no server world open"); return; };
    let mut guard = active(world_dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if let Some(problem) = &state.problem { error(game, format!("save error: {problem}")); return; }
    let Some(world) = state.world.as_ref() else { error(game, "world is unavailable"); return; };
    let actor = format!("{:032x}", player.uuid);
    let action = args.next().unwrap_or("help");
    if action == "help" {
        reply(game, "Civilizations: /civ list | info [name] | roster <name> | construction <name> | join <name> | title | promote | history | buy/sell <good> <1..64> | (OP) found <name>, actors <name>, day [days], construct <name>, war/peace <from> <to>. Goods: wheat, bread, wood (oak logs), stone (cobble), iron.");
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
    if action == "roster" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement: /civ roster <name>"); return; };
        let linked = loaded_actors(game, world, s.id);
        let listing: Vec<_> = s.residents.iter().map(|r| format!("{}:{}", r.name, r.job.name())).collect();
        reply(game, format!("{} actors: {}/{} nearby and loaded | {}", s.name, linked.len(), s.population(), listing.join(", ")));
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
        "actors" if player.permission >= 2 => (|| {
            let name = args.next().ok_or("usage: /civ actors <name>")?;
            let s = previous.by_name(name).ok_or("unknown settlement")?;
            if s.actors_spawned {
                return Ok(format!("Actors already summoned. {}/{} are currently linked; no duplicate spawns. Use /civ roster {name}.", loaded_actors(game, &previous, s.id).len(), s.population()));
            }
            if s.dimension != player.dimension { return Err("go to that dimension first".into()); }
            let dx = i64::from(s.center[0]) - player.position[0].floor() as i64;
            let dz = i64::from(s.center[2]) - player.position[2].floor() as i64;
            if dx * dx + dz * dz > 32 * 32 { return Err("stand within 32 blocks of the settlement center".into()); }
            // Reserve the one-shot summon durably before any physical entity
            // is created. If the process crashes mid-batch we do not silently
            // mint duplicates on restart; an operator must inspect/recover.
            state.world.as_mut().unwrap().settlements.get_mut(&s.id).unwrap().actors_spawned = true;
            save(state)?;
            already_saved = true;
            let mut spawned = 0usize;
            for (i, resident) in s.residents.iter().take(10).enumerate() {
                let x = f64::from(s.center[0]) + (i as f64 % 5.0) * 2.0 - 4.0 + 0.5;
                let z = f64::from(s.center[2]) + (i / 5) as f64 * 2.0 - 1.0 + 0.5;
                let opts = NativeSpawn {
                    name: Some(s.actor_name(resident)),
                    ..NativeSpawn::default()
                };
                if game.native_spawn_mob(&s.dimension, "minecraft:villager", [x, f64::from(s.center[1]), z], &opts).is_some() {
                    spawned += 1;
                } else { break; }
            }
            if spawned != 10 {
                // Actors already summoned stay in the world. Never auto-retry
                // a partial batch, which would duplicate them.
                Ok(format!("Summoned {spawned}/10 actors; retry disabled to prevent duplicates. They use vanilla villager AI."))
            } else {
                Ok(format!("Summoned {spawned} visible villagers; village jobs are simulated, not yet linked to their animations."))
            }
        })(),
        "found" | "day" | "war" | "peace" | "actors" | "construct" => Err("requires server operator level 2".into()),
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

/// Runs after an authoritative level tick: the selected dimension and its
/// loaded chunks are guaranteed to be the ones inspected/modified here.
pub fn level_tick(game: &mut Game<'_>, dimension: &str) {
    if game.dim.id() != dimension { return; }
    let Some(dir) = game.native_world_dir() else { return; };
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if state.problem.is_some() || state.world.is_none() { return; }
    state.physical_ticks = state.physical_ticks.saturating_add(1);
    // Navigation is bounded and only touches loaded actors in the current
    // dimension; it is not an offscreen tick or a physical gathering action.
    if state.physical_ticks % 20 == 0 {
        if let Some(world) = &state.world { guide_actors(game, world); }
    }
    if state.physical_ticks % 4 == 0 {
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
}

pub fn tick(game: &mut Game<'_>) {
    let Some(dir) = game.native_world_dir() else { return; };
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if state.problem.is_some() || state.world.is_none() { return; }
    state.server_ticks = state.server_ticks.saturating_add(1);
    if state.server_ticks % TICKS_PER_DAY != 0 { return; }
    let before = state.world.as_ref().unwrap().clone();
    let result = state.world.as_mut().unwrap().advance(1).and_then(|_| save(state));
    if let Err(e) = result {
        state.world = Some(before);
        state.problem = Some(format!("automatic day halted after save failure: {e}"));
        log::error!("[civilizations] disabled automatic simulation: {e}");
    }
}
