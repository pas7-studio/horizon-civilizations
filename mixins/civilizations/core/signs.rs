// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;

/// Bulletin signs are real blocks inside the civic hall; the native adapter
/// supplies validated, bounded sign text after the town hall is complete.
pub fn hall_board_positions(origin: [i32; 3]) -> [[i32; 3]; 2] {
    [[origin[0] - 1, origin[1] + 1, origin[2] - 3],
     [origin[0] + 1, origin[1] + 1, origin[2] - 3]]
}

/// Where the pre-0.1.16 hall boards stood (standing signs inside the hall).
/// Kept only so a completed hall can be migrated to wall signs in place.
pub fn legacy_hall_board_positions(origin: [i32; 3]) -> [[i32; 3]; 2] {
    [[origin[0] - 1, origin[1], origin[2] - 1],
     [origin[0] + 1, origin[1], origin[2] - 1]]
}

/// All six civic-hall boards, in `render_hall_boards` order. The first two
/// are part of the construction plan; the other four are added to already
/// completed halls by the native adapter only where the wall behind exists.
/// Row y+1: land, mayor, (door), needs, diplomacy. Row y+2: treasury, council.
pub fn hall_all_board_positions(origin: [i32; 3]) -> [[i32; 3]; 6] {
    let [x, y, z] = origin;
    [[x - 1, y + 1, z - 3], [x + 1, y + 1, z - 3],
     [x - 2, y + 1, z - 3], [x + 2, y + 1, z - 3],
     [x - 1, y + 2, z - 3], [x + 1, y + 2, z - 3]]
}

/// Semantic sign colors, chosen to stay readable on oak planks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignColor {
    Black,
    Heading,
    Good,
    Warn,
    Alert,
    Muted,
}

impl SignColor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Heading => "dark_blue",
            Self::Good => "dark_green",
            Self::Warn => "#9A5B00",
            Self::Alert => "dark_red",
            Self::Muted => "dark_gray",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignLine {
    pub text: String,
    pub color: SignColor,
    pub bold: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignSpec {
    pub lines: [SignLine; 4],
    /// Glowing ink is reserved for alerts that need attention.
    pub glow: bool,
}

impl SignSpec {
    /// The same sign as four plain strings (no colors), for versions whose
    /// sign NBT rejects rich text and for tests.
    pub fn plain(&self) -> [&str; 4] {
        [&self.lines[0].text, &self.lines[1].text, &self.lines[2].text, &self.lines[3].text]
    }
}

/// Bounded single-line text: control characters removed, width limited.
pub fn sign_fit(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(SIGN_WIDTH).collect()
}

fn sl(text: impl AsRef<str>, color: SignColor) -> SignLine {
    SignLine { text: sign_fit(text.as_ref()), color, bold: false }
}

fn sign_head(text: impl AsRef<str>) -> SignLine {
    SignLine { text: sign_fit(text.as_ref()), color: SignColor::Heading, bold: true }
}

fn sign_blank() -> SignLine {
    sl("", SignColor::Black)
}

/// ASCII progress bar such as `[#####-----]`, safe in every font.
pub fn sign_bar(done: u32, total: u32, width: usize) -> String {
    let filled = if total == 0 {
        0
    } else {
        ((u64::from(done.min(total)) * width as u64) / u64::from(total)) as usize
    };
    format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled))
}

/// Verified inputs for the civic-hall boards. `stored` is the live snapshot
/// of the two native hall barrels, never the abstract daily stock.
pub struct HallBoardInput<'a> {
    pub village: &'a Settlement,
    pub stored: &'a [(Good, u32)],
    pub wood_missing: u32,
    pub stone_missing: u32,
    pub journal_unsettled: bool,
}

/// Render the six hall boards. Pure and read-only: no ledger, block or save
/// state is touched, so it can be called every refresh and unit tested.
pub fn render_hall_boards(input: &HallBoardInput<'_>) -> [SignSpec; 6] {
    let v = input.village;
    let have = |good: Good| input.stored.iter()
        .find(|(g, _)| *g == good).map_or(0, |(_, n)| *n);
    let mayor = v.residents.iter().find(|r| r.alive && r.job == Job::Merchant)
        .map_or("none", |r| r.name.as_str());
    let (capacity, _, homeless) = v.physical_housing();
    let shortage = input.wood_missing > 0 || input.stone_missing > 0;

    let identity = SignSpec {
        lines: [
            sign_head(&v.name),
            sl("Town Hall", SignColor::Black),
            sl(format!("Mayor {mayor}"), SignColor::Black),
            sl(format!("Pop {} Mood {}", v.population(), v.morale), SignColor::Muted),
        ],
        glow: false,
    };

    let needs = if input.journal_unsettled {
        SignSpec {
            lines: [
                sign_head("WAREHOUSE"),
                sl("Trade pending", SignColor::Warn),
                sl("Check barrels", SignColor::Warn),
                sl(format!("Homeless {homeless}"), SignColor::Muted),
            ],
            glow: true,
        }
    } else {
        let resource_line = |label: &str, good: Good, missing: u32| {
            if missing > 0 {
                sl(format!("{label} {} -{missing}", have(good)), SignColor::Alert)
            } else {
                sl(format!("{label} {} OK", have(good)), SignColor::Good)
            }
        };
        SignSpec {
            lines: [
                sign_head(if shortage { "VILLAGE NEEDS" } else { "STOCK OK" }),
                resource_line("Wood", Good::Wood, input.wood_missing),
                resource_line("Stone", Good::Stone, input.stone_missing),
                if homeless > 0 {
                    sl(format!("Homeless {homeless}"), SignColor::Warn)
                } else {
                    sl(format!("Homes {capacity} OK"), SignColor::Good)
                },
            ],
            glow: shortage,
        }
    };

    let forecast = v.forecast_abstract_days(COUNCIL_FORECAST_DAYS);
    let hunger = forecast.iter().find(|d| d.starving_days > 0).map(|d| d.offset);
    let food_line = if v.starving_days > 0 {
        sl("STARVING NOW", SignColor::Alert)
    } else if let Some(day) = hunger {
        sl(format!("Hunger in {day}d"), SignColor::Warn)
    } else {
        sl("Food OK", SignColor::Good)
    };
    let treasury = SignSpec {
        lines: [
            sign_head("TREASURY"),
            sl(format!("Coins {}", v.treasury_coins), SignColor::Black),
            sl(format!("Bread {} Wht {}", have(Good::Bread), have(Good::Wheat)), SignColor::Black),
            food_line,
        ],
        glow: v.starving_days > 0,
    };

    let council = match v.council_priorities().into_iter().next() {
        Some(top) => {
            let color = if top.urgency >= 90 { SignColor::Alert }
                else if top.urgency >= 50 { SignColor::Warn } else { SignColor::Good };
            SignSpec {
                lines: [
                    sign_head("COUNCIL SAYS"),
                    sl(top.need.label().to_uppercase(), color),
                    sl(format!("Urgency {}", top.urgency), color),
                    sl("see /civ council", SignColor::Muted),
                ],
                glow: top.urgency >= 90,
            }
        }
        None => SignSpec {
            lines: [
                sign_head("COUNCIL SAYS"),
                sl("All calm", SignColor::Good),
                sl("No urgent need", SignColor::Muted),
                sign_blank(),
            ],
            glow: false,
        },
    };

    let land = SignSpec {
        lines: [
            sign_head("LAND OFFICE"),
            sl(format!("Plot {} coins", LAND_PRICE_COINS), SignColor::Black),
            sl(format!("Sold {}/{}", v.player_parcels.len(), MAX_PLAYER_PARCELS), SignColor::Muted),
            sl("/civ buyplot", SignColor::Muted),
        ],
        glow: false,
    };

    let wars = v.active_wars.len();
    let friends = v.relationships.values().filter(|r| **r > 0).count();
    let rivals = v.relationships.values().filter(|r| **r < 0).count();
    let diplomacy = SignSpec {
        lines: [
            sign_head("DIPLOMACY"),
            if wars > 0 {
                sl(format!("AT WAR x{wars}"), SignColor::Alert)
            } else {
                sl("PEACE", SignColor::Good)
            },
            sl(format!("Friends {friends}"), SignColor::Black),
            sl(format!("Rivals {rivals}"), SignColor::Black),
        ],
        glow: wars > 0,
    };

    [identity, needs, land, diplomacy, treasury, council]
}

/// Construction progress board text: `done` of `total` plan blocks.
pub fn render_build_progress(title: &str, done: usize, total: usize, blocked: bool) -> SignSpec {
    let total32 = total.min(u32::MAX as usize) as u32;
    let done32 = done.min(u32::MAX as usize) as u32;
    SignSpec {
        lines: [
            sign_head(format!("BUILDING {title}")),
            sl(sign_bar(done32, total32, 10), if blocked { SignColor::Alert } else { SignColor::Good }),
            sl(format!("Step {done}/{total}"), SignColor::Black),
            if blocked { sl("BLOCKED", SignColor::Alert) } else { sign_blank() },
        ],
        glow: blocked,
    }
}

/// Standing site board: on natural ground just outside the front-right
/// corner of a 5x5 building, clear of the wall, door and foundation.
pub fn site_board_position(origin: [i32; 3]) -> [i32; 3] {
    [origin[0] + 2, origin[1], origin[2] - 3]
}

/// Board for a finished public hut: how many it houses and how many are free.
pub fn render_hut_ready(village: &Settlement) -> SignSpec {
    let (capacity, assigned, waiting) = village.physical_housing();
    SignSpec {
        lines: [
            sign_head("VILLAGE HUT"),
            sl("Finished", SignColor::Good),
            sl(format!("Homes {capacity}"), SignColor::Black),
            if waiting > 0 {
                sl(format!("Homeless {waiting}"), SignColor::Warn)
            } else {
                sl(format!("Housed {assigned}"), SignColor::Good)
            },
        ],
        glow: false,
    }
}

/// Board for the bakery workshop once it is built.
pub fn render_bakery_ready(bakery: &BakeryWorkshop) -> SignSpec {
    SignSpec {
        lines: [
            sign_head("BAKERY"),
            sl(format!("Batches {}", bakery.batches), SignColor::Black),
            sl(format!("Wheat in {}", bakery.wheat_escrow), SignColor::Black),
            sl(format!("Bread out {}", bakery.bread_in_transit), SignColor::Black),
        ],
        glow: false,
    }
}
