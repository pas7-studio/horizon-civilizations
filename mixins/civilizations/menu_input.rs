// API: the trade menu's input slots at close. A confirmed sale takes the
// goods the village wants; everything left goes back to the player.
fn civilizations_menu_input_closed(
    game: &mut crate::game::Game<'_>,
    uuid: u128,
    window: i32,
    slots: &mut Vec<Option<horizon_client::items::ItemStack>>,
) {
    crate::game::civilizations_glue::menu_input_closed(game, uuid, window, slots);
}
