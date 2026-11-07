// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

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
pub(super) fn civic_sign_at(game: &Game<'_>, position: [i32; 3]) -> bool {
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
