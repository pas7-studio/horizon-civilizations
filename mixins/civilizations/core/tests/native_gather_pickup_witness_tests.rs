    use super::super::*;

    fn assigned_stone() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16,70,0], [17,70,0]).unwrap();
        world.assign_gather_worker(id, miner).unwrap();
        (world, id, miner)
    }

    #[test]
    fn no_stock_credit_between_issue_and_source_pickup_witness() {
        let (mut world, id, miner) = assigned_stone();
        assert!(world.witness_gather_pickup(id, miner).is_err());
        world.mark_gather_issued(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, miner).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn other_worker_or_unknown_id_cannot_claim_source_pickup() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let stranger = world.settlements[&id].residents.iter()
            .find(|r| r.id != miner).unwrap().id;
        assert!(world.witness_gather_pickup(id, stranger).is_err());
        assert!(world.witness_gather_pickup(id, u64::MAX).is_err());
        assert!(!world.settlements[&id].gather_job.as_ref().unwrap().pickup_witnessed);
        world.witness_gather_pickup(id, miner).unwrap();
        assert!(world.witness_gather_pickup(id, miner).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn witnessed_pickup_survives_reload_without_duplication() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, miner).unwrap();
        let mut restored: World =
            serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        assert!(restored.settlements[&id].gather_job.as_ref().unwrap().pickup_witnessed);
        assert!(restored.witness_gather_pickup(id, miner).is_err());
        restored.confirm_gather(id).unwrap();
        assert!(restored.confirm_gather(id).is_err());
        assert_eq!(restored.settlements[&id].warehouse_quantity(Good::Stone), 1);
    }

    #[test]
    fn every_new_oak_segment_resets_pickup_until_this_log_is_observed() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_short_oak_stem(id, [16,70,0], [17,70,0], 3).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, worker).unwrap();
        world.confirm_gather(id).unwrap();
        let next = world.settlements[&id].gather_job.as_ref().unwrap();
        assert_eq!(next.position, [16,71,0]);
        assert_eq!(next.worker_id, Some(worker));
        assert!(!next.issued && !next.pickup_witnessed);
        assert!(world.confirm_gather(id).is_err());
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        world.witness_gather_pickup(id, worker).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 2);
        world.validate().unwrap();
    }

    #[test]
    fn legacy_missing_witness_does_not_imply_picked_up_from_air() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let mut value = serde_json::to_value(&world).unwrap();
        value.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap()
            .get_mut("gather_job").unwrap().as_object_mut().unwrap()
            .remove("pickup_witnessed");
        let mut previous: World = serde_json::from_value(value).unwrap();
        previous.validate().unwrap();
        assert!(!previous.settlements[&id].gather_job.as_ref().unwrap().pickup_witnessed);
        assert!(previous.confirm_gather(id).is_err());
        previous.witness_gather_pickup(id, miner).unwrap();
        previous.confirm_gather(id).unwrap();
        previous.validate().unwrap();
    }

    #[test]
    fn corrupted_unissued_witness_is_rejected_by_domain_validation() {
        let (mut world, id, _) = assigned_stone();
        world.settlements.get_mut(&id).unwrap()
            .gather_job.as_mut().unwrap().pickup_witnessed = true;
        assert!(world.validate().is_err());
    }

    #[test]
    fn blocking_uncertain_issued_pickup_must_not_mint_any_resource() {
        let (mut world, id, miner) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.witness_gather_pickup(id, miner).unwrap();
        world.block_gather(id).unwrap();
        assert!(world.confirm_gather(id).is_err());
        world.abandon_blocked_issued_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.validate().unwrap();
    }
