use std::collections::{BTreeMap};
    use super::super::*;

    fn world_with_stem() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.plan_short_oak_stem(id, [16, 70, 0], [17, 70, 0], 4).unwrap();
        let worker = world.settlements[&id].residents.iter()
            .find(|r| r.alive && r.job == Job::Lumberjack).unwrap().id;
        world.assign_gather_worker(id, worker).unwrap();
        world.validate().unwrap();
        (world, id)
    }

    fn tree_blocks() -> BTreeMap<[i32; 3], String> {
        let mut blocks = BTreeMap::new();
        blocks.insert([0, 69, 0], "minecraft:grass_block".into());
        for y in 70..=73 {
            blocks.insert([0, y, 0], "minecraft:oak_log".into());
        }
        blocks.insert([1, 73, 0], "minecraft:oak_leaves".into());
        blocks
    }

    #[test]
    fn short_oak_requires_a_verified_root_and_reachable_height() {
        let blocks = tree_blocks();
        let observe = |pos| Some(blocks.get(&pos).cloned().unwrap_or("minecraft:air".into()));
        assert_eq!(short_oak_stem_height([0, 70, 0], observe), Some(4));
        assert_eq!(short_oak_stem_height([0, 71, 0], |pos|
            Some(blocks.get(&pos).cloned().unwrap_or("minecraft:air".into()))), None);
        let mut tall = blocks.clone();
        tall.insert([0, 74, 0], "minecraft:oak_log".into());
        assert_eq!(short_oak_stem_height([0, 70, 0], |pos|
            Some(tall.get(&pos).cloned().unwrap_or("minecraft:air".into()))), None);
    }

    #[test]
    fn actual_logs_advance_one_by_one_only_after_worker_delivery_receipt() {
        let (mut world, id) = world_with_stem();
        for cursor in 0..4_u8 {
            let job = world.settlements[&id].gather_job.as_ref().unwrap();
            assert_eq!(job.position, [16, 70 + i32::from(cursor), 0]);
            assert!(!job.issued);
            assert_eq!(world.settlements[&id].oak_stem.as_ref().unwrap().cursor, cursor);
            world.validate().unwrap();
            world.mark_gather_issued(id).unwrap();
            world.mark_gather_native_edit_confirmed(id).unwrap();
            assert!(world.mark_gather_issued(id).is_err());
            let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
            world.witness_gather_pickup(id, world_assigned).unwrap();
            world.confirm_gather(id).unwrap();
            assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood),
                       u32::from(cursor) + 1);
        }
        assert!(world.settlements[&id].oak_stem.is_none());
        assert!(world.settlements[&id].gather_job.is_none());
        assert!(world.confirm_gather(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn crash_reload_preserves_next_segment_and_does_not_reissue_prior_log() {
        let (mut world, id) = world_with_stem();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        let json = serde_json::to_string(&world).unwrap();
        let mut replay: World = serde_json::from_str(&json).unwrap();
        replay.validate().unwrap();
        assert_eq!(replay.settlements[&id].oak_stem.as_ref().unwrap().cursor, 1);
        assert_eq!(replay.settlements[&id].gather_job.as_ref().unwrap().position,
                   [16, 71, 0]);
        assert_eq!(replay.settlements[&id].warehouse_quantity(Good::Wood), 1);
        assert!(replay.confirm_gather(id).is_err());
        replay.mark_gather_issued(id).unwrap();
        replay.mark_gather_native_edit_confirmed(id).unwrap();
        assert!(replay.mark_gather_issued(id).is_err());
        let replay_assigned = replay.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        replay.witness_gather_pickup(id, replay_assigned).unwrap();
        replay.confirm_gather(id).unwrap();
        assert_eq!(replay.settlements[&id].warehouse_quantity(Good::Wood), 2);
    }

    #[test]
    fn corrupted_stem_cursor_or_missing_gather_is_rejected_on_load() {
        let (mut world, id) = world_with_stem();
        world.settlements.get_mut(&id).unwrap().oak_stem.as_mut().unwrap().cursor = 3;
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().oak_stem.as_mut().unwrap().cursor = 0;
        world.settlements.get_mut(&id).unwrap().gather_job = None;
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_world_without_new_field_still_loads() {
        let (world, id) = world_with_stem();
        let mut json = serde_json::to_value(&world).unwrap();
        json.get_mut("settlements").unwrap().as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap()
            .remove("oak_stem");
        let restored: World = serde_json::from_value(json).unwrap();
        assert!(restored.settlements[&id].oak_stem.is_none());
        restored.validate().unwrap();
    }

    #[test]
    fn warehouse_overflow_does_not_advance_stem_or_create_unpaid_log() {
        let (mut world, id) = world_with_stem();
        world.settlements.get_mut(&id).unwrap()
            .warehouse_credit(Good::Wood, 10_000_000).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let worker = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, worker).unwrap();
        let snapshot = serde_json::to_value(&world).unwrap();
        assert!(world.confirm_gather(id).is_err());
        assert_eq!(serde_json::to_value(&world).unwrap(), snapshot);
        assert_eq!(world.settlements[&id].oak_stem.as_ref().unwrap().cursor, 0);
    }

    #[test]
    fn abandoning_uncertain_segment_discards_remaining_tree_with_zero_credit() {
        let (mut world, id) = world_with_stem();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        let world_assigned = world.settlements[&id].gather_job.as_ref().unwrap().worker_id.unwrap();
        world.witness_gather_pickup(id, world_assigned).unwrap();
        world.confirm_gather(id).unwrap();
        world.mark_gather_issued(id).unwrap();
        world.mark_gather_native_edit_confirmed(id).unwrap();
        world.block_gather(id).unwrap();
        world.validate().unwrap();
        world.abandon_blocked_issued_gather(id).unwrap();
        assert!(world.settlements[&id].oak_stem.is_none());
        assert!(world.settlements[&id].gather_job.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 1);
        world.validate().unwrap();
    }

    #[test]
    fn native_followup_must_see_air_prefix_and_unchanged_leafy_crown() {
        let mut blocks = tree_blocks();
        let mut stem = OakStem {
            root: [0,70,0], stand: [1,70,0], height: 4, cursor: 0,
        };
        assert!(inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        stem.cursor = 1;
        // The next log cannot be cut until the previous one was really air.
        assert!(!inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        blocks.remove(&[0,70,0]);
        assert!(inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        blocks.remove(&[1,73,0]);
        assert!(!inspect_oak_stem(&stem, |p|
            Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        blocks.insert([1,73,0], "minecraft:oak_leaves".into());
        assert!(!inspect_oak_stem(&stem, |p|
            if p == [0,72,0] { None }
            else { Some(blocks.get(&p).cloned().unwrap_or("minecraft:air".into())) }));
    }
