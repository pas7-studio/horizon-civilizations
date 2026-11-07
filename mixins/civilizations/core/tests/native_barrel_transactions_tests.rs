    use super::super::*;

    fn town() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        (world,id)
    }

    #[test]
    fn barrel_origins_are_exact_and_inside_native_town_hall_plan() {
        let places = hall_storage_positions([0,70,0]);
        assert_eq!(places, [[-1,70,1],[1,70,1]]);
        let blueprint = town_hall_plan([0,70,0]);
        assert!(places.iter().all(|pos| blueprint.iter().any(|b|
            b.pos == *pos && b.block == "minecraft:barrel")));
        assert_ne!(places[0], places[1]);
    }

    #[test]
    fn historic_saves_have_empty_real_barrel_journal_and_escrow() {
        let (world,id) = town();
        let mut saved = serde_json::to_value(&world).unwrap();
        let village = saved["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap();
        village.remove("barrel_transfer");
        village.remove("market_purchase");
        village.remove("build_escrow");
        village.remove("wheat_in_transit");
        let migrated: World = serde_json::from_value(saved).unwrap();
        migrated.validate().unwrap();
        let v = &migrated.settlements[&id];
        assert!(v.barrel_transfer.is_none());
        assert!(v.market_purchase.is_none());
        assert!(v.build_escrow.is_empty());
        assert_eq!(v.wheat_in_transit, 0);
    }

    #[test]
    fn barrel_deposit_requires_real_incoming_receipts_and_a_named_hall_barrel() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 1, None).is_err());
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 8).unwrap();
        assert!(world.plan_barrel_transfer(id, [80,70,0], Good::Wood,
            BarrelDirection::ToBarrel, 0, 1, None).is_err());
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 9, None).is_err());
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 8, Some(0)).is_err());
        world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 0, 8, None).unwrap();
        world.validate().unwrap();
    }

    #[test]
    fn native_deposit_credits_container_only_after_later_observation() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 8).unwrap();
        world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBarrel, 3, 8, None).unwrap();
        assert!(world.confirm_barrel_transfer(id, 11).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        world.issue_barrel_transfer(id).unwrap();
        assert!(world.issue_barrel_transfer(id).is_err());
        assert!(world.confirm_barrel_transfer(id, 10).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        world.confirm_barrel_transfer(id, 11).unwrap(); // native observer
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.settlements[&id].barrel_transfer.is_none());
        assert!(world.confirm_barrel_transfer(id, 11).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn in_progress_native_transfer_never_reissues_or_mutates_unproven_stock() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Stone, 2).unwrap();
        world.plan_barrel_transfer(id, barrel, Good::Stone,
            BarrelDirection::ToBarrel, 0, 2, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        let json = serde_json::to_string(&world).unwrap();
        let mut rebooted: World = serde_json::from_str(&json).unwrap();
        rebooted.validate().unwrap();
        assert!(rebooted.issue_barrel_transfer(id).is_err());
        rebooted.block_barrel_transfer(id).unwrap();
        assert!(rebooted.confirm_barrel_transfer(id, 2).is_err());
        assert_eq!(rebooted.settlements[&id].warehouse_quantity(Good::Stone), 2);
        assert_eq!(rebooted.settlements[&id].barrel_transfer.as_ref().unwrap().phase,
                   BarrelPhase::Blocked);
        rebooted.validate().unwrap();
    }

    #[test]
    fn builder_only_funds_from_materials_extracted_from_real_barrel() {
        let (mut world,id) = town();
        world.advance(1).unwrap();
        world.start_visual_house(id, [20,70,0]).unwrap();
        let v = world.settlements.get_mut(&id).unwrap();
        v.warehouse_credit(Good::Wood, HUT_WOOD_LOGS).unwrap();
        v.warehouse_credit(Good::Stone, HUT_COBBLESTONE).unwrap();
        assert!(!world.reserve_hut_supplies(id).unwrap());
        let barrel = hall_storage_positions([0,70,0])[0];
        for (good,count) in [(Good::Wood, 8),(Good::Stone, 8)] {
            world.plan_barrel_transfer(id, barrel, good,
                BarrelDirection::ToBuilder, count, count, Some(0)).unwrap();
            world.issue_barrel_transfer(id).unwrap();
            world.confirm_barrel_transfer(id, 0).unwrap();
        }
        assert_eq!(world.settlements[&id].build_escrow[&Good::Wood], 8);
        assert!(!world.reserve_hut_supplies(id).unwrap());
        world.settlements.get_mut(&id).unwrap().build_escrow.insert(Good::Wood,HUT_WOOD_LOGS);
        world.settlements.get_mut(&id).unwrap().build_escrow.insert(Good::Stone,HUT_COBBLESTONE);
        assert!(world.reserve_hut_supplies(id).unwrap());
        assert!(!world.reserve_hut_supplies(id).unwrap());
        assert!(world.settlements[&id].hut_supplies_reserved);
        assert_eq!(world.settlements[&id].build_escrow[&Good::Wood], 0);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), HUT_WOOD_LOGS);
        world.validate().unwrap();
    }

    #[test]
    fn native_market_purchase_debits_currency_before_external_delivery() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        let coins = world.players["owner"].coins;
        let cost = world.begin_market_purchase("owner", id, Good::Wood,
            4, barrel, 0, 18).unwrap();
        assert_eq!(cost, 12);
        assert_eq!(world.players["owner"].coins, coins - 12);
        assert_eq!(world.settlements[&id].treasury_coins, 12);
        assert_eq!(world.settlements[&id].market_purchase.as_ref().unwrap().phase,
                   MarketPhase::Issued);
        assert!(world.begin_market_purchase("owner", id, Good::Wood,
            4, barrel, 0, 18).is_err());
        assert!(world.complete_market_purchase(id, "forged").is_err());
        world.complete_market_purchase(id, "owner").unwrap(); // native delivery validated
        assert!(world.settlements[&id].market_purchase.is_none());
        assert_eq!(world.players["owner"].coins, coins - 12);
        world.validate().unwrap();
    }

    #[test]
    fn unresolved_market_purchase_blocks_concurrent_native_stock_movements() {
        let (mut world,id) = town();
        let barrel = hall_storage_positions([0,70,0])[0];
        world.begin_market_purchase("owner", id, Good::Wood, 4, barrel, 0, 18).unwrap();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Stone, 3).unwrap();
        assert!(world.plan_barrel_transfer(id, barrel, Good::Stone,
            BarrelDirection::ToBarrel, 0, 3, None).is_err());
        world.block_market_purchase(id).unwrap();
        assert!(world.complete_market_purchase(id, "owner").is_err());
        let persisted: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        persisted.validate().unwrap();
        assert_eq!(persisted.settlements[&id].market_purchase.as_ref().unwrap().phase,
                   MarketPhase::Blocked);
    }

    #[test]
    fn receipt_wheat_is_delivered_once_by_farmer_before_barrel_transfer() {
        let (mut world,id) = town();
        world.settlements.get_mut(&id).unwrap().wheat_in_transit = 16;
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 0);
        assert_eq!(world.deliver_farm_wheat(id), Ok(true));
        assert_eq!(world.deliver_farm_wheat(id), Ok(false));
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 0);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 16);
        let barrel = hall_storage_positions([0,70,0])[1];
        world.plan_barrel_transfer(id, barrel, Good::Wheat,
            BarrelDirection::ToBarrel, 0, 8, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id, 8).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 8);
        world.validate().unwrap();
    }
