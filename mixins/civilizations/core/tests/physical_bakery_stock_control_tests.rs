use std::collections::{BTreeMap};
    use super::super::*;

    #[test]
    fn physical_bread_buffer_is_bounded_by_actual_residents() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let village = &world.settlements[&id];
        assert_eq!(physical_bread_target(village), village.population() * 2);
        assert!(needs_real_bread(village, 0));
        assert!(!needs_real_bread(village, physical_bread_target(village)));
    }

    #[test]
    fn physical_bread_stock_and_incoming_delivery_are_not_counted_twice() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let village = world.settlements.get_mut(&id).unwrap();
        let target = physical_bread_target(village);
        village.warehouse_credit(Good::Bread, 4).unwrap();
        assert!(needs_real_bread(village, target - 5));
        assert!(!needs_real_bread(village, target - 4));
        assert!(needs_real_bread(village, 0));
    }

    #[test]
    fn baker_in_transit_bread_counts_before_restocking() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let village = world.settlements.get_mut(&id).unwrap();
        village.bakery = Some(BakeryWorkshop {
            origin: [24, 70, 0], building: None, complete: true,
            supplies_reserved: true, construction_escrow: BTreeMap::new(),
            wheat_escrow: 0, bread_in_transit: 2, batches: 2,
        });
        let target = physical_bread_target(village);
        assert!(!needs_real_bread(village, target - 2));
        assert!(needs_real_bread(village, target - 3));
    }
