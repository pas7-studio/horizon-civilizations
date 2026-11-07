// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

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
