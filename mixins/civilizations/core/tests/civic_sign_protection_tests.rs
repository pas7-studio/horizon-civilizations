    use super::super::*;

    #[test]
    fn only_registered_boards_are_protected() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        let dim = "minecraft:overworld";
        for pos in hall_all_board_positions([0, 70, 0]) {
            assert!(world.is_civic_sign(dim, pos), "{pos:?}");
        }
        assert!(world.is_civic_sign(dim, site_board_position([0, 70, 0])));
        // An unrelated sign, another dimension and an unfinished hall stay editable.
        assert!(!world.is_civic_sign(dim, [40, 70, 40]));
        assert!(!world.is_civic_sign("minecraft:the_nether", hall_all_board_positions([0, 70, 0])[0]));
        world.settlements.get_mut(&id).unwrap().hall_complete = false;
        assert!(!world.is_civic_sign(dim, hall_all_board_positions([0, 70, 0])[0]));
    }

    #[test]
    fn hut_and_bakery_site_boards_are_protected() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        world.settlements.get_mut(&id).unwrap().visual_sites.push([20, 70, 20]);
        assert!(world.is_civic_sign("minecraft:overworld", site_board_position([20, 70, 20])));
        assert!(!world.is_civic_sign("minecraft:overworld", [20, 70, 20]));
    }
