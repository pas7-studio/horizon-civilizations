// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrelDirection {
    /// Saved, verified transit goods become one physical barrel item.
    ToBarrel,
    /// One physical barrel item becomes protected build escrow.
    ToBuilder,
    /// A builder collects the physical bakery's full materials.
    ToBakeryBuilder,
    /// A baker picks up real wheat for one or more vanilla bread recipes.
    ToBakerIngredients,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrelPhase {
    Planned,
    /// Persisted before native mutation: never automatically issue again.
    Issued,
    /// Interference/uncertain native result: OP inspection required.
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarrelTransfer {
    pub position: [i32; 3],
    pub good: Good,
    pub direction: BarrelDirection,
    pub phase: BarrelPhase,
    /// Exact count of this plain item in this specific barrel before issue.
    pub before: u32,
    /// One bounded batch per durable journal (at most 8 native items).
    pub count: u32,
    /// Exact native container slot (required for ToBuilder).
    pub slot: Option<usize>,
}

impl BarrelTransfer {
    pub fn expected_after(&self) -> Option<u32> {
        match self.direction {
            BarrelDirection::ToBarrel => self.before.checked_add(self.count),
            BarrelDirection::ToBuilder
            | BarrelDirection::ToBakeryBuilder
            | BarrelDirection::ToBakerIngredients => self.before.checked_sub(self.count),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketPhase {
    Issued, // coins committed before native extraction; never auto-retry
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketPurchase {
    pub buyer: String,
    pub good: Good,
    pub count: u32,
    pub price: u64,
    pub position: [i32; 3],
    pub slot: usize,
    pub before: u32,
    pub phase: MarketPhase,
}
