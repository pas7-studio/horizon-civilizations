// API 10: a click in a menu a mod opened. Villager conversation menus turn
// the click into navigation, joining, or a sale confirmation.
fn civilizations_menu_click(
    game: &mut crate::game::Game<'_>,
    uuid: u128,
    window: i32,
    slot: i32,
) {
    crate::game::civilizations_glue::menu_clicked(game, uuid, window, slot);
}
