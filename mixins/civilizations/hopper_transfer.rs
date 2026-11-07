// API 334: intercept a vanilla hopper movement before the source item
// can leave or enter a registered village public hall barrel.
fn civilizations_guard_hopper_transfer(
    game: &crate::game::Game<'_>,
    source: [i32; 3],
    destination: [i32; 3],
    item: &str,
    cancel: &mut bool,
) {
    crate::game::civilizations_glue::protect_public_hopper(
        game, source, destination, item, cancel
    );
}
