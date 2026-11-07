// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

pub fn register(commands: &mut Vec<NativeCommand>) {
    for usage in [
        "civ", "civ help", "civ list", "civ info", "civ info <name:word>",
        "civ roster <name:word>", "civ schedule <name:word>",
        "civ homes <name:word>", "civ survey <name:word>",
        "civ council <name:word>", "civ forecast <name:word>",
        "civ plots <name:word>", "civ development <name:word>",
        "civ hall <name:word>", "civ warehouse <name:word>", "civ bakery <name:word>",
        "civ posts <name:word>",
        "civ roles <name:word>",
        "civ status <name:word>",
        "civ chunks <name:word>",
        "civ diagnose <name:word>",
        "civ construction <name:word>", "civ farming <name:word>",
        "civ harvests <name:word>",
        "civ join <name:word>", "civ title", "civ promote", "civ history",
        "civ talk <name:word> <resident:integer>",
        "civ talk <name:word> <resident:integer> <page:word>",
        "civ talk <name:word> <resident:integer> <page:word> <a:word>",
        "civ talk <name:word> <resident:integer> <page:word> <a:word> <b:word>",
        "civ sell <good:word> <amount:integer>", "civ buy <good:word> <amount:integer>",
    ] {
        commands.push(NativeCommand::new(usage));
    }
    for usage in [
        "civ found <name:word>", "civ actors <name:word>",
        "civ reconcile <name:word>",
        "civ recoveractor <name:word> <resident:integer> <confirm:word>",
        "civ day [days:integer]", "civ construct <name:word>",
        "civ post <name:word> <kind:word>",
        "civ approvehouse <name:word> <slot:integer> <y:integer>",
        "civ autonomy <name:word> <mode:word>",
        "civ barrelcheck <name:word>",
        "civ gathercheck <name:word>",
        "civ discardgather <name:word> <observed:word>",
        "civ resolvebarrel <name:word> <mode:word> <count:integer>",
        "civ farm <name:word>", "civ recoverfarm <name:word>",
        "civ buyplot <name:word>", "civ buyhouse <name:word>", "civ commission <name:word>",
        "civ war <from:word> <to:word>",
        "civ peace <from:word> <to:word>",
    ] {
        commands.push(NativeCommand::op(usage, 2));
    }
}

pub(super) fn reply(game: &mut Game<'_>, message: impl Into<String>) {
    game.native_feedback(Text::literal(message.into()));
}

pub(super) fn error(game: &mut Game<'_>, message: impl Into<String>) {
    game.native_feedback(Text::literal(message.into()).with_color(text::RED));
}

pub(super) fn parse_amount(s: Option<&str>) -> Result<u32, String> {
    let n = s.ok_or("amount required")?.parse::<u32>().map_err(|_| "invalid amount")?;
    if !(1..=64).contains(&n) { return Err("amount must be 1..64".into()); }
    Ok(n)
}

pub(super) fn own_settlement(world: &World, player_id: &str) -> Result<u64, String> {
    world.players.get(player_id).and_then(|p| p.settlement).ok_or("join a settlement first".into())
}

pub fn perform(game: &mut Game<'_>, player: &NativeServerPlayer, command: &str, ran: &mut bool) {
    let mut args = command.split_whitespace();
    if args.next() != Some("civ") { return; }
    *ran = true;
    if !game.native_on(MOD_ID) {
        error(game, "Civilizations is disabled");
        return;
    }
    // Conversation windows: handled before the world lock is taken because
    // the handler reads and saves the world itself.
    if args.clone().next() == Some("talk") {
        args.next();
        talk_command(game, player, args);
        return;
    }
    let Some(world_dir) = game.native_world_dir() else { error(game, "no server world open"); return; };
    let mut guard = active(world_dir.join(DIR));
    let state = guard.as_mut().expect("active initialized");
    if let Some(problem) = &state.problem { error(game, format!("save error: {problem}")); return; }
    let Some(world) = state.world.as_ref() else { error(game, "world is unavailable"); return; };
    let actor = format!("{:032x}", player.uuid);
    let action = args.next().unwrap_or("help");
    if report(game, player, world, action, &actor, &mut args) {
        return;
    }
    let previous = world.clone();
    let (result, already_saved) = mutate(game, player, state, &previous, action, &actor, &mut args);
    match result {
        Ok(message) => {
            if already_saved {
                reply(game, message);
            } else if let Err(e) = save(state) {
                state.world = Some(previous);
                error(game, format!("failed to save world: {e}"));
            } else { reply(game, message); }
        }
        Err(message) => {
            if already_saved {
                // A previous step committed world.json. Rolling back only
                // RAM would make subsequent saves silently erase the
                // committed identity/transaction record. Fail closed.
                log::error!("[civilizations] command failed after durable save: {message}");
                state.problem = Some(format!(
                    "command failed after durable save; inspect before restarting: {message}"
                ));
                error(game, format!(
                    "Command stopped after a durable save; world halted for inspection: {message}"
                ));
            } else {
                state.world = Some(previous);
                error(game, message);
            }
        }
    }
}
