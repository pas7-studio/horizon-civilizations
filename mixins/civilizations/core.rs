// Civilization domain model. Deliberately independent from Minecraft/Horizon APIs.
// This source is also compiled as a standalone Cargo library for tests.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: u32 = 1;
pub const MAX_SETTLEMENTS: usize = 64;
pub const MAX_SIM_DAYS: u32 = 30;
pub const MIN_SETTLEMENT_DISTANCE: i64 = 192;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Good {
    Wheat,
    Bread,
    Wood,
    Stone,
    Iron,
}

impl Good {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "wheat" => Some(Self::Wheat),
            "bread" => Some(Self::Bread),
            "wood" | "logs" => Some(Self::Wood),
            "stone" => Some(Self::Stone),
            "iron" => Some(Self::Iron),
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
        }
    }

    pub fn price(self) -> u32 {
        match self {
            Self::Wheat => 2,
            Self::Bread => 5,
            Self::Wood => 3,
            Self::Stone => 2,
            Self::Iron => 15,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wheat => "wheat",
            Self::Bread => "bread",
            Self::Wood => "wood",
            Self::Stone => "stone",
            Self::Iron => "iron",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Job {
    Farmer,
    Baker,
    Lumberjack,
    Miner,
    Builder,
    Guard,
    Merchant,
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
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resident {
    pub id: u64,
    pub name: String,
    pub job: Job,
    pub age_days: u32,
    pub alive: bool,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settlement {
    pub id: u64,
    pub name: String,
    pub culture: String,
    pub dimension: String,
    pub center: [i32; 3],
    pub founder: String,
    pub residents: Vec<Resident>,
    pub stock: BTreeMap<Good, u32>,
    pub houses: u32,
    pub building_progress: u32,
    pub starving_days: u32,
    pub morale: i32,
    pub relationships: BTreeMap<u64, i16>,
    pub active_wars: BTreeSet<u64>,
    pub pending_visual_buildings: u32,
    pub actors_spawned: bool,
    /// Empty for existing v0.1.1 saves; these are *physical* build sites only.
    #[serde(default)]
    pub visual_sites: Vec<[i32; 3]>,
    #[serde(default)]
    pub visual_job: Option<VisualBuildJob>,
}

impl Settlement {
    pub fn count(&self, job: Job) -> u32 {
        self.residents.iter().filter(|r| r.alive && r.job == job).count() as u32
    }

    pub fn population(&self) -> u32 {
        self.residents.iter().filter(|r| r.alive).count() as u32
    }

    pub fn quantity(&self, good: Good) -> u32 {
        self.stock.get(&good).copied().unwrap_or(0)
    }

    /// Unique, stable vanilla entity name for non-destructive actor reattachment.
    /// This is a convenience label, not an authenticated NPC identity.
    pub fn actor_name(&self, resident: &Resident) -> String {
        format!("{} ({}) [civ:{}:{}]", resident.name, resident.job.name(), self.id, resident.id)
    }

    fn add(&mut self, good: Good, amount: u32) {
        let old = self.quantity(good);
        self.stock.insert(good, old.saturating_add(amount));
    }

    fn take(&mut self, good: Good, amount: u32) -> bool {
        if self.quantity(good) < amount {
            return false;
        }
        *self.stock.entry(good).or_default() -= amount;
        true
    }

    fn day(&mut self) -> Vec<String> {
        let mut events = Vec::new();
        self.add(Good::Wheat, self.count(Job::Farmer).saturating_mul(4));
        self.add(Good::Wood, self.count(Job::Lumberjack).saturating_mul(3));
        self.add(Good::Stone, self.count(Job::Miner).saturating_mul(2));
        for _ in 0..self.count(Job::Baker) {
            if self.take(Good::Wheat, 3) {
                self.add(Good::Bread, 2);
            }
        }
        let food = self.population().div_ceil(3);
        if self.take(Good::Bread, food) {
            self.starving_days = 0;
            self.morale = (self.morale + 1).min(100);
        } else {
            self.starving_days = self.starving_days.saturating_add(1);
            self.morale = (self.morale - 8).max(-100);
            events.push(format!("{}: food shortage ({} days)", self.name, self.starving_days));
        }
        // Avoid growing infinitely many abstract houses with a fixed population.
        let desired_houses = self.population().div_ceil(3).saturating_add(1);
        if self.count(Job::Builder) > 0 && self.population() > 0 && self.houses < desired_houses {
            // Buildings are simulated here. Physical blocks are not placed until a
            // construction adapter explicitly commits a structure in the level.
            if self.take(Good::Wood, 8) {
                if self.take(Good::Stone, 6) {
                    self.houses = self.houses.saturating_add(1);
                    self.pending_visual_buildings = self.pending_visual_buildings.saturating_add(1);
                    events.push(format!("{}: completed house #{} (simulation)", self.name, self.houses));
                } else {
                    self.add(Good::Wood, 8);
                }
            }
        }
        events
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub schema: u32,
    pub day: u64,
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
            next_id: 1,
            settlements: BTreeMap::new(),
            players: BTreeMap::new(),
            chronicle: Vec::new(),
        }
    }
}

impl World {
    /// Validate semantic invariants on load as well as JSON syntax. On corruption
    /// the adapter fails closed and never overwrites the damaged save.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA { return Err(format!("unknown schema {}", self.schema)); }
        if self.settlements.len() > MAX_SETTLEMENTS { return Err("too many settlements".into()); }
        if self.chronicle.len() > 200 { return Err("chronicle too large".into()); }
        if self.next_id <= self.settlements.keys().next_back().copied().unwrap_or(0) {
            return Err("settlement ID counter is not monotonic".into());
        }
        let mut seen = BTreeSet::new();
        for (&id, village) in &self.settlements {
            if village.id != id { return Err("settlement id/key mismatch".into()); }
            let normalized = village.name.to_ascii_lowercase();
            if !seen.insert(normalized) { return Err("duplicate settlement names".into()); }
            if village.residents.len() > 128 { return Err("resident cap exceeded".into()); }
            if village.visual_sites.len() > 32 { return Err("visual building site cap exceeded".into()); }
            if village.visual_sites.len() as u32 > village.houses { return Err("visual buildings exceed houses".into()); }
            if village.pending_visual_buildings > village.houses { return Err("pending physical houses exceed housing".into()); }
            if village.visual_sites.len() as u32 > village.houses.saturating_sub(village.pending_visual_buildings) {
                return Err("visual building count exceeds housing".into());
            }
            let mut unique_sites = BTreeSet::new();
            for site in &village.visual_sites {
                if i64::from(site[0]).abs() > 29_999_990 || i64::from(site[2]).abs() > 29_999_990
                    || !(-2048..=2048).contains(&site[1]) {
                    return Err("visual site outside world bounds".into());
                }
                if !unique_sites.insert((site[0], site[1], site[2])) { return Err("duplicate visual build site".into()); }
            }
            if let Some(job) = &village.visual_job {
                if i64::from(job.origin[0]).abs() > 29_999_990 || i64::from(job.origin[2]).abs() > 29_999_990
                    || !(-2048..=2048).contains(&job.origin[1]) {
                    return Err("construction site outside world bounds".into());
                }
                if job.cursor > hut_plan(job.origin).len() { return Err("construction cursor beyond plan".into()); }
                if village.pending_visual_buildings == 0 { return Err("visual job without pending house".into()); }
                if unique_sites.contains(&(job.origin[0], job.origin[1], job.origin[2])) {
                    return Err("building is already completed at the job site".into());
                }
            }
            if !(-100..=100).contains(&village.morale) { return Err("morale out of bounds".into()); }
            for (&other, &value) in &village.relationships {
                if other == id || !self.settlements.contains_key(&other) || !(-100..=100).contains(&value) {
                    return Err("invalid diplomatic relation".into());
                }
            }
            for &enemy in &village.active_wars {
                if enemy == id || !self.settlements.get(&enemy).is_some_and(|v| v.active_wars.contains(&id)) {
                    return Err("asymmetric or dangling war".into());
                }
            }
        }
        for p in self.players.values() {
            if p.settlement.is_some_and(|id| !self.settlements.contains_key(&id)) {
                return Err("player references missing settlement".into());
            }
            if p.reputation.keys().any(|id| !self.settlements.contains_key(id)) {
                return Err("reputation references missing settlement".into());
            }
        }
        Ok(())
    }

    fn event(&mut self, message: String) {
        self.chronicle.push(format!("day {}: {message}", self.day));
        if self.chronicle.len() > 200 {
            self.chronicle.drain(..self.chronicle.len() - 200);
        }
    }

    pub fn found(
        &mut self,
        name: &str,
        dimension: &str,
        center: [i32; 3],
        founder: &str,
    ) -> Result<u64, String> {
        if self.settlements.len() >= MAX_SETTLEMENTS {
            return Err("settlement cap reached".into());
        }
        if name.len() < 3 || name.len() > 24 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("name must be 3..24 ASCII letters, numbers or underscores".into());
        }
        if self.settlements.values().any(|v| v.name.eq_ignore_ascii_case(name)) {
            return Err("settlement name already exists".into());
        }
        if self.settlements.values().any(|v| {
            if v.dimension != dimension { return false; }
            let dx = i64::from(v.center[0]) - i64::from(center[0]);
            let dz = i64::from(v.center[2]) - i64::from(center[2]);
            dx * dx + dz * dz < MIN_SETTLEMENT_DISTANCE * MIN_SETTLEMENT_DISTANCE
        }) {
            return Err("another settlement is within 192 blocks".into());
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("ID counter exhausted")?;
        let roles = [
            Job::Farmer, Job::Farmer, Job::Farmer, Job::Baker, Job::Baker,
            Job::Lumberjack, Job::Miner, Job::Builder, Job::Guard, Job::Merchant,
        ];
        let first_names = ["Ada", "Bryn", "Cora", "Daren", "Eira", "Finn", "Gwen", "Hugo", "Iris", "Joren"];
        let residents = roles.iter().enumerate().map(|(i, job)| Resident {
            id: id.saturating_mul(1000) + i as u64,
            name: first_names[i].to_owned(),
            job: *job,
            age_days: 20 * 365 + i as u32 * 220,
            alive: true,
        }).collect();
        let settlement = Settlement {
            id,
            name: name.to_owned(),
            culture: "frontier".into(),
            dimension: dimension.to_owned(),
            center,
            founder: founder.to_owned(),
            residents,
            stock: BTreeMap::from([
                (Good::Wheat, 20), (Good::Bread, 12), (Good::Wood, 12),
                (Good::Stone, 8), (Good::Iron, 2),
            ]),
            houses: 2,
            building_progress: 0,
            starving_days: 0,
            morale: 50,
            relationships: BTreeMap::new(),
            active_wars: BTreeSet::new(),
            pending_visual_buildings: 0,
            actors_spawned: false,
            visual_sites: Vec::new(),
            visual_job: None,
        };
        self.settlements.insert(id, settlement);
        let p = self.players.entry(founder.to_owned()).or_default();
        p.settlement = Some(id);
        p.rank = Rank::Lord;
        p.coins = p.coins.saturating_add(25);
        p.reputation.insert(id, 50);
        self.event(format!("{name} founded"));
        Ok(id)
    }

    /// Request one explicitly approved physical construction project.
    /// Collision with world blocks is checked by the Horizon adapter first.
    pub fn start_visual_house(&mut self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if village.pending_visual_buildings == 0 { return Err("no completed abstract house awaits visualization".into()); }
        if i64::from(origin[0]).abs() > 29_999_990 || i64::from(origin[2]).abs() > 29_999_990
            || !(-2048..=2048).contains(&origin[1]) {
            return Err("site too close to world border".into());
        }
        if village.visual_job.is_some() { return Err("construction already in progress".into()); }
        if village.visual_sites.len() >= 32 { return Err("physical housing site limit reached".into()); }
        let dx = i64::from(origin[0]) - i64::from(village.center[0]);
        let dz = i64::from(origin[2]) - i64::from(village.center[2]);
        let dist = dx * dx + dz * dz;
        if !(12 * 12..=80 * 80).contains(&dist) { return Err("build 12..80 blocks from village center".into()); }
        for site in &village.visual_sites {
            let sx = i64::from(origin[0]) - i64::from(site[0]);
            let sz = i64::from(origin[2]) - i64::from(site[2]);
            if sx * sx + sz * sz < 9 * 9 { return Err("too close to another civilization house".into()); }
        }
        village.visual_job = Some(VisualBuildJob { origin, cursor: 0, blocked: false });
        Ok(())
    }

    /// Called only after all block states have been observed at their targets.
    pub fn finish_visual_house(&mut self, village_id: u64) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = village.visual_job.as_ref().ok_or("no construction project")?;
        if job.cursor != hut_plan(job.origin).len() { return Err("construction incomplete".into()); }
        let origin = job.origin;
        if village.pending_visual_buildings == 0 { return Err("house counter underflow".into()); }
        village.visual_job = None;
        village.visual_sites.push(origin);
        village.pending_visual_buildings -= 1;
        let name = village.name.clone();
        self.event(format!("{name}: house constructed at {},{},{}", origin[0], origin[1], origin[2]));
        Ok(())
    }

    pub fn by_name(&self, name: &str) -> Option<&Settlement> {
        self.settlements.values().find(|v| v.name.eq_ignore_ascii_case(name))
    }

    pub fn join(&mut self, player_id: &str, village_id: u64) -> Result<(), String> {
        if !self.settlements.contains_key(&village_id) {
            return Err("no such settlement".into());
        }
        let p = self.players.entry(player_id.to_owned()).or_default();
        if p.rank == Rank::Lord && p.settlement != Some(village_id) {
            return Err("a lord cannot abandon their settlement".into());
        }
        p.settlement = Some(village_id);
        if p.rank == Rank::Visitor {
            p.rank = Rank::Citizen;
            p.coins = p.coins.saturating_add(25);
        }
        p.reputation.entry(village_id).or_insert(5);
        Ok(())
    }

    pub fn promote(&mut self, player_id: &str) -> Result<Rank, String> {
        let p = self.players.get_mut(player_id).ok_or("join a settlement first")?;
        let id = p.settlement.ok_or("join a settlement first")?;
        let reputation = *p.reputation.get(&id).unwrap_or(&0);
        p.rank = match (p.rank, reputation) {
            (Rank::Citizen, n) if n >= 15 => Rank::Freeman,
            (Rank::Freeman, n) if n >= 40 => Rank::Knight,
            (Rank::Knight, n) if n >= 90 => Rank::Lord,
            _ => return Err("not enough reputation for the next title".into()),
        };
        Ok(p.rank)
    }

    pub fn sell(&mut self, player_id: &str, village_id: u64, good: Good, count: u32) -> Result<u64, String> {
        if count == 0 || count > 64 { return Err("amount must be 1..64".into()); }
        let p = self.players.get(player_id).ok_or("join a settlement first")?;
        if p.settlement != Some(village_id) { return Err("join this settlement first".into()); }
        let payout = u64::from(good.price()) * u64::from(count);
        if p.coins.checked_add(payout).is_none() { return Err("coin balance overflow".into()); }
        let s = self.settlements.get_mut(&village_id).ok_or("no settlement")?;
        s.add(good, count);
        let p = self.players.get_mut(player_id).ok_or("no player")?;
        p.coins += payout;
        let rep = p.reputation.entry(village_id).or_default();
        *rep = rep.saturating_add(1).min(100);
        Ok(payout)
    }

    pub fn buy(&mut self, player_id: &str, village_id: u64, good: Good, count: u32) -> Result<u64, String> {
        if count == 0 || count > 64 { return Err("amount must be 1..64".into()); }
        let cost = u64::from(good.price()) * u64::from(count);
        let p = self.players.get(player_id).ok_or("join a settlement first")?;
        if p.settlement != Some(village_id) { return Err("join this settlement first".into()); }
        if p.coins < cost { return Err("not enough coins".into()); }
        let s = self.settlements.get_mut(&village_id).ok_or("no settlement")?;
        if !s.take(good, count) { return Err("settlement stock is insufficient".into()); }
        self.players.get_mut(player_id).ok_or("no player")?.coins -= cost;
        Ok(cost)
    }

    pub fn relation(&self, a: u64, b: u64) -> i16 {
        self.settlements.get(&a).and_then(|x| x.relationships.get(&b).copied()).unwrap_or(0)
    }

    pub fn declare_war(&mut self, a: u64, b: u64) -> Result<(), String> {
        if a == b { return Err("a settlement cannot declare war on itself".into()); }
        if !self.settlements.contains_key(&a) || !self.settlements.contains_key(&b) {
            return Err("unknown settlement".into());
        }
        if self.settlements[&a].active_wars.contains(&b) { return Err("already at war".into()); }
        self.settlements.get_mut(&a).unwrap().active_wars.insert(b);
        self.settlements.get_mut(&b).unwrap().active_wars.insert(a);
        self.settlements.get_mut(&a).unwrap().relationships.insert(b, -100);
        self.settlements.get_mut(&b).unwrap().relationships.insert(a, -100);
        self.event(format!("war declared between {a} and {b}"));
        Ok(())
    }

    pub fn make_peace(&mut self, a: u64, b: u64) -> Result<(), String> {
        if !self.settlements.contains_key(&a) || !self.settlements.contains_key(&b) { return Err("unknown settlement".into()); }
        if !self.settlements[&a].active_wars.contains(&b) {
            return Err("settlements are not at war".into());
        }
        self.settlements.get_mut(&a).unwrap().active_wars.remove(&b);
        self.settlements.get_mut(&b).unwrap().active_wars.remove(&a);
        self.settlements.get_mut(&a).unwrap().relationships.insert(b, -40);
        self.settlements.get_mut(&b).unwrap().relationships.insert(a, -40);
        self.event(format!("peace between {a} and {b}"));
        Ok(())
    }

    pub fn advance(&mut self, days: u32) -> Result<(), String> {
        if days == 0 || days > MAX_SIM_DAYS { return Err("simulate 1..30 days at a time".into()); }
        for _ in 0..days {
            self.day = self.day.checked_add(1).ok_or("simulation day overflow")?;
            let mut events = Vec::new();
            for settlement in self.settlements.values_mut() {
                events.extend(settlement.day());
            }
            // Abstract warfare at strategic scale. There is no combat NPC AI yet.
            let pairs: Vec<(u64, u64)> = self.settlements.iter()
                .flat_map(|(&id, s)| s.active_wars.iter().filter(move |&&enemy| id < enemy).map(move |&enemy| (id, enemy)))
                .collect();
            for (a, b) in pairs {
                let a_power = self.settlements[&a].count(Job::Guard) + self.settlements[&a].quantity(Good::Iron) / 8;
                let b_power = self.settlements[&b].count(Job::Guard) + self.settlements[&b].quantity(Good::Iron) / 8;
                if self.day % 4 == 0 {
                    let target = if a_power >= b_power { b } else { a };
                    let s = self.settlements.get_mut(&target).expect("known settlement");
                    s.morale = (s.morale - 3).max(-100);
                    s.take(Good::Bread, 2);
                    events.push(format!("{} suffered war attrition (abstract)", s.name));
                }
            }
            for event in events { self.event(event); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> (World, u64) {
        let mut w = World::default();
        let id = w.found("Ashford", "minecraft:overworld", [0, 64, 0], "player-1").unwrap();
        (w, id)
    }

    #[test]
    fn founding_is_bounded_unique_and_grants_lordship() {
        let (mut w, id) = world();
        assert_eq!(w.settlements[&id].population(), 10);
        assert_eq!(w.players["player-1"].rank, Rank::Lord);
        assert!(w.found("Ashford", "minecraft:overworld", [400, 64, 0], "other").is_err());
        assert!(w.found("Village_2", "minecraft:overworld", [20, 64, 0], "other").is_err());
        assert!(w.found("Village_2", "minecraft:the_nether", [20, 64, 0], "other").is_ok());
    }

    #[test]
    fn economy_is_nonnegative_and_stocks_only_move_on_success() {
        let (mut w, id) = world();
        let wheat = w.settlements[&id].quantity(Good::Wheat);
        let coins = w.players["player-1"].coins;
        assert!(w.buy("player-1", id, Good::Wheat, 64).is_err());
        assert_eq!(w.players["player-1"].coins, coins);
        assert_eq!(w.settlements[&id].quantity(Good::Wheat), wheat);
        assert_eq!(w.sell("player-1", id, Good::Wheat, 4), Ok(8));
        assert_eq!(w.buy("player-1", id, Good::Wheat, 4), Ok(8));
        assert_eq!(w.settlements[&id].quantity(Good::Wheat), wheat);
        assert_eq!(w.players["player-1"].coins, coins);
    }

    #[test]
    fn offline_simulation_produces_food_and_houses() {
        let (mut w, id) = world();
        w.advance(12).unwrap();
        assert_eq!(w.day, 12);
        assert!(w.settlements[&id].houses > 2);
        assert_eq!(w.settlements[&id].population(), 10);
        assert_eq!(w.settlements[&id].starving_days, 0);
        assert!(w.advance(31).is_err());
    }

    #[test]
    fn wars_are_symmetric_and_peace_can_be_declared() {
        let (mut w, a) = world();
        let b = w.found("Brighthold", "minecraft:overworld", [450, 65, 0], "player-2").unwrap();
        w.declare_war(a, b).unwrap();
        assert!(w.settlements[&a].active_wars.contains(&b));
        assert!(w.settlements[&b].active_wars.contains(&a));
        w.advance(4).unwrap();
        w.make_peace(a, b).unwrap();
        assert!(w.settlements[&a].active_wars.is_empty());
        assert!(w.settlements[&b].active_wars.is_empty());
    }

    #[test]
    fn settlement_does_not_build_infinite_empty_houses() {
        let (mut w, id) = world();
        w.advance(30).unwrap();
        let village = &w.settlements[&id];
        assert_eq!(village.houses, village.population().div_ceil(3) + 1);
        assert_eq!(village.pending_visual_buildings, village.houses - 2);
    }

    #[test]
    fn workplace_and_entity_labels_are_stable() {
        let (w, id) = world();
        let village = &w.settlements[&id];
        let resident = &village.residents[0];
        let tag = village.actor_name(resident);
        assert!(tag.contains("[civ:1:1000]"));
        assert_eq!(resident.job.workplace(resident.id), resident.job.workplace(resident.id));
    }

    #[test]
    fn loads_reject_asymmetric_war_and_dangling_player() {
        let (mut w, a) = world();
        let b = w.found("Brighthold", "minecraft:overworld", [450, 65, 0], "player-2").unwrap();
        w.declare_war(a, b).unwrap();
        assert!(w.validate().is_ok());
        w.settlements.get_mut(&b).unwrap().active_wars.clear();
        assert!(w.validate().unwrap_err().contains("war"));
        w.settlements.get_mut(&b).unwrap().active_wars.insert(a);
        w.players.get_mut("player-1").unwrap().settlement = Some(999);
        assert!(w.validate().unwrap_err().contains("missing settlement"));
    }

    #[test]
    fn peace_requires_a_real_war() {
        let (mut w, a) = world();
        let b = w.found("Brighthold", "minecraft:overworld", [450, 65, 0], "player-2").unwrap();
        assert!(w.make_peace(a, b).is_err());
        assert!(w.settlements[&a].active_wars.is_empty());
    }

    #[test]
    fn twenty_settlements_simulate_without_broken_references() {
        let mut w = World::default();
        for i in 0..20 {
            let name = format!("Village_{i}");
            let founder = format!("p{i}");
            let id = w.found(&name, "minecraft:overworld", [i * 400, 64, 0], &founder).unwrap();
            assert_eq!(id, i as u64 + 1);
        }
        w.advance(30).unwrap();
        assert_eq!(w.day, 30);
        assert_eq!(w.settlements.len(), 20);
        assert!(w.validate().is_ok());
        for s in w.settlements.values() {
            assert!(s.houses <= s.population().div_ceil(3) + 1);
            assert!(s.quantity(Good::Bread) < 10000);
        }
    }

    #[test]
    fn duplicate_villager_names_within_village_do_not_share_tags() {
        let (w, id) = world();
        let s = &w.settlements[&id];
        let tags: BTreeSet<_> = s.residents.iter().map(|r| s.actor_name(r)).collect();
        assert_eq!(tags.len(), s.residents.len());
    }

    #[test]
    fn hut_is_small_deterministic_and_has_clear_doorway() {
        let a = hut_plan([100, 70, -30]);
        assert_eq!(a, hut_plan([100, 70, -30]));
        assert_eq!(a.iter().filter(|b| b.floor).count(), 25);
        assert!(!a.iter().any(|b| b.pos == [100, 70, -32] || b.pos == [100, 71, -32]));
        let unique: BTreeSet<_> = a.iter().map(|b| b.pos).collect();
        assert_eq!(a.len(), unique.len());
        assert!(a.len() < 110);
    }

    #[test]
    fn visual_house_requires_abstract_project_and_is_resume_safe() {
        let (mut w, id) = world();
        assert!(w.start_visual_house(id, [20, 64, 0]).is_err());
        w.advance(1).unwrap();
        w.start_visual_house(id, [20, 64, 0]).unwrap();
        assert!(w.start_visual_house(id, [30, 64, 0]).is_err());
        assert!(w.finish_visual_house(id).is_err());
        w.settlements.get_mut(&id).unwrap().visual_job.as_mut().unwrap().cursor = hut_plan([20, 64, 0]).len();
        let encoded = serde_json::to_string(&w).unwrap();
        let mut reloaded: World = serde_json::from_str(&encoded).unwrap();
        reloaded.validate().unwrap();
        reloaded.finish_visual_house(id).unwrap();
        assert_eq!(reloaded.settlements[&id].visual_sites, vec![[20, 64, 0]]);
        assert_eq!(reloaded.settlements[&id].pending_visual_buildings, 0);
        assert!(reloaded.start_visual_house(id, [21, 64, 0]).is_err());
        reloaded.validate().unwrap();
    }

    #[test]
    fn old_save_without_visual_fields_loads_cleanly() {
        let (w, id) = world();
        let mut j: serde_json::Value = serde_json::to_value(&w).unwrap();
        let key = id.to_string();
        let s = j["settlements"].get_mut(key.as_str()).unwrap().as_object_mut().unwrap();
        s.remove("visual_sites");
        s.remove("visual_job");
        let legacy: World = serde_json::from_value(j).unwrap();
        assert!(legacy.settlements[&id].visual_sites.is_empty());
        assert!(legacy.settlements[&id].visual_job.is_none());
        legacy.validate().unwrap();
    }

    #[test]
    fn saved_world_roundtrips_and_preserves_owners() {
        let (mut w, id) = world();
        w.advance(3).unwrap();
        let json = serde_json::to_string(&w).unwrap();
        let loaded: World = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.schema, SCHEMA);
        assert_eq!(loaded.day, 3);
        assert_eq!(loaded.settlements[&id].founder, "player-1");
    }
}
