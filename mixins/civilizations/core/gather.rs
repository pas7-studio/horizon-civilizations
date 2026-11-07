// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};

/// Choose one loaded, active village gather per native gather tick. The
/// bridge calls this once every 8 authoritative level ticks, so the
/// division advances the round-robin exactly once per opportunity.
/// Count=0 yields no turn (and never divides by zero). No saved cursor
/// or scheduler state can become stale across server restarts.
pub fn gather_turn_index(physical_ticks: u64, eligible_count: usize) -> Option<usize> {
    if eligible_count == 0 { return None; }
    Some(((physical_ticks / 8) % eligible_count as u64) as usize)
}

/// One pending real-resource extraction. Persist the plan before the block
/// command; issue state before granting any Minecraft-derived warehouse item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatherJob {
    pub position: [i32; 3],
    /// Verified empty two-high destination a worker can navigate toward.
    pub stand: [i32; 3],
    pub good: Good, // only Wood (oak log) or Stone (natural stone)
    /// Stable citizen ID assigned durably before any native resource edit.
    /// Old saves lack this field: an issued unbound receipt is quarantined,
    /// never credited by whichever worker happens to stand at the hall.
    #[serde(default)]
    pub worker_id: Option<u64>,
    pub issued: bool,
    /// Durable acknowledgement only after a successful native command AND
    /// an immediate loaded observation of the edited block as air. A crash
    /// between Issued persistence and this receipt is irreducibly ambiguous:
    /// block/quarantine it rather than crediting a player-removed block.
    #[serde(default)]
    pub native_edit_confirmed: bool,
    /// An actual loaded UUID-linked worker has observed the source block
    /// as air at the original saved work stand AFTER issue. Only then may
    /// their native route turn toward the village receiving area.
    /// Old saves default to false and cannot auto-deliver based on mere air.
    #[serde(default)]
    pub pickup_witnessed: bool,
    #[serde(default)]
    pub blocked: bool,
}

/// A short oak stem is harvested one real block and one UUID-worker
/// delivery at a time. The cursor tracks the NEXT block, not free wood.
/// Existing saves without this field keep their ordinary one-block jobs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OakStem {
    pub root: [i32; 3],
    pub stand: [i32; 3],
    /// Short, reachable stems only. A larger tree requires real scaffolding.
    pub height: u8,
    pub cursor: u8,
}

/// The native adapter first verifies a rooted leaf-bearing oak, then measures
/// the whole short stem. Trees taller than four logs are deliberately skipped:
/// our worker currently has no physical way to reach a taller canopy.
pub fn short_oak_stem_height(
    root: [i32; 3], mut read: impl FnMut([i32; 3]) -> Option<String>,
) -> Option<u8> {
    if !is_rooted_oak_harvest_origin(root, |pos| read(pos)) { return None; }
    let mut count = 0_u8;
    for offset in 0..=4_i32 {
        let y = root[1].checked_add(offset)?;
        match read([root[0], y, root[2]]).as_deref() {
            Some("minecraft:oak_log") if offset < 4 => count += 1,
            Some("minecraft:oak_log") => return None,
            Some(_) => break,
            None => return None,
        }
    }
    (3..=4).contains(&count).then_some(count)
}

/// Observe every step of the SAME saved stem before dispatching its next
/// world edit. Previous trunk logs must already be air, remaining ones
/// unchanged vanilla oak. The upper crown must still exist and all reads
/// must be loaded. No independent search for a second, possibly foreign tree.
pub fn inspect_oak_stem(
    stem: &OakStem, mut read: impl FnMut([i32; 3]) -> Option<String>,
) -> bool {
    if !(3..=4).contains(&stem.height) || stem.cursor >= stem.height {
        return false;
    }
    // On the first block this is still an actual rooted tree, not a stored
    // journal claiming that an externally placed timber pole is natural.
    if stem.cursor == 0
        && !is_rooted_oak_harvest_origin(stem.root, |pos| read(pos)) {
        return false;
    }
    for offset in 0..stem.height {
        let Some(y) = stem.root[1].checked_add(i32::from(offset)) else {
            return false;
        };
        let expected = if offset < stem.cursor { "minecraft:air" }
            else { "minecraft:oak_log" };
        if read([stem.root[0], y, stem.root[2]]).as_deref() != Some(expected) {
            return false;
        }
    }
    let Some(top_y) = stem.root[1].checked_add(i32::from(stem.height) - 1)
    else { return false; };
    let mut crown = false;
    for (dx, dz) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
        let (Some(x), Some(z)) =
            (stem.root[0].checked_add(dx), stem.root[2].checked_add(dz))
        else { return false; };
        let Some(block) = read([x, top_y, z]) else { return false; };
        if block == "minecraft:oak_leaves" { crown = true; }
    }
    crown
}

impl GatherJob {
    pub fn source_block(&self) -> &'static str {
        match self.good {
            Good::Wood => "minecraft:oak_log",
            Good::Stone => "minecraft:stone",
            Good::Sand => "minecraft:sand",
            Good::Dirt => "minecraft:dirt",
            _ => "",
        }
    }
}

/// Verify a *rooted*, leaf-bearing oak trunk before issuing any native
/// lumberjack destruction. A leaf next to an arbitrary isolated oak log is
/// not evidence of a natural tree: a genuine candidate must begin on real
/// soil, contain at least three vertically contiguous logs, and reach a
/// leaf-bearing upper crown. Only the lowest rooted log qualifies as the
/// first resource; upper floating/player-placed logs are never selected.
/// The caller supplies loaded, observed vanilla block states and must also
/// enforce native claim protection and a reachable worker stand.
/// This is conservative physical evidence, NOT provenance against a player
/// who deliberately plants a real tree outside registered claims.
pub fn is_rooted_oak_harvest_origin(
    origin: [i32; 3], mut read: impl FnMut([i32; 3]) -> Option<String>,
) -> bool {
    let [x, y, z] = origin;
    if read(origin).as_deref() != Some("minecraft:oak_log") {
        return false;
    }
    let Some(soil_y) = y.checked_sub(1) else { return false; };
    if !matches!(
        read([x, soil_y, z]).as_deref(),
        Some("minecraft:grass_block" | "minecraft:dirt" | "minecraft:podzol" |
             "minecraft:coarse_dirt" | "minecraft:rooted_dirt")
    ) {
        return false;
    }
    let mut upper_logs = 0_usize;
    let mut crown = false;
    // A bounded search avoids scanning remote/unloaded chunks. We deliberately
    // reject taller/uncertain trees until supported by an explicit tree claim.
    for step in 1..=8_i32 {
        let Some(top_y) = y.checked_add(step) else { return false; };
        match read([x, top_y, z]).as_deref() {
            Some("minecraft:oak_log") => {}
            Some(_) => break,
            None => return false,
        }
        upper_logs += 1;
        if step < 2 { continue; }
        for (dx, dz) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
            let (Some(tx), Some(tz)) = (x.checked_add(dx), z.checked_add(dz))
            else { return false; };
            let Some(leaf) = read([tx, top_y, tz]) else {
                return false; // unloaded evidence cannot approve extraction
            };
            if leaf == "minecraft:oak_leaves" { crown = true; }
        }
    }
    upper_logs >= 2 && crown
}
