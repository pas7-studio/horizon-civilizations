fn civilizations_lifecycle(phase: crate::game::native_lifecycle::WorldPhase) {
    use crate::game::native_lifecycle::WorldPhase;
    if matches!(phase, WorldPhase::WorldCloseFinal | WorldPhase::WorldCloseFailed) {
        crate::game::civilizations_glue::closed();
    }
}
