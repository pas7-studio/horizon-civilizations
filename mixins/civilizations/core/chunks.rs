// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use std::collections::{BTreeSet};

/// The exact chunks a village needs loaded to keep building and gathering
/// while the player is away. Deterministic, sorted and bounded: the civic
/// centre block plus one chunk per workplace and per corridor step. If that
/// exceeds the budget, corridors are dropped first, then everything but the
/// civic block.
pub fn village_chunk_footprint(center: [i32; 3], sites: &[[i32; 3]]) -> Vec<[i32; 2]> {
    let chunk_of = |block: [i32; 3]| [block[0].div_euclid(16), block[2].div_euclid(16)];
    let hall = chunk_of(center);
    let mut core_set = BTreeSet::new();
    for dz in -HALL_CHUNK_RADIUS..=HALL_CHUNK_RADIUS {
        for dx in -HALL_CHUNK_RADIUS..=HALL_CHUNK_RADIUS {
            core_set.insert([hall[0] + dx, hall[1] + dz]);
        }
    }
    let mut full = core_set.clone();
    for site in sites {
        full.insert(chunk_of(*site));
        // Sample the walking line every four blocks: with the engine's
        // neighbour ring that keeps the whole corridor loaded, without
        // holding a square block of countryside nobody uses.
        let dx = site[0].saturating_sub(center[0]);
        let dz = site[2].saturating_sub(center[2]);
        let steps = (dx.abs().max(dz.abs()) / 4).max(1);
        for step in 0..=steps {
            let x = center[0].saturating_add(dx.saturating_mul(step) / steps);
            let z = center[2].saturating_add(dz.saturating_mul(step) / steps);
            full.insert([x.div_euclid(16), z.div_euclid(16)]);
        }
    }
    let chosen = if full.len() <= MAX_HELD_VILLAGE_CHUNKS { full } else { core_set.clone() };
    let chosen = if chosen.len() <= MAX_HELD_VILLAGE_CHUNKS {
        chosen
    } else {
        BTreeSet::from([hall])
    };
    chosen.into_iter().collect()
}
