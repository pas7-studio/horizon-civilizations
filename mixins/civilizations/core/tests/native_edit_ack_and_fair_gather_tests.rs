    use super::super::*;

    fn assigned_stone() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Miner).unwrap().id;
        world.plan_gather(id, Good::Stone, [16,70,0], [17,70,0]).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        (world, id, worker)
    }

    #[test]
    fn rotating_active_villages_never_sticks_on_first_and_handles_empty() {
        assert_eq!(gather_turn_index(0, 0), None);
        assert_eq!(gather_turn_index(u64::MAX, 0), None);
        assert_eq!(gather_turn_index(0, 1), Some(0));
        assert_eq!(gather_turn_index(8, 1), Some(0));
        for turn in 0..12_u64 {
            assert_eq!(gather_turn_index(turn * 8, 3), Some((turn % 3) as usize));
        }
        assert_eq!(gather_turn_index(8, 3), Some(1));
        assert_eq!(gather_turn_index(16, 3), Some(2));
        assert_eq!(gather_turn_index(24, 3), Some(0));
    }

    #[test]
    fn unacknowledged_issued_edit_cannot_be_witnessed_or_credited() {
        let (mut world, id, worker) = assigned_stone();
        assert!(world.mark_gather_native_edit_confirmed(id).is_err());
        world.mark_gather_issued(id).unwrap();
        assert!(world.witness_gather_pickup(id, worker).is_err());
        assert!(world.confirm_gather(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 0);
        world.validate().unwrap();
    }

    #[test]
    fn native_ack_is_exactly_once_and_required_before_source_witness() {
        let (mut world, id, worker) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(world.mark_gather_native_edit_confirmed(id).is_err());
        world.witness_gather_pickup(id, worker).unwrap();
        world.confirm_gather(id).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 1);
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn old_issued_unconfirmed_receipt_remains_unpayable_after_restart() {
        let (mut world, id, worker) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        let serialized = serde_json::to_string(&world).unwrap();
        let mut restarted: World = serde_json::from_str(&serialized).unwrap();
        restarted.validate().unwrap();
        assert!(!restarted.settlements[&id].gather_job.as_ref()
            .unwrap().native_edit_confirmed);
        assert!(restarted.witness_gather_pickup(id, worker).is_err());
        assert!(restarted.confirm_gather(id).is_err());
        restarted.block_gather(id).unwrap();
        restarted.abandon_blocked_issued_gather(id).unwrap();
        assert_eq!(restarted.settlements[&id].warehouse_quantity(Good::Stone), 0);
        restarted.validate().unwrap();
    }

    #[test]
    fn corrupted_unissued_native_ack_and_unacked_pickup_are_rejected() {
        let (mut world, id, worker) = assigned_stone();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().native_edit_confirmed = true;
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().native_edit_confirmed = false;
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().pickup_witnessed = true;
        assert!(world.validate().is_err());
        assert!(world.witness_gather_pickup(id, worker).is_err());
    }

    #[test]
    fn ack_state_persists_and_next_oak_log_resets_ack_and_pickup() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.plan_short_oak_stem(id, [16,70,0], [17,70,0], 3).unwrap();
        world.assign_gather_worker(id, worker).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let mut restored: World = serde_json::from_str(
            &serde_json::to_string(&world).unwrap()).unwrap();
        assert!(restored.settlements[&id].gather_job.as_ref()
            .unwrap().native_edit_confirmed);
        restored.witness_gather_pickup(id, worker).unwrap();
        restored.confirm_gather(id).unwrap();
        let next = restored.settlements[&id].gather_job.as_ref().unwrap();
        assert_eq!(next.worker_id, Some(worker));
        assert!(!next.issued);
        assert!(!next.native_edit_confirmed);
        assert!(!next.pickup_witnessed);
        restored.validate().unwrap();
    }

    #[test]
    fn native_ack_without_assigned_worker_is_invalid_on_old_save() {
        let (mut world, id, _) = assigned_stone();
        world.mark_gather_issued(id).unwrap();
        world.settlements.get_mut(&id).unwrap().gather_job.as_mut()
            .unwrap().worker_id = None;
        assert!(world.mark_gather_native_edit_confirmed(id).is_err());
        world.validate().unwrap();
    }
