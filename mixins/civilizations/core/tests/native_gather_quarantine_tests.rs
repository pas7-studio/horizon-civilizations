    use super::super::*;

    #[test]
    fn issued_blocked_mining_receipt_requires_explicit_abandonment() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        // This record is a durable receipt of an issued natural stone edit.
        world.settlements.get_mut(&id).unwrap().gather_job = Some(GatherJob {
            position: [16,70,0], stand: [17,70,0],
            good: Good::Stone, worker_id: None, issued: true,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: true,
        });
        world.validate().unwrap();
        let before = world.settlements[&id].warehouse_quantity(Good::Stone);
        let restored: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert!(restored.settlements[&id].gather_job.as_ref().unwrap().blocked);
        assert!(world.mark_gather_issued(id).is_err());
        world.abandon_blocked_issued_gather(id).unwrap();
        assert!(world.settlements[&id].gather_job.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), before);
        assert!(world.abandon_blocked_issued_gather(id).is_err());
        world.validate().unwrap();
    }


    #[test]
    fn corrupted_gather_worker_stand_is_rejected_on_load() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        world.plan_gather(id, Good::Stone, [16,70,0], [17,70,0]).unwrap();
        world.validate().unwrap();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap().stand =
            [i32::MAX, 70, 0];
        assert!(world.validate().is_err());
    }

    #[test]
    fn manually_abandoning_unissued_or_unblocked_gather_is_forbidden() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().gather_job = Some(GatherJob {
            position: [16,70,0], stand: [17,70,0],
            good: Good::Wood, worker_id: None, issued: false,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: true,
        });
        assert!(world.abandon_blocked_issued_gather(id).is_err());
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap().issued = true;
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap().blocked = false;
        assert!(world.abandon_blocked_issued_gather(id).is_err());
    }
