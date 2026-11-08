// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// Last text written to each hall board, so an unchanged board costs no
/// command. Keyed by dimension and position; a missing sign block always
/// forces a rewrite regardless of this cache.
pub(super) static BOARD_CACHE: Mutex<BTreeMap<(String, [i32; 3]), String>> = Mutex::new(BTreeMap::new());

/// SNBT for one sign face. `rich` colors each line; the plain form is the
/// fallback for game versions whose sign NBT rejects text components.
pub(super) fn sign_data_command(
    dimension: &str, pos: [i32; 3], spec: &crate::game::civilizations_core::SignSpec, rich: bool,
) -> String {
    sign_data_command_for("minecraft:oak_wall_sign", dimension, pos, spec, rich)
}

/// Same as `sign_data_command` for any sign block (wall or standing).
pub(super) fn sign_data_command_for(
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
pub(super) fn refresh_site_boards(game: &mut Game<'_>, world: &World) {
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
pub(super) fn refresh_hall_boards(game: &mut Game<'_>, world: &World) {
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

/// Derive this village's roles from the workplaces it really has, persist one
/// change, and keep the visible name tags in step. Called before the first
/// spawn so no villager is ever labelled with a job the village cannot
/// provide. Only saved state and an entity name change here, never a block.
pub(super) fn staff_village(game: &mut Game<'_>, state: &mut ActiveWorld, id: u64) -> Result<(), String> {
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
pub(super) fn refresh_post_boards(game: &mut Game<'_>, world: &World) {
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
            // two blocks aside so it can never block the pit's walkable lane.
            let [sx, sy, sz] = [x + 1, y, z + 2];
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
