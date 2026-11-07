    use super::super::*;

    fn with_bakery_and_hut() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        world.start_bakery(id, [24, 70, 0]).unwrap();
        let village = world.settlements.get_mut(&id).unwrap();
        village.houses = 1;
        village.pending_visual_buildings = 1;
        (world, id)
    }

    #[test]
    fn two_unfunded_projects_need_both_bills_not_only_bakery() {
        let (world, id) = with_bakery_and_hut();
        let village = &world.settlements[&id];
        assert_eq!(physical_construction_shortfall(village, &[]),
            (BAKERY_WOOD_LOGS + HUT_WOOD_LOGS,
             BAKERY_COBBLESTONE + HUT_COBBLESTONE));
    }

    #[test]
    fn project_escrows_are_not_double_counted_as_public_stock() {
        let (mut world, id) = with_bakery_and_hut();
        let village = world.settlements.get_mut(&id).unwrap();
        village.build_escrow.insert(Good::Wood, 4);
        village.bakery.as_mut().unwrap().construction_escrow.insert(Good::Wood, 6);
        village.warehouse_credit(Good::Wood, 5).unwrap();
        let stock = [(Good::Wood, 12), (Good::Stone, 10)];
        // 44 total wood - 10 in protected project escrows - 17 available.
        assert_eq!(physical_construction_shortfall(village, &stock), (17, 48));
    }

    #[test]
    fn blocked_bakery_does_not_induce_more_resource_harvests() {
        let (mut world, id) = with_bakery_and_hut();
        let village = world.settlements.get_mut(&id).unwrap();
        village.bakery.as_mut().unwrap().building.as_mut().unwrap().blocked = true;
        assert_eq!(physical_construction_shortfall(village, &[]),
            (HUT_WOOD_LOGS, HUT_COBBLESTONE));
        village.pending_visual_buildings = 0;
        assert_eq!(physical_construction_shortfall(village, &[]), (0, 0));
    }


    #[test]
    fn no_new_extraction_into_obstructed_bakery_job() {
        let (mut world, id) = with_bakery_and_hut();
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().blocked = true;
        let barrel = hall_storage_positions([0, 70, 0])[0];
        assert!(world.plan_barrel_transfer(id, barrel, Good::Wood,
            BarrelDirection::ToBakeryBuilder, 8, 8, Some(0)).is_err());
        assert!(world.validate().is_ok());
    }

    #[test]
    fn saved_wheat_extraction_rejects_unfinished_or_full_bakery() {
        let (mut world, id) = with_bakery_and_hut();
        let barrel = hall_storage_positions([0, 70, 0])[0];
        world.settlements.get_mut(&id).unwrap().barrel_transfer = Some(BarrelTransfer {
            position: barrel, good: Good::Wheat,
            direction: BarrelDirection::ToBakerIngredients,
            phase: BarrelPhase::Planned, before: 2, count: 1, slot: Some(0),
        });
        assert!(world.validate().is_err()); // no completed workstation
        let bakery = world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
        bakery.building = None;
        bakery.supplies_reserved = true;
        bakery.complete = true;
        assert!(world.validate().is_ok());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .wheat_escrow = BREAD_WHEAT_RECIPE;
        assert!(world.validate().is_err()); // no more wheat may be carried
    }

    #[test]
    fn saved_bakery_transfer_requires_a_real_bakery_after_reboot() {
        let (mut world, id) = with_bakery_and_hut();
        let barrel = hall_storage_positions([0, 70, 0])[0];
        world.settlements.get_mut(&id).unwrap().barrel_transfer = Some(BarrelTransfer {
            position: barrel, good: Good::Wood,
            direction: BarrelDirection::ToBakeryBuilder,
            phase: BarrelPhase::Planned, before: 2, count: 1, slot: Some(0),
        });
        assert!(world.validate().is_ok());
        world.settlements.get_mut(&id).unwrap().bakery = None;
        assert!(world.validate().is_err());
    }
