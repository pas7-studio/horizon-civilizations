// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;

/// Goods this resident will buy from a player right now, each with the most
/// they will take. Only the mayor and the baker trade at all, and only for
/// what their own role needs: the mayor buys the village's real construction
/// shortfall (so a player can feed development); the baker buys wheat once the
/// bakery stands. Everyone else holds no trade.
pub fn resident_wants(
    village: &Settlement, job: Job, wood_missing: u32, stone_missing: u32,
) -> Vec<(Good, u32)> {
    let mut wants = Vec::new();
    match job {
        Job::Merchant => {
            if wood_missing > 0 { wants.push((Good::Wood, wood_missing.min(MAX_HAND_SALE))); }
            if stone_missing > 0 { wants.push((Good::Stone, stone_missing.min(MAX_HAND_SALE))); }
        }
        Job::Baker => {
            if village.bakery.as_ref().is_some_and(|b| b.complete) {
                wants.push((Good::Wheat, 12));
            }
        }
        _ => {}
    }
    wants
}

/// True for the professions that keep a stall at all.
pub fn job_trades(job: Job) -> bool {
    matches!(job, Job::Merchant | Job::Baker)
}

/// Pages of a villager's conversation window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkPage {
    Main,
    Village,
    Trade,
}

impl TalkPage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Village => "village",
            Self::Trade => "trade",
        }
    }
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "main" => Some(Self::Main),
            "village" => Some(Self::Village),
            "trade" => Some(Self::Trade),
            _ => None,
        }
    }
}

/// A slider the player drags to choose how much to sell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DlgSlider {
    pub key: &'static str,
    pub label: String,
    pub min: u32,
    pub max: u32,
    pub initial: u32,
}

/// A button. `command` runs as the player; with `template` it may contain
/// `$(key)` placeholders that the dialog fills from the sliders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DlgButton {
    pub label: String,
    pub tooltip: Option<String>,
    pub color: &'static str,
    pub command: String,
    pub template: bool,
}

/// An RPG-style conversation window: a portrait, paragraphs of speech,
/// optional sliders and a column of choices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialog {
    pub title: String,
    pub portrait: &'static str,
    pub portrait_caption: String,
    pub paragraphs: Vec<(String, &'static str)>,
    pub sliders: Vec<DlgSlider>,
    pub buttons: Vec<DlgButton>,
}

fn job_icon(job: Job) -> &'static str {
    match job {
        Job::Merchant => "minecraft:emerald",
        Job::Baker => "minecraft:bread",
        Job::Farmer => "minecraft:wheat",
        Job::Lumberjack => "minecraft:iron_axe",
        Job::Miner => "minecraft:iron_pickaxe",
        Job::Builder => "minecraft:bricks",
        Job::Guard => "minecraft:shield",
        _ => "minecraft:wooden_shovel",
    }
}

fn dialog_text(text: impl AsRef<str>) -> String {
    text.as_ref().chars().filter(|c| !c.is_control()).take(240).collect()
}

/// Everything a conversation page shows. Pure data: the native adapter reads
/// it from the live village and the player's pockets.
pub struct DialogueInput<'a> {
    pub village: &'a Settlement,
    pub resident: &'a Resident,
    pub is_member: bool,
    pub routine: String,
    pub on_duty: bool,
    pub wants: &'a [(Good, u32)],
    /// False when the hall barrels could not be read (chunk not loaded).
    pub stock_known: bool,
    pub wood_missing: u32,
    pub stone_missing: u32,
    /// How many of each good the player carries right now.
    pub carried: &'a [(Good, u32)],
    /// A line shown first, such as the result of the last action.
    pub notice: Option<String>,
}

fn greeting(input: &DialogueInput<'_>) -> String {
    let village = &input.village.name;
    let name = &input.resident.name;
    match input.resident.job {
        Job::Merchant => format!("Welcome to {village}, traveller. I keep the town hall's books and its warehouse."),
        Job::Baker => format!("The ovens of {village} are warm. I am {name}, the baker."),
        Job::Farmer => format!("The fields of {village} keep me busy. I am {name}."),
        Job::Lumberjack => format!("Mind the chips, friend. I am {name}, I fell trees for {village}."),
        Job::Miner => format!("Stone dust in my beard again. I am {name}, I quarry for {village}."),
        Job::Builder => format!("Another wall to raise! I am {name}, builder of {village}."),
        Job::Guard => format!("I keep watch over {village}. Move along, or state your business."),
        _ => format!("Good day. I am {name}, I help wherever {village} needs hands."),
    }
}

fn command_for(input: &DialogueInput<'_>, page: &str) -> String {
    format!("civ talk {} {} {}", input.village.name, input.resident.id, page)
}

fn back_button(input: &DialogueInput<'_>) -> DlgButton {
    DlgButton {
        label: "Back".into(), tooltip: None, color: "yellow",
        command: command_for(input, "main"), template: false,
    }
}

/// Build one conversation page.
pub fn build_dialog(page: TalkPage, input: &DialogueInput<'_>) -> Dialog {
    let village = input.village;
    let resident = input.resident;
    let role = if resident.job == Job::Merchant {
        "mayor and warehouse steward".to_owned()
    } else {
        resident.job.name().to_owned()
    };
    let mut paragraphs: Vec<(String, &'static str)> = Vec::new();
    if let Some(notice) = &input.notice {
        paragraphs.push((dialog_text(notice), "gold"));
    }
    let mut sliders = Vec::new();
    let mut buttons = Vec::new();
    match page {
        TalkPage::Main => {
            paragraphs.push((dialog_text(greeting(input)), "white"));
            paragraphs.push((dialog_text(format!(
                "{} - {}", input.routine, if input.on_duty { "on duty" } else { "off duty" }
            )), "gray"));
            let home = match village.home_slot(resident.id) {
                Some(slot) => format!("Lives in a village hut, place {}.", slot.lane + 1),
                None => "Sleeps in the commons.".to_owned(),
            };
            paragraphs.push((home, "gray"));
            buttons.push(DlgButton {
                label: format!("Ask about {}", village.name),
                tooltip: Some("Population, homes, treasury, needs and what the council worries about.".into()),
                color: "aqua", command: command_for(input, "village"), template: false,
            });
            if job_trades(resident.job) {
                let tooltip = if input.wants.is_empty() {
                    "Nothing is needed right now.".to_owned()
                } else {
                    let list: Vec<String> = input.wants.iter()
                        .map(|(g, n)| format!("{} up to {} at {} coins each", good_label(*g), n, g.price()))
                        .collect();
                    format!("I buy: {}.", list.join("; "))
                };
                buttons.push(DlgButton {
                    label: "Trade".into(), tooltip: Some(tooltip),
                    color: if input.wants.is_empty() { "gray" } else { "green" },
                    command: command_for(input, "trade"), template: false,
                });
            }
            if !input.is_member {
                buttons.push(DlgButton {
                    label: format!("Join {}", village.name),
                    tooltip: Some("Become a member: you can then trade with the village and earn standing.".into()),
                    color: "green", command: command_for(input, "join"), template: false,
                });
            }
        }
        TalkPage::Village => {
            let (capacity, _, homeless) = village.physical_housing();
            paragraphs.push((format!("{} - {} people", village.name, village.population()), "gold"));
            paragraphs.push((format!("Homes: {capacity}   homeless: {homeless}"), "gray"));
            paragraphs.push((format!("Treasury: {} coins   mood: {}", village.treasury_coins, village.morale), "gray"));
            paragraphs.push((
                if village.hall_complete { "The town hall stands.".to_owned() }
                else { "The town hall is not built yet.".to_owned() },
                "gray",
            ));
            paragraphs.push((
                if !input.stock_known {
                    "The hall barrels are out of sight.".to_owned()
                } else if input.wood_missing == 0 && input.stone_missing == 0 {
                    "Nothing is missing for building.".to_owned()
                } else {
                    format!("Missing for building: {} oak logs, {} cobblestone.",
                        input.wood_missing, input.stone_missing)
                },
                "gray",
            ));
            let council = village.council_priorities().first()
                .map(|p| format!("The council worries about {} ({}/100).", p.need.label(), p.urgency))
                .unwrap_or_else(|| "The council is calm.".to_owned());
            paragraphs.push((council, "gray"));
            paragraphs.push((
                if village.active_wars.is_empty() { "The village is at peace.".to_owned() }
                else { format!("The village is at war with {} settlement(s).", village.active_wars.len()) },
                if village.active_wars.is_empty() { "green" } else { "red" },
            ));
            buttons.push(back_button(input));
        }
        TalkPage::Trade => {
            if !job_trades(resident.job) {
                paragraphs.push(("I do not trade. The mayor buys what the village needs.".into(), "gray"));
            } else if !input.stock_known {
                paragraphs.push(("My stores are out of sight. Come back near the hall.".into(), "gray"));
            } else if input.wants.is_empty() {
                paragraphs.push(("We need nothing from you right now.".into(), "gray"));
            } else if !input.is_member {
                paragraphs.push(("Join the village first, then we can trade.".into(), "red"));
            } else {
                paragraphs.push(("Choose how much to sell. Every coin and every item is counted in the village books.".into(), "white"));
                for (good, cap) in input.wants {
                    let have = input.carried.iter().find(|(g, _)| g == good).map_or(0, |(_, n)| *n);
                    let max = have.min(*cap);
                    if max == 0 {
                        paragraphs.push((format!(
                            "{}: you carry none (the village needs up to {}).", good_label(*good), cap
                        ), "gray"));
                    } else {
                        sliders.push(DlgSlider {
                            key: good.as_str(),
                            label: format!("{} ({} coins each, you carry {})", good_label(*good), good.price(), have),
                            min: 0, max, initial: max,
                        });
                    }
                }
                if !sliders.is_empty() {
                    let args: Vec<String> = sliders.iter().map(|s| format!("{0}.$({0})", s.key)).collect();
                    buttons.push(DlgButton {
                        label: "Sell".into(),
                        tooltip: Some("Hand over the chosen amounts and receive coins.".into()),
                        color: "green",
                        command: format!("{} {}", command_for(input, "sell"), args.join(" ")),
                        template: true,
                    });
                }
            }
            buttons.push(back_button(input));
        }
    }
    let title = match page {
        TalkPage::Main => resident.name.clone(),
        TalkPage::Village => format!("{} - village", village.name),
        TalkPage::Trade => format!("{} - trade", resident.name),
    };
    Dialog {
        title: dialog_text(title),
        portrait: job_icon(resident.job),
        portrait_caption: dialog_text(format!("{} - {}", resident.name, role)),
        paragraphs,
        sliders,
        buttons,
    }
}

/// Turn the `good.amount` words a Sell button sends into amounts, never more
/// than the resident wants or the player carries. Unknown goods, goods the
/// resident does not buy and zero amounts are dropped; nothing left is an
/// error. Amounts may arrive as `8` or `8.0`.
pub fn parse_sale_args(
    wants: &[(Good, u32)], carried: &[(Good, u32)], words: &[&str],
) -> Result<Vec<(Good, u32)>, String> {
    let mut sale: Vec<(Good, u32)> = Vec::new();
    for word in words {
        let (key, value) = word.split_once('.').ok_or("malformed sale request")?;
        let good = Good::parse(key).ok_or("unknown good in the sale request")?;
        let Some((_, cap)) = wants.iter().find(|(g, _)| *g == good) else { continue };
        let amount: f64 = value.parse().map_err(|_| "malformed amount in the sale request")?;
        if !amount.is_finite() || amount < 1.0 { continue; }
        let have = carried.iter().find(|(g, _)| *g == good).map_or(0, |(_, n)| *n);
        let take = (amount.round().min(f64::from(u32::MAX)) as u32)
            .min(*cap).min(have).min(MAX_HAND_SALE);
        if take == 0 || sale.iter().any(|(g, _)| *g == good) { continue; }
        sale.push((good, take));
    }
    if sale.is_empty() {
        Err("nothing to sell: choose an amount above zero of something I buy".into())
    } else {
        Ok(sale)
    }
}

pub fn snbt_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn snbt_text(text: &str, color: &str, bold: bool) -> String {
    format!(
        "{{text:{},color:{}{}}}",
        snbt_string(text), snbt_string(color), if bold { ",bold:1b" } else { "" }
    )
}

/// The dialog as the inline SNBT that `/dialog show <player> <snbt>` takes.
/// Escape closes it, it never pauses the game, and every button runs a `civ`
/// command as the player.
pub fn dialog_snbt(dialog: &Dialog) -> String {
    let mut body = vec![format!(
        "{{type:\"minecraft:item\",item:{{id:{},count:1}},description:{{contents:{},width:240}},show_decorations:0b,show_tooltip:0b,width:16,height:16}}",
        snbt_string(dialog.portrait), snbt_text(&dialog.portrait_caption, "gold", true)
    )];
    for (text, color) in &dialog.paragraphs {
        body.push(format!(
            "{{type:\"minecraft:plain_message\",contents:{},width:300}}",
            snbt_text(text, color, false)
        ));
    }
    let inputs: Vec<String> = dialog.sliders.iter().map(|s| format!(
        "{{type:\"minecraft:number_range\",key:{},label:{},width:300,start:{},end:{},initial:{},step:1}}",
        snbt_string(s.key), snbt_text(&s.label, "white", false), s.min, s.max, s.initial
    )).collect();
    let actions: Vec<String> = dialog.buttons.iter().map(|b| {
        let action = if b.template {
            format!("{{type:\"minecraft:dynamic/run_command\",template:{}}}", snbt_string(&b.command))
        } else {
            format!("{{type:\"minecraft:run_command\",command:{}}}", snbt_string(&b.command))
        };
        let tooltip = b.tooltip.as_ref()
            .map(|t| format!(",tooltip:{}", snbt_text(t, "gray", false))).unwrap_or_default();
        format!(
            "{{label:{}{tooltip},width:300,action:{action}}}",
            snbt_text(&b.label, b.color, false)
        )
    }).collect();
    format!(
        "{{type:\"minecraft:multi_action\",title:{},can_close_with_escape:1b,pause:0b,after_action:\"close\",body:[{}],inputs:[{}],actions:[{}],columns:1,exit_action:{{label:{},width:300}}}}",
        snbt_text(&dialog.title, "gold", true),
        body.join(","), inputs.join(","), actions.join(","),
        snbt_text("Farewell", "gray", false)
    )
}
