    use super::super::*;

    fn village() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        (world, id)
    }

    fn resident_with(world: &World, id: u64, job: Job) -> Resident {
        let mut resident = world.settlements[&id].residents.first().cloned().expect("residents");
        resident.job = job;
        resident
    }

    fn input_for<'a>(
        v: &'a Settlement, resident: &'a Resident, wants: &'a [(Good, u32)],
        carried: &'a [(Good, u32)], is_member: bool,
    ) -> DialogueInput<'a> {
        DialogueInput {
            village: v, resident, is_member, routine: "work shift".into(), on_duty: true,
            wants, stock_known: true, wood_missing: 8, stone_missing: 0, carried, notice: None,
        }
    }

    #[test]
    fn only_mayor_and_baker_keep_a_stall() {
        assert!(job_trades(Job::Merchant) && job_trades(Job::Baker));
        for job in [Job::Farmer, Job::Lumberjack, Job::Miner, Job::Builder, Job::Guard, Job::Laborer] {
            assert!(!job_trades(job), "{job:?}");
        }
    }

    #[test]
    fn mayor_buys_exactly_the_real_construction_shortfall() {
        let (world, id) = village();
        let v = &world.settlements[&id];
        assert_eq!(resident_wants(v, Job::Merchant, 8, 0), vec![(Good::Wood, 8)]);
        assert_eq!(resident_wants(v, Job::Merchant, 100, 25),
            vec![(Good::Wood, 64), (Good::Stone, 25)]);
        assert!(resident_wants(v, Job::Merchant, 0, 0).is_empty());
        assert!(resident_wants(v, Job::Farmer, 8, 8).is_empty());
        assert!(resident_wants(v, Job::Baker, 8, 8).is_empty());
    }

    #[test]
    fn sale_is_capped_by_want_and_pockets_and_drops_junk() {
        let wants = [(Good::Wood, 8), (Good::Stone, 25)];
        let carried = [(Good::Wood, 40), (Good::Stone, 5)];
        assert_eq!(parse_sale_args(&wants, &carried, &["wood.64", "stone.25"]).unwrap(),
            vec![(Good::Wood, 8), (Good::Stone, 5)]);
        // Sliders may report 8 or 8.0.
        assert_eq!(parse_sale_args(&wants, &carried, &["wood.3.0"]).unwrap(), vec![(Good::Wood, 3)]);
        // A good the resident does not buy, or zero, or a repeat, is ignored.
        assert_eq!(parse_sale_args(&wants, &carried, &["wheat.5", "wood.0", "wood.2", "wood.4"]).unwrap(),
            vec![(Good::Wood, 2)]);
        assert!(parse_sale_args(&wants, &carried, &["wood.0"]).is_err());
        assert!(parse_sale_args(&wants, &carried, &["wheat.5"]).is_err());
        assert!(parse_sale_args(&wants, &carried, &[]).is_err());
        assert!(parse_sale_args(&wants, &carried, &["nonsense"]).is_err());
        assert!(parse_sale_args(&wants, &carried, &["gold.3"]).is_err());
        assert!(parse_sale_args(&wants, &carried, &["wood.NaN"]).is_err());
        // Nothing carried: nothing sells.
        assert!(parse_sale_args(&wants, &[], &["wood.5"]).is_err());
    }

    #[test]
    fn main_page_offers_trade_only_to_trading_roles_and_join_only_to_visitors() {
        let (world, id) = village();
        let v = &world.settlements[&id];
        let labels = |job: Job, member: bool| -> Vec<String> {
            let resident = resident_with(&world, id, job);
            let wants = [(Good::Wood, 8)];
            build_dialog(TalkPage::Main, &input_for(v, &resident, &wants, &[], member))
                .buttons.into_iter().map(|b| b.label).collect()
        };
        assert!(labels(Job::Merchant, true).contains(&"Trade".to_owned()));
        assert!(!labels(Job::Farmer, true).contains(&"Trade".to_owned()));
        assert!(labels(Job::Merchant, false).iter().any(|l| l.starts_with("Join ")));
        assert!(!labels(Job::Merchant, true).iter().any(|l| l.starts_with("Join ")));
    }

    #[test]
    fn trade_page_has_a_slider_only_for_goods_the_player_carries() {
        let (world, id) = village();
        let v = &world.settlements[&id];
        let mayor = resident_with(&world, id, Job::Merchant);
        let wants = [(Good::Wood, 8), (Good::Stone, 25)];
        let carried = [(Good::Wood, 40)];
        let dialog = build_dialog(TalkPage::Trade, &input_for(v, &mayor, &wants, &carried, true));
        assert_eq!(dialog.sliders.len(), 1);
        assert_eq!(dialog.sliders[0].key, "wood");
        assert_eq!((dialog.sliders[0].min, dialog.sliders[0].max, dialog.sliders[0].initial), (0, 8, 8));
        // The sell button is a template over exactly the sliders present.
        let sell = dialog.buttons.iter().find(|b| b.label == "Sell").unwrap();
        assert!(sell.template);
        assert_eq!(sell.command, format!("civ talk Ashford {} sell wood.$(wood)", mayor.id));
        assert!(dialog.paragraphs.iter().any(|(t, _)| t.contains("cobblestone: you carry none")));
    }

    #[test]
    fn trade_page_without_membership_or_goods_has_no_sell_button() {
        let (world, id) = village();
        let v = &world.settlements[&id];
        let mayor = resident_with(&world, id, Job::Merchant);
        let wants = [(Good::Wood, 8)];
        for (carried, member) in [(&[(Good::Wood, 9)][..], false), (&[][..], true)] {
            let dialog = build_dialog(TalkPage::Trade, &input_for(v, &mayor, &wants, carried, member));
            assert!(dialog.sliders.is_empty());
            assert!(dialog.buttons.iter().all(|b| b.label != "Sell"));
            assert!(dialog.buttons.iter().any(|b| b.label == "Back"));
        }
        let farmer = resident_with(&world, id, Job::Farmer);
        let dialog = build_dialog(TalkPage::Trade, &input_for(v, &farmer, &[], &[], true));
        assert!(dialog.paragraphs.iter().any(|(t, _)| t.contains("I do not trade")));
    }

    #[test]
    fn notice_comes_first_and_pages_round_trip_by_name() {
        let (world, id) = village();
        let v = &world.settlements[&id];
        let mayor = resident_with(&world, id, Job::Merchant);
        let mut input = input_for(v, &mayor, &[], &[], true);
        input.notice = Some("Sold 8 oak logs for 24 coins.".into());
        let dialog = build_dialog(TalkPage::Main, &input);
        assert_eq!(dialog.paragraphs[0].0, "Sold 8 oak logs for 24 coins.");
        for page in [TalkPage::Main, TalkPage::Village, TalkPage::Trade] {
            assert_eq!(TalkPage::parse(page.name()), Some(page));
        }
        assert_eq!(TalkPage::parse("evil"), None);
    }

    #[test]
    fn snbt_is_balanced_escaped_and_names_every_button_command() {
        let (world, id) = village();
        let v = &world.settlements[&id];
        let mut mayor = resident_with(&world, id, Job::Merchant);
        mayor.name = "Quote\"Back\\slash".into();
        let wants = [(Good::Wood, 8)];
        let carried = [(Good::Wood, 40)];
        let dialog = build_dialog(TalkPage::Trade, &input_for(v, &mayor, &wants, &carried, true));
        let snbt = dialog_snbt(&dialog);
        assert!(snbt.starts_with("{type:\"minecraft:multi_action\""));
        assert!(snbt.contains("minecraft:number_range"));
        assert!(snbt.contains("dynamic/run_command"));
        assert!(snbt.contains("Quote\\\"Back\\\\slash"));
        // Braces and brackets balance outside of string literals.
        let (mut depth, mut square, mut in_str, mut escaped) = (0_i32, 0_i32, false, false);
        for c in snbt.chars() {
            if in_str {
                if escaped { escaped = false; }
                else if c == '\\' { escaped = true; }
                else if c == '"' { in_str = false; }
                continue;
            }
            match c {
                '"' => in_str = true,
                '{' => depth += 1,
                '}' => depth -= 1,
                '[' => square += 1,
                ']' => square -= 1,
                _ => {}
            }
            assert!(depth >= 0 && square >= 0);
        }
        assert!(!in_str && depth == 0 && square == 0);
        for button in &dialog.buttons {
            assert!(snbt.contains(&snbt_string(&button.command)), "{}", button.command);
        }
        assert!(!snbt.contains('\n'));
    }
