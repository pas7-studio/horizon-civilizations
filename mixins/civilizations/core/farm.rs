// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmStage {
    Preparing,
    Growing,
    Harvesting,
    Replanting,
}

/// Evidence that all eight crops were observed as physically harvested.
/// This does NOT authorize any warehouse stock, player items or coin credit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarvestReceipt {
    pub sequence: u64,
    pub world_day: u64,
    pub crops: u8,
}

/// One operator-approved field (8 crops around a permanent center water block).
/// Harvest is visual-only: Minecraft blocks and simulation stock are distinct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FarmPlot {
    pub origin: [i32; 3],
    pub stage: FarmStage,
    pub cursor: usize,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub completed_cycles: u32,
    /// Persistent lifetime counter: older receipt entries may be compacted.
    #[serde(default)]
    pub harvest_sequence: u64,
    #[serde(default)]
    pub harvest_receipts: Vec<HarvestReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FarmStep {
    pub pos: [i32; 3],
    pub block: &'static str,
    pub floor: bool,
}

pub fn farm_plan(origin: [i32; 3]) -> Vec<FarmStep> {
    let mut steps = Vec::with_capacity(17);
    // Water at soil level keeps the eight neighboring farmland blocks hydrated.
    for z in -1..=1 {
        for x in -1..=1 {
            if x == 0 && z == 0 { continue; }
            steps.push(FarmStep {
                pos: [origin[0] + x, origin[1] - 1, origin[2] + z],
                block: "minecraft:farmland",
                floor: true,
            });
        }
    }
    steps.push(FarmStep {
        pos: [origin[0], origin[1] - 1, origin[2]],
        block: "minecraft:water",
        floor: true,
    });
    for z in -1..=1 {
        for x in -1..=1 {
            if x == 0 && z == 0 { continue; }
            steps.push(FarmStep {
                pos: [origin[0] + x, origin[1], origin[2] + z],
                block: "minecraft:wheat",
                floor: false,
            });
        }
    }
    steps
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FarmBlock {
    pub name: String,
    pub age: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FarmDecision {
    Unloaded,
    Paused,
    InvalidCursor,
    Blocked { position: [i32; 3], found: String },
    Rewind(usize),
    Advance,
    StageComplete,
    Place { position: [i32; 3], old_block: String, new_block: &'static str },
}

/// Pure observation-based state machine. No player items are granted and no
/// simulated resources are credited: block storage and the economy JSON are
/// not a single crash-atomic transaction.
pub fn inspect_farm(plot: &FarmPlot, mut read: impl FnMut([i32; 3]) -> Option<FarmBlock>) -> FarmDecision {
    if plot.blocked { return FarmDecision::Paused; }
    let plan = farm_plan(plot.origin);
    let count = match plot.stage {
        FarmStage::Preparing => plan.len(),
        _ => 8,
    };
    if plot.cursor > count { return FarmDecision::InvalidCursor; }
    if plot.stage != FarmStage::Preparing {
        // Never harvest or replant when the water/soil infrastructure changed.
        for step in plan.iter().take(9) {
            let Some(block) = read(step.pos) else { return FarmDecision::Unloaded; };
            if block.name != step.block {
                return FarmDecision::Blocked { position: step.pos, found: block.name };
            }
        }
    }
    if plot.stage == FarmStage::Growing {
        let mut all_mature = true;
        for step in plan.iter().skip(9) {
            let Some(block) = read(step.pos) else { return FarmDecision::Unloaded; };
            if block.name != "minecraft:wheat" {
                return FarmDecision::Blocked { position: step.pos, found: block.name };
            }
            all_mature &= block.age == Some(7);
        }
        return if all_mature { FarmDecision::StageComplete } else { FarmDecision::Paused };
    }
    if plot.stage == FarmStage::Preparing {
        for (i, step) in plan.iter().enumerate() {
            if i > plot.cursor { break; }
            let Some(block) = read(step.pos) else { return FarmDecision::Unloaded; };
            if block.name == step.block {
                if i == plot.cursor { return FarmDecision::Advance; }
                continue;
            }
            let safe = if step.floor { is_ground(&block.name) } else { block.name == "minecraft:air" };
            if !safe {
                return FarmDecision::Blocked { position: step.pos, found: block.name };
            }
            if i < plot.cursor { return FarmDecision::Rewind(i); }
            return FarmDecision::Place {
                position: step.pos, old_block: block.name, new_block: step.block,
            };
        }
        return FarmDecision::StageComplete;
    }
    for (i, step) in plan.iter().skip(9).enumerate() {
        if i > plot.cursor { break; }
        let Some(block) = read(step.pos) else { return FarmDecision::Unloaded; };
        let done = match plot.stage {
            FarmStage::Harvesting => block.name == "minecraft:air",
            FarmStage::Replanting => block.name == "minecraft:wheat",
            _ => unreachable!("handled above"),
        };
        if done {
            if i == plot.cursor { return FarmDecision::Advance; }
            continue;
        }
        let valid = match plot.stage {
            FarmStage::Harvesting => block.name == "minecraft:wheat" && block.age == Some(7),
            FarmStage::Replanting => block.name == "minecraft:air",
            _ => unreachable!("handled above"),
        };
        if !valid {
            return FarmDecision::Blocked { position: step.pos, found: block.name };
        }
        if i < plot.cursor { return FarmDecision::Rewind(i); }
        let (old_block, new_block) = match plot.stage {
            FarmStage::Harvesting => ("minecraft:wheat[age=7]", "minecraft:air"),
            FarmStage::Replanting => ("minecraft:air", "minecraft:wheat"),
            _ => unreachable!("handled above"),
        };
        return FarmDecision::Place {
            position: step.pos, old_block: old_block.to_owned(), new_block,
        };
    }
    FarmDecision::StageComplete
}
