// API: a sign's text was submitted. Village boards refuse every edit.
fn civilizations_guard_sign_edit(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    cancel: &mut bool,
) {
    crate::game::civilizations_glue::protect_civic_sign_edit(game, pos, cancel);
}
