// World state owner plus one inherent impl file per domain.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub schema: u32,
    pub day: u64,
    /// Last authoritative Horizon world day already observed by simulation.
    /// Old saves bootstrap this cursor on the next server tick, with no backlog.
    #[serde(default)]
    pub last_host_day: Option<i64>,
    pub next_id: u64,
    pub settlements: BTreeMap<u64, Settlement>,
    pub players: BTreeMap<String, Player>,
    pub chronicle: Vec<String>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            day: 0,
            last_host_day: None,
            next_id: 1,
            settlements: BTreeMap::new(),
            players: BTreeMap::new(),
            chronicle: Vec::new(),
        }
    }
}


mod bakery;
mod barrel;
mod diplomacy;
mod farm;
mod founding;
mod gather;
mod hall;
mod housing;
mod posts;
mod protection;
mod state;
mod trade;
