fn civilizations_register_commands(commands: &mut Vec<crate::game::native_server::NativeCommand>) {
    crate::game::civilizations_glue::register(commands);
}
