    use super::super::*;

    #[test]
    fn site_board_stands_outside_the_building_footprint() {
        let origin = [10, 64, 10];
        let [x, y, z] = site_board_position(origin);
        assert_eq!(y, origin[1]);
        // The 5x5 shell and foundation span origin +-2; the board is beyond z-2.
        assert!(z < origin[2] - 2);
        assert!(x >= origin[0] - 2 && x <= origin[0] + 2);
    }

    #[test]
    fn finished_boards_fit_and_report_real_counts() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        let hut = render_hut_ready(&world.settlements[&id]);
        assert!(hut.lines.iter().all(|l| l.text.chars().count() <= SIGN_WIDTH));
        assert_eq!(hut.plain()[1], "Finished");
        let bakery = BakeryWorkshop {
            origin: [0, 70, 20], building: None, complete: true, supplies_reserved: true,
            construction_escrow: Default::default(), wheat_escrow: 3,
            bread_in_transit: 1, batches: 12,
        };
        let board = render_bakery_ready(&bakery);
        assert_eq!(board.plain()[1], "Batches 12");
        assert_eq!(board.plain()[2], "Wheat in 3");
        assert_eq!(board.plain()[3], "Bread out 1");
    }
