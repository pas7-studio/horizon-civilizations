// Horizon Civilizations domain module, re-exported by core/mod.rs.

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

/// How far from its camp a worker may search for natural resources.
pub const POST_RADIUS: i32 = 24;

pub const MAX_POSTS: usize = 4;

/// A quarry pit is a staircase of at most this many steps down from its camp.
pub const MAX_QUARRY_STEPS: u8 = 24;

/// A village keeps its own work area ticking even with nobody standing in
/// it. Chunks are the smallest horizontal unit the engine can load, so the
/// goal is to hold FEW chunks, not smaller ones: only the chunk of the civic
/// centre, the chunk of every real workplace, and the corridor a villager
/// must cross to reach it. Everything else stays unloaded, and the engine
/// still gives each held chunk's immediate neighbours a lighter load level.
pub const HALL_CHUNK_RADIUS: i32 = 1;

pub const MAX_HELD_VILLAGE_CHUNKS: usize = 48;

/// Experimental physical home assignment: only completed visual 5x5 huts.
/// Three indoor standing spots per hut, not beds or a legal claim of land.
pub const PHYSICAL_HOME_CAPACITY: usize = 3;

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

/// A real sign line holds about 15 characters of the vanilla font.
pub const SIGN_WIDTH: usize = 15;

/// Most a single sale may take of one good, as the existing market enforces.
pub const MAX_HAND_SALE: u32 = 64;
