// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Job {
    Farmer,
    Baker,
    Lumberjack,
    Miner,
    Builder,
    Guard,
    Merchant,
    /// No workplace currently needs this resident. A laborer is a real
    /// role: they are not claimed to farm, bake or mine anything.
    Laborer,
    /// Digs sand and dirt at a pit camp. Kept apart from the miner so a
    /// quarry and a sand pit never compete for the same worker seat.
    Digger,
}

/// A persistent work camp. Gathering is restricted to its radius so a
/// villager has a real, visible workplace instead of roaming the whole map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostKind {
    Lumber,
    Quarry,
    Sand,
    Dirt,
}

impl PostKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lumber => "lumber",
            Self::Quarry => "quarry",
            Self::Sand => "sand",
            Self::Dirt => "dirt",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "lumber" | "lumberjack" | "wood" => Some(Self::Lumber),
            "quarry" | "stone" | "mine" => Some(Self::Quarry),
            "sand" | "sandpit" => Some(Self::Sand),
            "dirt" | "earth" | "soil" | "dig" => Some(Self::Dirt),
            _ => None,
        }
    }
    pub fn for_good(good: Good) -> Self {
        match good {
            Good::Stone => Self::Quarry,
            Good::Sand => Self::Sand,
            Good::Dirt => Self::Dirt,
            _ => Self::Lumber,
        }
    }
    pub fn good(self) -> Good {
        match self {
            Self::Lumber => Good::Wood,
            Self::Quarry => Good::Stone,
            Self::Sand => Good::Sand,
            Self::Dirt => Good::Dirt,
        }
    }
    pub fn job(self) -> Job {
        match self {
            Self::Lumber => Job::Lumberjack,
            Self::Quarry => Job::Miner,
            Self::Sand | Self::Dirt => Job::Digger,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lumber => "LUMBER CAMP",
            Self::Quarry => "QUARRY",
            Self::Sand => "SAND PIT",
            Self::Dirt => "DIRT PIT",
        }
    }
}

/// A camp is one saved site with an optional assigned resident. It is a
/// site, not an inventory: goods still arrive only through real receipts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPost {
    pub kind: PostKind,
    pub origin: [i32; 3],
    /// Stable citizen ID assigned so the same worker services this camp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<u64>,
}

/// Physical actor presence is an authoritative 3D observation, not an
/// X/Z-only check. A villager in a cave below the hall is not a courier.
/// The target is the native block coordinate (centered in X/Z), while the
/// entity Y is its feet position. No unseen actor or nonfinite coordinate
/// can authorize a physical resource transaction.
pub fn native_worker_near(
    actor: [f64; 3], target: [i32; 3], horizontal_radius: f64, vertical_radius: f64,
) -> bool {
    if !actor.iter().all(|v| v.is_finite())
        || !horizontal_radius.is_finite() || horizontal_radius < 0.0
        || !vertical_radius.is_finite() || vertical_radius < 0.0 {
        return false;
    }
    let dx = actor[0] - (f64::from(target[0]) + 0.5);
    let dy = actor[1] - f64::from(target[1]);
    let dz = actor[2] - (f64::from(target[2]) + 0.5);
    dy.abs() <= vertical_radius && dx * dx + dz * dz <= horizontal_radius * horizontal_radius
}

/// A native worker may only harvest a block inside a plausible 3D reach
/// sphere, measured from eye height (feet + 1.5). This prevents a worker
/// standing at the tree's root from felling a remote or elevated log merely
/// because the horizontal X/Z distance is small.
pub fn native_worker_reaches_block(
    actor: [f64; 3], block: [i32; 3], reach: f64,
) -> bool {
    if !actor.iter().all(|v| v.is_finite()) || !reach.is_finite() || reach < 0.0 {
        return false;
    }
    let dx = actor[0] - (f64::from(block[0]) + 0.5);
    let dy = actor[1] + 1.5 - (f64::from(block[1]) + 0.5);
    let dz = actor[2] - (f64::from(block[2]) + 0.5);
    dx * dx + dy * dy + dz * dz <= reach * reach
}

impl Job {
    pub fn name(self) -> &'static str {
        match self {
            Self::Farmer => "farmer",
            Self::Baker => "baker",
            Self::Lumberjack => "lumberjack",
            Self::Miner => "miner",
            Self::Builder => "builder",
            Self::Guard => "guard",
            Self::Merchant => "merchant",
            Self::Laborer => "laborer",
            Self::Digger => "digger",
        }
    }

    /// Relative work site, not a claim that the NPC physically gathered resources.
    /// Small stable offsets keep actor paths within the active settlement.
    pub fn workplace(self, resident_id: u64) -> [i32; 3] {
        let variation = (resident_id % 3) as i32 * 3;
        match self {
            Self::Farmer => [8 + variation, 0, 8],
            Self::Baker => [-4, 0, 5],
            Self::Lumberjack => [-10 - variation, 0, 7],
            Self::Miner => [-8, 0, -10],
            Self::Builder => [8, 0, -8],
            Self::Guard => [0, 0, -8],
            Self::Merchant => [0, 0, 3],
            Self::Laborer => [2, 0, 4],
            Self::Digger => [-6, 0, -2],
        }
    }
}

/// Deterministic movement windows derived from Horizon's persistent clock.
/// Navigation-only: this does not create beds, stock, or saved-state fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DailyPhase {
    Dawn,
    Work,
    Evening,
    Night,
}

impl DailyPhase {
    pub fn at(day_time: i64) -> Self {
        match day_time.rem_euclid(24_000) {
            0..=1_999 => Self::Dawn,
            2_000..=10_999 => Self::Work,
            11_000..=12_999 => Self::Evening,
            _ => Self::Night,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Dawn => "dawn / gathering",
            Self::Work => "work shift",
            Self::Evening => "evening / commons",
            Self::Night => "night / rest",
        }
    }

    /// Guards stay on watch; all other professions work in daylight only.
    pub fn on_shift(self, job: Job) -> bool {
        matches!(job, Job::Guard) || self == Self::Work
    }

    /// Shared near-center waypoints; never targets an unbuilt bed or home.
    /// Deterministic by resident ID, with no extra persistence requirements.
    pub fn offset(self, job: Job, resident_id: u64) -> [i32; 3] {
        if self.on_shift(job) {
            return job.workplace(resident_id);
        }
        let lane = (resident_id % 5) as i32 - 2;
        let depth = ((resident_id / 5) % 3) as i32;
        match self {
            Self::Dawn => [lane * 2, 0, 2 + depth],
            Self::Work => job.workplace(resident_id),
            Self::Evening => [lane * 2, 0, depth],
            Self::Night => [lane * 2, 0, -3 - depth],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rank {
    Visitor,
    Citizen,
    Freeman,
    Knight,
    Lord,
}

impl Rank {
    pub fn name(self) -> &'static str {
        match self {
            Self::Visitor => "visitor",
            Self::Citizen => "citizen",
            Self::Freeman => "freeman",
            Self::Knight => "knight",
            Self::Lord => "lord",
        }
    }
}
