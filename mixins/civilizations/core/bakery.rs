// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap};

/// The village has a single physical bakery, never a simulated abstract
/// building that can create bread without extracting real wheat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BakeryWorkshop {
    pub origin: [i32; 3],
    pub building: Option<VisualBuildJob>,
    pub complete: bool,
    pub supplies_reserved: bool,
    #[serde(default)]
    pub construction_escrow: BTreeMap<Good, u32>,
    /// Plain native wheat physically taken from the hall's barrels.
    #[serde(default)]
    pub wheat_escrow: u32,
    /// Newly crafted bread carried by a UUID-bound baker returning home.
    #[serde(default)]
    pub bread_in_transit: u32,
    #[serde(default)]
    pub batches: u64,
}
