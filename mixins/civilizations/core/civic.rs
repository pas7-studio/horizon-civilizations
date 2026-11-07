// Horizon Civilizations domain module, re-exported by core/mod.rs.

/// Read-only, deterministic village needs; ordering is a tie-breaker, not a
/// claim about a physical AI system or an authority to spend player resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CivicNeed {
    Food,
    Security,
    Stability,
    Identity,
    Housing,
    Materials,
}

impl CivicNeed {
    pub fn label(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Security => "security",
            Self::Stability => "stability",
            Self::Identity => "resident identity",
            Self::Housing => "housing",
            Self::Materials => "materials",
        }
    }
}

/// A council recommendation, never a ledger transaction or executable order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CivicPriority {
    pub need: CivicNeed,
    pub urgency: u8,
    pub reason: String,
    pub next_step: String,
}

/// Snapshot of the EXISTING abstract settlement day() method run on a clone.
/// Deliberately excludes separate world-level war attrition, player trades,
/// physical crop harvests, inventory changes and actual NPC movement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbstractDayForecast {
    pub offset: u8,
    pub wheat: u32,
    pub bread: u32,
    pub morale: i32,
    pub starving_days: u32,
    pub houses: u32,
    pub pending_visual_buildings: u32,
}
