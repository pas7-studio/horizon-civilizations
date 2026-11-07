    use super::super::*;

    #[test]
    fn previously_witnessed_issued_receipt_loads_but_cannot_be_paid() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16, 70, 0], [17, 70, 0]).unwrap();
        world.assign_gather_worker(id, miner).unwrap();
        world.mark_gather_issued(id).unwrap();
        // PR #30 legitimately stored this flag but had no post-command
        // native acknowledgement. It must not brick the entire save.
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap()
            .pickup_witnessed = true;
        let mut value = serde_json::to_value(&world).unwrap();
        value.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap()
            .get_mut("gather_job").unwrap().as_object_mut().unwrap()
            .remove("native_edit_confirmed");
        let mut migrated: World = serde_json::from_value(value).unwrap();
        migrated.validate().unwrap();
        let job = migrated.settlements[&id].gather_job.as_ref().unwrap();
        assert!(job.issued);
        assert!(job.pickup_witnessed);
        assert!(!job.native_edit_confirmed);
        assert!(migrated.confirm_gather(id).is_err());
        assert_eq!(migrated.settlements[&id].warehouse_quantity(Good::Stone), 0);
        // The bridge must block this uncertain issued job after reload.
        migrated.block_gather(id).unwrap();
        migrated.abandon_blocked_issued_gather(id).unwrap();
        assert_eq!(migrated.settlements[&id].warehouse_quantity(Good::Stone), 0);
        migrated.validate().unwrap();
    }
