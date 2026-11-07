fn civilizations_server_tick(
    game: &mut crate::game::Game<'_>,
    phase: crate::game::native_lifecycle::ServerTickPhase,
) {
    if matches!(phase, crate::game::native_lifecycle::ServerTickPhase::ServerTickEnd { .. }) {
        crate::game::civilizations_glue::tick(game);
    }
}
