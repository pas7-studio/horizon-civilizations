// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

pub(super) fn join_village(game: &Game<'_>, uuid: u128, village_id: u64) -> Result<String, String> {
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
