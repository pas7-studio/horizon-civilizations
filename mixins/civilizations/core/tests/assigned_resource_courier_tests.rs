    use super::super::*;

    fn assigned_wood_job() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_gather(id, Good::Wood, [16, 70, 0], [17, 70, 0]).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.validate().unwrap();
        (world, id, worker)
    }

    #[test]
    fn source_issue_requires_explicit_same_resident_binding() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.plan_gather(id, Good::Stone, [16, 70, 0], [17, 70, 0]).unwrap();
        assert!(world.mark_gather_issued(id).is_err());
        assert!(world.confirm_gather(id).is_err());
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.assign_gather_worker(id, miner).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.assign_gather_worker(id, miner).is_err());
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn unqualified_or_second_worker_cannot_steal_courier_assignment() {
        let (mut world, id, first) = assigned_wood_job();
        let baker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Baker).unwrap().id;
        assert!(world.assign_gather_worker(id, baker).is_err());
        let mut another = world.settlements[&id].residents.iter()
            .find(|r| r.id == first).unwrap().clone();
        another.id += 500;
        another.name = "Substitute".into();
        let second = another.id;
        world.settlements.get_mut(&id).unwrap().residents.push(another);
        assert!(world.assign_gather_worker(id, second).is_err());
        assert_eq!(world.settlements[&id].gather_job.as_ref().unwrap().worker_id, Some(first));
        world.validate().unwrap();
    }

    #[test]
    fn same_courier_persists_through_restart_and_next_real_oak_log() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_short_oak_stem(id, [16, 70, 0], [17, 70, 0], 3).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        let mut restored: World =
            serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        let next = restored.settlements[&id].gather_job.as_ref().unwrap();
        assert_eq!(next.worker_id, Some(worker));
        assert_eq!(next.position, [16, 71, 0]);
        assert!(!next.issued);
        assert!(restored.confirm_gather(id).is_err());
        restored.mark_gather_issued(id).unwrap();
        restored.mark_gather_native_edit_confirmed(id).unwrap();
        let restored_assigned = restored.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        restored.witness_gather_pickup(id, restored_assigned).unwrap();
        restored.confirm_gather(id).unwrap();
        assert_eq!(restored.settlements[&id].warehouse_quantity(Good::Wood), 2);
    }

    #[test]
    fn preexisting_unbound_issued_receipt_cannot_mint_stock() {
        let (mut world, id, _) = assigned_wood_job();
        world.mark_gather_issued(id).unwrap();
        let mut saved = serde_json::to_value(&world).unwrap();
        saved.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap()
            .get_mut("gather_job").unwrap().as_object_mut().unwrap()
            .remove("worker_id");
        let mut old: World = serde_json::from_value(saved).unwrap();
        old.validate().unwrap();
        assert!(old.settlements[&id].gather_job.as_ref().unwrap().worker_id.is_none());
        assert!(old.confirm_gather(id).is_err());
        assert_eq!(old.settlements[&id].warehouse_quantity(Good::Wood), 0);
        old.block_gather(id).unwrap();
        old.abandon_blocked_issued_gather(id).unwrap();
        old.validate().unwrap();
    }

    #[test]
    fn malformed_assigned_courier_id_is_rejected_by_save_validation() {
        let (mut world, id, _) = assigned_wood_job();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut().unwrap()
            .worker_id = Some(u64::MAX);
        assert!(world.validate().is_err());
    }

    #[test]
    fn dead_resident_cannot_be_newly_assigned_to_native_material_receipt() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let miner = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16, 70, 0], [17, 70, 0]).unwrap();
        world.settlements.get_mut(&id).unwrap().residents.iter_mut()
            .find(|r| r.id == miner).unwrap().alive = false;
        assert!(world.assign_gather_worker(id, miner).is_err());
        world.validate().unwrap();
    }
