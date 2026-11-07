    use super::super::*;

    fn village() -> (World, u64) {
        let mut world = World::default();
        let id = world
            .found("Ashford", "minecraft:overworld", [0, 70, 0], "owner")
            .unwrap();
        (world, id)
    }

    #[test]
    fn camps_are_bounded_unique_and_need_real_distance() {
        let (mut world, id) = village();
        assert!(world.add_post(id, PostKind::Lumber, [2, 70, 0]).is_err());
        world.add_post(id, PostKind::Lumber, [20, 70, 0]).unwrap();
        assert!(world.add_post(id, PostKind::Lumber, [24, 70, 0]).is_err());
        assert!(world.add_post(id, PostKind::Quarry, [20, 70, 0]).is_err());
        world.add_post(id, PostKind::Quarry, [-24, 70, 0]).unwrap();
        assert_eq!(world.post_origin(id, PostKind::Lumber), Some([20, 70, 0]));
        world.validate().unwrap();
    }

    #[test]
    fn a_camp_binds_only_the_matching_living_resident_once() {
        let (mut world, id) = village();
        world.add_post(id, PostKind::Lumber, [20, 70, 0]).unwrap();
        let lumberjack = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Lumberjack).unwrap().id;
        let baker = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Baker).unwrap().id;
        assert!(world.set_post_worker(id, PostKind::Lumber, baker).is_err());
        assert!(world.set_post_worker(id, PostKind::Lumber, lumberjack).unwrap());
        assert!(!world.set_post_worker(id, PostKind::Lumber, lumberjack).unwrap());
        let other = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Lumberjack && r.id != lumberjack);
        assert!(other.is_none());
        world.validate().unwrap();
    }

    #[test]
    fn corrupted_camp_worker_or_site_fails_closed_on_load() {
        let (mut world, id) = village();
        world.add_post(id, PostKind::Quarry, [0, 70, 24]).unwrap();
        world.settlements.get_mut(&id).unwrap().posts[0].worker_id = Some(u64::MAX);
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().posts[0].worker_id = None;
        world.settlements.get_mut(&id).unwrap().posts[0].origin = [0, 70, 1];
        world.validate().unwrap();
        world.settlements.get_mut(&id).unwrap().posts.push(WorkPost {
            kind: PostKind::Lumber,
            origin: [0, 70, 1],
            worker_id: None,
        });
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_save_without_camps_still_loads() {
        let (mut world, id) = village();
        world.add_post(id, PostKind::Lumber, [20, 70, 0]).unwrap();
        let mut value = serde_json::to_value(&world).unwrap();
        value["settlements"][id.to_string()].as_object_mut().unwrap().remove("posts");
        let restored: World = serde_json::from_value(value).unwrap();
        assert!(restored.settlements[&id].posts.is_empty());
        restored.validate().unwrap();
    }
