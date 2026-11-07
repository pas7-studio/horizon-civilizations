// Server-side entity right-click from a player. Only UUID-bound NPCs
// handled by Civilizations may claim this interaction.
fn civilizations_interact_entity(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    kind: &str,
    claimed: &mut bool,
) {
    crate::game::civilizations_glue::interact_entity(game, entity, kind, claimed);
}
