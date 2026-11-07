// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Good {
    Wheat,
    Bread,
    Wood,
    Stone,
    Iron,
    Sand,
    Dirt,
}

impl Good {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "wheat" => Some(Self::Wheat),
            "bread" => Some(Self::Bread),
            "wood" | "logs" => Some(Self::Wood),
            "stone" => Some(Self::Stone),
            "iron" => Some(Self::Iron),
            "sand" => Some(Self::Sand),
            "dirt" | "earth" | "soil" => Some(Self::Dirt),
            _ => None,
        }
    }

    pub fn item(self) -> &'static str {
        match self {
            Self::Wheat => "minecraft:wheat",
            Self::Bread => "minecraft:bread",
            Self::Wood => "minecraft:oak_log",
            Self::Stone => "minecraft:cobblestone",
            Self::Iron => "minecraft:iron_ingot",
            Self::Sand => "minecraft:sand",
            Self::Dirt => "minecraft:dirt",
        }
    }

    pub fn price(self) -> u32 {
        match self {
            Self::Wheat => 2,
            Self::Bread => 5,
            Self::Wood => 3,
            Self::Stone => 2,
            Self::Iron => 15,
            Self::Sand => 1,
            Self::Dirt => 1,
        }
    }

    /// The only role that may physically extract this good from the world.
    pub fn job(self) -> Option<Job> {
        match self {
            Self::Wood => Some(Job::Lumberjack),
            Self::Stone => Some(Job::Miner),
            Self::Sand | Self::Dirt => Some(Job::Digger),
            Self::Wheat | Self::Bread | Self::Iron => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wheat => "wheat",
            Self::Bread => "bread",
            Self::Wood => "wood",
            Self::Stone => "stone",
            Self::Iron => "iron",
            Self::Sand => "sand",
            Self::Dirt => "dirt",
        }
    }
}

/// A player-friendly name for a tradable good.
pub fn good_label(good: Good) -> &'static str {
    match good {
        Good::Wood => "oak logs",
        Good::Stone => "cobblestone",
        Good::Wheat => "wheat",
        Good::Bread => "bread",
        _ => good.as_str(),
    }
}
