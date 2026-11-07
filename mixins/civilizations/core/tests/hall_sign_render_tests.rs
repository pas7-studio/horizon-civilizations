    use super::super::*;

    fn founded() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        (world, id)
    }

    fn render(world: &World, id: u64, wood: u32, stone: u32, journal: bool) -> [SignSpec; 6] {
        let stored = [(Good::Wood, 12), (Good::Stone, 0), (Good::Bread, 6), (Good::Wheat, 9)];
        render_hall_boards(&HallBoardInput {
            village: &world.settlements[&id],
            stored: &stored,
            wood_missing: wood,
            stone_missing: stone,
            journal_unsettled: journal,
        })
    }

    #[test]
    fn every_line_of_every_board_fits_a_real_sign() {
        let (mut world, id) = founded();
        world.settlements.get_mut(&id).unwrap().name = "A_Very_Long_Village_Name".into();
        for journal in [false, true] {
            for (wood, stone) in [(0, 0), (8, 25), (4_000_000_000, 4_000_000_000)] {
                for board in render(&world, id, wood, stone, journal) {
                    for line in &board.lines {
                        assert!(line.text.chars().count() <= SIGN_WIDTH, "{:?}", line.text);
                    }
                }
            }
        }
    }

    #[test]
    fn shortage_is_red_glowing_and_stock_ok_is_green() {
        let (world, id) = founded();
        let short = render(&world, id, 8, 0, false);
        assert!(short[1].glow);
        assert_eq!(short[1].lines[0].text, "VILLAGE NEEDS");
        assert_eq!(short[1].lines[1].text, "Wood 12 -8");
        assert_eq!(short[1].lines[1].color, SignColor::Alert);
        let ok = render(&world, id, 0, 0, false);
        assert!(!ok[1].glow);
        assert_eq!(ok[1].lines[0].text, "STOCK OK");
        assert_eq!(ok[1].lines[2].color, SignColor::Good);
    }

    #[test]
    fn unsettled_journal_overrides_numeric_shortage() {
        let (world, id) = founded();
        let board = &render(&world, id, 8, 25, true)[1];
        assert_eq!(board.plain()[1], "Trade pending");
        assert!(board.glow);
    }

    #[test]
    fn treasury_and_land_office_show_real_values() {
        let (mut world, id) = founded();
        world.settlements.get_mut(&id).unwrap().treasury_coins = 140;
        let boards = render(&world, id, 0, 0, false);
        assert_eq!(boards[4].plain()[1], "Coins 140");
        assert_eq!(boards[4].plain()[2], "Bread 6 Wht 9");
        assert_eq!(boards[2].plain()[1], format!("Plot {} coins", LAND_PRICE_COINS));
    }

    #[test]
    fn war_state_is_visible_on_diplomacy_board() {
        let (mut world, id) = founded();
        assert_eq!(render(&world, id, 0, 0, false)[3].plain()[1], "PEACE");
        world.settlements.get_mut(&id).unwrap().active_wars.insert(99);
        let board = &render(&world, id, 0, 0, false)[3];
        assert_eq!(board.plain()[1], "AT WAR x1");
        assert!(board.glow);
    }

    #[test]
    fn progress_bar_is_bounded_and_blocked_is_alert() {
        assert_eq!(sign_bar(0, 0, 10), "[----------]");
        assert_eq!(sign_bar(20, 40, 10), "[#####-----]");
        assert_eq!(sign_bar(99, 40, 10), "[##########]");
        let blocked = render_build_progress("HUT", 5, 40, true);
        assert!(blocked.glow);
        assert_eq!(blocked.plain()[3], "BLOCKED");
    }

    #[test]
    fn board_positions_are_unique_and_plan_signs_stay_first() {
        let positions = hall_all_board_positions([10, 64, 10]);
        let plan = hall_board_positions([10, 64, 10]);
        assert_eq!(positions[..2], plan);
        let unique: std::collections::BTreeSet<_> = positions.iter().collect();
        assert_eq!(unique.len(), 6);
        // Every board hangs on the front wall directly outside z-2.
        assert!(positions.iter().all(|p| p[2] == 7));
    }
