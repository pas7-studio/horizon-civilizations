// Horizon bridge. All world logic lives in civilizations_core; this module
// translates server hooks, commands, inventory changes, and native NPC spawns.
// Split into focused modules; tools/mixin_sources.py flattens it for the
// Horizon mixin tool, which only accepts self-contained payloads.

use crate::game::{
    civilizations_core::{farm_plan, hut_plan, town_hall_plan, bakery_plan, hall_storage_positions,
        BarrelDirection, BarrelPhase, inspect_farm, inspect_hut, inspect_town_hall, inspect_bakery, is_ground,
        is_rooted_oak_harvest_origin, short_oak_stem_height, inspect_oak_stem,
        native_worker_near, native_worker_reaches_block, PostKind, POST_RADIUS, MAX_QUARRY_STEPS, gather_turn_index,
        BuildInspection, DailyPhase, FarmBlock, FarmDecision, FarmStage, Good, HomeSlot, Job, VisualBuildJob,
        World, inspect_home, HOUSING_PLOT_OFFSETS, MAX_HOUSING_SURVEY_RESULTS,
        MAX_SIM_DAYS},
    native_mobs::NativeSpawn,
    native_server::{NativeCommand, NativeServerPlayer},
    Game,
};
use horizon_client::text::{self, Text};
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, Tag};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use crate::game::civilizations_core::{
    build_dialog, dialog_snbt, good_label, parse_sale_args, resident_wants, DialogueInput, Resident,
    Settlement, TalkPage,
};

const MOD_ID: &str = "civilizations";
const DIR: &str = "horizon-civilizations";
const SAVE: &str = "world.json";

mod actions;
mod actors;
mod boards;
mod build;
mod chunks;
mod commands;
mod dialogue;
mod economy;
mod gather;
mod hooks;
mod interact;
mod logistics;
mod protect;
mod reports;
mod shop;
mod store;

#[cfg(test)]
mod interrupted_first_save_tests;


use actions::*;
use actors::*;
use boards::*;
use build::*;
use chunks::*;
use commands::*;
use dialogue::*;
use economy::*;
use gather::*;
use interact::*;
use logistics::*;
use reports::*;
use shop::*;
use store::*;

pub use commands::{perform, register};
pub use hooks::{level_tick, tick};
pub use interact::{interact_entity};
pub use protect::{protect_civilization_block, protect_hall_inventory, protect_public_hopper, protect_civic_sign_click, protect_civic_sign_edit};
pub use store::{closed};
