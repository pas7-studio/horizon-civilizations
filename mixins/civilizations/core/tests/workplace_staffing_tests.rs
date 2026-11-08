    use super::super::*;

    fn village() -> (World, u64) {
        let mut world = World::default();
        let id = world
            .found("Ashford", "minecraft:overworld", [0, 70, 0], "owner")
            .unwrap();
        (world, id)
    }

    #[test]
    fn nobody_holds_a_role_without_a_real_workplace() {
        let (mut world, id) = village();
        let v = world.settlements.get_mut(&id).unwrap();
        let changed = v.staff_workplaces();
        assert_eq!(changed.len(), 6);
        // No farm plot and no bakery exist yet, so no farmer or baker may be
        // claimed. Missing wood and stone still justify one camp worker each.
        assert_eq!(v.count(Job::Farmer), 0);
        assert_eq!(v.count(Job::Baker), 0);
        assert_eq!(v.count(Job::Merchant), 0);
        assert_eq!(v.count(Job::Guard), 1);
        assert_eq!(v.count(Job::Builder), 1);
        assert_eq!(v.count(Job::Lumberjack), 1);
        assert_eq!(v.count(Job::Miner), 1);
        assert_eq!(v.count(Job::Laborer), 6);
        assert_eq!(v.population(), 10);
        world.validate().unwrap();
    }

    #[test]
    fn a_finished_stocked_village_releases_both_camp_seats() {
        let (mut world, id) = village();
        let v = world.settlements.get_mut(&id).unwrap();
        v.hall_complete = true;
        v.pending_visual_buildings = 0;
        v.warehouse.insert(Good::Wood, HALL_WOOD_LOGS + 4);
        v.warehouse.insert(Good::Stone, HALL_COBBLESTONE + 4);
        v.staff_workplaces();
        assert_eq!(v.count(Job::Lumberjack), 0);
        assert_eq!(v.count(Job::Miner), 0);
        assert_eq!(v.count(Job::Merchant), 1);
        assert_eq!(v.count(Job::Guard), 1);
        assert_eq!(v.count(Job::Builder), 1);
        assert_eq!(v.count(Job::Laborer), 7);
        world.validate().unwrap();
    }

    #[test]
    fn a_pending_build_keeps_one_camp_seat_each_even_with_full_barrels() {
        let (mut world, id) = village();
        let v = world.settlements.get_mut(&id).unwrap();
        v.hall_complete = true;
        v.pending_visual_buildings = 1;
        v.warehouse.insert(Good::Wood, HALL_WOOD_LOGS + 4);
        v.warehouse.insert(Good::Stone, HALL_COBBLESTONE + 4);
        v.staff_workplaces();
        assert_eq!(v.count(Job::Lumberjack), 1);
        assert_eq!(v.count(Job::Miner), 1);
        world.validate().unwrap();
    }

    #[test]
    fn a_dirt_or_sand_pit_hires_one_digger_and_plans_its_gather() {
        let (mut world, id) = village();
        world.add_post(id, PostKind::Sand, [-24, 70, 0]).unwrap();
        world.add_post(id, PostKind::Dirt, [0, 70, 24]).unwrap();
        let v = world.settlements.get_mut(&id).unwrap();
        // Both pits share one digger seat; they never take a miner seat.
        assert_eq!(v.workplace_demand().get(&Job::Digger), Some(&1));
        v.staff_workplaces();
        assert_eq!(v.count(Job::Digger), 1);
        assert_eq!(v.count(Job::Miner), 1);
        let digger = v.residents.iter()
            .find(|r| r.job == Job::Digger).unwrap().id;
        assert!(world.set_post_worker(id, PostKind::Sand, digger).unwrap());
        world.plan_gather(id, Good::Sand, [20, 70, 0], [20, 72, 0]).unwrap();
        let job = world.settlements[&id].gather_job.clone().unwrap();
        assert_eq!(job.good, Good::Sand);
        assert_eq!(job.source_block(), "minecraft:sand");
        assert_eq!(Good::Sand.job(), Some(Job::Digger));
        assert_eq!(Good::Dirt.job(), Some(Job::Digger));
        assert_eq!(Good::Sand.item(), "minecraft:sand");
        assert_eq!(Good::Dirt.item(), "minecraft:dirt");
        assert_eq!(PostKind::for_good(Good::Sand), PostKind::Sand);
        world.validate().unwrap();
    }

    #[test]
    fn a_quarry_pit_is_bounded_and_needs_its_camp() {
        let (mut world, id) = village();
        assert!(world.advance_quarry(id).is_err());
        world.add_post(id, PostKind::Quarry, [-24, 70, 0]).unwrap();
        for step in 1..=u8::from(super::MAX_QUARRY_STEPS) {
            world.advance_quarry(id).unwrap();
            assert_eq!(world.settlements[&id].quarry_step, step);
        }
        assert!(world.advance_quarry(id).is_err());
        assert_eq!(world.settlements[&id].quarry_step, super::MAX_QUARRY_STEPS);
        world.validate().unwrap();
        world.settlements.get_mut(&id).unwrap().quarry_step = super::MAX_QUARRY_STEPS + 1;
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_save_without_a_pit_loads_with_zero_depth() {
        let (mut world, id) = village();
        world.add_post(id, PostKind::Quarry, [-24, 70, 0]).unwrap();
        let mut value = serde_json::to_value(&world).unwrap();
        value["settlements"][id.to_string()].as_object_mut().unwrap().remove("quarry_step");
        let restored: World = serde_json::from_value(value).unwrap();
        assert_eq!(restored.settlements[&id].quarry_step, 0);
        restored.validate().unwrap();
    }

    #[test]
    fn a_quarry_plans_its_own_pit_but_not_the_countryside() {
        let (mut world, id) = village();
        world.add_post(id, PostKind::Quarry, [-24, 70, 0]).unwrap();
        // Inside the yard and in the outer work area.
        world.plan_quarry_gather(id, [-24, 70, 0], [-23, 70, 0]).unwrap();
        assert_eq!(world.settlements[&id].gather_job.as_ref().unwrap().good, Good::Stone);
        assert!(world.gather_claim_ok(id, Good::Stone, [-24, 70, 0], [-23, 70, 0]));
        world.settlements.get_mut(&id).unwrap().gather_job = None;
        // Far outside the yard is refused even though the area is natural.
        assert!(world.plan_quarry_gather(id, [40, 70, 0], [41, 70, 0]).is_err());
        assert!(!world.gather_claim_ok(id, Good::Wood, [0, 70, 0], [1, 70, 0]));
        // Another gather may not be pending.
        world.plan_quarry_gather(id, [-24, 70, 0], [-23, 70, 0]).unwrap();
        assert!(world.plan_quarry_gather(id, [-24, 70, 1], [-23, 70, 1]).is_err());
        world.validate().unwrap();
        // Without a quarry the pit path does not exist.
        let (mut other, second) = village();
        assert!(other.plan_quarry_gather(second, [20, 70, 0], [21, 70, 0]).is_err());
    }

    #[test]
    fn staffing_is_stable_and_a_camp_keeps_its_worker() {        let (mut world, id) = village();
        world.settlements.get_mut(&id).unwrap().staff_workplaces();
        let lumberjack = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Lumberjack).unwrap().id;
        world.add_post(id, PostKind::Lumber, [20, 70, 0]).unwrap();
        assert!(world.set_post_worker(id, PostKind::Lumber, lumberjack).unwrap());
        // A second pass changes nothing and never rebinds the camp.
        assert!(world.settlements.get_mut(&id).unwrap().staff_workplaces().is_empty());
        assert_eq!(world.settlements[&id].posts[0].worker_id, Some(lumberjack));
        world.validate().unwrap();
    }

    #[test]
    fn a_dead_camp_worker_leaves_no_fake_role_or_binding() {
        let (mut world, id) = village();
        world.settlements.get_mut(&id).unwrap().staff_workplaces();
        let lumberjack = world.settlements[&id].residents.iter()
            .find(|r| r.job == Job::Lumberjack).unwrap().id;
        world.add_post(id, PostKind::Lumber, [20, 70, 0]).unwrap();
        world.set_post_worker(id, PostKind::Lumber, lumberjack).unwrap();
        world.settlements.get_mut(&id).unwrap().residents.iter_mut()
            .find(|r| r.id == lumberjack).unwrap().alive = false;
        let changed = world.settlements.get_mut(&id).unwrap().staff_workplaces();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].1, Job::Lumberjack);
        assert_eq!(world.settlements[&id].posts[0].worker_id, None);
        assert_eq!(world.settlements[&id].count(Job::Lumberjack), 1);
        world.validate().unwrap();
    }
