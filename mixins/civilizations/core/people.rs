// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeSlot {
    pub site: [i32; 3],
    /// Indoor lane: 0 = west, 1 = center, 2 = east.
    pub lane: u8,
}

impl HomeSlot {
    pub fn foot(self) -> [i32; 3] {
        [self.site[0] + i32::from(self.lane) - 1, self.site[1], self.site[2] - 1]
    }

    pub fn doorway(self) -> [i32; 3] {
        [self.site[0], self.site[1], self.site[2] - 2]
    }
}

/// Never route an actor into a saved "home" unless the corresponding real
/// blocks exist in already-loaded chunks. Does not repair/overwrite blocks.
pub fn inspect_home(home: HomeSlot, mut read: impl FnMut([i32; 3]) -> Option<String>) -> bool {
    if usize::from(home.lane) >= PHYSICAL_HOME_CAPACITY {
        return false;
    }
    let [x, y, z] = home.foot();
    let [dx, dy, dz] = home.doorway();
    for (position, expected) in [
        ([x, y - 1, z], "minecraft:cobblestone"),
        ([x, y, z], "minecraft:air"),
        ([x, y + 1, z], "minecraft:air"),
        ([dx, dy - 1, dz], "minecraft:cobblestone"),
        ([dx, dy, dz], "minecraft:air"),
        ([dx, dy + 1, dz], "minecraft:air"),
    ] {
        if read(position).as_deref() != Some(expected) {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resident {
    pub id: u64,
    pub name: String,
    pub job: Job,
    pub age_days: u32,
    pub alive: bool,
    /// Persistent Minecraft entity identity. No gameplay action may trust a name instead.
    #[serde(default, with = "entity_uuid_hex", skip_serializing_if = "Option::is_none")]
    pub entity_uuid: Option<u128>,
}

mod entity_uuid_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(uuid) => serializer.serialize_some(&format!("{uuid:032x}")),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        value
            .map(|hex| {
                if hex.len() != 32 || !hex.bytes().all(|ch| ch.is_ascii_hexdigit()) {
                    return Err(serde::de::Error::custom("entity UUID must be 32 hex characters"));
                }
                u128::from_str_radix(&hex, 16).map_err(serde::de::Error::custom)
            })
            .transpose()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub settlement: Option<u64>,
    pub rank: Rank,
    pub coins: u64,
    pub reputation: BTreeMap<u64, i32>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            settlement: None,
            rank: Rank::Visitor,
            coins: 0,
            reputation: BTreeMap::new(),
        }
    }
}
