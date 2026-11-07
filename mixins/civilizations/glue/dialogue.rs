// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// Dialogs waiting to be shown: (player name, inline dialog SNBT). The server
/// command that shows a dialog only runs from the server tick, never inside a
/// player's command or hook, so everything is queued and flushed there.
pub(super) static DIALOG_QUEUE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

pub(super) fn dialog_queue() -> MutexGuard<'static, Vec<(String, String)>> {
    DIALOG_QUEUE.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Called every server tick: show queued conversation windows.
pub fn process_dialog_queue(game: &mut Game<'_>) {
    let queued: Vec<(String, String)> = std::mem::take(&mut *dialog_queue());
    for (name, snbt) in queued {
        // Player names are 1..16 of letters, digits and underscore; refuse
        // anything else rather than build a command from it.
        let safe = !name.is_empty()
            && name.len() <= 16
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !safe {
            log::warn!("[civilizations] conversation not shown: unusual player name");
            continue;
        }
        if !game.native_run_command(&format!("dialog show {name} {snbt}")) {
            log::warn!("[civilizations] conversation window could not be shown to {name}");
        }
    }
}

/// What one conversation page needs, read from the live village and the
/// player's pockets. Only valid where the player's state is swapped in (a
/// command or an interaction hook), which is where it is called.
pub(super) struct Talk {
    pub(super) village: Settlement,
    pub(super) resident: Resident,
    pub(super) is_member: bool,
    pub(super) wants: Vec<(Good, u32)>,
    pub(super) stock_known: bool,
    pub(super) wood_missing: u32,
    pub(super) stone_missing: u32,
    pub(super) carried: Vec<(Good, u32)>,
}

pub(super) fn read_talk(game: &mut Game<'_>, uuid: u128, village_id: u64, resident_id: u64) -> Option<Talk> {
    let dir = game.native_world_dir()?;
    let (village, resident, is_member) = {
        let guard = active(dir.join(DIR));
        let world = guard.as_ref()
            .filter(|state| state.problem.is_none())
            .and_then(|state| state.world.as_ref())?;
        let village = world.settlements.get(&village_id)?.clone();
        let resident = village.residents.iter()
            .find(|r| r.id == resident_id && r.alive)?.clone();
        let member = world.players.get(&format!("{uuid:032x}"))
            .and_then(|p| p.settlement) == Some(village_id);
        (village, resident, member)
    };
    let stored = physical_barrel_stock(game, &village.dimension, village.center);
    let (wood_missing, stone_missing) = stored.as_ref().map_or((0, 0), |stock| {
        crate::game::civilizations_core::physical_construction_shortfall(&village, stock)
    });
    let wants = resident_wants(&village, resident.job, wood_missing, stone_missing);
    let carried = [Good::Wood, Good::Stone, Good::Wheat].iter()
        .map(|good| (*good, game.native_count_items(good.item())))
        .collect();
    Some(Talk {
        village, resident, is_member, wants, stock_known: stored.is_some(),
        wood_missing, stone_missing, carried,
    })
}

/// Build one conversation page and queue it for the server tick.
pub(super) fn show_talk(
    game: &mut Game<'_>, player: &NativeServerPlayer, village_id: u64, resident_id: u64,
    page: TalkPage, notice: Option<String>,
) -> bool {
    let Some(talk) = read_talk(game, player.uuid, village_id, resident_id) else {
        return false;
    };
    let phase = DailyPhase::at(game.world.day_time);
    let input = DialogueInput {
        village: &talk.village,
        resident: &talk.resident,
        is_member: talk.is_member,
        routine: phase.name().to_owned(),
        on_duty: phase.on_shift(talk.resident.job),
        wants: &talk.wants,
        stock_known: talk.stock_known,
        wood_missing: talk.wood_missing,
        stone_missing: talk.stone_missing,
        carried: &talk.carried,
        notice,
    };
    let snbt = dialog_snbt(&build_dialog(page, &input));
    dialog_queue().push((player.name.clone(), snbt));
    true
}

/// Sell what the sliders chose. The coins, the warehouse credit and the save
/// are committed together with taking the items from the player; if any step
/// fails the items are given back and the world is restored.
pub(super) fn sell_to_resident(
    game: &mut Game<'_>, player: &NativeServerPlayer, village_id: u64, resident_id: u64,
    words: &[&str],
) -> Result<String, String> {
    let talk = read_talk(game, player.uuid, village_id, resident_id)
        .ok_or("this villager is not available")?;
    if !talk.is_member {
        return Err("join the village first".into());
    }
    let sale = parse_sale_args(&talk.wants, &talk.carried, words)?;
    let dir = game.native_world_dir().ok_or("world folder unavailable")?;
    let mut guard = active(dir.join(DIR));
    let state = guard.as_mut()
        .filter(|state| state.problem.is_none() && state.world.is_some())
        .ok_or("civilizations is paused for this world")?;
    let previous = state.world.as_ref().unwrap().clone();
    let actor = format!("{:032x}", player.uuid);
    let mut total = 0_u64;
    let mut parts = Vec::new();
    for (good, count) in &sale {
        match state.world.as_mut().unwrap().sell(&actor, village_id, *good, *count) {
            Ok(payout) => {
                total += payout;
                parts.push(format!("{count} {}", good_label(*good)));
            }
            Err(error) => {
                state.world = Some(previous);
                return Err(error);
            }
        }
    }
    let mut taken: Vec<(Good, u32)> = Vec::new();
    for (good, count) in &sale {
        let got = game.native_take_items(good.item(), *count);
        taken.push((*good, got));
        if got != *count {
            for (given, n) in &taken {
                if *n > 0 { game.native_give_items(given.item(), *n); }
            }
            state.world = Some(previous);
            return Err("your inventory changed during the sale".into());
        }
    }
    if let Err(error) = save(state) {
        for (given, n) in &taken {
            if *n > 0 { game.native_give_items(given.item(), *n); }
        }
        log::error!("[civilizations] conversation sale not saved, items returned: {error}");
        state.world = Some(previous);
        return Err(format!("sale not committed: {error}"));
    }
    Ok(format!("Sold {} for {total} coins.", parts.join(", ")))
}

/// `/civ talk <village> <resident> [page] [good.amount ...]`: every button of a
/// conversation window runs this. It never trusts the window: the village,
/// the resident, the page and the amounts are all checked again here.
pub(super) fn talk_command(game: &mut Game<'_>, player: &NativeServerPlayer, mut args: std::str::SplitWhitespace<'_>) {
    let (Some(village_name), Some(resident_word)) = (args.next(), args.next()) else {
        error(game, "usage: /civ talk <village> <resident> [page]");
        return;
    };
    let Ok(resident_id) = resident_word.parse::<u64>() else {
        error(game, "invalid resident");
        return;
    };
    let page_word = args.next().unwrap_or("main");
    let rest: Vec<&str> = args.collect();
    let Some(dir) = game.native_world_dir() else {
        error(game, "no server world open");
        return;
    };
    let found = {
        let guard = active(dir.join(DIR));
        guard.as_ref()
            .filter(|state| state.problem.is_none())
            .and_then(|state| state.world.as_ref())
            .and_then(|world| world.by_name(village_name))
            .map(|village| (village.id, village.dimension.clone(), village.center))
    };
    let Some((village_id, dimension, center)) = found else {
        error(game, "unknown settlement");
        return;
    };
    let dx = player.position[0] - f64::from(center[0]);
    let dz = player.position[2] - f64::from(center[2]);
    if player.dimension != dimension || !(dx * dx + dz * dz <= 64.0 * 64.0) {
        error(game, "you are too far from this village to talk to its people");
        return;
    }
    let (page, notice) = match page_word {
        "sell" => {
            let outcome = sell_to_resident(game, player, village_id, resident_id, &rest);
            (TalkPage::Trade, Some(outcome.unwrap_or_else(|reason| format!("Nothing sold: {reason}."))))
        }
        "join" => {
            let outcome = join_village(game, player.uuid, village_id);
            (TalkPage::Main, Some(outcome.unwrap_or_else(|reason| format!("Could not join: {reason}."))))
        }
        other => match TalkPage::parse(other) {
            Some(page) => (page, None),
            None => {
                error(game, "unknown conversation page");
                return;
            }
        },
    };
    if !show_talk(game, player, village_id, resident_id, page, notice) {
        error(game, "this villager is not available any more");
    }
}
