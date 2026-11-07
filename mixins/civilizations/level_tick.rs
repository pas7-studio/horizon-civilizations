// Hooked after the authoritative level tick. Do not run this from a player
// command: native world edits require a server/level context.
fn civilizations_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::civilizations_glue::level_tick(game, dimension);
}
