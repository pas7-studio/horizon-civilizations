use std::collections::{BTreeSet};
    use super::super::*;

    fn world_with_ready_house() -> World {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        world.settlements.get_mut(&id).unwrap().pending_visual_buildings = 1;
        world.validate().unwrap();
        world
    }

    #[test]
    fn candidates_are_unique_bounded_stable_and_never_reserve_a_plot() {
        let world = world_with_ready_house();
        let mut unique = BTreeSet::new();
        for slot in 1..=HOUSING_PLOT_OFFSETS.len() {
            let origin = world.proposed_hut(1, slot, 70).unwrap();
            assert!(unique.insert(origin));
            assert_eq!(origin, world.proposed_hut(1, slot, 70).unwrap());
        }
        assert_eq!(world.proposed_hut(1, 1, 70).unwrap(), [20, 70, 0]);
        assert!(world.proposed_hut(1, 0, 70).is_err());
        assert!(world.proposed_hut(1, HOUSING_PLOT_OFFSETS.len() + 1, 70).is_err());
        assert!(world.settlements[&1].visual_job.is_none());
        assert!(world.settlements[&1].visual_sites.is_empty());
        world.validate().unwrap();
    }

    #[test]
    fn requires_existing_abstract_build_budget_and_real_housing_demand() {
        let mut world = world_with_ready_house();
        world.settlements.get_mut(&1).unwrap().pending_visual_buildings = 0;
        assert!(world.proposed_hut(1, 1, 70).is_err());
        world.settlements.get_mut(&1).unwrap().pending_visual_buildings = 1;
        {
            let village = world.settlements.get_mut(&1).unwrap();
            village.houses = 5;
            village.visual_sites.extend([
                [20, 70, 0], [-20, 70, 0], [0, 70, 20], [0, 70, -20],
            ]);
            assert_eq!(village.physical_housing(), (12, 10, 0));
        }
        assert!(world.proposed_hut(1, 1, 70).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn completed_huts_farms_and_jobs_reject_overlaps_for_manual_and_proposed_paths() {
        let mut world = world_with_ready_house();
        world.start_farm(1, [20, 70, 0]).unwrap();
        assert!(world.proposed_hut(1, 1, 70).is_err());
        assert!(world.start_visual_house(1, [20, 70, 0]).is_err());
        let second = world.proposed_hut(1, 2, 70).unwrap();
        world.start_visual_house(1, second).unwrap();
        assert!(world.proposed_hut(1, 3, 70).is_err());
        assert!(world.start_visual_house(1, [0, 70, 20]).is_err());
        world.validate().unwrap();

        let mut world = world_with_ready_house();
        world.settlements.get_mut(&1).unwrap().visual_sites.push([20, 70, 0]);
        assert!(world.start_visual_house(1, [21, 70, 0]).is_err());
        assert!(world.proposed_hut(1, 1, 70).is_err());
    }

    #[test]
    fn other_settlement_structures_are_respected_within_same_dimension_only() {
        let mut world = world_with_ready_house();
        let other = world.found("Birch", "minecraft:overworld", [220, 70, 0], "op").unwrap();
        {
            let village = world.settlements.get_mut(&other).unwrap();
            village.visual_sites.push([20, 70, 0]);
        }
        assert!(world.proposed_hut(1, 1, 70).is_err());
        world.settlements.get_mut(&other).unwrap().visual_sites.clear();

        let distant = world.found("Nether", "minecraft:the_nether", [0, 70, 0], "op").unwrap();
        world.settlements.get_mut(&distant).unwrap().visual_sites.push([20, 70, 0]);
        assert_eq!(world.proposed_hut(1, 1, 70).unwrap(), [20, 70, 0]);
    }

    #[test]
    fn extreme_coordinates_and_height_cannot_overflow_or_escape_world() {
        let mut world = World::default();
        let id = world.found("Border", "minecraft:overworld",
                             [29_999_990, 70, 0], "op").unwrap();
        world.settlements.get_mut(&id).unwrap().pending_visual_buildings = 1;
        assert!(world.proposed_hut(id, 1, 70).is_err());
        assert!(world.proposed_hut(id, 2, 70).is_ok());
        assert!(world.proposed_hut(id, 2, i32::MAX).is_err());
        assert!(world.proposed_hut(id, 2, i32::MIN).is_err());

        // The farthest possible signed i32 coordinates exceed i64's
        // *combined* squared-distance range. This must fail, not panic.
        let mut distant = World::default();
        let other = distant.found("Distant", "minecraft:overworld",
                                  [i32::MIN, 70, i32::MIN], "op").unwrap();
        distant.settlements.get_mut(&other).unwrap().pending_visual_buildings = 1;
        assert!(distant.check_visual_house_site(other,
            [29_999_990, 70, 29_999_990]).is_err());
    }
