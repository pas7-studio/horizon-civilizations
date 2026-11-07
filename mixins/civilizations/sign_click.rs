// API: a player right-clicks a block. Village boards never open an editor.
fn civilizations_guard_sign_click(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    crate::game::civilizations_glue::protect_civic_sign_click(game, pos, canceled);
}
