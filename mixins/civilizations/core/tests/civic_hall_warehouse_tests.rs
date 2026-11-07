use std::collections::{BTreeSet};
    use super::super::*;

    fn settlement() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        (world, id)
    }

    fn stock_hall(world: &mut World, id: u64) {
        let v = world.settlements.get_mut(&id).unwrap();
        v.warehouse_credit(Good::Wood, HALL_WOOD_LOGS).unwrap();
        v.warehouse_credit(Good::Stone, HALL_COBBLESTONE).unwrap();
    }

    #[test]
    fn hall_is_a_unique_physical_blueprint_with_mayor_desk_and_two_bulletins() {
        let origin = [0, 70, 0];
        let hall = town_hall_plan(origin);
        let sites: BTreeSet<_> = hall.iter().map(|b| b.pos).collect();
        assert_eq!(hall.len(), sites.len());
        assert_eq!(hall.iter().filter(|b| b.floor).count(), 25);
        assert_eq!(hall.iter().filter(|b| b.block == "minecraft:barrel").count(), 2);
        assert_eq!(hall.iter().filter(|b| b.block == "minecraft:oak_wall_sign").count(), 2);
        assert_eq!(hall.iter().filter(|b| b.block == "minecraft:lectern").count(), 1);
        assert_eq!(hall_board_positions(origin), [[-1, 71, -3], [1, 71, -3]]);
        // Readable from outside the front door, one block past the north wall.
        assert!(hall.iter().any(|b| b.pos == [-1, 71, -3]));
        assert!(hall.iter().any(|b| b.pos == [1, 71, -3]));
        assert_eq!(hall, town_hall_plan(origin));
    }

    #[test]
    fn new_and_legacy_villages_have_zero_real_warehouse_supply() {
        let (world,id) = settlement();
        let v = &world.settlements[&id];
        assert!(v.quantity(Good::Wood) > 0); // only abstract virtual stock
        assert_eq!(v.warehouse_quantity(Good::Wood), 0);
        assert_eq!(v.hall_materials_missing(), (HALL_WOOD_LOGS, HALL_COBBLESTONE));
        assert!(!v.hall_complete);
        assert!(v.hall_job.is_none());
        let mut older = serde_json::to_value(&world).unwrap();
        let v = older["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap();
        v.remove("warehouse");
        v.remove("hall_job");
        v.remove("hall_complete");
        v.remove("gather_job");
        v.remove("hut_supplies_reserved");
        let migrated: World = serde_json::from_value(older).unwrap();
        migrated.validate().unwrap();
        assert_eq!(migrated.settlements[&id].warehouse_quantity(Good::Stone), 0);
        assert!(!migrated.settlements[&id].hall_complete);
    }

    #[test]
    fn hall_reserves_real_supplies_once_and_only_finishes_after_observation() {
        let (mut world,id) = settlement();
        assert!(!world.reserve_town_hall(id).unwrap());
        stock_hall(&mut world,id);
        assert!(world.reserve_town_hall(id).unwrap());
        assert!(!world.reserve_town_hall(id).unwrap());
        let v = &world.settlements[&id];
        assert_eq!(v.warehouse_quantity(Good::Stone), 0);
        assert_eq!(v.warehouse_quantity(Good::Wood), 0);
        assert!(world.finish_town_hall(id).is_err());
        world.settlements.get_mut(&id).unwrap().hall_job.as_mut().unwrap().cursor =
            town_hall_plan([0,70,0]).len();
        world.finish_town_hall(id).unwrap();
        assert!(world.settlements[&id].hall_complete);
        assert!(world.settlements[&id].hall_job.is_none());
        assert!(!world.reserve_town_hall(id).unwrap());
        let loaded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        loaded.validate().unwrap();
        assert!(loaded.settlements[&id].hall_complete);
    }

    #[test]
    fn hall_reconciler_halts_on_player_blocks_and_will_not_clear_modifications() {
        let job = VisualBuildJob { origin: [30,70,0], cursor: 0, blocked: false };
        let floor = town_hall_plan(job.origin)[0].pos;
        assert!(matches!(
            inspect_town_hall(&job, |p| if p == floor {
                Some("minecraft:chest".to_owned())
            } else { Some("minecraft:air".to_owned()) }),
            BuildInspection::Blocked { .. }
        ));
        assert_eq!(inspect_town_hall(&job, |_| None), BuildInspection::Unloaded);
    }

    #[test]
    fn physically_received_trade_goods_do_not_double_credit_abstract_stock() {
        let (mut world,id) = settlement();
        let before = world.settlements[&id].quantity(Good::Wood);
        assert_eq!(world.sell("owner", id, Good::Wood, 8), Ok(24));
        assert_eq!(world.settlements[&id].quantity(Good::Wood), before);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        assert_eq!(world.buy("owner", id, Good::Wood, 3), Ok(9));
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 5);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), before);
        world.validate().unwrap();
    }

    #[test]
    fn a_hut_must_reserve_actual_materials_once_before_it_can_complete() {
        let (mut world,id) = settlement();
        world.advance(1).unwrap();
        world.start_visual_house(id, [20,70,0]).unwrap();
        assert!(!world.reserve_hut_supplies(id).unwrap());
        {
            let v = world.settlements.get_mut(&id).unwrap();
            v.warehouse_credit(Good::Wood, HUT_WOOD_LOGS).unwrap();
            v.warehouse_credit(Good::Stone, HUT_COBBLESTONE).unwrap();
        }
        assert!(world.reserve_hut_supplies(id).unwrap());
        assert!(!world.reserve_hut_supplies(id).unwrap());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        world.settlements.get_mut(&id).unwrap().visual_job.as_mut().unwrap().cursor =
            hut_plan([20,70,0]).len();
        world.finish_visual_house(id).unwrap();
        assert!(!world.settlements[&id].hut_supplies_reserved);
        assert_eq!(world.settlements[&id].public_huts(), 1);
        world.validate().unwrap();
    }

    #[test]
    fn gathering_requires_native_issued_receipt_and_cannot_credit_twice() {
        let (mut world,id) = settlement();
        assert!(world.plan_gather(id, Good::Bread, [24,70,0],[25,70,0]).is_err());
        assert!(world.plan_gather(id, Good::Wood, [0,70,0],[1,70,0]).is_err());
        assert!(world.plan_gather(id, Good::Wood, [24,70,0],[200,70,0]).is_err());
        world.plan_gather(id, Good::Wood, [24,70,0],[25,70,0]).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.confirm_gather(id).is_err());
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        assert!(world.mark_gather_issued(id).is_err()); // unassigned
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.mark_gather_issued(id).is_err());
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn obstructed_gather_is_discarded_without_credit_or_reissue() {
        let (mut world,id) = settlement();
        world.plan_gather(id, Good::Stone, [24,69,0],[25,70,0]).unwrap();
        world.block_gather(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        assert!(world.mark_gather_issued(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.discard_blocked_gather(id).unwrap();
        assert!(world.settlements[&id].gather_job.is_none());
        world.validate().unwrap();
    }

    #[test]
    fn only_confirmed_crop_cycles_credit_physical_warehouse_once() {
        let (mut world,id) = settlement();
        world.start_farm(id, [20,70,0]).unwrap();
        {
            let f = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
            f.stage = FarmStage::Harvesting;
            f.cursor = 8;
        }
        let abstract_before = world.settlements[&id].quantity(Good::Wheat);
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 0);
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 8);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), abstract_before);
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().harvest_sequence, 1);
        assert!(world.advance_farm(id, FarmDecision::StageComplete).is_err());
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 8);
        // The native adapter must first verify a UUID-bound farmer returned
        // to the hall. This unit test exercises only its domain commit.
        assert_eq!(world.deliver_farm_wheat(id), Ok(true));
        assert_eq!(world.deliver_farm_wheat(id), Ok(false));
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wheat), 8);
        assert_eq!(world.settlements[&id].wheat_delivery_pending(), 0);
        let restored: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.settlements[&id].warehouse_quantity(Good::Wheat), 8);
    }
