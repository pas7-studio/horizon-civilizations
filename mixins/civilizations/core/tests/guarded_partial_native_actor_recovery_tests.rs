    use super::super::*;

    fn partial_spawn() -> (World, u64, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let first = world.settlements[&id].residents[0].id;
        world.settlements.get_mut(&id).unwrap().actors_spawned = true;
        world.bind_actor(id, first, 0x123456).unwrap();
        let missing = world.settlements[&id].residents[7].id;
        world.validate().unwrap();
        (world, id, missing)
    }

    #[test]
    fn cannot_recover_before_initial_spawn_is_durably_reserved() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let worker = world.settlements[&id].residents[7].id;
        assert!(world.reserve_actor_recovery(id, worker).is_err());
        assert!(world.settlements[&id].actor_recovery_issued.is_empty());
        world.validate().unwrap();
    }

    #[test]
    fn pending_recovery_can_only_be_reserved_once_even_after_reload() {
        let (mut world, id, worker) = partial_spawn();
        world.reserve_actor_recovery(id, worker).unwrap();
        assert!(world.reserve_actor_recovery(id, worker).is_err());
        let mut restored: World = serde_json::from_str(
            &serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        assert!(restored.settlements[&id].actor_recovery_issued.contains(&worker));
        assert!(restored.reserve_actor_recovery(id, worker).is_err());
    }

    #[test]
    fn native_uuid_link_clears_pending_recovery_without_new_residents() {
        let (mut world, id, worker) = partial_spawn();
        let original_count = world.settlements[&id].residents.len();
        world.reserve_actor_recovery(id, worker).unwrap();
        world.bind_actor(id, worker, 0x789abc).unwrap();
        assert!(world.settlements[&id].actor_recovery_issued.is_empty());
        assert_eq!(world.settlements[&id].residents.len(), original_count);
        assert_eq!(world.settlements[&id].residents.iter().find(
            |r| r.id == worker).unwrap().entity_uuid, Some(0x789abc));
        assert!(world.reserve_actor_recovery(id, worker).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn another_citizens_uuid_never_clears_pending_recovery() {
        let (mut world, id, worker) = partial_spawn();
        world.reserve_actor_recovery(id, worker).unwrap();
        let other = world.settlements[&id].residents[8].id;
        world.bind_actor(id, other, 0xabc).unwrap();
        assert!(world.settlements[&id].actor_recovery_issued.contains(&worker));
        world.validate().unwrap();
    }

    #[test]
    fn unknown_dead_and_already_linked_resident_ids_cannot_be_reserved() {
        let (mut world, id, missing) = partial_spawn();
        assert!(world.reserve_actor_recovery(id, u64::MAX).is_err());
        let linked = world.settlements[&id].residents[0].id;
        assert!(world.reserve_actor_recovery(id, linked).is_err());
        world.settlements.get_mut(&id).unwrap().residents.iter_mut()
            .find(|r| r.id == missing).unwrap().alive = false;
        assert!(world.reserve_actor_recovery(id, missing).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn dead_resident_with_prior_recovery_attempt_is_safe_to_reload_but_not_respawn() {
        let (mut world, id, worker) = partial_spawn();
        world.reserve_actor_recovery(id, worker).unwrap();
        world.settlements.get_mut(&id).unwrap().residents.iter_mut()
            .find(|r| r.id == worker).unwrap().alive = false;
        world.validate().unwrap();
        let mut restored: World = serde_json::from_str(
            &serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        assert!(restored.settlements[&id].actor_recovery_issued.contains(&worker));
        assert!(restored.reserve_actor_recovery(id, worker).is_err());
    }

    #[test]
    fn invalid_recovery_marker_fails_json_validation() {
        let (mut world, id, _) = partial_spawn();
        world.settlements.get_mut(&id).unwrap()
            .actor_recovery_issued.insert(u64::MAX);
        assert!(world.validate().is_err());
    }

    #[test]
    fn recovery_marker_cannot_precede_initial_actor_reservation() {
        let (mut world, id, worker) = partial_spawn();
        world.reserve_actor_recovery(id, worker).unwrap();
        world.settlements.get_mut(&id).unwrap().actors_spawned = false;
        assert!(world.validate().is_err());
    }
