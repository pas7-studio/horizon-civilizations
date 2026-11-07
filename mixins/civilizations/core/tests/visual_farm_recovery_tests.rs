    use super::super::*;

    #[test]
    fn recovery_requires_a_blocked_plot_and_preserves_stock() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        assert!(world.recover_farm(id).is_err());
        let initial_stock = world.settlements[&id].stock.clone();
        world.advance_farm(id, FarmDecision::Blocked {
            position: [20, 69, 0],
            found: "minecraft:chest".into(),
        }).unwrap();
        assert!(world.settlements[&id].farm.as_ref().unwrap().blocked);
        world.recover_farm(id).unwrap();
        assert!(!world.settlements[&id].farm.as_ref().unwrap().blocked);
        assert_eq!(world.settlements[&id].stock, initial_stock);
        world.validate().unwrap();
        let decoded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert!(!decoded.settlements[&id].farm.as_ref().unwrap().blocked);
    }
