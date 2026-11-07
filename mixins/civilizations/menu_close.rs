// API 10: a menu a mod opened closed. Drop that conversation's session.
fn civilizations_menu_closed(
    game: &mut crate::game::Game<'_>,
    uuid: u128,
    window: i32,
) {
    crate::game::civilizations_glue::menu_closed(game, uuid, window);
}
