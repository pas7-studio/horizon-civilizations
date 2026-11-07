// Horizon Civilizations domain module, re-exported by core/mod.rs.
use serde::{Deserialize, Serialize};

/// Mod-level ownership of a 5x5 plot; not a vanilla/other-mod block-protection
/// system. Minecraft block preflight still checks for player structures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerParcel {
    pub origin: [i32; 3],
    pub owner: String, // authenticated acting player's UUID in hex
    /// True for a private house ordered on a bought plot OR an existing
    /// village hut purchased with its completed title.
    #[serde(default)]
    pub commissioned: bool,
}

pub fn near_plot(a: [i32; 3], b: [i32; 3]) -> bool {
    let dx = i128::from(a[0]) - i128::from(b[0]);
    let dz = i128::from(a[2]) - i128::from(b[2]);
    dx * dx + dz * dz < 9_i128 * 9
}

/// One block of an intentionally small starter hut; only a safe, flat, empty
/// plot can be claimed. Placement is done by the Horizon adapter, never here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HutBlock {
    pub pos: [i32; 3],
    pub block: &'static str,
    pub floor: bool,
}

pub fn hut_plan(origin: [i32; 3]) -> Vec<HutBlock> {
    let mut plan = Vec::new();
    // Cobblestone foundation. The player selects a level, natural surface.
    for z in -2..=2 {
        for x in -2..=2 {
            plan.push(HutBlock {
                pos: [origin[0] + x, origin[1] - 1, origin[2] + z],
                block: "minecraft:cobblestone",
                floor: true,
            });
        }
    }
    for y in 0..=2 {
        for z in -2i32..=2 {
            for x in -2i32..=2 {
                if x.abs() != 2 && z.abs() != 2 { continue; }
                // Two-block high doorway at the front.
                if x == 0 && z == -2 && y <= 1 { continue; }
                let window = y == 1 && ((z == 0 && x.abs() == 2) || (x == 0 && z == 2));
                plan.push(HutBlock {
                    pos: [origin[0] + x, origin[1] + y, origin[2] + z],
                    block: if window { "minecraft:glass" } else { "minecraft:oak_planks" },
                    floor: false,
                });
            }
        }
    }
    for z in -2..=2 {
        for x in -2..=2 {
            plan.push(HutBlock {
                pos: [origin[0] + x, origin[1] + 3, origin[2] + z],
                block: "minecraft:oak_planks",
                floor: false,
            });
        }
    }
    plan
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualBuildJob {
    pub origin: [i32; 3],
    pub cursor: usize,
    #[serde(default)]
    pub blocked: bool,
}

/// Pure, non-mutating decision after inspecting the actual loaded world blocks.
/// Horizon performs writes separately, and confirms each result on a later tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildInspection {
    Unloaded,
    Paused,
    Rewind(usize),
    Blocked { position: [i32; 3], found: String },
    InvalidCursor,
    Advance,
    Complete,
    Place { position: [i32; 3], old_block: String, new_block: &'static str },
}

/// Natural solid soil a civic floor may be laid on. Exotic grounds such as
/// snow or gravel are real terrain in snowy and river villages, not player
/// blocks, so refusing them would strand those settlements forever.
pub fn is_ground(name: &str) -> bool {
    matches!(name,
        "minecraft:grass_block" | "minecraft:dirt" | "minecraft:stone"
        | "minecraft:coarse_dirt" | "minecraft:podzol" | "minecraft:rooted_dirt"
        | "minecraft:sand" | "minecraft:red_sand" | "minecraft:gravel"
        | "minecraft:snow_block" | "minecraft:andesite" | "minecraft:deepslate")
}

/// Check every previously committed step; the save file and chunk data are
/// not atomic, so a committed cursor does not prove the blocks still exist.
pub fn inspect_hut(job: &VisualBuildJob, mut read: impl FnMut([i32; 3]) -> Option<String>) -> BuildInspection {
    if job.blocked {
        return BuildInspection::Paused;
    }
    let plan = hut_plan(job.origin);
    if job.cursor > plan.len() {
        return BuildInspection::InvalidCursor;
    }
    for (index, step) in plan.iter().enumerate() {
        if index > job.cursor {
            break;
        }
        let Some(actual) = read(step.pos) else {
            return BuildInspection::Unloaded;
        };
        if actual == step.block {
            if index == job.cursor {
                return BuildInspection::Advance;
            }
            continue;
        }
        let may_replace = if step.floor { is_ground(&actual) } else { actual == "minecraft:air" };
        if !may_replace {
            return BuildInspection::Blocked { position: step.pos, found: actual };
        }
        if index < job.cursor {
            return BuildInspection::Rewind(index);
        }
        return BuildInspection::Place {
            position: step.pos,
            old_block: actual,
            new_block: step.block,
        };
    }
    BuildInspection::Complete
}
