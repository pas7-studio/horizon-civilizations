// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// Places a villager must reach while keeping the village going. The civic
/// centre itself is implicit; these are the real workplaces and homes.
pub(super) fn settlement_work_sites(
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
pub(super) fn hold_village_chunks(game: &mut Game<'_>, world: &World) {
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
