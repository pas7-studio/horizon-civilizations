// Civilization domain model. Deliberately independent from Minecraft/Horizon APIs.
// This source is also compiled as a standalone Cargo library for tests.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: u32 = 1;
pub const MAX_SETTLEMENTS: usize = 64;
pub const MAX_SIM_DAYS: u32 = 30;
/// Limit catch-up when a server clock jumps far forward, without spinning.
/// A skipped gap is intentional; no endless offline farming minting.
pub const MAX_HOST_CATCHUP_DAYS: u32 = 30;
/// Bounded on-disk audit of observed visual harvests.
pub const MAX_HARVEST_RECEIPTS: usize = 32;
pub const MIN_SETTLEMENT_DISTANCE: i64 = 192;
/// Fixed, bounded rings for *suggested* plots. Numbers are 1-based in commands.
/// These are not territorial claims or permission to edit blocks.
pub const HOUSING_PLOT_OFFSETS: [[i32; 2]; 20] = [
    [20, 0], [-20, 0], [0, 20], [0, -20],
    [28, 14], [-28, 14], [28, -14], [-28, -14],
    [14, 28], [-14, 28], [14, -28], [-14, -28],
    [40, 0], [-40, 0], [0, 40], [0, -40],
    [40, 24], [-40, 24], [40, -24], [-40, -24],
];
pub const MAX_HOUSING_SURVEY_RESULTS: usize = 5;
/// The board forecasts a few abstract days, not simulated physical work.
pub const COUNCIL_FORECAST_DAYS: u8 = 3;
pub const MAX_COUNCIL_FORECAST_DAYS: u8 = 7;
pub const LAND_PRICE_COINS: u64 = 10;
pub const COMMISSION_PRICE_COINS: u64 = 15;
pub const FINISHED_HUT_PRICE_COINS: u64 = 35;
pub const MAX_PLAYER_PARCELS: usize = 32;

fn autonomous_by_default() -> bool { true }

/// Mod-level ownership of a 5x5 plot; not a vanilla/other-mod block-protection
/// system. Minecraft block preflight still checks for player structures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerParcel {
    pub origin: [i32; 3],
    pub owner: String, // authenticated acting player's UUID in hex
    /// True for a private house ordered on a bought plot OR an existing
    /// village hut purchased with its completed title.
    #[serde(default)]
    pub commissioned: bool,
}

pub fn near_plot(a: [i32; 3], b: [i32; 3]) -> bool {
    let dx = i128::from(a[0]) - i128::from(b[0]);
    let dz = i128::from(a[2]) - i128::from(b[2]);
    dx * dx + dz * dz < 9_i128 * 9
}


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

/// Choose one loaded, active village gather per native gather tick. The
/// bridge calls this once every 8 authoritative level ticks, so the
/// division advances the round-robin exactly once per opportunity.
/// Count=0 yields no turn (and never divides by zero). No saved cursor
/// or scheduler state can become stale across server restarts.
pub fn gather_turn_index(physical_ticks: u64, eligible_count: usize) -> Option<usize> {
    if eligible_count == 0 { return None; }
    Some(((physical_ticks / 8) % eligible_count as u64) as usize)
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

/// Experimental physical home assignment: only completed visual 5x5 huts.
/// Three indoor standing spots per hut, not beds or a legal claim of land.
pub const PHYSICAL_HOME_CAPACITY: usize = 3;

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

/// One pending real-resource extraction. Persist the plan before the block
/// command; issue state before granting any Minecraft-derived warehouse item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatherJob {
    pub position: [i32; 3],
    /// Verified empty two-high destination a worker can navigate toward.
    pub stand: [i32; 3],
    pub good: Good, // only Wood (oak log) or Stone (natural stone)
    /// Stable citizen ID assigned durably before any native resource edit.
    /// Old saves lack this field: an issued unbound receipt is quarantined,
    /// never credited by whichever worker happens to stand at the hall.
    #[serde(default)]
    pub worker_id: Option<u64>,
    pub issued: bool,
    /// Durable acknowledgement only after a successful native command AND
    /// an immediate loaded observation of the edited block as air. A crash
    /// between Issued persistence and this receipt is irreducibly ambiguous:
    /// block/quarantine it rather than crediting a player-removed block.
    #[serde(default)]
    pub native_edit_confirmed: bool,
    /// An actual loaded UUID-linked worker has observed the source block
    /// as air at the original saved work stand AFTER issue. Only then may
    /// their native route turn toward the village receiving area.
    /// Old saves default to false and cannot auto-deliver based on mere air.
    #[serde(default)]
    pub pickup_witnessed: bool,
    #[serde(default)]
    pub blocked: bool,
}

/// A short oak stem is harvested one real block and one UUID-worker
/// delivery at a time. The cursor tracks the NEXT block, not free wood.
/// Existing saves without this field keep their ordinary one-block jobs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OakStem {
    pub root: [i32; 3],
    pub stand: [i32; 3],
    /// Short, reachable stems only. A larger tree requires real scaffolding.
    pub height: u8,
    pub cursor: u8,
}

/// The native adapter first verifies a rooted leaf-bearing oak, then measures
/// the whole short stem. Trees taller than four logs are deliberately skipped:
/// our worker currently has no physical way to reach a taller canopy.
pub fn short_oak_stem_height(
    root: [i32; 3], mut read: impl FnMut([i32; 3]) -> Option<String>,
) -> Option<u8> {
    if !is_rooted_oak_harvest_origin(root, |pos| read(pos)) { return None; }
    let mut count = 0_u8;
    for offset in 0..=4_i32 {
        let y = root[1].checked_add(offset)?;
        match read([root[0], y, root[2]]).as_deref() {
            Some("minecraft:oak_log") if offset < 4 => count += 1,
            Some("minecraft:oak_log") => return None,
            Some(_) => break,
            None => return None,
        }
    }
    (3..=4).contains(&count).then_some(count)
}

/// Observe every step of the SAME saved stem before dispatching its next
/// world edit. Previous trunk logs must already be air, remaining ones
/// unchanged vanilla oak. The upper crown must still exist and all reads
/// must be loaded. No independent search for a second, possibly foreign tree.
pub fn inspect_oak_stem(
    stem: &OakStem, mut read: impl FnMut([i32; 3]) -> Option<String>,
) -> bool {
    if !(3..=4).contains(&stem.height) || stem.cursor >= stem.height {
        return false;
    }
    // On the first block this is still an actual rooted tree, not a stored
    // journal claiming that an externally placed timber pole is natural.
    if stem.cursor == 0
        && !is_rooted_oak_harvest_origin(stem.root, |pos| read(pos)) {
        return false;
    }
    for offset in 0..stem.height {
        let Some(y) = stem.root[1].checked_add(i32::from(offset)) else {
            return false;
        };
        let expected = if offset < stem.cursor { "minecraft:air" }
            else { "minecraft:oak_log" };
        if read([stem.root[0], y, stem.root[2]]).as_deref() != Some(expected) {
            return false;
        }
    }
    let Some(top_y) = stem.root[1].checked_add(i32::from(stem.height) - 1)
    else { return false; };
    let mut crown = false;
    for (dx, dz) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
        let (Some(x), Some(z)) =
            (stem.root[0].checked_add(dx), stem.root[2].checked_add(dz))
        else { return false; };
        let Some(block) = read([x, top_y, z]) else { return false; };
        if block == "minecraft:oak_leaves" { crown = true; }
    }
    crown
}

impl GatherJob {
    pub fn source_block(&self) -> &'static str {
        match self.good {
            Good::Wood => "minecraft:oak_log",
            Good::Stone => "minecraft:stone",
            _ => "",
        }
    }
}

/// Verify a *rooted*, leaf-bearing oak trunk before issuing any native
/// lumberjack destruction. A leaf next to an arbitrary isolated oak log is
/// not evidence of a natural tree: a genuine candidate must begin on real
/// soil, contain at least three vertically contiguous logs, and reach a
/// leaf-bearing upper crown. Only the lowest rooted log qualifies as the
/// first resource; upper floating/player-placed logs are never selected.
/// The caller supplies loaded, observed vanilla block states and must also
/// enforce native claim protection and a reachable worker stand.
/// This is conservative physical evidence, NOT provenance against a player
/// who deliberately plants a real tree outside registered claims.
pub fn is_rooted_oak_harvest_origin(
    origin: [i32; 3], mut read: impl FnMut([i32; 3]) -> Option<String>,
) -> bool {
    let [x, y, z] = origin;
    if read(origin).as_deref() != Some("minecraft:oak_log") {
        return false;
    }
    let Some(soil_y) = y.checked_sub(1) else { return false; };
    if !matches!(
        read([x, soil_y, z]).as_deref(),
        Some("minecraft:grass_block" | "minecraft:dirt" | "minecraft:podzol" |
             "minecraft:coarse_dirt" | "minecraft:rooted_dirt")
    ) {
        return false;
    }
    let mut upper_logs = 0_usize;
    let mut crown = false;
    // A bounded search avoids scanning remote/unloaded chunks. We deliberately
    // reject taller/uncertain trees until supported by an explicit tree claim.
    for step in 1..=8_i32 {
        let Some(top_y) = y.checked_add(step) else { return false; };
        match read([x, top_y, z]).as_deref() {
            Some("minecraft:oak_log") => {}
            Some(_) => break,
            None => return false,
        }
        upper_logs += 1;
        if step < 2 { continue; }
        for (dx, dz) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
            let (Some(tx), Some(tz)) = (x.checked_add(dx), z.checked_add(dz))
            else { return false; };
            let Some(leaf) = read([tx, top_y, tz]) else {
                return false; // unloaded evidence cannot approve extraction
            };
            if leaf == "minecraft:oak_leaves" { crown = true; }
        }
    }
    upper_logs >= 2 && crown
}

/// Founding civic hall uses the existing 5x5 safe physical shell, with an
/// identifiable mayor desk, two resource barrels and two bulletin signs.
/// Barrel contents are not treated as a vanilla chest inventory: the guarded
/// civic warehouse ledger is authoritative until a verified container API
/// can support crash-atomic external transfers.
pub const HALL_WOOD_LOGS: u32 = 24;
pub const HALL_COBBLESTONE: u32 = 25;
pub const HUT_WOOD_LOGS: u32 = 20;
pub const HUT_COBBLESTONE: u32 = 25;
/// Includes real wheat-processing workstations as well as the shell.
pub const BAKERY_WOOD_LOGS: u32 = 24;
pub const BAKERY_COBBLESTONE: u32 = 33;
pub const BREAD_WHEAT_RECIPE: u32 = 3;


pub fn town_hall_plan(origin: [i32; 3]) -> Vec<HutBlock> {
    let mut plan = hut_plan(origin);
    for (dx, dz, block) in [
        (-1, 1, "minecraft:barrel"),
        (1, 1, "minecraft:barrel"),
        (-1, -1, "minecraft:oak_sign"),
        (1, -1, "minecraft:oak_sign"),
        (0, 1, "minecraft:lectern"),
    ] {
        plan.push(HutBlock {
            pos: [origin[0] + dx, origin[1], origin[2] + dz],
            block, floor: false,
        });
    }
    plan
}

/// Physical independent workshop: walls and floor like a village hut,
/// with an actual crafting table and smoker. Bread is crafted from three
/// real wheat items at the table; a smoker is a visible workplace detail.
pub fn bakery_plan(origin: [i32; 3]) -> Vec<HutBlock> {
    let mut plan = hut_plan(origin);
    for (dx, dz, block) in [
        (-1, 0, "minecraft:crafting_table"),
        (1, 0, "minecraft:smoker"),
    ] {
        plan.push(HutBlock {
            pos: [origin[0] + dx, origin[1], origin[2] + dz],
            block, floor: false,
        });
    }
    plan
}

/// The actual vanilla barrel blocks inside the public town hall, not
/// surrogate entries in the settlement's JSON. Never allow player parcels
/// or unverified nearby chests to substitute for these positions.
pub fn hall_storage_positions(origin: [i32; 3]) -> [[i32; 3]; 2] {
    [[origin[0] - 1, origin[1], origin[2] + 1],
     [origin[0] + 1, origin[1], origin[2] + 1]]
}

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

/// Bulletin signs are real blocks inside the civic hall; the native adapter
/// supplies validated, bounded sign text after the town hall is complete.
pub fn hall_board_positions(origin: [i32; 3]) -> [[i32; 3]; 2] {
    [[origin[0] - 1, origin[1], origin[2] - 1],
     [origin[0] + 1, origin[1], origin[2] - 1]]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualBuildJob {
    pub origin: [i32; 3],
    pub cursor: usize,
    #[serde(default)]
    pub blocked: bool,
}

 
/// Pure, non-mutating decision after inspecting the actual loaded world blocks.
/// Horizon performs writes separately, and confirms each result on a later tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildInspection {
    Unloaded,
    Paused,
    Rewind(usize),
    Blocked { position: [i32; 3], found: String },
    InvalidCursor,
    Advance,
    Complete,
    Place { position: [i32; 3], old_block: String, new_block: &'static str },
}

pub fn is_ground(name: &str) -> bool {
    matches!(name, "minecraft:grass_block" | "minecraft:dirt" | "minecraft:stone")
}

/// Check every previously committed step; the save file and chunk data are
/// not atomic, so a committed cursor does not prove the blocks still exist.
pub fn inspect_hut(job: &VisualBuildJob, mut read: impl FnMut([i32; 3]) -> Option<String>) -> BuildInspection {
    if job.blocked {
        return BuildInspection::Paused;
    }
    let plan = hut_plan(job.origin);
    if job.cursor > plan.len() {
        return BuildInspection::InvalidCursor;
    }
    for (index, step) in plan.iter().enumerate() {
        if index > job.cursor {
            break;
        }
        let Some(actual) = read(step.pos) else {
            return BuildInspection::Unloaded;
        };
        if actual == step.block {
            if index == job.cursor {
                return BuildInspection::Advance;
            }
            continue;
        }
        let may_replace = if step.floor { is_ground(&actual) } else { actual == "minecraft:air" };
        if !may_replace {
            return BuildInspection::Blocked { position: step.pos, found: actual };
        }
        if index < job.cursor {
            return BuildInspection::Rewind(index);
        }
        return BuildInspection::Place {
            position: step.pos,
            old_block: actual,
            new_block: step.block,
        };
    }
    BuildInspection::Complete
}


/// Same fail-closed crash reconciliation as ordinary huts, but with
/// the distinct civic center blueprint and actual warehouse blocks.
pub fn inspect_town_hall(
    job: &VisualBuildJob, mut read: impl FnMut([i32; 3]) -> Option<String>
) -> BuildInspection {
    if job.blocked { return BuildInspection::Paused; }
    let plan = town_hall_plan(job.origin);
    if job.cursor > plan.len() { return BuildInspection::InvalidCursor; }
    for (index, step) in plan.iter().enumerate() {
        if index > job.cursor { break; }
        let Some(actual) = read(step.pos) else { return BuildInspection::Unloaded; };
        if actual == step.block {
            if index == job.cursor { return BuildInspection::Advance; }
            continue;
        }
        let replaceable = if step.floor { is_ground(&actual) } else { actual == "minecraft:air" };
        if !replaceable { return BuildInspection::Blocked { position: step.pos, found: actual }; }
        if index < job.cursor { return BuildInspection::Rewind(index); }
        return BuildInspection::Place {
            position: step.pos, old_block: actual, new_block: step.block,
        };
    }
    BuildInspection::Complete
}

/// Check actual vanilla workshop blocks before advancing each saved step.
pub fn inspect_bakery(
    job: &VisualBuildJob, mut read: impl FnMut([i32; 3]) -> Option<String>
) -> BuildInspection {
    if job.blocked { return BuildInspection::Paused; }
    let plan = bakery_plan(job.origin);
    if job.cursor > plan.len() { return BuildInspection::InvalidCursor; }
    for (index, step) in plan.iter().enumerate() {
        if index > job.cursor { break; }
        let Some(actual) = read(step.pos) else { return BuildInspection::Unloaded; };
        if actual == step.block {
            if index == job.cursor { return BuildInspection::Advance; }
            continue;
        }
        let replaceable = if step.floor { is_ground(&actual) } else { actual == "minecraft:air" };
        if !replaceable { return BuildInspection::Blocked {position: step.pos, found: actual}; }
        if index < job.cursor { return BuildInspection::Rewind(index); }
        return BuildInspection::Place {
            position: step.pos, old_block: actual, new_block: step.block,
        };
    }
    BuildInspection::Complete
}

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
    /// Actual player deliveries and confirmed native harvests pending
    /// placement in the visible hall barrels. NOT virtual daily stock and
    /// NOT duplicated by a successful barrel deposit.
    #[serde(default)]
    pub warehouse: BTreeMap<Good, u32>,
    /// Goods already physically removed from a barrel for the current
    /// construction request. No native items remain in the container.
    #[serde(default)]
    pub build_escrow: BTreeMap<Good, u32>,
    /// Single journalled native barrel operation. Write-ahead state prevents
    /// repeated external insertion/extraction on crash or reload.
    #[serde(default)]
    pub barrel_transfer: Option<BarrelTransfer>,
    /// One direct native-barrel player purchase, precharged in world.json.
    #[serde(default)]
    pub market_purchase: Option<MarketPurchase>,
    /// Confirmed physical wheat harvested but still being carried by
    /// UUID-linked farmers toward the central village receiving area.
    #[serde(default)]
    pub wheat_in_transit: u32,
    /// One physical bakery, built and operated by actual NPC workers.
    #[serde(default)]
    pub bakery: Option<BakeryWorkshop>,
    /// Active physical civic hall construction; never inferred from a JSON
    /// house counter, and can resume after restarting with observed blocks.
    #[serde(default)]
    pub hall_job: Option<VisualBuildJob>,
    /// The civic center is proven complete only after all plan blocks read.
    #[serde(default)]
    pub hall_complete: bool,
    /// Pending actual tree/stone harvest; reconciled against native blocks.
    #[serde(default)]
    pub gather_job: Option<GatherJob>,
    /// Optional continuation for a proven short native oak trunk.
    /// Each log is still a separate issued gather and delivered receipt.
    #[serde(default)]
    pub oak_stem: Option<OakStem>,
    /// Player civic coin payments are transferred here, not silently burned.
    #[serde(default)]
    pub treasury_coins: u64,
    pub houses: u32,
    pub building_progress: u32,
    pub starving_days: u32,
    pub morale: i32,
    pub relationships: BTreeMap<u64, i16>,
    pub active_wars: BTreeSet<u64>,
    pub pending_visual_buildings: u32,
    pub actors_spawned: bool,
    /// One-shot, durably reserved OP recovery attempts after partial native
    /// actor spawning. A crash can occur AFTER the entity is created but
    /// BEFORE its UUID is persisted: never automatically reissue this spawn.
    /// The OP must first inspect/reconcile any already-existing villager.
    #[serde(default)]
    pub actor_recovery_issued: BTreeSet<u64>,
    /// Villages develop by default. OP can pause selected settlements.
    #[serde(default = "autonomous_by_default")]
    pub autonomy_enabled: bool,
    /// Land purchased by players; no village physical job may use these
    /// parcels unless its owner explicitly commissions a building.
    #[serde(default)]
    pub player_parcels: Vec<PlayerParcel>,
    /// Empty for existing v0.1.1 saves; these are *physical* build sites only.
    #[serde(default)]
    pub visual_sites: Vec<[i32; 3]>,
    #[serde(default)]
    pub visual_job: Option<VisualBuildJob>,
    /// Physical bill-of-materials deducted once per active hut project.
    #[serde(default)]
    pub hut_supplies_reserved: bool,
    /// Optional one-field living crop simulation, retained across saves.
    #[serde(default)]
    pub farm: Option<FarmPlot>,
}

/// Compute *combined* outstanding construction demand using verified native
/// barrel counts and confirmed inbound receipts. Each project's escrow is
/// deducted from its OWN bill, never reused to satisfy another project.
/// Caller must pass a fresh complete snapshot of BOTH native hall barrels.
/// No virtual daily stock is involved.
pub fn physical_construction_shortfall(
    village: &Settlement, native_stock: &[(Good, u32)],
) -> (u32, u32) {
    let bakery = village.bakery.as_ref().filter(|b| {
        !b.supplies_reserved
            && b.building.as_ref().is_some_and(|job| !job.blocked)
    });
    let hut_needed = village.visual_job.as_ref().is_some_and(|job| {
        !job.blocked && !village.hut_supplies_reserved
    }) || (village.visual_job.is_none()
        && village.pending_visual_buildings > 0
        && village.physical_housing().2 > 0);

    let unpaid = |good: Good| {
        let bakery_bill = if let Some(b) = bakery {
            let total = if good == Good::Wood { BAKERY_WOOD_LOGS }
                else { BAKERY_COBBLESTONE };
            total.saturating_sub(b.construction_escrow.get(&good).copied().unwrap_or(0))
        } else { 0 };
        let hut_bill = if hut_needed {
            let total = if good == Good::Wood { HUT_WOOD_LOGS }
                else { HUT_COBBLESTONE };
            total.saturating_sub(village.build_escrow.get(&good).copied().unwrap_or(0))
        } else { 0 };
        bakery_bill.saturating_add(hut_bill)
    };
    let available = |good: Good| {
        let stored = native_stock.iter().find(|(g, _)| *g == good)
            .map(|(_, count)| *count).unwrap_or(0);
        stored.saturating_add(village.warehouse_quantity(good))
    };
    (unpaid(Good::Wood).saturating_sub(available(Good::Wood)),
     unpaid(Good::Stone).saturating_sub(available(Good::Stone)))
}

/// Bounded village bread buffer, backed only by actual native hall barrels,
/// confirmed inbound receipts and the baker's already-crafted delivery.
pub fn physical_bread_target(village: &Settlement) -> u32 {
    village.population().saturating_mul(2).min(64)
}

/// Prevent infinite physical baking and storage-filling after a settlement
/// has enough real bread. The observer MUST read both native hall barrels;
/// an unavailable block entity pauses production instead of treating it as
/// an empty container. This pure function never credits abstract stock.
pub fn needs_real_bread(village: &Settlement, observed_barrel_bread: u32) -> bool {
    let pending = village.warehouse_quantity(Good::Bread)
        .saturating_add(village.bakery.as_ref()
            .map(|b| b.bread_in_transit).unwrap_or(0));
    observed_barrel_bread.saturating_add(pending) < physical_bread_target(village)
}

impl Settlement {
    pub fn count(&self, job: Job) -> u32 {
        self.residents.iter().filter(|r| r.alive && r.job == job).count() as u32
    }

    pub fn population(&self) -> u32 {
        self.residents.iter().filter(|r| r.alive).count() as u32
    }

    /// Derived physical housing capacity. Abstract houses are NOT included.
    /// Allocations are recalculated from the current living roster and ordered
    /// completed sites; no new data is serialized to legacy world saves.
    /// The player's commissioned houses are not public NPC lodging. The
    /// physical_housing and home_slot computations share this same filter.
    pub fn public_huts(&self) -> usize {
        self.visual_sites.iter().filter(|site| !self.player_parcels.iter()
            .any(|plot| plot.commissioned && plot.origin == **site)).count()
    }

    pub fn physical_housing(&self) -> (usize, usize, usize) {
        let capacity = self.public_huts() * PHYSICAL_HOME_CAPACITY;
        let residents = self.residents.iter().filter(|resident| resident.alive).count();
        (capacity, residents.min(capacity), residents.saturating_sub(capacity))
    }

    /// Stable for the same roster/sites, independent of resident vector order.
    /// The first three living IDs get the first completed hut, next three the
    /// second hut; no spot is invented in an abstract or unfinished house.
    pub fn home_slot(&self, resident_id: u64) -> Option<HomeSlot> {
        let mut ids: Vec<u64> = self.residents.iter()
            .filter(|resident| resident.alive).map(|resident| resident.id).collect();
        ids.sort_unstable();
        let position = ids.binary_search(&resident_id).ok()?;
        let site = self.visual_sites.iter()
            .filter(|site| !self.player_parcels.iter().any(|plot| plot.commissioned && plot.origin == **site))
            .nth(position / PHYSICAL_HOME_CAPACITY).copied()?;
        let lane = (position % PHYSICAL_HOME_CAPACITY) as u8;
        Some(HomeSlot { site, lane })
    }

    /// Run the existing daily economy on an in-memory clone. The forecast
    /// cannot place blocks, grant items, save data or append world history.
    /// Only the next 1..=7 abstract days are accepted for presentation.
    pub fn forecast_abstract_days(&self, days: u8) -> Vec<AbstractDayForecast> {
        let mut hypothetical = self.clone();
        let mut report = Vec::new();
        for offset in 1..=days.min(MAX_COUNCIL_FORECAST_DAYS) {
            // Events belong to the hypothetical clone, not World.chronicle.
            let _discarded_events = hypothetical.day();
            report.push(AbstractDayForecast {
                offset,
                wheat: hypothetical.quantity(Good::Wheat),
                bread: hypothetical.quantity(Good::Bread),
                morale: hypothetical.morale,
                starving_days: hypothetical.starving_days,
                houses: hypothetical.houses,
                pending_visual_buildings: hypothetical.pending_visual_buildings,
            });
        }
        report
    }

    /// Rank *current* settlement risks from its own already-saved economy,
    /// population, physical housing and identity state. No AI inference,
    /// no random actions, and no implicit permission to trade/build/fight.
    pub fn council_priorities(&self) -> Vec<CivicPriority> {
        let mut priorities = Vec::new();
        let population = self.population();
        if population == 0 {
            return priorities;
        }
        let food_per_day = population.div_ceil(3);
        let predicted = self.forecast_abstract_days(COUNCIL_FORECAST_DAYS);
        let starvation_forecast = predicted.iter().find(|d| d.starving_days > 0)
            .map(|d| d.offset);
        let bread = self.quantity(Good::Bread);
        if self.starving_days > 0 {
            priorities.push(CivicPriority {
                need: CivicNeed::Food, urgency: 100,
                reason: format!(
                    "food shortage has lasted {} day(s); {} bread/day needed",
                    self.starving_days, food_per_day
                ),
                next_step: format!("Inspect /civ info {} and food production before advancing time", self.name),
            });
        } else if let Some(day) = starvation_forecast {
            priorities.push(CivicPriority {
                need: CivicNeed::Food, urgency: 90,
                reason: format!(
                    "abstract economy forecasts starvation on day +{}; current bread {}",
                    day, bread
                ),
                next_step: format!("Inspect /civ forecast {} and bread supply; physical harvests do not credit stock", self.name),
            });
        } else if bread < food_per_day.saturating_mul(3) {
            priorities.push(CivicPriority {
                need: CivicNeed::Food, urgency: 55,
                reason: format!(
                    "bread reserve {} is below three-day consumption {} (forecast currently fed)",
                    bread, food_per_day.saturating_mul(3)
                ),
                next_step: format!("Inspect /civ forecast {} before changing the economy", self.name),
            });
        }

        if !self.active_wars.is_empty() {
            priorities.push(CivicPriority {
                need: CivicNeed::Security, urgency: 95,
                reason: format!(
                    "{} active war(s), {} living guard(s); warfare is abstract",
                    self.active_wars.len(), self.count(Job::Guard)
                ),
                next_step: format!("Review /civ history and /civ info {}; diplomacy remains player/operator-controlled", self.name),
            });
        }
        if self.morale < 30 {
            priorities.push(CivicPriority {
                need: CivicNeed::Stability,
                urgency: if self.morale < 0 { 85 } else { 60 },
                reason: format!(
                    "morale {} below 30; recent shortage/war may be contributors",
                    self.morale
                ),
                next_step: format!("Inspect /civ info {} and /civ history before making policy changes", self.name),
            });
        }
        if !self.actors_spawned {
            priorities.push(CivicPriority {
                need: CivicNeed::Identity, urgency: 70,
                reason: format!(
                    "{} residents exist only in the abstract simulation; spawn not reserved",
                    population
                ),
                next_step: format!("OP may use /civ actors {} once in a disposable world, then /civ roster {}", self.name, self.name),
            });
        } else {
            let missing = self.residents.iter()
                .filter(|r| r.alive && r.entity_uuid.is_none()).count();
            if missing > 0 {
                priorities.push(CivicPriority {
                    need: CivicNeed::Identity, urgency: 88,
                    reason: format!(
                        "{} living resident(s) lack a durable entity UUID; no automatic respawn",
                        missing
                    ),
                    next_step: format!("Inspect /civ roster {} and reconcile only known legacy actors; never spawn blindly", self.name),
                });
            }
        }

        let (_, _, homeless) = self.physical_housing();
        if homeless > 0 {
            let (urgency, reason, next_step) = if let Some(job) = &self.visual_job {
                (45, format!(
                    "{} residents lack completed physical hut slots; active build at {:?}{}",
                    homeless, job.origin, if job.blocked { " is blocked" } else { "" }
                ), format!("Inspect /civ construction {}; do not approve another site while a job is active", self.name))
            } else if self.pending_visual_buildings > 0 {
                (65, format!(
                    "{} residents lack physical housing; {} abstract house(s) ready for visual construction",
                    homeless, self.pending_visual_buildings
                ), format!("Inspect /civ survey {}; OP approval is required before building", self.name))
            } else {
                (40, format!(
                    "{} residents lack completed physical hut slots; no abstract house awaits visualization",
                    homeless
                ), format!("Inspect /civ homes {}; abstract houses cannot be auto-converted to physical huts", self.name))
            };
            priorities.push(CivicPriority {
                need: CivicNeed::Housing, urgency, reason, next_step,
            });
        }

        let desired_houses = population.div_ceil(3).saturating_add(1);
        let wood = self.quantity(Good::Wood);
        let stone = self.quantity(Good::Stone);
        if self.houses < desired_houses && self.count(Job::Builder) > 0
            && (wood < 8 || stone < 6) {
            priorities.push(CivicPriority {
                need: CivicNeed::Materials, urgency: 60,
                reason: format!(
                    "future abstract house needs 8 wood/6 stone; available {wood} wood/{stone} stone"
                ),
                next_step: format!(
                    "Inspect /civ info {}; existing physical job materials were already paid in the abstract simulation",
                    self.name
                ),
            });
        }
        priorities.sort_by(|a, b| b.urgency.cmp(&a.urgency).then(a.need.cmp(&b.need)));
        priorities.truncate(5);
        priorities
    }

    pub fn wheat_delivery_pending(&self) -> u32 { self.wheat_in_transit }

    /// Called only after the native server sees an actual UUID-bound
    /// farmer return to the hall. Same world-JSON save commits the pending
    /// receipt and delivered stock; no second crop receipt is created.
    pub fn deliver_farm_wheat(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown village")?;
        if v.wheat_in_transit == 0 { return Ok(false); }
        let next = v.warehouse_quantity(Good::Wheat)
            .checked_add(v.wheat_in_transit).ok_or("wheat delivery overflow")?;
        if next > 10_000_000 { return Err("village wheat receiving capacity exceeded".into()); }
        let delivered = v.wheat_in_transit;
        v.warehouse.insert(Good::Wheat, next);
        v.wheat_in_transit = 0;
        let name = v.name.clone();
        self.event(format!("{name}: farmer brought {delivered} verified wheat to the hall"));
        Ok(true)
    }

    pub fn warehouse_quantity(&self, good: Good) -> u32 {
        self.warehouse.get(&good).copied().unwrap_or(0)
    }

    /// Physical receipt; reject overflow rather than silently mint or lose items.
    pub fn warehouse_credit(&mut self, good: Good, count: u32) -> Result<(), String> {
        let next = self.warehouse_quantity(good).checked_add(count)
            .ok_or("physical warehouse capacity overflow")?;
        if next > 10_000_000 { return Err("physical warehouse capacity exceeded".into()); }
        self.warehouse.insert(good, next);
        Ok(())
    }

    pub fn warehouse_debit(&mut self, good: Good, count: u32) -> Result<(), String> {
        let current = self.warehouse_quantity(good);
        if current < count { return Err(format!(
            "warehouse lacks {} {}: {} available, {} required",
            good.as_str(), current, count
        )); }
        self.warehouse.insert(good, current - count);
        Ok(())
    }

    pub fn hall_materials_missing(&self) -> (u32, u32) {
        (HALL_WOOD_LOGS.saturating_sub(self.warehouse_quantity(Good::Wood)),
         HALL_COBBLESTONE.saturating_sub(self.warehouse_quantity(Good::Stone)))
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
        // Homes sold to players or commissioned privately are no longer
        // public stock. The abstract economy must eventually replace them
        // using its normal wood/stone costs rather than mint free houses.
        let private_houses = self.player_parcels.iter()
            .filter(|plot| plot.commissioned).count() as u32;
        let public_houses = self.houses.saturating_sub(private_houses);
        if self.count(Job::Builder) > 0 && self.population() > 0 && public_houses < desired_houses {
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
        let mut all_entity_uuids = BTreeSet::new();
        let mut all_parcels: Vec<(String, [i32; 3])> = Vec::new();
        for (&id, village) in &self.settlements {
            if village.id != id { return Err("settlement id/key mismatch".into()); }
            let normalized = village.name.to_ascii_lowercase();
            if !seen.insert(normalized) { return Err("duplicate settlement names".into()); }
            if let Some(stem) = &village.oak_stem {
                let gather = village.gather_job.as_ref()
                    .ok_or("oak stem saved without an active physical gather")?;
                let expected_y = stem.root[1].checked_add(i32::from(stem.cursor))
                    .ok_or("oak stem Y coordinate overflow")?;
                if !(3..=4).contains(&stem.height) || stem.cursor >= stem.height
                    || gather.good != Good::Wood
                    || gather.position != [stem.root[0], expected_y, stem.root[2]]
                    || gather.stand != stem.stand {
                    return Err("invalid saved short oak stem continuation".into());
                }
            }
            if let Some(gather) = &village.gather_job {
                if let Some(worker_id) = gather.worker_id {
                    let role = if gather.good == Good::Wood { Job::Lumberjack }
                        else { Job::Miner };
                    if !village.residents.iter().any(|r|
                        r.id == worker_id && r.job == role) {
                        return Err("physical gather worker ID is invalid".into());
                    }
                }
                if gather.native_edit_confirmed
                    && (!gather.issued || gather.worker_id.is_none()) {
                    return Err("native edit receipt without issued worker identity".into());
                }
                // Historic PR #30 saves can contain a witnessed pickup
                // without the new post-command acknowledgement field.
                // Accept that save structurally so the whole settlement
                // remains loadable; the native adapter QUARANTINES this
                // ambiguous Issued job before processing any more receipts.
                // Never infer native_edit_confirmed from pickup_witnessed.
                if gather.pickup_witnessed
                    && (!gather.issued || gather.worker_id.is_none()) {
                    return Err("pickup witness without issued worker identity".into());
                }
                let [x, y, z] = gather.position;
                let [sx, sy, sz] = gather.stand;
                let walk_dx = i128::from(sx) - i128::from(x);
                let walk_dy = i128::from(sy) - i128::from(y);
                let walk_dz = i128::from(sz) - i128::from(z);
                let village_dx = i128::from(x) - i128::from(village.center[0]);
                let village_dz = i128::from(z) - i128::from(village.center[2]);
                if !matches!(gather.good, Good::Wood | Good::Stone)
                    || i64::from(x).abs() > 29_999_988
                    || i64::from(z).abs() > 29_999_988
                    || !(-2046..=2046).contains(&y)
                    || i64::from(sx).abs() > 29_999_988
                    || i64::from(sz).abs() > 29_999_988
                    || !(-2046..=2046).contains(&sy)
                    || walk_dx * walk_dx + walk_dy * walk_dy + walk_dz * walk_dz > 16
                    || !(12_i128 * 12..=80_i128 * 80)
                        .contains(&(village_dx * village_dx + village_dz * village_dz)) {
                    return Err("invalid physical gather record".into());
                }
            }
            if let Some(purchase) = &village.market_purchase {
                if !village.hall_complete
                    || !hall_storage_positions(village.center).contains(&purchase.position)
                    || !(1..=64).contains(&purchase.count) || purchase.count > purchase.before
                    || purchase.price != u64::from(purchase.good.price()) * u64::from(purchase.count)
                    || !self.players.contains_key(&purchase.buyer) {
                    return Err("invalid pending native market purchase".into());
                }
            }
            if let Some(b) = &village.bakery {
                if !village.hall_complete
                    || i64::from(b.origin[0]).abs() > 29_999_988
                    || i64::from(b.origin[2]).abs() > 29_999_988
                    || !(-2046..=2046).contains(&b.origin[1])
                    || b.wheat_escrow > BREAD_WHEAT_RECIPE || b.bread_in_transit > 8
                    || b.construction_escrow.values().any(|&amount| amount > BAKERY_COBBLESTONE)
                    || (b.complete && (b.building.is_some() || !b.supplies_reserved))
                    || (!b.complete && b.building.is_none())
                    || b.building.as_ref().is_some_and(|job|
                        job.origin != b.origin || job.cursor > bakery_plan(b.origin).len()) {
                    return Err("invalid saved physical bakery state".into());
                }
            }
            if village.wheat_in_transit > 10_000_000 {
                return Err("wheat in physical transit exceeds cap".into());
            }
            if village.build_escrow.values().any(|&amount| amount > 10_000_000) {
                return Err("physical builder escrow entry too large".into());
            }
            if let Some(t) = &village.barrel_transfer {
                if !village.hall_complete || !hall_storage_positions(village.center).contains(&t.position)
                    || t.before > 10_000_000 || !(1..=8).contains(&t.count)
                    || t.expected_after().is_none() {
                    return Err("invalid official barrel transfer record".into());
                }
                match t.direction {
                    BarrelDirection::ToBuilder if t.slot.is_none()
                        || !matches!(t.good, Good::Wood | Good::Stone) =>
                        return Err("invalid builder barrel extraction slot".into()),
                    BarrelDirection::ToBarrel if t.slot.is_some() =>
                        return Err("native barrel deposit cannot name a slot".into()),
                    _ => {}
                }
                match t.direction {
                    BarrelDirection::ToBakeryBuilder if t.slot.is_none()
                        || !matches!(t.good, Good::Wood | Good::Stone)
                        || village.bakery.as_ref().is_none_or(|b|
                            b.supplies_reserved || b.complete
                                || b.building.as_ref().is_none_or(|job| job.blocked)) =>
                        return Err("invalid native bakery material extraction".into()),
                    BarrelDirection::ToBakerIngredients if t.slot.is_none()
                        || t.good != Good::Wheat
                        || village.bakery.as_ref().is_none_or(|b|
                            !b.complete || b.bread_in_transit != 0
                                || b.wheat_escrow.checked_add(t.count)
                                    .is_none_or(|n| n > BREAD_WHEAT_RECIPE)) =>
                        return Err("invalid native baker wheat extraction".into()),
                    _ => {}
                }
            }
            if village.hall_complete && village.hall_job.is_some() {
                return Err("civic hall cannot be simultaneously complete and under construction".into());
            }
            if let Some(job) = &village.hall_job {
                if job.origin != village.center
                    || job.cursor > town_hall_plan(job.origin).len() {
                    return Err("civic hall cursor/site invalid".into());
                }
            }
            for &amount in village.warehouse.values() {
                if amount > 10_000_000 { return Err("physical warehouse entry too large".into()); }
            }
            if village.player_parcels.len() > MAX_PLAYER_PARCELS { return Err("too many player parcels".into()); }
            for parcel in &village.player_parcels {
                if parcel.owner.is_empty() || !self.players.contains_key(&parcel.owner) {
                    return Err("parcel owner has no player record".into());
                }
                let [x, y, z] = parcel.origin;
                if i64::from(x).abs() > 29_999_990 || i64::from(z).abs() > 29_999_990
                    || !(-2048..=2048).contains(&y) {
                    return Err("parcel outside world bounds".into());
                }
                if all_parcels.iter().any(|(dimension, point)| {
                    dimension == &village.dimension && near_plot(*point, parcel.origin)
                }) {
                    return Err("overlapping player land parcels".into());
                }
                if parcel.commissioned
                    && !village.visual_sites.contains(&parcel.origin)
                    && !village.visual_job.as_ref().is_some_and(|job| job.origin == parcel.origin) {
                    return Err("commissioned parcel has no recorded house job or completed hut".into());
                }
                all_parcels.push((village.dimension.clone(), parcel.origin));
            }
            if village.residents.len() > 128 { return Err("resident cap exceeded".into()); }
            if !village.actor_recovery_issued.is_empty() && !village.actors_spawned {
                return Err("recovery attempts exist before actor spawn reservation".into());
            }
            for recovery_id in &village.actor_recovery_issued {
                if !village.residents.iter().any(|r|
                    r.id == *recovery_id && r.alive && r.entity_uuid.is_none()) {
                    return Err("invalid pending actor recovery ID or already bound actor".into());
                }
            }
            let mut resident_ids = BTreeSet::new();
            for resident in &village.residents {
                if !resident_ids.insert(resident.id) {
                    return Err("duplicate resident ID".into());
                }
                if let Some(uuid) = resident.entity_uuid {
                    if uuid == 0 || !village.actors_spawned {
                        return Err("entity UUID is invalid for this settlement".into());
                    }
                    if !all_entity_uuids.insert(uuid) {
                        return Err("duplicate actor entity UUID across settlements".into());
                    }
                }
            }
            if let Some(plot) = &village.farm {
                let [x, y, z] = plot.origin;
                if i64::from(x).abs() > 29_999_988 || i64::from(z).abs() > 29_999_988
                    || !(-2046..=2046).contains(&y) {
                    return Err("farm outside world bounds".into());
                }
                let count = if plot.stage == FarmStage::Preparing { farm_plan(plot.origin).len() } else { 8 };
                if plot.cursor > count { return Err("farm cursor beyond phase".into()); }
                if plot.completed_cycles > 1_000_000 { return Err("farm cycle counter invalid".into()); }
                if plot.harvest_receipts.len() > MAX_HARVEST_RECEIPTS {
                    return Err("farm harvest receipt history too large".into());
                }
                let mut previous = 0_u64;
                for receipt in &plot.harvest_receipts {
                    if receipt.sequence <= previous
                        || receipt.sequence > plot.harvest_sequence
                        || receipt.crops != 8
                        || receipt.world_day > self.day {
                        return Err("farm receipt invalid or out of order".into());
                    }
                    previous = receipt.sequence;
                }
                if plot.harvest_receipts.last().is_some_and(|r| r.sequence != plot.harvest_sequence) {
                    return Err("farm receipt tail does not match sequence".into());
                }
            }
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
            if village.hut_supplies_reserved && village.visual_job.is_none() {
                return Err("reserved hut materials without an active job".into());
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
            entity_uuid: None,
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
            // Founding abstract stock is NOT physical inventory.
            warehouse: BTreeMap::new(),
            build_escrow: BTreeMap::new(),
            barrel_transfer: None,
            market_purchase: None,
            wheat_in_transit: 0,
            bakery: None,
            hall_job: None,
            hall_complete: false,
            gather_job: None,
            oak_stem: None,
            treasury_coins: 0,
            houses: 2,
            building_progress: 0,
            starving_days: 0,
            morale: 50,
            relationships: BTreeMap::new(),
            active_wars: BTreeSet::new(),
            pending_visual_buildings: 0,
            actors_spawned: false,
            actor_recovery_issued: BTreeSet::new(),
            autonomy_enabled: true,
            player_parcels: Vec::new(),
            visual_sites: Vec::new(),
            visual_job: None,
            hut_supplies_reserved: false,
            farm: None,
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

    /// Domain-only collision policy; physical construction still separately
    /// requires clear loaded terrain. Player parcels are hard exclusions.
    pub fn check_visual_house_site(&self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        self.check_visual_house_site_for(village_id, origin, None, true)
    }

    /// A commission may use *only its authenticated owner's* bought lot.
    /// All other protections and structure separation remain in force.
    fn check_visual_house_site_for(
        &self, village_id: u64, origin: [i32; 3],
        owner: Option<&str>, require_budget: bool
    ) -> Result<(), String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if require_budget && village.pending_visual_buildings == 0 {
            return Err("no completed abstract house awaits visualization".into());
        }
        if village.visual_job.is_some() {
            return Err("construction already in progress".into());
        }
        if village.visual_sites.len() >= 32 {
            return Err("physical housing site limit reached".into());
        }
        if i64::from(origin[0]).abs() > 29_999_990 || i64::from(origin[2]).abs() > 29_999_990
            || !(-2048..=2048).contains(&origin[1]) {
            return Err("site too close to world border".into());
        }
        // i32 coordinates may differ by 2^32-1. Squaring two such deltas
        // overflows i64; use i128 even for corrupted or untrusted save data.
        let distance = |a: [i32; 3], b: [i32; 3]| {
            let dx = i128::from(a[0]) - i128::from(b[0]);
            let dz = i128::from(a[2]) - i128::from(b[2]);
            dx * dx + dz * dz
        };
        if !(12_i128 * 12..=80_i128 * 80).contains(&distance(village.center, origin)) {
            return Err("build 12..80 blocks from village center".into());
        }
        for other in self.settlements.values().filter(|s| s.dimension == village.dimension) {
            if other.player_parcels.iter().any(|plot| {
                near_plot(plot.origin, origin) && owner != Some(plot.owner.as_str())
            }) {
                return Err("plot is owned by a player; construction requires its owner's commission".into());
            }
            if other.visual_sites.iter().any(|&site| distance(site, origin) < 9_i128 * 9)
                || other.visual_job.as_ref().is_some_and(|job| distance(job.origin, origin) < 9_i128 * 9)
                || other.farm.as_ref().is_some_and(|farm| distance(farm.origin, origin) < 9_i128 * 9)
                || other.bakery.as_ref().is_some_and(|b| distance(b.origin, origin) < 9_i128 * 9) {
                return Err("plot overlaps an existing civilization structure or farm".into());
            }
        }
        Ok(())
    }

    /// A read-only village proposal. Slot is 1-based and always resolves to
    /// the same X/Z for this settlement. The operator supplies the exact Y
    /// level after a *live* survey; nothing is reserved by this method.
    pub fn proposed_hut(&self, village_id: u64, slot: usize, y: i32) -> Result<[i32; 3], String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if village.physical_housing().2 == 0 {
            return Err("all residents already have physical hut capacity".into());
        }
        let [dx, dz] = *HOUSING_PLOT_OFFSETS.get(slot.checked_sub(1).ok_or("invalid housing slot")?)
            .ok_or("invalid housing slot")?;
        let x = i64::from(village.center[0]) + i64::from(dx);
        let z = i64::from(village.center[2]) + i64::from(dz);
        let x = i32::try_from(x).map_err(|_| "housing candidate outside world bounds")?;
        let z = i32::try_from(z).map_err(|_| "housing candidate outside world bounds")?;
        let origin = [x, y, z];
        self.check_visual_house_site(village_id, origin)?;
        Ok(origin)
    }

    /// The default-on village reserves a 5x5 public bakery plot after its
    /// hall is constructed. The native server must independently preflight
    /// loaded real ground and clear air before calling this domain method.
    pub fn start_bakery(&mut self, id: u64, origin: [i32; 3]) -> Result<(), String> {
        let village = self.settlements.get(&id).ok_or("unknown village")?;
        if !village.autonomy_enabled || !village.hall_complete {
            return Err("village needs a completed hall and active autonomy".into());
        }
        if village.bakery.is_some() { return Err("bakery already reserved or constructed".into()); }
        if village.count(Job::Builder) == 0 || village.count(Job::Baker) == 0 {
            return Err("bakery requires a living builder and baker".into());
        }
        if i64::from(origin[0]).abs() > 29_999_988
            || i64::from(origin[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&origin[1]) {
            return Err("bakery plot outside safe construction bounds".into());
        }
        let dx = i128::from(origin[0]) - i128::from(village.center[0]);
        let dz = i128::from(origin[2]) - i128::from(village.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("bakery must be 12..80 blocks from its hall".into());
        }
        for other in self.settlements.values().filter(|s| s.dimension == village.dimension) {
            if near_plot(other.center, origin)
                || other.player_parcels.iter().any(|p| near_plot(p.origin, origin))
                || other.visual_sites.iter().any(|s| near_plot(*s, origin))
                || other.visual_job.as_ref().is_some_and(|job| near_plot(job.origin, origin))
                || other.farm.as_ref().is_some_and(|p| near_plot(p.origin, origin))
                || other.bakery.as_ref().is_some_and(|b| near_plot(b.origin, origin)) {
                return Err("bakery would overlap an owned plot or another physical structure".into());
            }
        }
        let v = self.settlements.get_mut(&id).unwrap();
        v.bakery = Some(BakeryWorkshop {
            origin,
            building: Some(VisualBuildJob { origin, cursor: 0, blocked: false }),
            complete: false, supplies_reserved: false,
            construction_escrow: BTreeMap::new(),
            wheat_escrow: 0, bread_in_transit: 0, batches: 0,
        });
        let name = v.name.clone();
        self.event(format!("{name}: planned real public bakery at {origin:?}"));
        Ok(())
    }

    /// Materials are physically extracted from real hall barrels by the
    /// native bridge BEFORE this call, into the bakery-specific escrow.
    pub fn reserve_bakery_supplies(&mut self, id: u64) -> Result<bool, String> {
        let bakery = self.settlements.get_mut(&id).ok_or("unknown village")?
            .bakery.as_mut().ok_or("no bakery project")?;
        if bakery.complete || bakery.supplies_reserved { return Ok(false); }
        let job = bakery.building.as_ref().ok_or("no bakery construction job")?;
        if job.blocked || job.cursor != 0 {
            return Err("unfunded partial bakery job must be inspected".into());
        }
        if bakery.construction_escrow.get(&Good::Wood).copied().unwrap_or(0) < BAKERY_WOOD_LOGS
            || bakery.construction_escrow.get(&Good::Stone).copied().unwrap_or(0) < BAKERY_COBBLESTONE {
            return Ok(false);
        }
        *bakery.construction_escrow.get_mut(&Good::Wood).unwrap() -= BAKERY_WOOD_LOGS;
        *bakery.construction_escrow.get_mut(&Good::Stone).unwrap() -= BAKERY_COBBLESTONE;
        bakery.supplies_reserved = true;
        Ok(true)
    }

    pub fn finish_bakery(&mut self, id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown village")?;
        let bakery = v.bakery.as_mut().ok_or("no bakery")?;
        let job = bakery.building.as_ref().ok_or("no bakery construction job")?;
        if bakery.complete || !bakery.supplies_reserved
            || job.cursor != bakery_plan(job.origin).len() {
            return Err("unfunded or physically incomplete bakery cannot open".into());
        }
        bakery.building = None;
        bakery.complete = true;
        let name = v.name.clone();
        self.event(format!("{name}: verified bakery opened at {:?}", bakery.origin));
        Ok(())
    }

    /// At the physically present crafting table, turn three actually
    /// extracted native wheat items into ONE bread. Output is a saved
    /// delivery receipt, not an abstract daily-production bonus.
    pub fn craft_bread(&mut self, id: u64) -> Result<bool, String> {
        let village = self.settlements.get_mut(&id).ok_or("unknown village")?;
        if village.count(Job::Baker) == 0 { return Ok(false); }
        let b = village.bakery.as_mut().ok_or("no bakery")?;
        if !b.complete || b.wheat_escrow < BREAD_WHEAT_RECIPE
            || b.bread_in_transit >= 8 { return Ok(false); }
        let next = b.batches.checked_add(1).ok_or("bread production counter overflow")?;
        b.wheat_escrow -= BREAD_WHEAT_RECIPE;
        b.bread_in_transit += 1;
        b.batches = next;
        let name = village.name.clone();
        self.event(format!("{name}: real baker made one bread from three native wheat items"));
        Ok(true)
    }

    /// A UUID-bound baker must first return to the hall; then their
    /// completed bread enters the steward's bounded delivery queue.
    pub fn deliver_bakery_bread(&mut self, id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown village")?;
        let pending = v.bakery.as_ref().ok_or("no bakery")?.bread_in_transit;
        if pending == 0 { return Ok(false); }
        v.warehouse_credit(Good::Bread, pending)?;
        let b = v.bakery.as_mut().unwrap();
        b.bread_in_transit = 0;
        let name = v.name.clone();
        self.event(format!("{name}: baker delivered {pending} crafted bread to the hall"));
        Ok(true)
    }

    /// Reserve the actual resource budget for the village's one central hall.
    /// No free town hall spawns in a new or migrated world.
    pub fn reserve_town_hall(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if !v.autonomy_enabled || v.hall_complete || v.hall_job.is_some() { return Ok(false); }
        if v.hall_materials_missing() != (0, 0) {
            return Ok(false);
        }
        if i64::from(v.center[0]).abs() > 29_999_988
            || i64::from(v.center[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&v.center[1]) {
            return Err("civic center outside safe build bounds".into());
        }
        // Domain claims are only a first pass; native preflight checks all
        // world blocks, loaded chunks and a safe location BEFORE this call.
        if self.settlements.values().filter(|s| s.dimension == v.dimension).any(|s| {
            s.player_parcels.iter().any(|plot| near_plot(plot.origin, v.center))
            || s.visual_sites.iter().any(|site| near_plot(*site, v.center))
            || s.visual_job.as_ref().is_some_and(|job| near_plot(job.origin, v.center))
            || s.farm.as_ref().is_some_and(|farm| near_plot(farm.origin, v.center))
            || s.bakery.as_ref().is_some_and(|b| near_plot(b.origin, v.center))
        }) {
            return Err("civic hall site overlaps a registered building or player parcel".into());
        }
        let v = self.settlements.get_mut(&village_id).unwrap();
        v.warehouse_debit(Good::Wood, HALL_WOOD_LOGS)?;
        v.warehouse_debit(Good::Stone, HALL_COBBLESTONE)?;
        let center = v.center;
        v.hall_job = Some(VisualBuildJob { origin: center, cursor: 0, blocked: false });
        let name = v.name.clone();
        self.event(format!("{name}: civic hall construction funded and started at {center:?}"));
        Ok(true)
    }

    /// A full native block read is mandatory before recording completion.
    pub fn finish_town_hall(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.hall_job.as_ref().ok_or("no civic hall construction")?;
        if job.cursor != town_hall_plan(job.origin).len() {
            return Err("civic hall building plan incomplete".into());
        }
        let name = v.name.clone();
        v.hall_job = None;
        v.hall_complete = true;
        self.event(format!("{name}: physical civic hall completed"));
        Ok(())
    }

    /// Prepare one and only one actual vanilla barrel mutation; native code
    /// must read the exact barrel count/slot before calling. Planning itself
    /// creates no item and deducts no warehouse quantity.
    pub fn plan_barrel_transfer(
        &mut self, id: u64, position: [i32; 3], good: Good,
        direction: BarrelDirection, before: u32, count: u32, slot: Option<usize>,
    ) -> Result<(), String> {
        let v = self.settlements.get(&id).ok_or("unknown settlement")?;
        if !v.hall_complete { return Err("physical town hall not complete".into()); }
        if v.barrel_transfer.is_some() || v.market_purchase.is_some() {
            return Err("barrel transfer or player purchase already pending".into());
        }
        if !hall_storage_positions(v.center).contains(&position) {
            return Err("barrel is not an official village hall warehouse".into());
        }
        if before > 10_000_000 || !(1..=8).contains(&count) {
            return Err("invalid native barrel batch or item count".into());
        }
        match direction {
            BarrelDirection::ToBarrel => {
                if v.warehouse_quantity(good) < count {
                    return Err("insufficient confirmed goods awaiting barrel delivery".into());
                }
                if slot.is_some() { return Err("deposit is not a slot extraction".into()); }
                before.checked_add(count).ok_or("barrel count overflow")?;
            }
            BarrelDirection::ToBuilder => {
                if v.visual_job.is_none() || v.hut_supplies_reserved {
                    return Err("no unfunded physical hut work order".into());
                }
                let required = match good {
                    Good::Wood => HUT_WOOD_LOGS,
                    Good::Stone => HUT_COBBLESTONE,
                    _ => return Err("builders only reserve verified wood and cobblestone".into()),
                };
                if v.build_escrow.get(&good).copied().unwrap_or(0)
                    .checked_add(count).is_none_or(|n| n > required) {
                    return Err("building material batch exceeds project bill".into());
                }
                if before < count || slot.is_none() {
                    return Err("missing native barrel item/slot for extraction".into());
                }
            }
            BarrelDirection::ToBakeryBuilder => {
                let bakery = v.bakery.as_ref().ok_or("no planned bakery")?;
                if bakery.building.as_ref().is_none_or(|job| job.blocked)
                    || bakery.supplies_reserved || bakery.complete {
                    return Err("no valid unfunded physical bakery project".into());
                }
                let required = match good {
                    Good::Wood => BAKERY_WOOD_LOGS,
                    Good::Stone => BAKERY_COBBLESTONE,
                    _ => return Err("bakery construction requires wood or cobblestone".into()),
                };
                if bakery.construction_escrow.get(&good).copied().unwrap_or(0)
                    .checked_add(count).is_none_or(|n| n > required) {
                    return Err("bakery materials exceed the real construction bill".into());
                }
                if before < count || slot.is_none() {
                    return Err("no verified native barrel slot for bakery supplies".into());
                }
            }
            BarrelDirection::ToBakerIngredients => {
                let bakery = v.bakery.as_ref().ok_or("no bakery")?;
                if !bakery.complete || good != Good::Wheat || slot.is_none()
                    || before < count || bakery.wheat_escrow
                        .checked_add(count).is_none_or(|n| n > BREAD_WHEAT_RECIPE)
                    || bakery.bread_in_transit != 0 {
                    return Err("baker cannot take additional unverified wheat".into());
                }
            }

        }
        self.settlements.get_mut(&id).unwrap().barrel_transfer = Some(BarrelTransfer {
            position, good, direction, phase: BarrelPhase::Planned, before, count, slot,
        });
        Ok(())
    }

    /// Persist this state BEFORE inserting or extracting from native barrels.
    /// Issued transactions must NEVER be dispatched again after a reload.
    pub fn issue_barrel_transfer(&mut self, id: u64) -> Result<(), String> {
        let transfer = self.settlements.get_mut(&id).ok_or("unknown settlement")?
            .barrel_transfer.as_mut().ok_or("no barrel transfer")?;
        if transfer.phase != BarrelPhase::Planned {
            return Err("barrel transfer already issued/blocked".into());
        }
        transfer.phase = BarrelPhase::Issued;
        Ok(())
    }

    /// Compare a later native snapshot of precisely the same barrel.
    pub fn confirm_barrel_transfer(&mut self, id: u64, after: u32) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        let t = v.barrel_transfer.as_ref().ok_or("no barrel transfer")?;
        if t.phase != BarrelPhase::Issued || t.expected_after() != Some(after) {
            return Err("barrel item count does not match the issued transfer".into());
        }
        let good = t.good;
        let direction = t.direction;
        let count = t.count;
        match direction {
            BarrelDirection::ToBarrel => v.warehouse_debit(good, count)?,
            BarrelDirection::ToBuilder => {
                let previous = v.build_escrow.get(&good).copied().unwrap_or(0);
                let next = previous.checked_add(count).ok_or("building escrow overflow")?;
                v.build_escrow.insert(good, next);
            }
            BarrelDirection::ToBakeryBuilder => {
                let b = v.bakery.as_mut().ok_or("no bakery")?;
                let prev = b.construction_escrow.get(&good).copied().unwrap_or(0);
                b.construction_escrow.insert(good, prev.checked_add(count)
                    .ok_or("bakery materials overflow")?);
            }
            BarrelDirection::ToBakerIngredients => {
                let b = v.bakery.as_mut().ok_or("no bakery")?;
                b.wheat_escrow = b.wheat_escrow.checked_add(count)
                    .ok_or("baker wheat carry overflow")?;
            }
        }
        v.barrel_transfer = None;
        Ok(())
    }

    /// A refused/ambiguous external mutation never retries automatically.
    pub fn block_barrel_transfer(&mut self, id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        let t = v.barrel_transfer.as_mut().ok_or("no barrel transfer")?;
        t.phase = BarrelPhase::Blocked;
        Ok(())
    }

    /// An operator may cancel ONLY a never-issued Planned intent, after the
    /// adapter compares a loaded official native barrel's exact item count to
    /// the saved before-count. No external mutation is possible before Issued
    /// is durably persisted by the normal bridge.
    pub fn cancel_planned_barrel_transfer(&mut self, id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        if v.market_purchase.is_some() {
            return Err("cannot cancel while a player purchase is unresolved".into());
        }
        let t = v.barrel_transfer.as_ref().ok_or("no warehouse transfer")?;
        if t.phase != BarrelPhase::Planned {
            return Err("only a never-issued planned transfer can be canceled".into());
        }
        v.barrel_transfer = None;
        let name = v.name.clone();
        self.event(format!("{name}: operator canceled an unissued native barrel plan"));
        Ok(())
    }

    /// An operator may explicitly reconcile an already BLOCKED receipt only
    /// after confirming the exact expected native after-count via a fresh
    /// loaded snapshot. No item is inserted/extracted by this operation.
    /// We reuse the normal once-only receipt transition and rollback on any
    /// domain error; this NEVER retries an uncertain native API call.
    pub fn reconcile_blocked_barrel_transfer(
        &mut self, id: u64, observed_after: u32,
    ) -> Result<(), String> {
        let v = self.settlements.get(&id).ok_or("unknown settlement")?;
        if v.market_purchase.is_some() {
            return Err("cannot reconcile while player market purchase is unresolved".into());
        }
        let t = v.barrel_transfer.as_ref().ok_or("no warehouse transfer")?;
        if t.phase != BarrelPhase::Blocked
            || t.expected_after() != Some(observed_after) {
            return Err("blocked receipt does not match the verified native count".into());
        }
        let backup = self.clone();
        self.settlements.get_mut(&id).unwrap().barrel_transfer.as_mut().unwrap().phase =
            BarrelPhase::Issued;
        if let Err(error) = self.confirm_barrel_transfer(id, observed_after) {
            *self = backup;
            return Err(error);
        }
        let name = self.settlements[&id].name.clone();
        self.event(format!(
            "{name}: operator reconciled one blocked barrel receipt against actual native inventory"
        ));
        Ok(())
    }

    /// Called only after the adapter has checked a live barrel slot, the
    /// authenticated player identity, and available inventory space.
    /// Coins are durably debited BEFORE the native item extraction.
    pub fn begin_market_purchase(
        &mut self, buyer: &str, id: u64, good: Good, count: u32,
        position: [i32; 3], slot: usize, before: u32,
    ) -> Result<u64, String> {
        if !(1..=64).contains(&count) || count > before {
            return Err("purchase exceeds one visible native stack".into());
        }
        let v = self.settlements.get(&id).ok_or("unknown settlement")?;
        if !v.hall_complete { return Err("real warehouse is not constructed".into()); }
        if !hall_storage_positions(v.center).contains(&position) {
            return Err("purchase source is not an official town hall barrel".into());
        }
        if v.barrel_transfer.is_some() || v.market_purchase.is_some() {
            return Err("warehouse has an outstanding transfer or purchase".into());
        }
        let person = self.players.get(buyer).ok_or("join village before buying")?;
        if person.settlement != Some(id) { return Err("join this village before buying".into()); }
        let price = u64::from(good.price()) * u64::from(count);
        if person.coins < price { return Err("insufficient civic coins".into()); }
        let treasury = v.treasury_coins.checked_add(price).ok_or("treasury coin overflow")?;
        self.players.get_mut(buyer).unwrap().coins -= price;
        let v = self.settlements.get_mut(&id).unwrap();
        v.treasury_coins = treasury;
        v.market_purchase = Some(MarketPurchase {
            buyer: buyer.to_owned(), good, count, price, position,
            slot, before, phase: MarketPhase::Issued,
        });
        Ok(price)
    }

    /// Only clear a pending purchase after the external native barrel item
    /// was removed and the exact item was handed to the authenticated buyer.
    pub fn complete_market_purchase(&mut self, id: u64, buyer: &str) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        let order = v.market_purchase.as_ref().ok_or("no physical market purchase")?;
        if order.buyer != buyer || order.phase != MarketPhase::Issued {
            return Err("buyer identity or market state mismatch".into());
        }
        let good = order.good;
        let count = order.count;
        v.market_purchase = None;
        let name = v.name.clone();
        self.event(format!("{name}: player acquired {count} real {} from a hall barrel",
            good.as_str()));
        Ok(())
    }

    pub fn block_market_purchase(&mut self, id: u64) -> Result<(), String> {
        let order = self.settlements.get_mut(&id).ok_or("unknown settlement")?
            .market_purchase.as_mut().ok_or("no market order")?;
        order.phase = MarketPhase::Blocked;
        Ok(())
    }

    /// Re-evaluate live settlement ownership before executing an already
    /// planned physical resource action. Land may be bought or new buildings
    /// reserved between the initial plan and the worker reaching its target.
    /// This is shared with planning so neither path can bypass a new parcel.
    pub fn gather_site_unclaimed(
        &self, village_id: u64, position: [i32; 3], stand: [i32; 3],
    ) -> bool {
        let Some(village) = self.settlements.get(&village_id) else { return false; };
        let protected = |other: &Settlement| {
            near_plot(other.center, position) || near_plot(other.center, stand)
                || other.player_parcels.iter().any(|lot|
                    near_plot(lot.origin, position) || near_plot(lot.origin, stand))
                || other.visual_sites.iter().any(|site|
                    near_plot(*site, position) || near_plot(*site, stand))
                || other.visual_job.as_ref().is_some_and(|job|
                    near_plot(job.origin, position) || near_plot(job.origin, stand))
                || other.farm.as_ref().is_some_and(|farm|
                    near_plot(farm.origin, position) || near_plot(farm.origin, stand))
                || other.bakery.as_ref().is_some_and(|b|
                    near_plot(b.origin, position) || near_plot(b.origin, stand))
        };
        !self.settlements.values().filter(|other| other.dimension == village.dimension)
            .any(protected)
    }

    /// Planning creates no items. Both native and domain checks forbid
    /// extraction inside the town hall, another structure or player lot.
    pub fn plan_gather(
        &mut self, village_id: u64, good: Good, position: [i32; 3], stand: [i32; 3]
    ) -> Result<(), String> {
        if !matches!(good, Good::Wood | Good::Stone) {
            return Err("only physical oak logs or exposed stone may be gathered".into());
        }
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if v.gather_job.is_some() { return Err("another physical gather is pending".into()); }
        if !v.autonomy_enabled { return Err("village autonomy paused".into()); }
        let worker_job = if good == Good::Wood { Job::Lumberjack } else { Job::Miner };
        if v.count(worker_job) == 0 { return Err("no qualified living worker".into()); }
        if i64::from(position[0]).abs() > 29_999_988
            || i64::from(position[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&position[1]) {
            return Err("gather point outside safe world bounds".into());
        }
        let wx = i128::from(stand[0]) - i128::from(position[0]);
        let wy = i128::from(stand[1]) - i128::from(position[1]);
        let wz = i128::from(stand[2]) - i128::from(position[2]);
        if wx * wx + wy * wy + wz * wz > 16 {
            return Err("worker stand is not reachable from resource within 4 blocks".into());
        }
        let dx = i128::from(position[0]) - i128::from(v.center[0]);
        let dz = i128::from(position[2]) - i128::from(v.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("gather only in the village outer work area".into());
        }
        if !self.gather_site_unclaimed(village_id, position, stand) {
            return Err("natural resource overlaps a building, town hall or player claim".into());
        }
        self.settlements.get_mut(&village_id).unwrap().gather_job = Some(GatherJob {
            position, stand, good, worker_id: None, issued: false,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: false,
        });
        Ok(())
    }

    /// Start a whole SHORT tree only after the adapter observed all native
    /// trunk and crown blocks. No wood is credited until individual segments
    /// have actually disappeared and the bound lumberjack returns to hall.
    pub fn plan_short_oak_stem(
        &mut self, village_id: u64, root: [i32; 3], stand: [i32; 3], height: u8,
    ) -> Result<(), String> {
        if !(3..=4).contains(&height) {
            return Err("only 3-4-log naturally reachable oak stems are supported".into());
        }
        let last_y = root[1].checked_add(i32::from(height) - 1)
            .ok_or("oak tree height overflows native world coordinate")?;
        if !(-2046..=2046).contains(&last_y) {
            return Err("tree reaches outside safe native build height".into());
        }
        if self.settlements.get(&village_id)
            .ok_or("unknown village")?.oak_stem.is_some() {
            return Err("previous oak stem still pending".into());
        }
        self.plan_gather(village_id, Good::Wood, root, stand)?;
        self.settlements.get_mut(&village_id).unwrap().oak_stem =
            Some(OakStem { root, stand, height, cursor: 0 });
        Ok(())
    }

    /// Bind the selected UUID-linked citizen's stable resident ID to the
    /// native job and save before any external block mutation. The adapter
    /// MUST independently verify that this resident's UUID-bound entity is
    /// loaded, at the recorded work stand and within genuine 3D block reach.
    /// An already assigned job can never be silently stolen by another NPC.
    pub fn assign_gather_worker(
        &mut self, village_id: u64, resident_id: u64,
    ) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id)
            .ok_or("unknown settlement")?;
        let job = village.gather_job.as_ref().ok_or("no gather job")?;
        if job.issued || job.blocked {
            return Err("cannot assign an already issued or blocked gather".into());
        }
        let required = if job.good == Good::Wood { Job::Lumberjack }
            else if job.good == Good::Stone { Job::Miner }
            else { return Err("unsupported native gather good".into()); };
        if !village.residents.iter().any(|resident|
            resident.alive && resident.id == resident_id && resident.job == required) {
            return Err("worker is not a living qualified settlement resident".into());
        }
        if job.worker_id.is_some_and(|assigned| assigned != resident_id) {
            return Err("gather already assigned to a different worker".into());
        }
        village.gather_job.as_mut().unwrap().worker_id = Some(resident_id);
        Ok(())
    }

    pub fn mark_gather_issued(&mut self, village_id: u64) -> Result<(), String> {
        let job = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?
            .gather_job.as_mut().ok_or("no planned gather")?;
        if job.blocked || job.issued { return Err("gather may not be re-issued".into()); }
        if job.worker_id.is_none() {
            return Err("native gather must have a durably assigned citizen before issue".into());
        }
        job.issued = true;
        Ok(())
    }

    /// The adapter must call this ONLY after its native conditional
    /// command returned success and a FRESH read of that loaded source
    /// block returned air. Save this acknowledgement before pickup.
    /// Domain code cannot itself attest to native engine observations.
    pub fn mark_gather_native_edit_confirmed(&mut self, village_id: u64)
        -> Result<(), String> {
        let job = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?
            .gather_job.as_mut().ok_or("no gather job")?;
        if !job.issued || job.blocked || job.worker_id.is_none()
            || job.native_edit_confirmed {
            return Err("native edit receipt requires a new issued assigned gather".into());
        }
        job.native_edit_confirmed = true;
        Ok(())
    }

    /// The bridge has just observed the issued source as air AND the same
    /// UUID-linked worker at the saved loaded source work stand, inside
    /// actual 3D reach. Do not mark pickup before the native block edit.
    /// This journal step is saved before the worker gets a return route.
    pub fn witness_gather_pickup(
        &mut self, village_id: u64, resident_id: u64,
    ) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.gather_job.as_mut().ok_or("no gather job")?;
        if !job.issued || !job.native_edit_confirmed
            || job.blocked || job.pickup_witnessed {
            return Err("gather not awaiting a first source pickup witness".into());
        }
        if job.worker_id != Some(resident_id) {
            return Err("source pickup identity does not match assigned worker".into());
        }
        job.pickup_witnessed = true;
        Ok(())
    }

    /// A planned job whose block was removed externally is NOT a harvest:
    /// either player intervention or a crash before issued-state persistence
    /// must fail closed, never credit phantom inventory.
    pub fn block_gather(&mut self, village_id: u64) -> Result<(), String> {
        let job = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?
            .gather_job.as_mut().ok_or("no gather job")?;
        job.blocked = true;
        Ok(())
    }

    /// Discard an obstructed gather without any material credit, then retry
    /// another site on the next planning cycle; no foreign blocks are edited.
    pub fn discard_blocked_gather(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if !v.gather_job.as_ref().is_some_and(|job| job.blocked) {
            return Err("only a blocked gather can be discarded".into());
        }
        v.gather_job = None;
        v.oak_stem = None;
        Ok(())
    }

    /// Explicit operator decision after checking native block evidence.
    /// The adapter MUST require an OP in the correct dimension and a loaded,
    /// observed source block (or air). This method only abandons a blocked,
    /// previously issued receipt: it never credits wood/stone or issues edits.
    pub fn abandon_blocked_issued_gather(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if !v.gather_job.as_ref().is_some_and(|job| job.issued && job.blocked) {
            return Err("only a blocked issued gather may be manually abandoned".into());
        }
        let name = v.name.clone();
        self.discard_blocked_gather(village_id)?;
        self.event(format!("{name}: operator abandoned an uncertain gather; zero resource credit"));
        Ok(())
    }

    pub fn confirm_gather(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.gather_job.as_ref().ok_or("no gather job")?;
        if !job.issued || !job.native_edit_confirmed
            || !job.pickup_witnessed || job.blocked {
            return Err("unproven pickup or gather cannot be credited".into());
        }
        let good = job.good;
        let pos = job.position;
        let worker_id = job.worker_id.ok_or("unassigned gather cannot be credited")?;
        // Compute the exact next source before committing inventory, so a
        // corrupted height cannot partially credit or advance this stem.
        let continuation = if let Some(stem) = &v.oak_stem {
            let next = stem.cursor.checked_add(1).ok_or("oak cursor overflow")?;
            if next < stem.height {
                let y = stem.root[1].checked_add(i32::from(next))
                    .ok_or("oak height overflow")?;
                Some((next, GatherJob {
                    position: [stem.root[0], y, stem.root[2]],
                    stand: stem.stand, good: Good::Wood,
                    worker_id: Some(worker_id), issued: false,
                    native_edit_confirmed: false, pickup_witnessed: false, blocked: false,
                }))
            } else {
                None
            }
        } else { None };
        v.warehouse_credit(good, 1)?;
        if let Some((cursor, next_job)) = continuation {
            v.oak_stem.as_mut().unwrap().cursor = cursor;
            v.gather_job = Some(next_job);
        } else {
            v.oak_stem = None;
            v.gather_job = None;
        }
        let name = v.name.clone();
        self.event(format!("{name}: physically gathered one {} at {pos:?}", good.as_str()));
        Ok(())
    }

    /// Already-paid abstract houses become physical jobs automatically. This
    /// never changes the abstract number of houses or manufactures resources.
    /// Player-commissioned homes are separately counted in visual_sites.
    pub fn prepare_village_housing(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if !v.autonomy_enabled { return Ok(false); }
        let completed = v.visual_sites.len() as u32;
        let covered = completed.saturating_add(v.pending_visual_buildings);
        let physical_needed = v.population().div_ceil(PHYSICAL_HOME_CAPACITY as u32);
        let missing = physical_needed.saturating_sub(v.public_huts() as u32)
            .saturating_sub(v.pending_visual_buildings);
        let already_paid = v.houses.saturating_sub(covered);
        let queue = missing.min(already_paid);
        if queue == 0 { return Ok(false); }
        v.pending_visual_buildings = v.pending_visual_buildings
            .checked_add(queue).ok_or("visual backlog overflow")?;
        Ok(true)
    }

    /// Simple authorization control. Autonomy is ON by default even in old
    /// saves. Off pauses new site selection, but preserves existing jobs.
    pub fn set_autonomy(&mut self, village_id: u64, enabled: bool) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        village.autonomy_enabled = enabled;
        let name = village.name.clone();
        self.event(format!("{name}: autonomous construction {}", if enabled { "enabled" } else { "paused" }));
        Ok(())
    }

    /// A purchased 5x5 parcel is the player's local build right in this mod.
    /// Terrain and the actor UUID are checked by the native command adapter.
    pub fn buy_plot(&mut self, player_id: &str, village_id: u64, origin: [i32; 3]) -> Result<u64, String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        let p = self.players.get(player_id).ok_or("join the village first")?;
        if p.settlement != Some(village_id) { return Err("join this village before buying land".into()); }
        if p.coins < LAND_PRICE_COINS { return Err("not enough coins to buy a parcel".into()); }
        if v.player_parcels.len() >= MAX_PLAYER_PARCELS { return Err("plot registry is full".into()); }
        let treasury_after = v.treasury_coins.checked_add(LAND_PRICE_COINS)
            .ok_or("village treasury overflow")?;
        if i64::from(origin[0]).abs() > 29_999_990 || i64::from(origin[2]).abs() > 29_999_990
            || !(-2048..=2048).contains(&origin[1]) {
            return Err("land outside safe world bounds".into());
        }
        let dx = i128::from(origin[0]) - i128::from(v.center[0]);
        let dz = i128::from(origin[2]) - i128::from(v.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("buy land 12..80 blocks from village center".into());
        }
        for other in self.settlements.values().filter(|s| s.dimension == v.dimension) {
            if other.player_parcels.iter().any(|plot| near_plot(plot.origin, origin))
                || other.visual_sites.iter().any(|site| near_plot(*site, origin))
                || other.visual_job.as_ref().is_some_and(|job| near_plot(job.origin, origin))
                || other.farm.as_ref().is_some_and(|farm| near_plot(farm.origin, origin))
                || other.bakery.as_ref().is_some_and(|b| near_plot(b.origin, origin)) {
                return Err("land overlaps a registered building, field, bakery or another player's parcel".into());
            }
        }
        self.players.get_mut(player_id).unwrap().coins -= LAND_PRICE_COINS;
        let v = self.settlements.get_mut(&village_id).unwrap();
        v.player_parcels.push(PlayerParcel {
            origin, owner: player_id.to_owned(), commissioned: false
        });
        v.treasury_coins = treasury_after;
        let name = self.settlements[&village_id].name.clone();
        self.event(format!("{name}: player-owned 5x5 parcel sold at {},{},{}", origin[0], origin[1], origin[2]));
        Ok(LAND_PRICE_COINS)
    }

    /// Buy a finished, genuinely recorded public hut, not an abstract house.
    /// Native command verifies its real loaded Minecraft blocks first.
    /// Residents leave the sold building; future daily economy may pay for
    /// replacement public housing using normal material consumption.
    pub fn buy_finished_house(
        &mut self, player_id: &str, village_id: u64, origin: [i32; 3]
    ) -> Result<u64, String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if !v.visual_sites.contains(&origin) {
            return Err("no completed physical village hut at this site".into());
        }
        let p = self.players.get(player_id).ok_or("join the village first")?;
        if p.settlement != Some(village_id) { return Err("join the village first".into()); }
        if p.coins < FINISHED_HUT_PRICE_COINS { return Err("not enough civic coins for this hut".into()); }
        if v.player_parcels.len() >= MAX_PLAYER_PARCELS { return Err("plot registry is full".into()); }
        let treasury_after = v.treasury_coins.checked_add(FINISHED_HUT_PRICE_COINS)
            .ok_or("village treasury overflow")?;
        if self.settlements.values().filter(|s| s.dimension == v.dimension).any(|s| {
            s.player_parcels.iter().any(|plot| near_plot(plot.origin, origin))
        }) {
            return Err("this physical hut is already privately owned".into());
        }
        self.players.get_mut(player_id).unwrap().coins -= FINISHED_HUT_PRICE_COINS;
        let v = self.settlements.get_mut(&village_id).unwrap();
        v.player_parcels.push(PlayerParcel {
            origin, owner: player_id.to_owned(), commissioned: true
        });
        v.treasury_coins = treasury_after;
        let name = v.name.clone();
        self.event(format!(
            "{name}: finished hut sold into private ownership at {},{},{}",
            origin[0], origin[1], origin[2]
        ));
        Ok(FINISHED_HUT_PRICE_COINS)
    }

    /// Commission a physical hut on a bought plot. This is a WORLD-JSON
    /// transaction: coin labor fee and abstract stock materials are consumed
    /// together with the construction job. Block writes occur later.
    pub fn commission_house(&mut self, player_id: &str, village_id: u64, origin: [i32; 3]) -> Result<u64, String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        let parcel_index = village.player_parcels.iter()
            .position(|plot| plot.origin == origin && plot.owner == player_id && !plot.commissioned)
            .ok_or("buy this empty plot first; already commissioned lots cannot be reused")?;
        let p = self.players.get(player_id).ok_or("join the village first")?;
        if p.settlement != Some(village_id) { return Err("join this village before commissioning".into()); }
        if p.coins < COMMISSION_PRICE_COINS { return Err("not enough coins for a building commission".into()); }
        let houses = village.houses.checked_add(1).ok_or("house count overflow")?;
        let pending = village.pending_visual_buildings.checked_add(1).ok_or("pending job overflow")?;
        let treasury_after = village.treasury_coins.checked_add(COMMISSION_PRICE_COINS)
            .ok_or("village treasury overflow")?;
        self.check_visual_house_site_for(village_id, origin, Some(player_id), false)?;
        // All validations above are read-only; all mutations below are in
        // the same save object. The command adapter rolls back on save error.
        self.players.get_mut(player_id).unwrap().coins -= COMMISSION_PRICE_COINS;
        let village = self.settlements.get_mut(&village_id).unwrap();
        // The owner's labor fee pays the civic treasury. The physical
        // bill-of-materials is charged exactly once later, immediately
        // before the first world block, by reserve_hut_supplies().
        // No virtual abstract stone/wood may pay for that construction.
        village.houses = houses;
        village.pending_visual_buildings = pending;
        village.treasury_coins = treasury_after;
        village.player_parcels[parcel_index].commissioned = true;
        village.visual_job = Some(VisualBuildJob { origin, cursor: 0, blocked: false });
        village.hut_supplies_reserved = false;
        let name = village.name.clone();
        self.event(format!("{name}: commissioned player-owned hut at {},{},{}",origin[0],origin[1],origin[2]));
        Ok(COMMISSION_PRICE_COINS)
    }

    /// Reserve one operator-approved house; a later native level tick performs
    /// one conditional placement at a time after checking live world blocks.
    pub fn start_visual_house(&mut self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        self.check_visual_house_site(village_id, origin)?;
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let name = village.name.clone();
        village.visual_job = Some(VisualBuildJob { origin, cursor: 0, blocked: false });
        village.hut_supplies_reserved = false;
        self.event(format!(
            "{name}: physical construction project reserved at {},{},{}",
            origin[0], origin[1], origin[2]
        ));
        Ok(())
    }


    /// Backed by actual items delivered by players or confirmed physically
    /// collected by the village. An existing abstract house is NOT a supply.
    pub fn reserve_hut_supplies(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.visual_job.as_ref().ok_or("no active hut construction")?;
        if v.hut_supplies_reserved { return Ok(false); }
        if job.cursor != 0 || job.blocked {
            return Err("unfunded partial/blocked hut needs inspection before any physical work".into());
        }
        // A completed town hall changes the authoritative storage source:
        // only goods physically extracted from its real vanilla barrels
        // into the write-ahead builder escrow can pay this construction bill.
        let available = if v.hall_complete {
            v.build_escrow.get(&Good::Wood).copied().unwrap_or(0) >= HUT_WOOD_LOGS
                && v.build_escrow.get(&Good::Stone).copied().unwrap_or(0) >= HUT_COBBLESTONE
        } else {
            // Historical pre-hall work orders may finish from their already
            // accounted delivered ledger; new automatic huts wait for the
            // real completed civic hall.
            v.warehouse_quantity(Good::Wood) >= HUT_WOOD_LOGS
                && v.warehouse_quantity(Good::Stone) >= HUT_COBBLESTONE
        };
        if !available { return Ok(false); }
        if v.hall_complete {
            *v.build_escrow.get_mut(&Good::Wood).ok_or("missing wood escrow")? -= HUT_WOOD_LOGS;
            *v.build_escrow.get_mut(&Good::Stone).ok_or("missing stone escrow")? -= HUT_COBBLESTONE;
        } else {
            v.warehouse_debit(Good::Wood, HUT_WOOD_LOGS)?;
            v.warehouse_debit(Good::Stone, HUT_COBBLESTONE)?;
        }
        v.hut_supplies_reserved = true;
        Ok(true)
    }

    /// Rotate across loaded, unblocked physical jobs in a deterministic order.
    /// Never let an unloaded first village stall all other civilizations.
    pub fn next_visual_job(
        &self,
        dimension: &str,
        sequence: usize,
        mut loaded: impl FnMut([i32; 3]) -> bool,
    ) -> Option<(u64, VisualBuildJob)> {
        let eligible: Vec<_> = self.settlements.values()
            .filter(|s| s.dimension == dimension)
            .filter_map(|s| {
                let job = s.visual_job.as_ref()?;
                (!job.blocked && loaded(job.origin)).then(|| (s.id, job.clone()))
            })
            .collect();
        if eligible.is_empty() {
            None
        } else {
            Some(eligible[sequence % eligible.len()].clone())
        }
    }


    /// Reserve a real 3x3 growing patch without manufacturing economic stock.
    /// The Horizon adapter must preflight the terrain before calling this.
    pub fn start_farm(&mut self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if village.farm.is_some() { return Err("farm already assigned".into()); }
        if village.count(Job::Farmer) == 0 { return Err("village has no farmer".into()); }
        if i64::from(origin[0]).abs() > 29_999_988 || i64::from(origin[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&origin[1]) {
            return Err("farm outside safe build coordinates".into());
        }
        let distance = |a: [i32; 3], b: [i32; 3]| {
            let dx = i128::from(a[0]) - i128::from(b[0]);
            let dz = i128::from(a[2]) - i128::from(b[2]);
            dx * dx + dz * dz
        };
        if !(12_i128 * 12..=80_i128 * 80).contains(&distance(village.center, origin)) {
            return Err("farm must be 12..80 blocks from the village center".into());
        }
        for s in self.settlements.values().filter(|s| s.dimension == village.dimension) {
            if s.player_parcels.iter().any(|plot| near_plot(plot.origin, origin)) {
                return Err("farm site is owned by a player".into());
            }
            if s.farm.as_ref().is_some_and(|p| distance(p.origin, origin) < 9 * 9)
                || s.visual_job.as_ref().is_some_and(|p| distance(p.origin, origin) < 9 * 9)
                || s.visual_sites.iter().any(|&site| distance(site, origin) < 9 * 9)
                || s.bakery.as_ref().is_some_and(|b| distance(b.origin, origin) < 9 * 9) {
                return Err("farm overlaps an existing civilization structure".into());
            }
        }
        self.settlements.get_mut(&village_id).unwrap().farm = Some(FarmPlot {
            origin, stage: FarmStage::Preparing, cursor: 0, blocked: false, completed_cycles: 0,
            harvest_sequence: 0, harvest_receipts: Vec::new(),
        });
        Ok(())
    }

    /// The adapter must first inspect the unblocked plot against the real
    /// loaded blocks before issuing an operator-approved recovery.
    pub fn recover_farm(&mut self, village_id: u64) -> Result<(), String> {
        let farm = self.settlements.get_mut(&village_id)
            .ok_or("unknown settlement")?.farm.as_mut().ok_or("no farm")?;
        if !farm.blocked { return Err("farm is not blocked".into()); }
        farm.blocked = false;
        Ok(())
    }

    pub fn advance_farm(&mut self, village_id: u64, decision: FarmDecision) -> Result<bool, String> {
        let world_day = self.day;
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        // Harvest credit must be validated BEFORE mutating its receipt/cursor.
        // Each receipt and warehouse credit commit in the same world JSON save.
        let credit_harvest = matches!(&decision, FarmDecision::StageComplete)
            && village.farm.as_ref().is_some_and(|farm| farm.stage == FarmStage::Harvesting);
        let new_wheat = if credit_harvest {
            let pending = village.wheat_in_transit.checked_add(8)
                .ok_or("wheat farm delivery overflow")?;
            if pending > 10_000_000 { return Err("wheat delivery queue exceeded".into()); }
            Some(pending)
        } else { None };
        let farm = village.farm.as_mut().ok_or("no farm")?;
        match decision {
            FarmDecision::Unloaded | FarmDecision::Paused | FarmDecision::Place { .. } => Ok(false),
            FarmDecision::InvalidCursor => Err("farm has invalid cursor".into()),
            FarmDecision::Blocked { .. } => {
                farm.blocked = true;
                Ok(true)
            }
            FarmDecision::Rewind(cursor) => {
                farm.cursor = cursor;
                Ok(true)
            }
            FarmDecision::Advance => {
                let limit = if farm.stage == FarmStage::Preparing { farm_plan(farm.origin).len() } else { 8 };
                if farm.cursor >= limit {
                    return Err("cannot advance farm beyond phase step count".into());
                }
                farm.cursor += 1;
                Ok(true)
            }
            FarmDecision::StageComplete => {
                let steps = if farm.stage == FarmStage::Preparing { farm_plan(farm.origin).len() } else { 8 };
                if farm.stage != FarmStage::Growing && farm.cursor != steps {
                    return Err("farm phase cannot complete before all blocks are verified".into());
                }
                farm.stage = match farm.stage {
                    FarmStage::Preparing => FarmStage::Growing,
                    FarmStage::Growing => FarmStage::Harvesting,
                    FarmStage::Harvesting => {
                        let sequence = farm.harvest_sequence.checked_add(1)
                            .ok_or("harvest sequence overflow")?;
                        farm.harvest_sequence = sequence;
                        farm.harvest_receipts.push(HarvestReceipt {
                            sequence, world_day, crops: 8,
                        });
                        if farm.harvest_receipts.len() > MAX_HARVEST_RECEIPTS {
                            farm.harvest_receipts.remove(0);
                        }
                        // Receipt + phase + physical warehouse wheat credit
                        // are committed in ONE JSON save. No abstract stock,
                        // native item drops or player coins are credited.
                        FarmStage::Replanting
                    },
                    FarmStage::Replanting => {
                        farm.completed_cycles = farm.completed_cycles.saturating_add(1);
                        FarmStage::Growing
                    }
                };
                farm.cursor = 0;
                // Harvest is physically observed and has exactly one
                // receipt. The stock remains in the farmer's transit queue
                // until a live UUID-bound farmer returns to the hall.
                if let Some(count) = new_wheat {
                    village.wheat_in_transit = count;
                }
                Ok(true)
            }
        }
    }

    /// Each selected plot is considered only when its chunk is already loaded.
    pub fn next_farm(&self, dimension: &str, sequence: usize,
                     mut loaded: impl FnMut([i32; 3]) -> bool) -> Option<(u64, FarmPlot)> {
        let eligible: Vec<_> = self.settlements.values()
            .filter(|s| s.dimension == dimension)
            .filter_map(|s| {
                let farm = s.farm.as_ref()?;
                (!farm.blocked && loaded(farm.origin)).then(|| (s.id, farm.clone()))
            })
            .collect();
        if eligible.is_empty() { None } else { Some(eligible[sequence % eligible.len()].clone()) }
    }

    /// Called only after all block states have been observed at their targets.
    pub fn finish_visual_house(&mut self, village_id: u64) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = village.visual_job.as_ref().ok_or("no construction project")?;
        if job.cursor != hut_plan(job.origin).len() { return Err("construction incomplete".into()); }
        let origin = job.origin;
        if village.pending_visual_buildings == 0 { return Err("house counter underflow".into()); }
        if !village.hut_supplies_reserved {
            return Err("hut cannot complete without proven physical material reservation".into());
        }
        village.visual_job = None;
        village.hut_supplies_reserved = false;
        village.visual_sites.push(origin);
        village.pending_visual_buildings -= 1;
        let name = village.name.clone();
        self.event(format!("{name}: house constructed at {},{},{}", origin[0], origin[1], origin[2]));
        Ok(())
    }

    /// Bind a resident to the actual saved Horizon entity UUID, once.
    /// The operation is idempotent for the same identity and rejects reassignment.
    /// The OP explicitly requested ONE missing villager after inspecting
    /// loaded native actors. Save this reservation before sending any spawn
    /// command, so a crash or native refusal does not cause a second entity
    /// to be spawned by automated logic on a later tick.
    pub fn reserve_actor_recovery(
        &mut self, settlement_id: u64, resident_id: u64,
    ) -> Result<(), String> {
        let settlement = self.settlements.get_mut(&settlement_id)
            .ok_or("unknown settlement")?;
        if !settlement.actors_spawned {
            return Err("initial villager batch has not been reserved".into());
        }
        let missing = settlement.residents.iter().any(|r|
            r.id == resident_id && r.alive && r.entity_uuid.is_none());
        if !missing {
            return Err("resident is dead, missing or already UUID-linked".into());
        }
        if !settlement.actor_recovery_issued.insert(resident_id) {
            return Err("actor recovery already attempted; inspect/reconcile, never spawn twice".into());
        }
        Ok(())
    }

    pub fn bind_actor(&mut self, settlement_id: u64, resident_id: u64, uuid: u128) -> Result<bool, String> {
        if uuid == 0 {
            return Err("cannot bind zero entity UUID".into());
        }
        if self.settlements.iter().any(|(&sid, s)| {
            s.residents.iter().any(|r| {
                r.entity_uuid == Some(uuid) && (sid != settlement_id || r.id != resident_id)
            })
        }) {
            return Err("entity UUID is already owned by another resident".into());
        }
        let settlement = self.settlements.get_mut(&settlement_id).ok_or("unknown settlement")?;
        if !settlement.actors_spawned {
            return Err("cannot bind before actor spawning has been reserved".into());
        }
        let resident = settlement.residents.iter_mut()
            .find(|r| r.id == resident_id && r.alive)
            .ok_or("unknown or inactive resident")?;
        match resident.entity_uuid {
            Some(existing) if existing == uuid => Ok(false),
            Some(_) => Err("resident already bound to a different entity UUID".into()),
            None => {
                resident.entity_uuid = Some(uuid);
                // Atomic in-memory state saved by the bridge. If the save
                // fails, the disk retains the pending one-shot reservation.
                settlement.actor_recovery_issued.remove(&resident_id);
                Ok(true)
            }
        }
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
        // Real inventory transfer is checked and committed in the adapter.
        // Do NOT also credit abstract stock: that would count the same item
        // as both a Minecraft-derived resource and virtual daily production.
        s.warehouse_credit(good, count)?;
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
        s.warehouse_debit(good, count)?;
        self.players.get_mut(player_id).ok_or("no player")?.coins -= cost;
        Ok(cost)
    }

    /// Pure authorisation for native player block-breaking hook: purchased
    /// land belongs to its authenticated owner, while shared civic plots,
    /// native farms and physical workshop footprints are protected.
    /// OP bypass is handled separately by the native host's permission.
    pub fn player_may_break(&self, player: &str, dimension: &str, pos: [i32; 3]) -> bool {
        let in_hut = |origin: [i32;3]| {
            (i64::from(pos[0]) - i64::from(origin[0])).abs() <= 2
                && (i64::from(pos[2]) - i64::from(origin[2])).abs() <= 2
                && (-1..=3).contains(&(i64::from(pos[1]) - i64::from(origin[1])))
        };
        let in_field = |origin: [i32;3]| {
            (i64::from(pos[0]) - i64::from(origin[0])).abs() <= 1
                && (i64::from(pos[2]) - i64::from(origin[2])).abs() <= 1
                && (i64::from(pos[1]) - i64::from(origin[1])).abs() <= 2
        };
        for v in self.settlements.values().filter(|v| v.dimension == dimension) {
            // Private property is protected from other players; the owner
            // retains the right to edit bought plots and completed houses.
            if let Some(lot) = v.player_parcels.iter().find(|lot| in_hut(lot.origin)) {
                if lot.owner != player { return false; }
                continue;
            }
            if (v.hall_complete || v.hall_job.is_some()) && in_hut(v.center) {
                return false;
            }
            if v.bakery.as_ref().is_some_and(|b| in_hut(b.origin))
                || v.farm.as_ref().is_some_and(|f| in_field(f.origin))
                || v.visual_sites.iter().any(|site| in_hut(*site))
                || v.visual_job.as_ref().is_some_and(|job| in_hut(job.origin)) {
                return false;
            }
        }
        true
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

    /// Synchronize the abstract economy to the real Horizon day counter.
    /// On first run/reload from an old save, anchor without manufacturing
    /// historical output. Time rewinds never replay already accounted days.
    /// A giant forward jump catches up only MAX_HOST_CATCHUP_DAYS once.
    pub fn observe_host_day(&mut self, host_day: i64) -> Result<bool, String> {
        match self.last_host_day {
            None => {
                self.last_host_day = Some(host_day);
                Ok(true)
            }
            Some(previous) if host_day <= previous => Ok(false),
            Some(previous) => {
                // i128 prevents overflow across the entire signed i64 range.
                let elapsed = (i128::from(host_day) - i128::from(previous))
                    .min(i128::from(MAX_HOST_CATCHUP_DAYS)) as u32;
                self.advance(elapsed)?;
                self.last_host_day = Some(host_day);
                Ok(true)
            }
        }
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
                if self.day.is_multiple_of(4) {
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
        // This older fixture bypasses native block placement; explicitly
        // emulate a previously persisted warehouse materials reservation.
        reloaded.settlements.get_mut(&id).unwrap().hut_supplies_reserved = true;
        reloaded.finish_visual_house(id).unwrap();
        assert_eq!(reloaded.settlements[&id].visual_sites, vec![[20, 64, 0]]);
        assert_eq!(reloaded.settlements[&id].pending_visual_buildings, 0);
        assert!(reloaded.start_visual_house(id, [21, 64, 0]).is_err());
        reloaded.validate().unwrap();
    }


    #[test]
    fn physical_builder_never_overwrites_an_unexpected_block() {
        let job = VisualBuildJob { origin: [32, 70, -16], cursor: 0, blocked: false };
        let first = hut_plan(job.origin)[0].pos;
        assert_eq!(
            inspect_hut(&job, |p| (p == first).then(|| "minecraft:chest".to_owned())),
            BuildInspection::Blocked { position: first, found: "minecraft:chest".to_owned() }
        );
    }

    #[test]
    fn physical_builder_places_only_a_natural_foundation_or_air() {
        let job = VisualBuildJob { origin: [32, 70, -16], cursor: 0, blocked: false };
        let first = hut_plan(job.origin)[0].pos;
        assert_eq!(
            inspect_hut(&job, |_| Some("minecraft:dirt".to_owned())),
            BuildInspection::Place {
                position: first,
                old_block: "minecraft:dirt".to_owned(),
                new_block: "minecraft:cobblestone",
            }
        );
        assert_eq!(
            inspect_hut(&job, |_| Some("minecraft:cobblestone".to_owned())),
            BuildInspection::Advance
        );
    }

    #[test]
    fn physical_builder_repairs_a_cursor_ahead_of_chunk_persistence() {
        let job = VisualBuildJob { origin: [32, 70, -16], cursor: 3, blocked: false };
        let plan = hut_plan(job.origin);
        let first = plan[0].pos;
        assert_eq!(
            inspect_hut(&job, |p| Some(if p == first { "minecraft:grass_block" } else { "minecraft:cobblestone" }.into())),
            BuildInspection::Rewind(0)
        );
        let second = plan[1].pos;
        assert_eq!(
            inspect_hut(&job, |p| Some(if p == second { "minecraft:bedrock" } else { "minecraft:cobblestone" }.into())),
            BuildInspection::Blocked { position: second, found: "minecraft:bedrock".to_owned() }
        );
    }

    #[test]
    fn physical_builder_pauses_on_unloaded_chunk_and_finishes_only_after_recheck() {
        let origin = [32, 70, -16];
        let mut job = VisualBuildJob { origin, cursor: 0, blocked: false };
        assert_eq!(inspect_hut(&job, |_| None), BuildInspection::Unloaded);
        job.blocked = true;
        assert_eq!(inspect_hut(&job, |_| Some("minecraft:dirt".into())), BuildInspection::Paused);
        job.blocked = false;
        job.cursor = hut_plan(origin).len();
        let by_pos: BTreeMap<[i32; 3], String> = hut_plan(origin)
            .into_iter().map(|step| (step.pos, step.block.to_owned())).collect();
        assert_eq!(inspect_hut(&job, |p| by_pos.get(&p).cloned()), BuildInspection::Complete);
        let missing = hut_plan(origin)[5].pos;
        assert_eq!(inspect_hut(&job, |p| Some(if p == missing { "minecraft:dirt".to_owned() }
            else { by_pos[&p].clone() })), BuildInspection::Rewind(5));
        job.cursor += 1;
        assert_eq!(inspect_hut(&job, |p| by_pos.get(&p).cloned()), BuildInspection::InvalidCursor);
    }

    #[test]
    fn loaded_construction_jobs_take_fair_turns_and_ignore_unloaded_worlds() {
        let (mut sim, a) = world();
        let b = sim.found("Brightfield", "minecraft:overworld", [450, 70, 0], "other").unwrap();
        sim.advance(1).unwrap();
        sim.start_visual_house(a, [20, 64, 0]).unwrap();
        sim.start_visual_house(b, [470, 70, 0]).unwrap();
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |_| true).unwrap().0, a);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 1, |_| true).unwrap().0, b);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 2, |_| true).unwrap().0, a);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |p| p[0] > 400).unwrap().0, b);
        assert_eq!(sim.next_visual_job("minecraft:the_nether", 0, |_| true), None);
        sim.settlements.get_mut(&a).unwrap().visual_job.as_mut().unwrap().blocked = true;
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |_| true).unwrap().0, b);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |_| false), None);
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

#[cfg(test)]
mod actor_identity_tests {
    use super::*;

    fn ready_world() -> (World, u64, u64) {
        let mut world = World::default();
        let settlement = world.found("Ashford", "minecraft:overworld", [0, 64, 0], "player").unwrap();
        world.settlements.get_mut(&settlement).unwrap().actors_spawned = true;
        let resident = world.settlements[&settlement].residents[0].id;
        (world, settlement, resident)
    }

    #[test]
    fn uuid_persists_as_32_hex_characters_with_no_json_precision_loss() {
        let (mut world, settlement, resident) = ready_world();
        let uuid = 0xdeadbeef_01234567_89abcdef_01234567_u128;
        assert!(world.bind_actor(settlement, resident, uuid).unwrap());
        let encoded = serde_json::to_string(&world).unwrap();
        assert!(encoded.contains("deadbeef0123456789abcdef01234567"));
        let recovered: World = serde_json::from_str(&encoded).unwrap();
        recovered.validate().unwrap();
        assert_eq!(recovered.settlements[&settlement].residents[0].entity_uuid, Some(uuid));
        assert!(!world.bind_actor(settlement, resident, uuid).unwrap());
    }

    #[test]
    fn uuid_binding_rejects_reassignment_collisions_and_unreserved_spawning() {
        let (mut world, settlement, resident) = ready_world();
        assert!(world.bind_actor(settlement, resident, 0).is_err());
        assert!(world.bind_actor(settlement, resident, 123).unwrap());
        assert!(world.bind_actor(settlement, resident, 124).is_err());
        let second = world.settlements[&settlement].residents[1].id;
        assert!(world.bind_actor(settlement, second, 123).is_err());
        world.settlements.get_mut(&settlement).unwrap().actors_spawned = false;
        assert!(world.bind_actor(settlement, second, 987).is_err());
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_world_without_entity_uuid_remains_loadable() {
        let (world, settlement, _) = ready_world();
        let serialized = serde_json::to_string(&world).unwrap();
        assert!(!serialized.contains("entity_uuid"));
        let old: World = serde_json::from_str(&serialized).unwrap();
        old.validate().unwrap();
        assert!(old.settlements[&settlement].residents.iter().all(|r| r.entity_uuid.is_none()));
    }

    #[test]
    fn malformed_or_repeated_uuids_fail_closed_on_load() {
        let (mut world, settlement, resident) = ready_world();
        world.bind_actor(settlement, resident, 0x1111).unwrap();
        let mut data: serde_json::Value = serde_json::to_value(&world).unwrap();
        data["settlements"][settlement.to_string()]["residents"][0]["entity_uuid"] =
            serde_json::Value::String("INVALID".into());
        assert!(serde_json::from_value::<World>(data).is_err());
        let second = world.settlements[&settlement].residents[1].id;
        world.settlements.get_mut(&settlement).unwrap().residents[1].entity_uuid = Some(0x1111);
        assert_eq!(second, world.settlements[&settlement].residents[1].id);
        assert!(world.validate().is_err());
    }
}

#[cfg(test)]
mod visual_farm_tests {
    use super::*;

    fn plot() -> FarmPlot {
        FarmPlot { origin: [25, 72, -20], stage: FarmStage::Preparing, cursor: 0, blocked: false,
            completed_cycles: 0, harvest_sequence: 0, harvest_receipts: Vec::new() }
    }
    fn block(name: &str, age: Option<u8>) -> Option<FarmBlock> {
        Some(FarmBlock { name: name.into(), age })
    }
    fn farm_in_world() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 72, 0], "founder").unwrap();
        world.start_farm(id, [25, 72, 0]).unwrap();
        (world, id)
    }
    #[test]
    fn field_is_nine_soil_tiles_and_eight_wheat_crops() {
        let plan = farm_plan([25, 72, -20]);
        assert_eq!(plan.len(), 17);
        assert_eq!(plan.iter().filter(|s| s.block == "minecraft:farmland").count(), 8);
        assert_eq!(plan.iter().filter(|s| s.block == "minecraft:water").count(), 1);
        assert_eq!(plan.iter().filter(|s| s.block == "minecraft:wheat").count(), 8);
        let positions: BTreeSet<_> = plan.iter().map(|s| s.pos).collect();
        assert_eq!(positions.len(), plan.len());
    }
    #[test]
    fn preparing_only_replaces_natural_ground_and_air() {
        let plot = plot();
        let first = farm_plan(plot.origin)[0].pos;
        assert_eq!(inspect_farm(&plot, |_| block("minecraft:grass_block", None)),
            FarmDecision::Place { position: first, old_block: "minecraft:grass_block".into(), new_block: "minecraft:farmland" });
        assert_eq!(inspect_farm(&plot, |_| block("minecraft:chest", None)),
            FarmDecision::Blocked { position: first, found: "minecraft:chest".into() });
        assert_eq!(inspect_farm(&plot, |_| None), FarmDecision::Unloaded);
    }
    #[test]
    fn phase_changes_only_after_all_confirmed_blocks() {
        let (mut world, id) = farm_in_world();
        world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap().cursor = 17;
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.cursor, 0);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn crop_growth_uses_real_maturity_and_never_credits_stock() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Growing;
        let origin = farm.origin;
        let mature = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:wheat", Some(7)) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(mature, FarmDecision::StageComplete);
        let immature = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:wheat", Some(4)) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(immature, FarmDecision::Paused);
        world.advance_farm(id, mature).unwrap();
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().stage, FarmStage::Harvesting);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn harvesting_is_conditional_and_never_drops_items_or_rewinds_stock() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Harvesting;
        let origin = farm.origin;
        let expected = farm_plan(origin)[9].pos;
        let result = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:wheat", Some(7)) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(result, FarmDecision::Place {
            position: expected, old_block: "minecraft:wheat[age=7]".into(), new_block: "minecraft:air",
        });
        assert!(!world.advance_farm(id, result).unwrap());
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn interrupted_harvest_rewinds_without_minting_wheat() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Harvesting;
        farm.cursor = 3;
        let origin = farm.origin;
        let first = farm_plan(origin)[9].pos;
        let decision = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] {
                if pos == first { block("minecraft:wheat", Some(7)) }
                else { block("minecraft:air", None) }
            } else if pos[0] == origin[0] && pos[2] == origin[2] {
                block("minecraft:water", None)
            } else { block("minecraft:farmland", None) }
        });
        assert_eq!(decision, FarmDecision::Rewind(0));
        world.advance_farm(id, decision).unwrap();
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().cursor, 0);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn replants_then_restarts_growing_idempotently() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Replanting;
        let origin = farm.origin;
        let first = farm_plan(origin)[9].pos;
        let decision = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:air", None) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(decision, FarmDecision::Place {
            position: first, old_block: "minecraft:air".into(), new_block: "minecraft:wheat",
        });
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.cursor = 8;
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.completed_cycles, 1);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn farm_is_persisted_and_legacy_without_farm_loads() {
        let (world, id) = farm_in_world();
        let json = serde_json::to_string(&world).unwrap();
        let decoded: World = serde_json::from_str(&json).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.settlements[&id].farm.as_ref().unwrap().origin, [25, 72, 0]);
        let mut data: serde_json::Value = serde_json::from_str(&json).unwrap();
        data["settlements"][id.to_string()].as_object_mut().unwrap().remove("farm");
        let legacy: World = serde_json::from_value(data).unwrap();
        legacy.validate().unwrap();
        assert!(legacy.settlements[&id].farm.is_none());
    }
    #[test]
    fn site_collision_and_wrong_dimensions_are_rejected() {
        let (mut world, id) = farm_in_world();
        assert!(world.start_farm(id, [60, 72, 0]).is_err());
        let second = world.found("Brighthold", "minecraft:overworld", [300, 72, 0], "other").unwrap();
        assert!(world.start_farm(second, [25, 72, 0]).is_err());
        world.start_farm(second, [325, 72, 0]).unwrap();
        assert!(world.validate().is_ok());
    }
}

#[cfg(test)]
mod visual_farm_recovery_tests {
    use super::*;

    #[test]
    fn recovery_requires_a_blocked_plot_and_preserves_stock() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        assert!(world.recover_farm(id).is_err());
        let initial_stock = world.settlements[&id].stock.clone();
        world.advance_farm(id, FarmDecision::Blocked {
            position: [20, 69, 0],
            found: "minecraft:chest".into(),
        }).unwrap();
        assert!(world.settlements[&id].farm.as_ref().unwrap().blocked);
        world.recover_farm(id).unwrap();
        assert!(!world.settlements[&id].farm.as_ref().unwrap().blocked);
        assert_eq!(world.settlements[&id].stock, initial_stock);
        world.validate().unwrap();
        let decoded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert!(!decoded.settlements[&id].farm.as_ref().unwrap().blocked);
    }
}

#[cfg(test)]
mod visual_farm_transition_tests {
    use super::*;

    #[test]
    fn incomplete_phase_cannot_commit_or_overrun_cursor() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        assert!(world.advance_farm(id, FarmDecision::StageComplete).is_err());
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().stage, FarmStage::Preparing);
        world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap().cursor = 17;
        assert!(world.advance_farm(id, FarmDecision::Advance).is_err());
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.cursor, 0);
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod harvest_receipt_tests {
    use super::*;

    fn harvest_world() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        (world, id)
    }

    #[test]
    fn full_harvest_records_once_and_never_mints_stock() {
        let (mut world, id) = harvest_world();
        let prior_stock = world.settlements[&id].stock.clone();
        {
            let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
            farm.stage = FarmStage::Harvesting;
            farm.cursor = 8;
        }
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Replanting);
        assert_eq!(farm.harvest_sequence, 1);
        assert_eq!(farm.harvest_receipts, vec![HarvestReceipt {
            sequence: 1, world_day: 0, crops: 8,
        }]);
        assert_eq!(world.settlements[&id].stock, prior_stock);
        // Another callback cannot record the same completed harvest again.
        assert!(world.advance_farm(id, FarmDecision::StageComplete).is_err());
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().harvest_sequence, 1);
        world.validate().unwrap();
        let decoded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.settlements[&id].farm.as_ref().unwrap().harvest_receipts.len(), 1);
    }

    #[test]
    fn receipt_ring_is_bounded_and_rejects_duplicate_or_invalid_evidence() {
        let (mut world, id) = harvest_world();
        for _ in 0..MAX_HARVEST_RECEIPTS + 3 {
            let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
            farm.stage = FarmStage::Harvesting;
            farm.cursor = 8;
            world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        }
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.harvest_sequence, (MAX_HARVEST_RECEIPTS + 3) as u64);
        assert_eq!(farm.harvest_receipts.len(), MAX_HARVEST_RECEIPTS);
        assert_eq!(farm.harvest_receipts[0].sequence, 4);
        world.validate().unwrap();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.harvest_receipts[1].sequence = farm.harvest_receipts[0].sequence;
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_farm_missing_audit_fields_loads_with_no_invented_harvests() {
        let (world, id) = harvest_world();
        let mut raw: serde_json::Value = serde_json::to_value(&world).unwrap();
        let map = raw["settlements"][id.to_string()]["farm"].as_object_mut().unwrap();
        map.remove("harvest_sequence");
        map.remove("harvest_receipts");
        let loaded: World = serde_json::from_value(raw).unwrap();
        loaded.validate().unwrap();
        let farm = loaded.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.harvest_sequence, 0);
        assert!(farm.harvest_receipts.is_empty());
    }
}

#[cfg(test)]
mod harvest_full_cycle_in_memory_tests {
    use super::*;

    fn plot_world() -> (World, u64, BTreeMap<[i32; 3], FarmBlock>) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        let mut blocks = BTreeMap::new();
        for step in farm_plan(farm.origin) {
            let name = if step.floor { "minecraft:dirt" } else { "minecraft:air" };
            blocks.insert(step.pos, FarmBlock { name: name.into(), age: None });
        }
        (world, id, blocks)
    }

    fn run_until_waiting(
        world: &mut World,
        village_id: u64,
        blocks: &mut BTreeMap<[i32; 3], FarmBlock>,
    ) {
        for _ in 0..200 {
            let snapshot = world.settlements[&village_id].farm.as_ref().unwrap().clone();
            let decision = inspect_farm(&snapshot, |pos| blocks.get(&pos).cloned());
            match decision {
                FarmDecision::Place { position, new_block, .. } => {
                    // Mock the Horizon command without immediately advancing JSON.
                    blocks.insert(position, FarmBlock {
                        name: new_block.into(),
                        age: (new_block == "minecraft:wheat").then_some(0),
                    });
                }
                FarmDecision::Paused => return,
                FarmDecision::Unloaded | FarmDecision::Blocked { .. }
                | FarmDecision::InvalidCursor => panic!("valid farm unexpectedly stopped: {:?}", decision),
                other => {
                    world.advance_farm(village_id, other).unwrap();
                }
            }
        }
        panic!("farm did not become idle within bounded work");
    }

    #[test]
    fn complete_wheat_cycle_tracks_real_blocks_and_one_audit_receipt() {
        let (mut world, id, mut blocks) = plot_world();
        let original_stock = world.settlements[&id].stock.clone();

        // Simulate a block committed to a chunk, followed by a crash before
        // the world.json cursor is committed. On reload it must be observed.
        let snapshot = world.settlements[&id].farm.as_ref().unwrap().clone();
        let first = inspect_farm(&snapshot, |pos| blocks.get(&pos).cloned());
        let FarmDecision::Place { position, new_block, .. } = first else {
            panic!("expected first terrain step");
        };
        blocks.insert(position, FarmBlock { name: new_block.into(), age: None });
        world = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();

        run_until_waiting(&mut world, id, &mut blocks);
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().stage, FarmStage::Growing);
        let plan = farm_plan(world.settlements[&id].farm.as_ref().unwrap().origin);
        for step in plan.iter().skip(9) {
            let wheat = blocks.get_mut(&step.pos).unwrap();
            assert_eq!(wheat.name, "minecraft:wheat");
            assert_eq!(wheat.age, Some(0));
            wheat.age = Some(7);
        }
        run_until_waiting(&mut world, id, &mut blocks);
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.completed_cycles, 1);
        assert_eq!(farm.harvest_sequence, 1);
        assert_eq!(farm.harvest_receipts.len(), 1);
        assert_eq!(world.settlements[&id].stock, original_stock);
        for step in plan.iter().skip(9) {
            let planted = blocks.get(&step.pos).unwrap();
            assert_eq!(planted.name, "minecraft:wheat");
            assert_eq!(planted.age, Some(0));
        }
        world.validate().unwrap();
        let loaded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.settlements[&id].farm.as_ref().unwrap().harvest_sequence, 1);
    }
}

#[cfg(test)]
mod world_host_clock_tests {
    use super::*;

    fn world() -> World {
        let mut w = World::default();
        w.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        w
    }

    #[test]
    fn bootstrap_is_nonproductive_and_legacy_json_is_compatible() {
        let mut w = world();
        let before = w.settlements[&1].stock.clone();
        assert!(w.observe_host_day(5_000).unwrap());
        assert_eq!(w.day, 0);
        assert_eq!(w.settlements[&1].stock, before);
        assert_eq!(w.last_host_day, Some(5_000));
        let value = serde_json::to_value(&w).unwrap();
        let mut old = value;
        old.as_object_mut().unwrap().remove("last_host_day");
        let mut loaded: World = serde_json::from_value(old).unwrap();
        assert_eq!(loaded.last_host_day, None);
        assert!(loaded.observe_host_day(5_100).unwrap());
        assert_eq!(loaded.day, 0);
        loaded.validate().unwrap();
    }

    #[test]
    fn restart_and_clock_rewinds_never_double_apply_day() {
        let mut w = world();
        w.observe_host_day(100).unwrap();
        assert!(w.observe_host_day(101).unwrap());
        assert_eq!(w.day, 1);
        assert!(!w.observe_host_day(101).unwrap());
        assert!(!w.observe_host_day(90).unwrap());
        assert_eq!(w.day, 1);
        let mut reloaded: World = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert!(!reloaded.observe_host_day(100).unwrap());
        assert!(!reloaded.observe_host_day(101).unwrap());
        assert_eq!(reloaded.day, 1);
        assert!(reloaded.observe_host_day(102).unwrap());
        assert_eq!(reloaded.day, 2);
        reloaded.validate().unwrap();
    }

    #[test]
    fn fast_forward_is_bounded_and_does_not_repeat_on_followup_tick() {
        let mut w = world();
        w.observe_host_day(5).unwrap();
        assert!(w.observe_host_day(5 + 100_000).unwrap());
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS));
        assert_eq!(w.last_host_day, Some(100_005));
        assert!(!w.observe_host_day(100_005).unwrap());
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS));
        assert!(w.observe_host_day(100_006).unwrap());
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS) + 1);
    }

    #[test]
    fn extremely_large_time_delta_is_safe_from_arithmetic_overflow() {
        let mut w = world();
        w.observe_host_day(i64::MIN).unwrap();
        w.observe_host_day(i64::MAX).unwrap();
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS));
        w.validate().unwrap();
    }
}

#[cfg(test)]
mod npc_daily_routine_tests {
    use super::*;

    #[test]
    fn boundaries_follow_host_clock_rollovers_and_rewinds() {
        use DailyPhase::{Dawn, Evening, Night, Work};
        for (tick, expected) in [
            (0, Dawn), (1_999, Dawn), (2_000, Work),
            (10_999, Work), (11_000, Evening), (12_999, Evening),
            (13_000, Night), (23_999, Night), (24_000, Dawn),
            (-1, Night), (-24_000, Dawn), (26_000, Work),
        ] {
            assert_eq!(DailyPhase::at(tick), expected, "clock tick {tick}");
        }
        assert_eq!(
            DailyPhase::at(i64::MAX),
            DailyPhase::at(i64::MAX.rem_euclid(24_000))
        );
        assert_eq!(
            DailyPhase::at(i64::MIN),
            DailyPhase::at(i64::MIN.rem_euclid(24_000))
        );
    }

    #[test]
    fn guard_watch_continues_while_other_jobs_stop_at_night() {
        for phase in [
            DailyPhase::Dawn, DailyPhase::Work,
            DailyPhase::Evening, DailyPhase::Night,
        ] {
            assert!(phase.on_shift(Job::Guard));
        }
        for job in [
            Job::Farmer, Job::Baker, Job::Lumberjack,
            Job::Miner, Job::Builder, Job::Merchant,
        ] {
            assert!(DailyPhase::Work.on_shift(job));
            assert!(!DailyPhase::Dawn.on_shift(job));
            assert!(!DailyPhase::Evening.on_shift(job));
            assert!(!DailyPhase::Night.on_shift(job));
        }
    }

    #[test]
    fn off_duty_routes_are_stable_bounded_and_require_no_save_change() {
        for id in 0..200 {
            for phase in [
                DailyPhase::Dawn, DailyPhase::Evening,
                DailyPhase::Night,
            ] {
                let offset = phase.offset(Job::Farmer, id);
                assert_eq!(offset, phase.offset(Job::Farmer, id));
                assert!((-4..=4).contains(&offset[0]));
                assert_eq!(offset[1], 0);
                assert!((-5..=4).contains(&offset[2]));
            }
            assert_eq!(
                DailyPhase::Night.offset(Job::Guard, id),
                Job::Guard.workplace(id)
            );
            assert_eq!(
                DailyPhase::Work.offset(Job::Builder, id),
                Job::Builder.workplace(id)
            );
        }
        let encoded = serde_json::to_value(World::default()).unwrap();
        assert!(encoded.get("daily_phase").is_none());
        assert!(encoded.get("routine").is_none());
        let recovered: World = serde_json::from_value(encoded).unwrap();
        recovered.validate().unwrap();
    }
}

#[cfg(test)]
mod physical_home_tests {
    use super::*;

    fn village() -> World {
        let mut world = World::default();
        world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        world
    }

    #[test]
    fn abstract_or_pending_houses_are_never_allocated_as_real_homes() {
        let mut world = village();
        let settlement = world.settlements.get_mut(&1).unwrap();
        settlement.houses = 5;
        settlement.pending_visual_buildings = 5;
        assert_eq!(settlement.physical_housing(), (0, 0, 10));
        for resident in &settlement.residents {
            assert_eq!(settlement.home_slot(resident.id), None);
        }
        world.validate().unwrap();
    }

    #[test]
    fn completed_huts_allocate_three_spots_in_immutable_site_order() {
        let mut world = village();
        let settlement = world.settlements.get_mut(&1).unwrap();
        settlement.houses = 3;
        settlement.visual_sites.push([20, 70, 10]);
        let mut ids: Vec<u64> = settlement.residents.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        assert_eq!(settlement.physical_housing(), (3, 3, 7));
        for (lane, id) in ids.iter().take(3).enumerate() {
            assert_eq!(settlement.home_slot(*id), Some(HomeSlot {
                site: [20, 70, 10], lane: lane as u8
            }));
        }
        assert_eq!(settlement.home_slot(ids[3]), None);
        let previous: Vec<_> = ids.iter().take(3)
            .map(|id| settlement.home_slot(*id)).collect();
        settlement.visual_sites.push([40, 70, 10]);
        assert_eq!(settlement.physical_housing(), (6, 6, 4));
        for (index, id) in ids.iter().enumerate().take(6) {
            let site = if index < 3 { [20, 70, 10] } else { [40, 70, 10] };
            assert_eq!(settlement.home_slot(*id), Some(HomeSlot {
                site, lane: (index % 3) as u8
            }));
        }
        settlement.residents.reverse();
        assert_eq!(previous, ids.iter().take(3)
            .map(|id| settlement.home_slot(*id)).collect::<Vec<_>>());
        world.validate().unwrap();
        let encoded = serde_json::to_string(&world).unwrap();
        assert!(!encoded.contains("home_slot"));
        let loaded: World = serde_json::from_str(&encoded).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.settlements[&1].home_slot(ids[0]), previous[0]);
    }

    #[test]
    fn home_inspection_is_fail_closed_for_unloaded_or_modified_blocks() {
        use std::collections::BTreeMap;
        let home = HomeSlot { site: [20, 70, 10], lane: 0 };
        let [x, y, z] = home.foot();
        let [dx, dy, dz] = home.doorway();
        assert_eq!([x, y, z], [19, 70, 9]);
        assert_eq!([dx, dy, dz], [20, 70, 8]);
        let mut world = BTreeMap::from([
            ([x, y - 1, z], "minecraft:cobblestone".to_owned()),
            ([x, y, z], "minecraft:air".to_owned()),
            ([x, y + 1, z], "minecraft:air".to_owned()),
            ([dx, dy - 1, dz], "minecraft:cobblestone".to_owned()),
            ([dx, dy, dz], "minecraft:air".to_owned()),
            ([dx, dy + 1, dz], "minecraft:air".to_owned()),
        ]);
        assert!(inspect_home(home, |p| world.get(&p).cloned()));
        world.insert([x, y, z], "minecraft:chest".into());
        assert!(!inspect_home(home, |p| world.get(&p).cloned()));
        world.remove(&[x, y, z]);
        assert!(!inspect_home(home, |p| world.get(&p).cloned()));
        assert!(!inspect_home(home, |_| None));
        assert!(!inspect_home(HomeSlot { lane: 3, ..home }, |_| {
            Some("minecraft:air".into())
        }));
    }

    #[test]
    fn housing_rebalances_on_death_but_never_targets_dead_residents() {
        let mut world = village();
        let village = world.settlements.get_mut(&1).unwrap();
        village.houses = 1;
        village.visual_sites.push([20, 70, 10]);
        let mut ids: Vec<u64> = village.residents.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        let index = village.residents.iter().position(|r| r.id == ids[0]).unwrap();
        village.residents[index].alive = false;
        assert_eq!(village.home_slot(ids[0]), None);
        assert_eq!(village.home_slot(ids[1]).unwrap().lane, 0);
        assert_eq!(village.physical_housing(), (3, 3, 6));
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod safe_housing_proposal_tests {
    use super::*;

    fn world_with_ready_house() -> World {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        world.settlements.get_mut(&id).unwrap().pending_visual_buildings = 1;
        world.validate().unwrap();
        world
    }

    #[test]
    fn candidates_are_unique_bounded_stable_and_never_reserve_a_plot() {
        let world = world_with_ready_house();
        let mut unique = BTreeSet::new();
        for slot in 1..=HOUSING_PLOT_OFFSETS.len() {
            let origin = world.proposed_hut(1, slot, 70).unwrap();
            assert!(unique.insert(origin));
            assert_eq!(origin, world.proposed_hut(1, slot, 70).unwrap());
        }
        assert_eq!(world.proposed_hut(1, 1, 70).unwrap(), [20, 70, 0]);
        assert!(world.proposed_hut(1, 0, 70).is_err());
        assert!(world.proposed_hut(1, HOUSING_PLOT_OFFSETS.len() + 1, 70).is_err());
        assert!(world.settlements[&1].visual_job.is_none());
        assert!(world.settlements[&1].visual_sites.is_empty());
        world.validate().unwrap();
    }

    #[test]
    fn requires_existing_abstract_build_budget_and_real_housing_demand() {
        let mut world = world_with_ready_house();
        world.settlements.get_mut(&1).unwrap().pending_visual_buildings = 0;
        assert!(world.proposed_hut(1, 1, 70).is_err());
        world.settlements.get_mut(&1).unwrap().pending_visual_buildings = 1;
        {
            let village = world.settlements.get_mut(&1).unwrap();
            village.houses = 5;
            village.visual_sites.extend([
                [20, 70, 0], [-20, 70, 0], [0, 70, 20], [0, 70, -20],
            ]);
            assert_eq!(village.physical_housing(), (12, 10, 0));
        }
        assert!(world.proposed_hut(1, 1, 70).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn completed_huts_farms_and_jobs_reject_overlaps_for_manual_and_proposed_paths() {
        let mut world = world_with_ready_house();
        world.start_farm(1, [20, 70, 0]).unwrap();
        assert!(world.proposed_hut(1, 1, 70).is_err());
        assert!(world.start_visual_house(1, [20, 70, 0]).is_err());
        let second = world.proposed_hut(1, 2, 70).unwrap();
        world.start_visual_house(1, second).unwrap();
        assert!(world.proposed_hut(1, 3, 70).is_err());
        assert!(world.start_visual_house(1, [0, 70, 20]).is_err());
        world.validate().unwrap();

        let mut world = world_with_ready_house();
        world.settlements.get_mut(&1).unwrap().visual_sites.push([20, 70, 0]);
        assert!(world.start_visual_house(1, [21, 70, 0]).is_err());
        assert!(world.proposed_hut(1, 1, 70).is_err());
    }

    #[test]
    fn other_settlement_structures_are_respected_within_same_dimension_only() {
        let mut world = world_with_ready_house();
        let other = world.found("Birch", "minecraft:overworld", [220, 70, 0], "op").unwrap();
        {
            let village = world.settlements.get_mut(&other).unwrap();
            village.visual_sites.push([20, 70, 0]);
        }
        assert!(world.proposed_hut(1, 1, 70).is_err());
        world.settlements.get_mut(&other).unwrap().visual_sites.clear();

        let distant = world.found("Nether", "minecraft:the_nether", [0, 70, 0], "op").unwrap();
        world.settlements.get_mut(&distant).unwrap().visual_sites.push([20, 70, 0]);
        assert_eq!(world.proposed_hut(1, 1, 70).unwrap(), [20, 70, 0]);
    }

    #[test]
    fn extreme_coordinates_and_height_cannot_overflow_or_escape_world() {
        let mut world = World::default();
        let id = world.found("Border", "minecraft:overworld",
                             [29_999_990, 70, 0], "op").unwrap();
        world.settlements.get_mut(&id).unwrap().pending_visual_buildings = 1;
        assert!(world.proposed_hut(id, 1, 70).is_err());
        assert!(world.proposed_hut(id, 2, 70).is_ok());
        assert!(world.proposed_hut(id, 2, i32::MAX).is_err());
        assert!(world.proposed_hut(id, 2, i32::MIN).is_err());

        // The farthest possible signed i32 coordinates exceed i64's
        // *combined* squared-distance range. This must fail, not panic.
        let mut distant = World::default();
        let other = distant.found("Distant", "minecraft:overworld",
                                  [i32::MIN, 70, i32::MIN], "op").unwrap();
        distant.settlements.get_mut(&other).unwrap().pending_visual_buildings = 1;
        assert!(distant.check_visual_house_site(other,
            [29_999_990, 70, 29_999_990]).is_err());
    }
}

#[cfg(test)]
mod civic_council_tests {
    use super::*;

    fn founded() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        (world, id)
    }

    #[test]
    fn new_village_has_unspawned_identity_and_unbuilt_housing_needs() {
        let (world, id) = founded();
        let needs = world.settlements[&id].council_priorities();
        assert_eq!(needs[0].need, CivicNeed::Identity);
        assert!(needs.iter().any(|p| p.need == CivicNeed::Housing));
        assert!(needs.iter().all(|p| p.urgency <= 100));
    }

    #[test]
    fn hunger_crisis_outranks_theoretical_housing_and_follows_existing_economy() {
        let (mut world, id) = founded();
        let settlement = world.settlements.get_mut(&id).unwrap();
        settlement.starving_days = 3;
        let priorities = settlement.council_priorities();
        assert_eq!(priorities[0].need, CivicNeed::Food);
        assert_eq!(priorities[0].urgency, 100);
        assert!(priorities[0].reason.contains("3 day(s)"));
    }

    #[test]
    fn economic_forecast_is_bounded_and_never_mutates_real_stock_or_world() {
        let (world, id) = founded();
        let before = serde_json::to_string(&world).unwrap();
        let village = &world.settlements[&id];
        assert!(village.forecast_abstract_days(0).is_empty());
        let three = village.forecast_abstract_days(COUNCIL_FORECAST_DAYS);
        assert_eq!(three.len(), 3);
        assert_eq!(three[0].offset, 1);
        assert_eq!(three[2].offset, 3);
        assert_eq!(three, village.forecast_abstract_days(3));
        assert_eq!(village.forecast_abstract_days(u8::MAX).len(), 7);
        assert!(three[0].houses >= village.houses);
        assert_eq!(serde_json::to_string(&world).unwrap(), before);
    }

    #[test]
    fn forecast_detects_no_bakers_before_actual_starvation() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.stock.insert(Good::Bread, 0);
        for person in &mut village.residents {
            if person.job == Job::Baker { person.alive = false; }
        }
        let priorities = village.council_priorities();
        let food = priorities.iter().find(|p| p.need == CivicNeed::Food).unwrap();
        assert_eq!(food.urgency, 90);
        assert!(food.reason.contains("day +1"));
        assert_eq!(village.quantity(Good::Bread), 0);
    }

    #[test]
    fn war_priority_is_abstract_security_only_no_automatic_combat() {
        let (mut world, a) = founded();
        let b = world.found("Brightwell", "minecraft:overworld", [300, 70, 0], "op2").unwrap();
        world.declare_war(a, b).unwrap();
        let priorities = world.settlements[&a].council_priorities();
        assert_eq!(priorities[0].need, CivicNeed::Security);
        assert_eq!(priorities[0].urgency, 95);
        assert!(priorities[0].next_step.contains("operator-controlled"));
    }

    #[test]
    fn materials_are_for_future_abstract_houses_not_physical_stock() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.stock.insert(Good::Wood, 1);
        village.stock.insert(Good::Stone, 2);
        let priorities = village.council_priorities();
        let materials = priorities.iter().find(|p| p.need == CivicNeed::Materials).unwrap();
        assert_eq!(materials.urgency, 60);
        assert!(materials.reason.contains("8 wood/6 stone"));
        village.houses = village.population().div_ceil(3) + 1;
        assert!(!village.council_priorities().iter().any(|p| p.need == CivicNeed::Materials));
    }

    #[test]
    fn partial_spawn_reports_identity_repair_not_a_new_spawn() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.actors_spawned = true;
        village.residents[0].entity_uuid = Some(9);
        let priorities = village.council_priorities();
        let identity = priorities.iter().find(|p| p.need == CivicNeed::Identity).unwrap();
        assert_eq!(identity.urgency, 88);
        assert!(identity.next_step.contains("never spawn blindly"));
        world.validate().unwrap();
    }

    #[test]
    fn fixed_tie_order_and_no_needs_are_deterministic_without_save_fields() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.houses = 5;
        village.visual_sites = vec![[20, 70, 0], [-20, 70, 0], [0, 70, 20], [0, 70, -20]];
        village.actors_spawned = true;
        for (index, resident) in village.residents.iter_mut().enumerate() {
            resident.entity_uuid = Some(index as u128 + 1);
        }
        assert_eq!(village.physical_housing(), (12, 10, 0));
        assert!(village.council_priorities().is_empty());
        let original = serde_json::to_string(&world).unwrap();
        assert!(!original.contains("council_priorities"));
        assert!(!original.contains("forecast_abstract_days"));
        world.validate().unwrap();
        assert_eq!(serde_json::to_string(&world).unwrap(), original);
        let recovered: World = serde_json::from_str(&original).unwrap();
        assert!(recovered.settlements[&id].council_priorities().is_empty());
    }
}

#[cfg(test)]
mod village_land_and_autonomy_tests {
    use super::*;

    fn founded() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        (world, id)
    }

    #[test]
    fn town_autonomy_is_on_in_new_and_legacy_saves() {
        let (world, id) = founded();
        assert!(world.settlements[&id].autonomy_enabled);
        let mut legacy = serde_json::to_value(&world).unwrap();
        let village = legacy["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap();
        village.remove("autonomy_enabled");
        village.remove("player_parcels");
        village.remove("treasury_coins");
        let loaded: World = serde_json::from_value(legacy).unwrap();
        loaded.validate().unwrap();
        assert!(loaded.settlements[&id].autonomy_enabled);
        assert!(loaded.settlements[&id].player_parcels.is_empty());
    }

    #[test]
    fn already_paid_abstract_houses_become_real_work_backlog_once() {
        let (mut world, id) = founded();
        assert_eq!(world.settlements[&id].houses, 2);
        assert_eq!(world.settlements[&id].pending_visual_buildings, 0);
        assert!(world.prepare_village_housing(id).unwrap());
        assert_eq!(world.settlements[&id].pending_visual_buildings, 2);
        assert!(!world.prepare_village_housing(id).unwrap());
        assert_eq!(world.settlements[&id].houses, 2);
        world.validate().unwrap();
        world.set_autonomy(id, false).unwrap();
        assert!(!world.settlements[&id].autonomy_enabled);
        assert!(!world.prepare_village_housing(id).unwrap());
        world.set_autonomy(id, true).unwrap();
        assert!(!world.prepare_village_housing(id).unwrap());
    }

    #[test]
    fn land_purchase_is_paid_and_blocks_ordinary_village_construction() {
        let (mut world, id) = founded();
        let before = world.players["owner"].coins;
        assert_eq!(world.buy_plot("owner", id, [20, 70, 0]), Ok(LAND_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, before - LAND_PRICE_COINS);
        assert_eq!(world.settlements[&id].treasury_coins, LAND_PRICE_COINS);
        assert_eq!(world.settlements[&id].player_parcels.len(), 1);
        assert!(world.buy_plot("owner", id, [20, 70, 0]).is_err());
        assert!(world.buy_plot("owner", id, [24, 70, 0]).is_err());
        assert_eq!(world.players["owner"].coins, before - LAND_PRICE_COINS);
        world.prepare_village_housing(id).unwrap();
        assert!(world.start_visual_house(id, [20, 70, 0]).is_err());
        assert!(world.start_farm(id, [20, 70, 0]).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn commission_is_paid_once_built_in_player_parcel_and_not_public_housing() {
        let (mut world, id) = founded();
        world.buy_plot("owner", id, [20, 70, 0]).unwrap();
        assert_eq!(world.players["owner"].coins, 15);
        assert_eq!(world.commission_house("owner", id, [20, 70, 0]), Ok(COMMISSION_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, 0);
        assert_eq!(world.settlements[&id].treasury_coins, LAND_PRICE_COINS + COMMISSION_PRICE_COINS);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), 12);
        assert_eq!(world.settlements[&id].quantity(Good::Stone), 8);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.commission_house("owner", id, [20, 70, 0]).is_err());
        assert_eq!(world.settlements[&id].houses, 3);
        assert_eq!(world.settlements[&id].pending_visual_buildings, 1);
        world.validate().unwrap();
        world.settlements.get_mut(&id).unwrap().visual_job.as_mut().unwrap().cursor =
            hut_plan([20, 70, 0]).len();
        world.settlements.get_mut(&id).unwrap().hut_supplies_reserved = true;
        world.finish_visual_house(id).unwrap();
        assert_eq!(world.settlements[&id].public_huts(), 0);
        assert_eq!(world.settlements[&id].physical_housing(), (0, 0, 10));
        let saved = serde_json::to_string(&world).unwrap();
        let recovered: World = serde_json::from_str(&saved).unwrap();
        recovered.validate().unwrap();
        assert_eq!(recovered.settlements[&id].public_huts(), 0);
    }

    #[test]
    fn another_player_cannot_commission_on_owned_plot() {
        let (mut world, id) = founded();
        world.join("other", id).unwrap();
        world.buy_plot("owner", id, [20, 70, 0]).unwrap();
        let before = world.players["other"].coins;
        assert!(world.commission_house("other", id, [20, 70, 0]).is_err());
        assert_eq!(world.players["other"].coins, before);
        assert!(!world.settlements[&id].player_parcels[0].commissioned);
        world.validate().unwrap();
    }

    #[test]
    fn no_implicit_claims_and_no_buildings_in_other_registered_land() {
        let (mut world, id) = founded();
        world.join("other", id).unwrap();
        world.buy_plot("other", id, [20, 70, 0]).unwrap();
        let before = serde_json::to_string(&world).unwrap();
        assert!(world.check_visual_house_site(id, [20, 70, 0]).is_err());
        assert!(world.start_farm(id, [20, 70, 0]).is_err());
        assert!(world.commission_house("owner", id, [20, 70, 0]).is_err());
        assert_eq!(serde_json::to_string(&world).unwrap(), before);
    }

    #[test]
    fn commission_queues_without_virtual_materials_but_cannot_build_unfunded() {
        let (mut world, id) = founded();
        world.buy_plot("owner", id, [20, 70, 0]).unwrap();
        world.settlements.get_mut(&id).unwrap().stock.insert(Good::Wood, 0);
        world.settlements.get_mut(&id).unwrap().stock.insert(Good::Stone, 0);
        assert_eq!(world.commission_house("owner", id, [20, 70, 0]), Ok(COMMISSION_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, 0);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), 0);
        assert_eq!(world.settlements[&id].quantity(Good::Stone), 0);
        assert!(!world.reserve_hut_supplies(id).unwrap());
        assert!(!world.settlements[&id].hut_supplies_reserved);
        world.validate().unwrap();
    }

    #[test]
    fn buying_finished_hut_transfers_title_and_funds_replacement() {
        let (mut world, id) = founded();
        world.players.get_mut("owner").unwrap().coins += 20;
        world.settlements.get_mut(&id).unwrap().visual_sites.push([20, 70, 0]);
        assert_eq!(world.settlements[&id].public_huts(), 1);
        assert_eq!(world.buy_finished_house("owner", id, [20,70,0]),
                   Ok(FINISHED_HUT_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, 10);
        assert_eq!(world.settlements[&id].treasury_coins, FINISHED_HUT_PRICE_COINS);
        assert_eq!(world.settlements[&id].public_huts(), 0);
        assert_eq!(world.settlements[&id].physical_housing(), (0, 0, 10));
        assert_eq!(world.settlements[&id].player_parcels[0].owner, "owner");
        assert!(world.settlements[&id].player_parcels[0].commissioned);
        let saved = serde_json::to_string(&world).unwrap();
        assert!(world.buy_finished_house("owner", id, [20,70,0]).is_err());
        assert_eq!(world.players["owner"].coins, 10);
        assert_eq!(world.settlements[&id].treasury_coins, FINISHED_HUT_PRICE_COINS);
        world.validate().unwrap();
        let loaded: World = serde_json::from_str(&saved).unwrap();
        loaded.validate().unwrap();

        // A sold hut is NOT a public house. Existing abstract day() must
        // replenish civic housing via normal resource consumption.
        world.advance(1).unwrap();
        assert_eq!(world.settlements[&id].houses, 3);
        assert_eq!(world.settlements[&id].pending_visual_buildings, 1);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), 7);
        assert_eq!(world.settlements[&id].quantity(Good::Stone), 4);
        world.validate().unwrap();
    }

    #[test]
    fn physical_housing_counts_only_public_huts() {
        let (mut world, id) = founded();
        let v = world.settlements.get_mut(&id).unwrap();
        v.houses = 2;
        v.player_parcels.push(PlayerParcel { origin: [20,70,0], owner: "owner".into(), commissioned: true });
        v.visual_sites.push([20,70,0]);
        v.visual_sites.push([40,70,0]);
        assert_eq!(v.public_huts(), 1);
        assert_eq!(v.physical_housing(), (3, 3, 7));
        let mut ids: Vec<u64> = v.residents.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        assert_eq!(v.home_slot(ids[0]).unwrap().site, [40,70,0]);
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod civic_hall_warehouse_tests {
    use super::*;

    fn settlement() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        (world, id)
    }

    fn stock_hall(world: &mut World, id: u64) {
        let v = world.settlements.get_mut(&id).unwrap();
        v.warehouse_credit(Good::Wood, HALL_WOOD_LOGS).unwrap();
        v.warehouse_credit(Good::Stone, HALL_COBBLESTONE).unwrap();
    }

    #[test]
    fn hall_is_a_unique_physical_blueprint_with_mayor_desk_and_two_bulletins() {
        let origin = [0, 70, 0];
        let hall = town_hall_plan(origin);
        let sites: BTreeSet<_> = hall.iter().map(|b| b.pos).collect();
        assert_eq!(hall.len(), sites.len());
        assert_eq!(hall.iter().filter(|b| b.floor).count(), 25);
        assert_eq!(hall.iter().filter(|b| b.block == "minecraft:barrel").count(), 2);
        assert_eq!(hall.iter().filter(|b| b.block == "minecraft:oak_sign").count(), 2);
        assert_eq!(hall.iter().filter(|b| b.block == "minecraft:lectern").count(), 1);
        assert_eq!(hall_board_positions(origin), [[-1,70,-1], [1,70,-1]]);
        assert_eq!(hall, town_hall_plan(origin));
    }

    #[test]
    fn new_and_legacy_villages_have_zero_real_warehouse_supply() {
        let (world,id) = settlement();
        let v = &world.settlements[&id];
        assert!(v.quantity(Good::Wood) > 0); // only abstract virtual stock
        assert_eq!(v.warehouse_quantity(Good::Wood), 0);
        assert_eq!(v.hall_materials_missing(), (HALL_WOOD_LOGS, HALL_COBBLESTONE));
        assert!(!v.hall_complete);
        assert!(v.hall_job.is_none());
        let mut older = serde_json::to_value(&world).unwrap();
        let v = older["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap();
        v.remove("warehouse");
        v.remove("hall_job");
        v.remove("hall_complete");
        v.remove("gather_job");
        v.remove("hut_supplies_reserved");
        let migrated: World = serde_json::from_value(older).unwrap();
        migrated.validate().unwrap();
        assert_eq!(migrated.settlements[&id].warehouse_quantity(Good::Stone), 0);
        assert!(!migrated.settlements[&id].hall_complete);
    }

    #[test]
    fn hall_reserves_real_supplies_once_and_only_finishes_after_observation() {
        let (mut world,id) = settlement();
        assert!(!world.reserve_town_hall(id).unwrap());
        stock_hall(&mut world,id);
        assert!(world.reserve_town_hall(id).unwrap());
        assert!(!world.reserve_town_hall(id).unwrap());
        let v = &world.settlements[&id];
        assert_eq!(v.warehouse_quantity(Good::Stone), 0);
        assert_eq!(v.warehouse_quantity(Good::Wood), 0);
        assert!(world.finish_town_hall(id).is_err());
        world.settlements.get_mut(&id).unwrap().hall_job.as_mut().unwrap().cursor =
            town_hall_plan([0,70,0]).len();
        world.finish_town_hall(id).unwrap();
        assert!(world.settlements[&id].hall_complete);
        assert!(world.settlements[&id].hall_job.is_none());
        assert!(!world.reserve_town_hall(id).unwrap());
        let loaded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        loaded.validate().unwrap();
        assert!(loaded.settlements[&id].hall_complete);
    }

    #[test]
    fn hall_reconciler_halts_on_player_blocks_and_will_not_clear_modifications() {
        let job = VisualBuildJob { origin: [30,70,0], cursor: 0, blocked: false };
        let floor = town_hall_plan(job.origin)[0].pos;
        assert!(matches!(
            inspect_town_hall(&job, |p| if p == floor {
                Some("minecraft:chest".to_owned())
            } else { Some("minecraft:air".to_owned()) }),
            BuildInspection::Blocked { .. }
        ));
        assert_eq!(inspect_town_hall(&job, |_| None), BuildInspection::Unloaded);
    }

    #[test]
    fn physically_received_trade_goods_do_not_double_credit_abstract_stock() {
        let (mut world,id) = settlement();
        let before = world.settlements[&id].quantity(Good::Wood);
        assert_eq!(world.sell("owner", id, Good::Wood, 8), Ok(24));
        assert_eq!(world.settlements[&id].quantity(Good::Wood), before);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        assert_eq!(world.buy("owner", id, Good::Wood, 3), Ok(9));
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 5);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), before);
        world.validate().unwrap();
    }

    #[test]
    fn a_hut_must_reserve_actual_materials_once_before_it_can_complete() {
        let (mut world,id) = settlement();
        world.advance(1).unwrap();
        world.start_visual_house(id, [20,70,0]).unwrap();
        assert!(!world.reserve_hut_supplies(id).unwrap());
        {
            let v = world.settlements.get_mut(&id).unwrap();
            v.warehouse_credit(Good::Wood, HUT_WOOD_LOGS).unwrap();
            v.warehouse_credit(Good::Stone, HUT_COBBLESTONE).unwrap();
        }
        assert!(world.reserve_hut_supplies(id).unwrap());
        assert!(!world.reserve_hut_supplies(id).unwrap());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        world.settlements.get_mut(&id).unwrap().visual_job.as_mut().unwrap().cursor =
            hut_plan([20,70,0]).len();
        world.finish_visual_house(id).unwrap();
        assert!(!world.settlements[&id].hut_supplies_reserved);
        assert_eq!(world.settlements[&id].public_huts(), 1);
        world.validate().unwrap();
    }

    #[test]
    fn gathering_requires_native_issued_receipt_and_cannot_credit_twice() {
        let (mut world,id) = settlement();
        assert!(world.plan_gather(id, Good::Bread, [24,70,0],[25,70,0]).is_err());
        assert!(world.plan_gather(id, Good::Wood, [0,70,0],[1,70,0]).is_err());
        assert!(world.plan_gather(id, Good::Wood, [24,70,0],[200,70,0]).is_err());
        world.plan_gather(id, Good::Wood, [24,70,0],[25,70,0]).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.confirm_gather(id).is_err());
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        assert!(world.mark_gather_issued(id).is_err()); // unassigned
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.mark_gather_issued(id).is_err());
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn obstructed_gather_is_discarded_without_credit_or_reissue() {
        let (mut world,id) = settlement();
        world.plan_gather(id, Good::Stone, [24,69,0],[25,70,0]).unwrap();
        world.block_gather(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        assert!(world.mark_gather_issued(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.discard_blocked_gather(id).unwrap();
        assert!(world.settlements[&id].gather_job.is_none());
        world.validate().unwrap();
    }

    #[test]
    fn only_confirmed_crop_cycles_credit_physical_warehouse_once() {
        let (mut world,id) = settlement();
        world.start_farm(id, [20,70,0]).unwrap();
        {
            let f = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
            f.stage = FarmStage::Harvesting;
            f.cursor = 8;
        }
        let abstract_before = world.settlements[&id].quantity(Good::Wheat);
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 0);
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 8);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), abstract_before);
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().harvest_sequence, 1);
        assert!(world.advance_farm(id, FarmDecision::StageComplete).is_err());
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 8);
        // The native adapter must first verify a UUID-bound farmer returned
        // to the hall. This unit test exercises only its domain commit.
        assert_eq!(world.deliver_farm_wheat(id), Ok(true));
        assert_eq!(world.deliver_farm_wheat(id), Ok(false));
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 8);
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 0);
        let restored: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.settlements[&id].warehouse_quantity(Good::Wheat), 8);
    }
}

#[cfg(test)]
mod native_barrel_transactions_tests {
    use super::*;

    fn town() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        (world,id)
    }

    #[test]
    fn barrel_origins_are_exact_and_inside_native_town_hall_plan() {
        let places = hall_storage_positions([0,70,0]);
        assert_eq!(places, [[-1,70,1],[1,70,1]]);
        let blueprint = town_hall_plan([0,70,0]);
        assert!(places.iter().all(|pos| blueprint.iter().any(|b|
            b.pos == *pos && b.block == "minecraft:barrel")));
        assert_ne!(places[0], places[1]);
    }

    #[test]
    fn historic_saves_have_empty_real_barrel_journal_and_escrow() {
        let (world,id) = town();
        let mut saved = serde_json::to_value(&world).unwrap();
        let village = saved["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap();
        village.remove("barrel_transfer");
        village.remove("market_purchase");
        village.remove("build_escrow");
        village.remove("wheat_in_transit");
        let migrated: World = serde_json::from_value(saved).unwrap();
        migrated.validate().unwrap();
        let v = &migrated.settlements[&id];
        assert!(v.barrel_transfer.is_none());
        assert!(v.market_purchase.is_none());
        assert!(v.build_escrow.is_empty());
        assert_eq!(v.wheat_in_transit, 0);
    }

    #[test]
    fn barrel_deposit_requires_real_incoming_receipts_and_a_named_hall_barrel() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 1, None).is_err());
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 8).unwrap();
        assert!(world.plan_barrel_transfer(id, [80,70,0], Good::Wood,
            BarrelDirection::ToBarrel, 0, 1, None).is_err());
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 9, None).is_err());
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 8, Some(0)).is_err());
        world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 8, None).unwrap();
        world.validate().unwrap();
    }

    #[test]
    fn native_deposit_credits_container_only_after_later_observation() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 8).unwrap();
        world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 3, 8, None).unwrap();
        assert!(world.confirm_barrel_transfer(id, 11).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        world.issue_barrel_transfer(id).unwrap();
        assert!(world.issue_barrel_transfer(id).is_err());
        assert!(world.confirm_barrel_transfer(id, 10).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        world.confirm_barrel_transfer(id, 11).unwrap(); // native observer
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.settlements[&id].barrel_transfer.is_none());
        assert!(world.confirm_barrel_transfer(id, 11).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn in_progress_native_transfer_never_reissues_or_mutates_unproven_stock() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Stone, 2).unwrap();
        world.plan_barrel_transfer(id, barrel, Good::Stone,
            BarrelDirection::ToBarrel, 0, 2, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        let json = serde_json::to_string(&world).unwrap();
        let mut rebooted: World = serde_json::from_str(&json).unwrap();
        rebooted.validate().unwrap();
        assert!(rebooted.issue_barrel_transfer(id).is_err());
        rebooted.block_barrel_transfer(id).unwrap();
        assert!(rebooted.confirm_barrel_transfer(id, 2).is_err());
        assert_eq!(rebooted.settlements[&id].warehouse_quantity(Good::Stone), 2);
        assert_eq!(rebooted.settlements[&id].barrel_transfer.as_ref().unwrap().phase,
                   BarrelPhase::Blocked);
        rebooted.validate().unwrap();
    }

    #[test]
    fn builder_only_funds_from_materials_extracted_from_real_barrel() {
        let (mut world,id) = town();
        world.advance(1).unwrap();
        world.start_visual_house(id, [20,70,0]).unwrap();
        let v = world.settlements.get_mut(&id).unwrap();
        v.warehouse_credit(Good::Wood, HUT_WOOD_LOGS).unwrap();
        v.warehouse_credit(Good::Stone, HUT_COBBLESTONE).unwrap();
        assert!(!world.reserve_hut_supplies(id).unwrap());
        let barrel = hall_storage_positions([0,70,0])[0];
        for (good,count) in [(Good::Wood, 8),(Good::Stone, 8)] {
            world.plan_barrel_transfer(id, barrel, good,
                BarrelDirection::ToBuilder, count, count, Some(0)).unwrap();
            world.issue_barrel_transfer(id).unwrap();
            world.confirm_barrel_transfer(id, 0).unwrap();
        }
        assert_eq!(world.settlements[&id].build_escrow[&Good::Wood], 8);
        assert!(!world.reserve_hut_supplies(id).unwrap());
        world.settlements.get_mut(&id).unwrap().build_escrow.insert(Good::Wood,HUT_WOOD_LOGS);
        world.settlements.get_mut(&id).unwrap().build_escrow.insert(Good::Stone,HUT_COBBLESTONE);
        assert!(world.reserve_hut_supplies(id).unwrap());
        assert!(!world.reserve_hut_supplies(id).unwrap());
        assert!(world.settlements[&id].hut_supplies_reserved);
        assert_eq!(world.settlements[&id].build_escrow[&Good::Wood], 0);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), HUT_WOOD_LOGS);
        world.validate().unwrap();
    }

    #[test]
    fn native_market_purchase_debits_currency_before_external_delivery() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        let coins = world.players["owner"].coins;
        let cost = world.begin_market_purchase("owner", id, Good::Wood,
            4, barrel, 0, 18).unwrap();
        assert_eq!(cost, 12);
        assert_eq!(world.players["owner"].coins, coins - 12);
        assert_eq!(world.settlements[&id].treasury_coins, 12);
        assert_eq!(world.settlements[&id].market_purchase.as_ref().unwrap().phase,
                   MarketPhase::Issued);
        assert!(world.begin_market_purchase("owner", id, Good::Wood,
            4, barrel, 0, 18).is_err());
        assert!(world.complete_market_purchase(id, "forged").is_err());
        world.complete_market_purchase(id, "owner").unwrap(); // native delivery validated
        assert!(world.settlements[&id].market_purchase.is_none());
        assert_eq!(world.players["owner"].coins, coins - 12);
        world.validate().unwrap();
    }

    #[test]
    fn unresolved_market_purchase_blocks_concurrent_native_stock_movements() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        world.begin_market_purchase("owner", id, Good::Wood, 4, barrel, 0, 18).unwrap();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Stone, 3).unwrap();
        assert!(world.plan_barrel_transfer(id, barrel, Good::Stone,
            BarrelDirection::ToBarrel, 0, 3, None).is_err());
        world.block_market_purchase(id).unwrap();
        assert!(world.complete_market_purchase(id, "owner").is_err());
        let persisted: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        persisted.validate().unwrap();
        assert_eq!(persisted.settlements[&id].market_purchase.as_ref().unwrap().phase,
                   MarketPhase::Blocked);
    }

    #[test]
    fn receipt_wheat_is_delivered_once_by_farmer_before_barrel_transfer() {
        let (mut world,id) = town();
        world.settlements.get_mut(&id).unwrap().wheat_in_transit = 16;
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 0);
        assert_eq!(world.deliver_farm_wheat(id), Ok(true));
        assert_eq!(world.deliver_farm_wheat(id), Ok(false));
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 0);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 16);
        let barrel = hall_storage_positions([0,70,0])[1];
        world.plan_barrel_transfer(id, barrel, Good::Wheat,
            BarrelDirection::ToBarrel, 0, 8, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id, 8).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 8);
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod native_bakery_economy_tests {
    use super::*;

    fn village() -> (World,u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        (world,id)
    }
    fn hall() -> (World,u64) {
        let (mut world,id) = village();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        (world,id)
    }
    fn ready_bakery() -> (World,u64) {
        let (mut world,id) = hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        {
            let b = world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
            b.construction_escrow.insert(Good::Wood, BAKERY_WOOD_LOGS);
            b.construction_escrow.insert(Good::Stone, BAKERY_COBBLESTONE);
        }
        assert!(world.reserve_bakery_supplies(id).unwrap());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor = bakery_plan([24,70,0]).len();
        world.finish_bakery(id).unwrap();
        world.validate().unwrap();
        (world,id)
    }

    #[test]
    fn native_bakery_has_one_crafting_table_and_real_workstation() {
        let blueprint = bakery_plan([24,70,0]);
        let positions: BTreeSet<_> = blueprint.iter().map(|b| b.pos).collect();
        assert_eq!(positions.len(), blueprint.len());
        assert_eq!(blueprint.iter().filter(|b| b.block == "minecraft:crafting_table").count(),1);
        assert_eq!(blueprint.iter().filter(|b| b.block == "minecraft:smoker").count(),1);
        assert_eq!(blueprint.iter().filter(|b| b.floor).count(),25);
        assert_eq!(blueprint,bakery_plan([24,70,0]));
    }

    #[test]
    fn legacy_saves_do_not_forge_a_free_existing_bakery() {
        let (world,id)=village();
        let json=serde_json::to_string(&world).unwrap();
        let mut value: serde_json::Value=serde_json::from_str(&json).unwrap();
        value["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap().remove("bakery");
        let migrated: World=serde_json::from_value(value).unwrap();
        migrated.validate().unwrap();
        assert!(migrated.settlements[&id].bakery.is_none());
    }

    #[test]
    fn bakery_requires_real_hall_and_an_unclaimed_safe_plot() {
        let (mut world,id)=village();
        assert!(world.start_bakery(id,[24,70,0]).is_err());
        world.settlements.get_mut(&id).unwrap().hall_complete=true;
        assert!(world.start_bakery(id,[2,70,0]).is_err());
        world.buy_plot("owner",id,[24,70,0]).unwrap();
        assert!(world.start_bakery(id,[24,70,0]).is_err());
        world.start_bakery(id,[40,70,0]).unwrap();
        assert!(world.start_bakery(id,[60,70,0]).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn unbacked_bakery_never_constructs_and_funding_is_once_only() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        let v=world.settlements.get_mut(&id).unwrap();
        v.warehouse_credit(Good::Wood,100).unwrap();
        v.warehouse_credit(Good::Stone,100).unwrap();
        assert!(!world.reserve_bakery_supplies(id).unwrap()); // queue is not a real barrel
        assert!(world.finish_bakery(id).is_err());
        {
            let b=world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
            b.construction_escrow.insert(Good::Wood,BAKERY_WOOD_LOGS);
            b.construction_escrow.insert(Good::Stone,BAKERY_COBBLESTONE-1);
        }
        assert!(!world.reserve_bakery_supplies(id).unwrap());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .construction_escrow.insert(Good::Stone,BAKERY_COBBLESTONE);
        assert!(world.reserve_bakery_supplies(id).unwrap());
        assert!(!world.reserve_bakery_supplies(id).unwrap());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood),100);
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap()
            .construction_escrow[&Good::Wood],0);
        world.validate().unwrap();
    }

    #[test]
    fn bakery_only_finishes_after_all_workstations_physically_verified() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().complete,false);
        assert!(world.finish_bakery(id).is_err());
        let job=world.settlements[&id].bakery.as_ref().unwrap().building.as_ref().unwrap().clone();
        assert!(matches!(inspect_bakery(&job, |_|None),BuildInspection::Unloaded));
        let station=bakery_plan(job.origin).last().unwrap().pos;
        assert!(matches!(inspect_bakery(&job, |p| if p==station {
            Some("minecraft:player_block".into())
        } else { Some("minecraft:air".into()) }),BuildInspection::Blocked {..}
            | BuildInspection::Place {..}));
        // Simulate the native world's complete observation, not merely
        // declaring a structure built with abstract house counters.
        let b=world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
        b.construction_escrow.insert(Good::Wood,BAKERY_WOOD_LOGS);
        b.construction_escrow.insert(Good::Stone,BAKERY_COBBLESTONE);
        world.reserve_bakery_supplies(id).unwrap();
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor = bakery_plan([24,70,0]).len();
        world.finish_bakery(id).unwrap();
        assert!(world.settlements[&id].bakery.as_ref().unwrap().complete);
        world.validate().unwrap();
    }

    #[test]
    fn real_barrel_materials_move_to_bakery_escrow_not_virtual_stock() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        let pos=hall_storage_positions([0,70,0])[0];
        assert!(world.plan_barrel_transfer(id,pos,Good::Bread,
            BarrelDirection::ToBakeryBuilder,8,8,Some(0)).is_err());
        world.plan_barrel_transfer(id,pos,Good::Wood,
            BarrelDirection::ToBakeryBuilder,8,8,Some(0)).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        assert!(world.confirm_barrel_transfer(id,8).is_err());
        world.confirm_barrel_transfer(id,0).unwrap();
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap()
            .construction_escrow[&Good::Wood],8);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood),0);
        world.validate().unwrap();
    }

    #[test]
    fn a_baker_never_uses_virtual_stock_and_crafts_exact_recipe() {
        let (mut world,id)=ready_bakery();
        assert!(world.settlements[&id].quantity(Good::Wheat)>0);
        assert_eq!(world.craft_bread(id),Ok(false));
        let pos=hall_storage_positions([0,70,0])[1];
        world.plan_barrel_transfer(id,pos,Good::Wheat,
            BarrelDirection::ToBakerIngredients,3,3,Some(0)).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id,0).unwrap();
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().wheat_escrow,3);
        assert_eq!(world.craft_bread(id),Ok(true));
        assert_eq!(world.craft_bread(id),Ok(false));
        let bakery=world.settlements[&id].bakery.as_ref().unwrap();
        assert_eq!(bakery.wheat_escrow,0);
        assert_eq!(bakery.bread_in_transit,1);
        assert_eq!(bakery.batches,1);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Bread),0);
        world.validate().unwrap();
    }

    #[test]
    fn baker_delivers_one_receipt_then_steward_inserts_actual_bread() {
        let (mut world,id)=ready_bakery();
        let pos=hall_storage_positions([0,70,0])[1];
        world.plan_barrel_transfer(id,pos,Good::Wheat,
            BarrelDirection::ToBakerIngredients,3,3,Some(0)).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id,0).unwrap();
        world.craft_bread(id).unwrap();
        assert_eq!(world.deliver_bakery_bread(id),Ok(true));
        assert_eq!(world.deliver_bakery_bread(id),Ok(false));
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Bread),1);
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().bread_in_transit,0);
        world.plan_barrel_transfer(id,pos,Good::Bread,
            BarrelDirection::ToBarrel,0,1,None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id,1).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Bread),0);
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().batches,1);
        let restored: World=serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
    }

    #[test]
    fn native_block_ownership_protects_public_work_and_private_lots() {
        let (mut world,id)=ready_bakery();
        assert!(!world.player_may_break("stranger","minecraft:overworld",[0,70,0]));
        assert!(!world.player_may_break("stranger","minecraft:overworld",[24,70,0]));
        assert!(world.player_may_break("stranger","minecraft:the_nether",[24,70,0]));
        assert!(world.player_may_break("stranger","minecraft:overworld",[100,70,0]));
        world.buy_plot("owner",id,[40,70,0]).unwrap();
        assert!(world.player_may_break("owner","minecraft:overworld",[40,70,0]));
        assert!(!world.player_may_break("stranger","minecraft:overworld",[40,70,0]));
        assert!(world.player_may_break("stranger","minecraft:overworld",[40,60,0]));
        world.validate().unwrap();
    }

    #[test]
    fn bakery_job_rejects_blocked_or_overflowed_saved_work() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor=bakery_plan([24,70,0]).len()+1;
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor=0;
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap().wheat_escrow=13;
        assert!(world.validate().is_err());
    }
}

#[cfg(test)]
mod physical_logistics_hardening_tests {
    use super::*;

    fn with_bakery_and_hut() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        world.start_bakery(id, [24, 70, 0]).unwrap();
        let village = world.settlements.get_mut(&id).unwrap();
        village.houses = 1;
        village.pending_visual_buildings = 1;
        (world, id)
    }

    #[test]
    fn two_unfunded_projects_need_both_bills_not_only_bakery() {
        let (world, id) = with_bakery_and_hut();
        let village = &world.settlements[&id];
        assert_eq!(physical_construction_shortfall(village, &[]),
            (BAKERY_WOOD_LOGS + HUT_WOOD_LOGS,
             BAKERY_COBBLESTONE + HUT_COBBLESTONE));
    }

    #[test]
    fn project_escrows_are_not_double_counted_as_public_stock() {
        let (mut world, id) = with_bakery_and_hut();
        let village = world.settlements.get_mut(&id).unwrap();
        village.build_escrow.insert(Good::Wood, 4);
        village.bakery.as_mut().unwrap().construction_escrow.insert(Good::Wood, 6);
        village.warehouse_credit(Good::Wood, 5).unwrap();
        let stock = [(Good::Wood, 12), (Good::Stone, 10)];
        // 44 total wood - 10 in protected project escrows - 17 available.
        assert_eq!(physical_construction_shortfall(village, &stock), (17, 48));
    }

    #[test]
    fn blocked_bakery_does_not_induce_more_resource_harvests() {
        let (mut world, id) = with_bakery_and_hut();
        let village = world.settlements.get_mut(&id).unwrap();
        village.bakery.as_mut().unwrap().building.as_mut().unwrap().blocked = true;
        assert_eq!(physical_construction_shortfall(village, &[]),
            (HUT_WOOD_LOGS, HUT_COBBLESTONE));
        village.pending_visual_buildings = 0;
        assert_eq!(physical_construction_shortfall(village, &[]), (0, 0));
    }


    #[test]
    fn no_new_extraction_into_obstructed_bakery_job() {
        let (mut world, id) = with_bakery_and_hut();
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().blocked = true;
        let barrel = hall_storage_positions([0, 70, 0])[0];
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBakeryBuilder, 8, 8, Some(0)).is_err());
        assert!(world.validate().is_ok());
    }

    #[test]
    fn saved_wheat_extraction_rejects_unfinished_or_full_bakery() {
        let (mut world, id) = with_bakery_and_hut();
        let barrel = hall_storage_positions([0, 70, 0])[0];
        world.settlements.get_mut(&id).unwrap().barrel_transfer = Some(BarrelTransfer {
            position: barrel, good: Good::Wheat,
            direction: BarrelDirection::ToBakerIngredients,
            phase: BarrelPhase::Planned, before: 2, count: 1, slot: Some(0),
        });
        assert!(world.validate().is_err()); // no completed workstation
        let bakery = world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
        bakery.building = None;
        bakery.supplies_reserved = true;
        bakery.complete = true;
        assert!(world.validate().is_ok());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .wheat_escrow = BREAD_WHEAT_RECIPE;
        assert!(world.validate().is_err()); // no more wheat may be carried
    }

    #[test]
    fn saved_bakery_transfer_requires_a_real_bakery_after_reboot() {
        let (mut world, id) = with_bakery_and_hut();
        let barrel = hall_storage_positions([0, 70, 0])[0];
        world.settlements.get_mut(&id).unwrap().barrel_transfer = Some(BarrelTransfer {
            position: barrel, good: Good::Wood,
            direction: BarrelDirection::ToBakeryBuilder,
            phase: BarrelPhase::Planned, before: 2, count: 1, slot: Some(0),
        });
        assert!(world.validate().is_ok());
        world.settlements.get_mut(&id).unwrap().bakery = None;
        assert!(world.validate().is_err());
    }
}

#[cfg(test)]
mod manual_warehouse_recovery_tests {
    use super::*;

    fn ready() -> (World, u64, [i32; 3]) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        let pos = hall_storage_positions([0, 70, 0])[0];
        (world, id, pos)
    }

    #[test]
    fn unissued_native_intent_can_be_canceled_once() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 8).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Wood,
            BarrelDirection::ToBarrel, 0, 8, None).unwrap();
        world.cancel_planned_barrel_transfer(id).unwrap();
        assert!(world.settlements[&id].barrel_transfer.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        assert!(world.cancel_planned_barrel_transfer(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn issued_and_blocked_transactions_cannot_be_canceled_without_evidence() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Stone, 3).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Stone,
            BarrelDirection::ToBarrel, 0, 3, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        assert!(world.cancel_planned_barrel_transfer(id).is_err());
        world.block_barrel_transfer(id).unwrap();
        assert!(world.cancel_planned_barrel_transfer(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 3);
    }

    #[test]
    fn blocked_receipt_matches_exact_physical_after_count_only() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 4).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Wood,
            BarrelDirection::ToBarrel, 10, 4, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.block_barrel_transfer(id).unwrap();
        let prior = serde_json::to_value(&world).unwrap();
        assert!(world.reconcile_blocked_barrel_transfer(id, 10).is_err());
        assert_eq!(serde_json::to_value(&world).unwrap(), prior);
        world.reconcile_blocked_barrel_transfer(id, 14).unwrap();
        assert!(world.settlements[&id].barrel_transfer.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.reconcile_blocked_barrel_transfer(id, 14).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn reconciliation_rolls_back_when_inbound_receipt_was_lost() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 4).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Wood,
            BarrelDirection::ToBarrel, 0, 4, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.block_barrel_transfer(id).unwrap();
        world.settlements.get_mut(&id).unwrap().warehouse_debit(Good::Wood, 4).unwrap();
        let prior = serde_json::to_value(&world).unwrap();
        assert!(world.reconcile_blocked_barrel_transfer(id, 4).is_err());
        assert_eq!(serde_json::to_value(&world).unwrap(), prior);
        assert_eq!(world.settlements[&id].barrel_transfer.as_ref().unwrap().phase,
            BarrelPhase::Blocked);
    }
}

#[cfg(test)]
mod physical_bakery_stock_control_tests {
    use super::*;

    #[test]
    fn physical_bread_buffer_is_bounded_by_actual_residents() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let village = &world.settlements[&id];
        assert_eq!(physical_bread_target(village), village.population() * 2);
        assert!(needs_real_bread(village, 0));
        assert!(!needs_real_bread(village, physical_bread_target(village)));
    }

    #[test]
    fn physical_bread_stock_and_incoming_delivery_are_not_counted_twice() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let village = world.settlements.get_mut(&id).unwrap();
        let target = physical_bread_target(village);
        village.warehouse_credit(Good::Bread, 4).unwrap();
        assert!(needs_real_bread(village, target - 5));
        assert!(!needs_real_bread(village, target - 4));
        assert!(needs_real_bread(village, 0));
    }

    #[test]
    fn baker_in_transit_bread_counts_before_restocking() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let village = world.settlements.get_mut(&id).unwrap();
        village.bakery = Some(BakeryWorkshop {
            origin: [24, 70, 0], building: None, complete: true,
            supplies_reserved: true, construction_escrow: BTreeMap::new(),
            wheat_escrow: 0, bread_in_transit: 2, batches: 2,
        });
        let target = physical_bread_target(village);
        assert!(!needs_real_bread(village, target - 2));
        assert!(needs_real_bread(village, target - 3));
    }
}

#[cfg(test)]
mod native_forest_origin_tests {
    use super::*;

    fn growing_tree() -> BTreeMap<[i32; 3], String> {
        let mut blocks = BTreeMap::new();
        blocks.insert([0, 69, 0], "minecraft:grass_block".into());
        for y in 70..=73 {
            blocks.insert([0, y, 0], "minecraft:oak_log".into());
        }
        blocks.insert([1, 73, 0], "minecraft:oak_leaves".into());
        blocks
    }

    #[test]
    fn real_rooted_oak_qualifies_at_base_not_upper_trunk() {
        let b = growing_tree();
        let read = |p| Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()));
        assert!(is_rooted_oak_harvest_origin([0, 70, 0], read));
        assert!(!is_rooted_oak_harvest_origin([0, 71, 0], |p|
            Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()))));
    }

    #[test]
    fn decorated_timber_and_disconnected_leaves_are_not_forestry_receipts() {
        let mut b = growing_tree();
        b.remove(&[0, 72, 0]);
        assert!(!is_rooted_oak_harvest_origin([0, 70, 0], |p|
            Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        b.insert([0, 72, 0], "minecraft:oak_log".into());
        b.insert([0, 69, 0], "minecraft:cobblestone".into());
        assert!(!is_rooted_oak_harvest_origin([0, 70, 0], |p|
            Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()))));
    }

    #[test]
    fn unloaded_crown_is_not_proof_of_harvestable_wood() {
        let b = growing_tree();
        assert!(!is_rooted_oak_harvest_origin([0, 70, 0], |p|
            if p == [1, 72, 0] { None }
            else { Some(b.get(&p).cloned().unwrap_or("minecraft:air".into())) }));
    }


    #[test]
    fn resource_job_queued_before_lot_purchase_loses_permission() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let origin = [24, 70, 0];
        let stand = [25, 70, 0];
        assert!(world.gather_site_unclaimed(id, origin, stand));
        world.plan_gather(id, Good::Wood, origin, stand).unwrap();
        world.settlements.get_mut(&id).unwrap().player_parcels.push(PlayerParcel {
            origin, owner: "owner".into(), commissioned: false,
        });
        assert!(!world.gather_site_unclaimed(id, origin, stand));
        world.validate().unwrap();
    }

    #[test]
    fn worksite_cannot_approach_occupied_housing_or_bakery_from_side() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let origin = [32,70,0];
        assert!(world.gather_site_unclaimed(id, origin, [33,70,0]));
        world.settlements.get_mut(&id).unwrap().player_parcels.push(PlayerParcel {
            origin: [42,70,0], owner: "owner".into(), commissioned: false,
        });
        // The harvesting BLOCK is distant, but the worker stand touches
        // somebody else's registered parcel exclusion radius.
        assert!(!world.gather_site_unclaimed(id, origin, [34,70,0]));
    }

    #[test]
    fn legacy_world_roundtrip_needs_no_new_forestry_save_fields() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.validate().unwrap();
        let restored: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert!(restored.settlements[&id].gather_job.is_none());
        restored.validate().unwrap();
    }
}

#[cfg(test)]
mod native_gather_quarantine_tests {
    use super::*;

    #[test]
    fn issued_blocked_mining_receipt_requires_explicit_abandonment() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        // This record is a durable receipt of an issued natural stone edit.
        world.settlements.get_mut(&id).unwrap().gather_job = Some(GatherJob {
            position: [16,70,0], stand: [17,70,0],
            good: Good::Stone, worker_id: None, issued: true,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: true,
        });
        world.validate().unwrap();
        let before = world.settlements[&id].warehouse_quantity(Good::Stone);
        let restored: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert!(restored.settlements[&id].gather_job.as_ref().unwrap().blocked);
        assert!(world.mark_gather_issued(id).is_err());
        world.abandon_blocked_issued_gather(id).unwrap();
        assert!(world.settlements[&id].gather_job.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), before);
        assert!(world.abandon_blocked_issued_gather(id).is_err());
        world.validate().unwrap();
    }


    #[test]
    fn corrupted_gather_worker_stand_is_rejected_on_load() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        world.plan_gather(id, Good::Stone, [16,70,0], [17,70,0]).unwrap();
        world.validate().unwrap();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap().stand =
            [i32::MAX, 70, 0];
        assert!(world.validate().is_err());
    }

    #[test]
    fn manually_abandoning_unissued_or_unblocked_gather_is_forbidden() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().gather_job = Some(GatherJob {
            position: [16,70,0], stand: [17,70,0],
            good: Good::Wood, worker_id: None, issued: false,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: true,
        });
        assert!(world.abandon_blocked_issued_gather(id).is_err());
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap().issued = true;
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap().blocked = false;
        assert!(world.abandon_blocked_issued_gather(id).is_err());
    }
}

#[cfg(test)]
mod verified_short_oak_stem_tests {
    use super::*;

    fn world_with_stem() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.plan_short_oak_stem(id, [16, 70, 0], [17, 70, 0], 4).unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.assign_gather_worker(id, worker).unwrap();
        world.validate().unwrap();
        (world, id)
    }

    fn tree_blocks() -> BTreeMap<[i32; 3], String> {
        let mut blocks = BTreeMap::new();
        blocks.insert([0, 69, 0], "minecraft:grass_block".into());
        for y in 70..=73 {
            blocks.insert([0, y, 0], "minecraft:oak_log".into());
        }
        blocks.insert([1, 73, 0], "minecraft:oak_leaves".into());
        blocks
    }

    #[test]
    fn short_oak_requires_a_verified_root_and_reachable_height() {
        let blocks = tree_blocks();
        let observe = |pos| Some(blocks.get(&pos).cloned().unwrap_or("minecraft:air".into()));
        assert_eq!(short_oak_stem_height([0, 70, 0], observe), Some(4));
        assert_eq!(short_oak_stem_height([0, 71, 0], |pos|
            Some(blocks.get(&pos).cloned().unwrap_or("minecraft:air".into()))), None);
        let mut tall = blocks.clone();
        tall.insert([0, 74, 0], "minecraft:oak_log".into());
        assert_eq!(short_oak_stem_height([0, 70, 0], |pos|
            Some(tall.get(&pos).cloned().unwrap_or("minecraft:air".into()))), None);
    }

    #[test]
    fn actual_logs_advance_one_by_one_only_after_worker_delivery_receipt() {
        let (mut world, id) = world_with_stem();
        for cursor in 0..4_u8 {
            let job = world.settlements[&id].gather_job.as_ref().unwrap();
            assert_eq!(job.position, [16, 70 + i32::from(cursor), 0]);
            assert!(!job.issued);
            assert_eq!(world.settlements[&id].oak_stem.as_ref().unwrap().cursor, cursor);
            world.validate().unwrap();
            world.mark_gather_issued(id).unwrap();
            world.mark_gather_native_edit_confirmed(id).unwrap();
            assert!(world.mark_gather_issued(id).is_err());
            let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
            world.witness_gather_pickup(id, world_assigned).unwrap();
            world.confirm_gather(id).unwrap();
            assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood),
                       u32::from(cursor) + 1);
        }
        assert!(world.settlements[&id].oak_stem.is_none());
        assert!(world.settlements[&id].gather_job.is_none());
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn crash_reload_preserves_next_segment_and_does_not_reissue_prior_log() {
        let (mut world, id) = world_with_stem();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        let json = serde_json::to_string(&world).unwrap();
        let mut replay: World = serde_json::from_str(&json).unwrap();
        replay.validate().unwrap();
        assert_eq!(replay.settlements[&id].oak_stem.as_ref().unwrap().cursor, 1);
        assert_eq!(replay.settlements[&id].gather_job.as_ref().unwrap().position,
                   [16, 71, 0]);
        assert_eq!(replay.settlements[&id].warehouse_quantity(Good::Wood), 1);
        assert!(replay.confirm_gather(id).is_err());
        replay.mark_gather_issued(id).unwrap();
        replay.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(replay.mark_gather_issued(id).is_err());
        let replay_assigned = replay.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        replay.witness_gather_pickup(id, replay_assigned).unwrap();
        replay.confirm_gather(id).unwrap();
        assert_eq!(replay.settlements[&id].warehouse_quantity(Good::Wood), 2);
    }

    #[test]
    fn corrupted_stem_cursor_or_missing_gather_is_rejected_on_load() {
        let (mut world, id) = world_with_stem();
        world.settlements.get_mut(&id).unwrap().oak_stem.as_mut().unwrap().cursor = 3;
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().oak_stem.as_mut().unwrap().cursor = 0;
        world.settlements.get_mut(&id).unwrap().gather_job = None;
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_world_without_new_field_still_loads() {
        let (world, id) = world_with_stem();
        let mut json = serde_json::to_value(&world).unwrap();
        json.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap()
            .remove("oak_stem");
        let restored: World = serde_json::from_value(json).unwrap();
        assert!(restored.settlements[&id].oak_stem.is_none());
        restored.validate().unwrap();
    }

    #[test]
    fn warehouse_overflow_does_not_advance_stem_or_create_unpaid_log() {
        let (mut world, id) = world_with_stem();
        world.settlements.get_mut(&id).unwrap()
            .warehouse_credit(Good::Wood, 10_000_000).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let worker = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, worker).unwrap();
        let snapshot = serde_json::to_value(&world).unwrap();
        assert!(world.confirm_gather(id).is_err());
        assert_eq!(serde_json::to_value(&world).unwrap(), snapshot);
        assert_eq!(world.settlements[&id].oak_stem.as_ref().unwrap().cursor, 0);
    }

    #[test]
    fn abandoning_uncertain_segment_discards_remaining_tree_with_zero_credit() {
        let (mut world, id) = world_with_stem();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.block_gather(id).unwrap();
        world.validate().unwrap();
        world.abandon_blocked_issued_gather(id).unwrap();
        assert!(world.settlements[&id].oak_stem.is_none());
        assert!(world.settlements[&id].gather_job.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 1);
        world.validate().unwrap();
    }

    #[test]
    fn native_followup_must_see_air_prefix_and_unchanged_leafy_crown() {
        let mut blocks = tree_blocks();
        let mut stem = OakStem {
            root: [0,70,0], stand: [1,70,0], height: 4, cursor: 0,
        };
        assert!(inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        stem.cursor = 1;
        // The next log cannot be cut until the previous one was really air.
        assert!(!inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        blocks.remove(&[0,70,0]);
        assert!(inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        blocks.remove(&[1,73,0]);
        assert!(!inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        blocks.insert([1,73,0], "minecraft:oak_leaves".into());
        assert!(!inspect_oak_stem(&stem, |p|
            if p == [0,72,0] { None }
            else { Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into())) }));
    }
}

#[cfg(test)]
mod native_worker_distance_contract_tests {
    use super::*;

    #[test]
    fn worker_directly_at_hall_is_near_but_same_xz_deep_below_is_not() {
        let hall = [20, 70, 35];
        assert!(native_worker_near([20.5, 70.0, 35.5], hall, 8.0, 4.0));
        assert!(!native_worker_near([20.5, 45.0, 35.5], hall, 8.0, 4.0));
        assert!(!native_worker_near([20.5, 93.0, 35.5], hall, 8.0, 4.0));
        assert!(!native_worker_near([40.5, 70.0, 35.5], hall, 8.0, 4.0));
    }

    #[test]
    fn worker_can_reach_verified_short_tree_without_felling_distant_trunks() {
        let feet = [17.5, 70.0, 0.5];
        assert!(native_worker_reaches_block(feet, [16, 70, 0], 4.5));
        assert!(native_worker_reaches_block(feet, [16, 73, 0], 4.5));
        assert!(!native_worker_reaches_block(feet, [16, 76, 0], 4.5));
        assert!(!native_worker_reaches_block(feet, [30, 73, 0], 4.5));
        assert!(!native_worker_reaches_block([17.5, 40.0, 0.5], [16, 73, 0], 4.5));
    }

    #[test]
    fn invalid_actor_or_radius_cannot_authorize_native_delivery() {
        assert!(!native_worker_near(
            [f64::NAN, 70.0, 0.5], [0, 70, 0], 8.0, 4.0
        ));
        assert!(!native_worker_near(
            [0.5, 70.0, 0.5], [0, 70, 0], f64::INFINITY, 4.0
        ));
        assert!(!native_worker_near(
            [0.5, 70.0, 0.5], [0, 70, 0], 8.0, -1.0
        ));
        assert!(!native_worker_reaches_block(
            [0.5, f64::INFINITY, 0.5], [0, 70, 0], 4.5
        ));
    }
}

#[cfg(test)]
mod assigned_resource_courier_tests {
    use super::*;

    fn assigned_wood_job() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_gather(id, Good::Wood, [16, 70, 0], [17, 70, 0]).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.validate().unwrap();
        (world, id, worker)
    }

    #[test]
    fn source_issue_requires_explicit_same_resident_binding() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.plan_gather(id, Good::Stone, [16, 70, 0], [17, 70, 0]).unwrap();
        assert!(world.mark_gather_issued(id).is_err());
        assert!(world.confirm_gather(id).is_err());
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.assign_gather_worker(id, miner).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.assign_gather_worker(id, miner).is_err());
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn unqualified_or_second_worker_cannot_steal_courier_assignment() {
        let (mut world, id, first) = assigned_wood_job();
        let baker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Baker).unwrap().id;
        assert!(world.assign_gather_worker(id, baker).is_err());
        let mut another = world.settlements[&id].residents.iter()
            .find(|r| r.id == first).unwrap().clone();
        another.id += 500;
        another.name = "Substitute".into();
        let second = another.id;
        world.settlements.get_mut(&id).unwrap().residents.push(another);
        assert!(world.assign_gather_worker(id, second).is_err());
        assert_eq!(world.settlements[&id].gather_job.as_ref().unwrap().worker_id, Some(first));
        world.validate().unwrap();
    }

    #[test]
    fn same_courier_persists_through_restart_and_next_real_oak_log() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_short_oak_stem(id, [16, 70, 0], [17, 70, 0], 3).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        let mut restored: World =
            serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        let next = restored.settlements[&id].gather_job.as_ref().unwrap();
        assert_eq!(next.worker_id, Some(worker));
        assert_eq!(next.position, [16, 71, 0]);
        assert!(!next.issued);
        assert!(restored.confirm_gather(id).is_err());
        restored.mark_gather_issued(id).unwrap();
        restored.mark_gather_native_edit_confirmed(id).unwrap();
        let restored_assigned = restored.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        restored.witness_gather_pickup(id, restored_assigned).unwrap();
        restored.confirm_gather(id).unwrap();
        assert_eq!(restored.settlements[&id].warehouse_quantity(Good::Wood), 2);
    }

    #[test]
    fn preexisting_unbound_issued_receipt_cannot_mint_stock() {
        let (mut world, id, _) = assigned_wood_job();
        world.mark_gather_issued(id).unwrap();
        let mut saved = serde_json::to_value(&world).unwrap();
        saved.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap()
            .get_mut("gather_job").unwrap().as_object_mut().unwrap()
            .remove("worker_id");
        let mut old: World = serde_json::from_value(saved).unwrap();
        old.validate().unwrap();
        assert!(old.settlements[&id].gather_job.as_ref().unwrap().worker_id.is_none());
        assert!(old.confirm_gather(id).is_err());
        assert_eq!(old.settlements[&id].warehouse_quantity(Good::Wood), 0);
        old.block_gather(id).unwrap();
        old.abandon_blocked_issued_gather(id).unwrap();
        old.validate().unwrap();
    }

    #[test]
    fn malformed_assigned_courier_id_is_rejected_by_save_validation() {
        let (mut world, id, _) = assigned_wood_job();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap()
            .worker_id = Some(u64::MAX);
        assert!(world.validate().is_err());
    }

    #[test]
    fn dead_resident_cannot_be_newly_assigned_to_native_material_receipt() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16, 70, 0], [17, 70, 0]).unwrap();
        world.settlements.get_mut(&id).unwrap().residents.iter_mut()
            .find(|r| r.id == miner).unwrap().alive = false;
        assert!(world.assign_gather_worker(id, miner).is_err());
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod native_gather_pickup_witness_tests {
    use super::*;

    fn assigned_stone() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16,70,0], [17,70,0]).unwrap();
        world.assign_gather_worker(id, miner).unwrap();
        (world, id, miner)
    }

    #[test]
    fn no_stock_credit_between_issue_and_source_pickup_witness() {
        let (mut world, id, miner) = assigned_stone();
        assert!(world.witness_gather_pickup(id, miner).is_err());
        world.mark_gather_issued(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, miner).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn other_worker_or_unknown_id_cannot_claim_source_pickup() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let stranger = world.settlements[&id].residents.iter()
            .find(|r| r.id != miner).unwrap().id;
        assert!(world.witness_gather_pickup(id, stranger).is_err());
        assert!(world.witness_gather_pickup(id, u64::MAX).is_err());
        assert!(!world.settlements[&id].gather_job.as_ref().unwrap().pickup_witnessed);
        world.witness_gather_pickup(id, miner).unwrap();
        assert!(world.witness_gather_pickup(id, miner).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn witnessed_pickup_survives_reload_without_duplication() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, miner).unwrap();
        let mut restored: World =
            serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        assert!(restored.settlements[&id].gather_job.as_ref().unwrap().pickup_witnessed);
        assert!(restored.witness_gather_pickup(id, miner).is_err());
        restored.confirm_gather(id).unwrap();
        assert!(restored.confirm_gather(id).is_err());
        assert_eq!(restored.settlements[&id].warehouse_quantity(Good::Stone), 1);
    }

    #[test]
    fn every_new_oak_segment_resets_pickup_until_this_log_is_observed() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_short_oak_stem(id, [16,70,0], [17,70,0], 3).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, worker).unwrap();
        world.confirm_gather(id).unwrap();
        let next = world.settlements[&id].gather_job.as_ref().unwrap();
        assert_eq!(next.position, [16,71,0]);
        assert_eq!(next.worker_id, Some(worker));
        assert!(!next.issued && !next.pickup_witnessed);
        assert!(world.confirm_gather(id).is_err());
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        world.witness_gather_pickup(id, worker).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 2);
        world.validate().unwrap();
    }

    #[test]
    fn legacy_missing_witness_does_not_imply_picked_up_from_air() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let mut value = serde_json::to_value(&world).unwrap();
        value.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap()
            .get_mut("gather_job").unwrap().as_object_mut().unwrap()
            .remove("pickup_witnessed");
        let mut previous: World = serde_json::from_value(value).unwrap();
        previous.validate().unwrap();
        assert!(!previous.settlements[&id].gather_job.as_ref().unwrap().pickup_witnessed);
        assert!(previous.confirm_gather(id).is_err());
        previous.witness_gather_pickup(id, miner).unwrap();
        previous.confirm_gather(id).unwrap();
        previous.validate().unwrap();
    }

    #[test]
    fn corrupted_unissued_witness_is_rejected_by_domain_validation() {
        let (mut world, id, _) = assigned_stone();
        world.settlements.get_mut(&id).unwrap()
            .gather_job.as_mut().unwrap().pickup_witnessed = true;
        assert!(world.validate().is_err());
    }

    #[test]
    fn blocking_uncertain_issued_pickup_must_not_mint_any_resource() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, miner).unwrap();
        world.block_gather(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        world.abandon_blocked_issued_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod native_edit_ack_and_fair_gather_tests {
    use super::*;

    fn assigned_stone() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16,70,0], [17,70,0]).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        (world, id, worker)
    }

    #[test]
    fn rotating_active_villages_never_sticks_on_first_and_handles_empty() {
        assert_eq!(gather_turn_index(0, 0), None);
        assert_eq!(gather_turn_index(u64::MAX, 0), None);
        assert_eq!(gather_turn_index(0, 1), Some(0));
        assert_eq!(gather_turn_index(8, 1), Some(0));
        for turn in 0..12_u64 {
            assert_eq!(gather_turn_index(turn * 8, 3), Some((turn % 3) as usize));
        }
        assert_eq!(gather_turn_index(8, 3), Some(1));
        assert_eq!(gather_turn_index(16, 3), Some(2));
        assert_eq!(gather_turn_index(24, 3), Some(0));
    }

    #[test]
    fn unacknowledged_issued_edit_cannot_be_witnessed_or_credited() {
        let (mut world, id, worker) = assigned_stone();
        assert!(world.mark_gather_native_edit_confirmed(id).is_err());
        world.mark_gather_issued(id).unwrap();
        assert!(world.witness_gather_pickup(id, worker).is_err());
        assert!(world.confirm_gather(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.validate().unwrap();
    }

    #[test]
    fn native_ack_is_exactly_once_and_required_before_source_witness() {
        let (mut world, id, worker) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.mark_gather_native_edit_confirmed(id).is_err());
        world.witness_gather_pickup(id, worker).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn old_issued_unconfirmed_receipt_remains_unpayable_after_restart() {
        let (mut world, id, worker) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        let serialized = serde_json::to_string(&world).unwrap();
        let mut restarted: World = serde_json::from_str(&serialized).unwrap();
        restarted.validate().unwrap();
        assert!(!restarted.settlements[&id].gather_job.as_ref()
            .unwrap().native_edit_confirmed);
        assert!(restarted.witness_gather_pickup(id, worker).is_err());
        assert!(restarted.confirm_gather(id).is_err());
        restarted.block_gather(id).unwrap();
        restarted.abandon_blocked_issued_gather(id).unwrap();
        assert_eq!(restarted.settlements[&id].warehouse_quantity(Good::Stone), 0);
        restarted.validate().unwrap();
    }

    #[test]
    fn corrupted_unissued_native_ack_and_unacked_pickup_are_rejected() {
        let (mut world, id, worker) = assigned_stone();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().native_edit_confirmed = true;
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().native_edit_confirmed = false;
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().pickup_witnessed = true;
        assert!(world.validate().is_err());
        assert!(world.witness_gather_pickup(id, worker).is_err());
    }

    #[test]
    fn ack_state_persists_and_next_oak_log_resets_ack_and_pickup() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_short_oak_stem(id, [16,70,0], [17,70,0], 3).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let mut restored: World = serde_json::from_str(
            &serde_json::to_string(&world).unwrap()).unwrap();
        assert!(restored.settlements[&id].gather_job.as_ref()
            .unwrap().native_edit_confirmed);
        restored.witness_gather_pickup(id, worker).unwrap();
        restored.confirm_gather(id).unwrap();
        let next = restored.settlements[&id].gather_job.as_ref().unwrap();
        assert_eq!(next.worker_id, Some(worker));
        assert!(!next.issued);
        assert!(!next.native_edit_confirmed);
        assert!(!next.pickup_witnessed);
        restored.validate().unwrap();
    }

    #[test]
    fn native_ack_without_assigned_worker_is_invalid_on_old_save() {
        let (mut world, id, _) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().worker_id = None;
        assert!(world.mark_gather_native_edit_confirmed(id).is_err());
        world.validate().unwrap();
    }
}

#[cfg(test)]
mod legacy_witnessed_gather_migration_tests {
    use super::*;

    #[test]
    fn previously_witnessed_issued_receipt_loads_but_cannot_be_paid() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16, 70, 0], [17, 70, 0]).unwrap();
        world.assign_gather_worker(id, miner).unwrap();
        world.mark_gather_issued(id).unwrap();
        // PR #30 legitimately stored this flag but had no post-command
        // native acknowledgement. It must not brick the entire save.
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap()
            .pickup_witnessed = true;
        let mut value = serde_json::to_value(&world).unwrap();
        value.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap()
            .get_mut("gather_job").unwrap().as_object_mut().unwrap()
            .remove("native_edit_confirmed");
        let mut migrated: World = serde_json::from_value(value).unwrap();
        migrated.validate().unwrap();
        let job = migrated.settlements[&id].gather_job.as_ref().unwrap();
        assert!(job.issued);
        assert!(job.pickup_witnessed);
        assert!(!job.native_edit_confirmed);
        assert!(migrated.confirm_gather(id).is_err());
        assert_eq!(migrated.settlements[&id].warehouse_quantity(Good::Stone), 0);
        // The bridge must block this uncertain issued job after reload.
        migrated.block_gather(id).unwrap();
        migrated.abandon_blocked_issued_gather(id).unwrap();
        assert_eq!(migrated.settlements[&id].warehouse_quantity(Good::Stone), 0);
        migrated.validate().unwrap();
    }
}
