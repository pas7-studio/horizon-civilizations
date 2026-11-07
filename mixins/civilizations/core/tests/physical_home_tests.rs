    use super::super::*;

    fn village() -> World {
        let mut world = World::default();
        world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        world
    }

    #[test]
    fn abstract_or_pending_houses_are_never_allocated_as_real_homes() {
        let mut world = village();
        let settlement = world.settlements.get_mut(&1).unwrap();
        settlement.houses = 5;
        settlement.pending_visual_buildings = 5;
        assert_eq!(settlement.physical_housing(), (0, 0, 10));
        for resident in &settlement.residents {
            assert_eq!(settlement.home_slot(resident.id), None);
        }
        world.validate().unwrap();
    }

    #[test]
    fn completed_huts_allocate_three_spots_in_immutable_site_order() {
        let mut world = village();
        let settlement = world.settlements.get_mut(&1).unwrap();
        settlement.houses = 3;
        settlement.visual_sites.push([20, 70, 10]);
        let mut ids: Vec<u64> = settlement.residents.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        assert_eq!(settlement.physical_housing(), (3, 3, 7));
        for (lane, id) in ids.iter().take(3).enumerate() {
            assert_eq!(settlement.home_slot(*id), Some(HomeSlot {
                site: [20, 70, 10], lane: lane as u8
            }));
        }
        assert_eq!(settlement.home_slot(ids[3]), None);
        let previous: Vec<_> = ids.iter().take(3)
            .map(|id| settlement.home_slot(*id)).collect();
        settlement.visual_sites.push([40, 70, 10]);
        assert_eq!(settlement.physical_housing(), (6, 6, 4));
        for (index, id) in ids.iter().enumerate().take(6) {
            let site = if index < 3 { [20, 70, 10] } else { [40, 70, 10] };
            assert_eq!(settlement.home_slot(*id), Some(HomeSlot {
                site, lane: (index % 3) as u8
            }));
        }
        settlement.residents.reverse();
        assert_eq!(previous, ids.iter().take(3)
            .map(|id| settlement.home_slot(*id)).collect::<Vec<_>>());
        world.validate().unwrap();
        let encoded = serde_json::to_string(&world).unwrap();
        assert!(!encoded.contains("home_slot"));
        let loaded: World = serde_json::from_str(&encoded).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.settlements[&1].home_slot(ids[0]), previous[0]);
    }

    #[test]
    fn home_inspection_is_fail_closed_for_unloaded_or_modified_blocks() {
        use std::collections::BTreeMap;
        let home = HomeSlot { site: [20, 70, 10], lane: 0 };
        let [x, y, z] = home.foot();
        let [dx, dy, dz] = home.doorway();
        assert_eq!([x, y, z], [19, 70, 9]);
        assert_eq!([dx, dy, dz], [20, 70, 8]);
        let mut world = BTreeMap::from([
            ([x, y - 1, z], "minecraft:cobblestone".to_owned()),
            ([x, y, z], "minecraft:air".to_owned()),
            ([x, y + 1, z], "minecraft:air".to_owned()),
            ([dx, dy - 1, dz], "minecraft:cobblestone".to_owned()),
            ([dx, dy, dz], "minecraft:air".to_owned()),
            ([dx, dy + 1, dz], "minecraft:air".to_owned()),
        ]);
        assert!(inspect_home(home, |p| world.get(&p).cloned()));
        world.insert([x, y, z], "minecraft:chest".into());
        assert!(!inspect_home(home, |p| world.get(&p).cloned()));
        world.remove(&[x, y, z]);
        assert!(!inspect_home(home, |p| world.get(&p).cloned()));
        assert!(!inspect_home(home, |_| None));
        assert!(!inspect_home(HomeSlot { lane: 3, ..home }, |_| {
            Some("minecraft:air".into())
        }));
    }

    #[test]
    fn housing_rebalances_on_death_but_never_targets_dead_residents() {
        let mut world = village();
        let village = world.settlements.get_mut(&1).unwrap();
        village.houses = 1;
        village.visual_sites.push([20, 70, 10]);
        let mut ids: Vec<u64> = village.residents.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        let index = village.residents.iter().position(|r| r.id == ids[0]).unwrap();
        village.residents[index].alive = false;
        assert_eq!(village.home_slot(ids[0]), None);
        assert_eq!(village.home_slot(ids[1]).unwrap().lane, 0);
        assert_eq!(village.physical_housing(), (3, 3, 6));
        world.validate().unwrap();
    }
