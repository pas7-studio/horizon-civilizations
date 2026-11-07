// API 14: prevent ordinary players breaking public village structures
// or another player's purchased private land. OP recovery remains possible.
fn civilizations_guard_structure_break(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    cancel: &mut bool,
) {
    crate::game::civilizations_glue::protect_civilization_block(
        game, pos, cancel
    );
}
