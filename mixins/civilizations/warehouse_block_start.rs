// Native Horizon API 233: a server player attempts to open the inventory
// supplied by a real block. Only village-hall barrels are intercepted.
fn civilizations_warehouse_block_start(
    game: &mut crate::game::Game<'_>,
    player: u128,
    name: &str,
    pos: [i32; 3],
    cancel: &mut bool,
) {
    crate::game::civilizations_glue::protect_hall_inventory(
        game, player, name, pos, cancel
    );
}
