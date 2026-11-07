// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;

pub fn town_hall_plan(origin: [i32; 3]) -> Vec<HutBlock> {
    let mut plan = hut_plan(origin);
    for (dx, dz, block) in [
        (-1, 1, "minecraft:barrel"),
        (1, 1, "minecraft:barrel"),
        (0, 1, "minecraft:lectern"),
    ] {
        plan.push(HutBlock {
            pos: [origin[0] + dx, origin[1], origin[2] + dz],
            block, floor: false,
        });
    }
    // Two readable wall signs flank the front door on the OUTSIDE of the
    // north wall, at eye height. Standing signs on the floor were unreadable
    // from outside and occupied the hall's own interior.
    for dx in [-1, 1] {
        plan.push(HutBlock {
            pos: [origin[0] + dx, origin[1] + 1, origin[2] - 3],
            block: "minecraft:oak_wall_sign",
            floor: false,
        });
    }
    plan
}

/// Physical independent workshop: walls and floor like a village hut,
/// with an actual crafting table and smoker. Bread is crafted from three
/// real wheat items at the table; a smoker is a visible workplace detail.
pub fn bakery_plan(origin: [i32; 3]) -> Vec<HutBlock> {
    let mut plan = hut_plan(origin);
    for (dx, dz, block) in [
        (-1, 0, "minecraft:crafting_table"),
        (1, 0, "minecraft:smoker"),
    ] {
        plan.push(HutBlock {
            pos: [origin[0] + dx, origin[1], origin[2] + dz],
            block, floor: false,
        });
    }
    plan
}

/// The actual vanilla barrel blocks inside the public town hall, not
/// surrogate entries in the settlement's JSON. Never allow player parcels
/// or unverified nearby chests to substitute for these positions.
pub fn hall_storage_positions(origin: [i32; 3]) -> [[i32; 3]; 2] {
    [[origin[0] - 1, origin[1], origin[2] + 1],
     [origin[0] + 1, origin[1], origin[2] + 1]]
}

/// Same fail-closed crash reconciliation as ordinary huts, but with
/// the distinct civic center blueprint and actual warehouse blocks.
pub fn inspect_town_hall(
    job: &VisualBuildJob, mut read: impl FnMut([i32; 3]) -> Option<String>
) -> BuildInspection {
    if job.blocked { return BuildInspection::Paused; }
    let plan = town_hall_plan(job.origin);
    if job.cursor > plan.len() { return BuildInspection::InvalidCursor; }
    for (index, step) in plan.iter().enumerate() {
        if index > job.cursor { break; }
        let Some(actual) = read(step.pos) else { return BuildInspection::Unloaded; };
        if actual == step.block {
            if index == job.cursor { return BuildInspection::Advance; }
            continue;
        }
        let replaceable = if step.floor { is_ground(&actual) } else { actual == "minecraft:air" };
        if !replaceable { return BuildInspection::Blocked { position: step.pos, found: actual }; }
        if index < job.cursor { return BuildInspection::Rewind(index); }
        return BuildInspection::Place {
            position: step.pos, old_block: actual, new_block: step.block,
        };
    }
    BuildInspection::Complete
}

/// Check actual vanilla workshop blocks before advancing each saved step.
pub fn inspect_bakery(
    job: &VisualBuildJob, mut read: impl FnMut([i32; 3]) -> Option<String>
) -> BuildInspection {
    if job.blocked { return BuildInspection::Paused; }
    let plan = bakery_plan(job.origin);
    if job.cursor > plan.len() { return BuildInspection::InvalidCursor; }
    for (index, step) in plan.iter().enumerate() {
        if index > job.cursor { break; }
        let Some(actual) = read(step.pos) else { return BuildInspection::Unloaded; };
        if actual == step.block {
            if index == job.cursor { return BuildInspection::Advance; }
            continue;
        }
        let replaceable = if step.floor { is_ground(&actual) } else { actual == "minecraft:air" };
        if !replaceable { return BuildInspection::Blocked {position: step.pos, found: actual}; }
        if index < job.cursor { return BuildInspection::Rewind(index); }
        return BuildInspection::Place {
            position: step.pos, old_block: actual, new_block: step.block,
        };
    }
    BuildInspection::Complete
}
