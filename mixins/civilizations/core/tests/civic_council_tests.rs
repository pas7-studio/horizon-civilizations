    use super::super::*;

    fn founded() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "op").unwrap();
        (world, id)
    }

    #[test]
    fn new_village_has_unspawned_identity_and_unbuilt_housing_needs() {
        let (world, id) = founded();
        let needs = world.settlements[&id].council_priorities();
        assert_eq!(needs[0].need, CivicNeed::Identity);
        assert!(needs.iter().any(|p| p.need == CivicNeed::Housing));
        assert!(needs.iter().all(|p| p.urgency <= 100));
    }

    #[test]
    fn hunger_crisis_outranks_theoretical_housing_and_follows_existing_economy() {
        let (mut world, id) = founded();
        let settlement = world.settlements.get_mut(&id).unwrap();
        settlement.starving_days = 3;
        let priorities = settlement.council_priorities();
        assert_eq!(priorities[0].need, CivicNeed::Food);
        assert_eq!(priorities[0].urgency, 100);
        assert!(priorities[0].reason.contains("3 day(s)"));
    }

    #[test]
    fn economic_forecast_is_bounded_and_never_mutates_real_stock_or_world() {
        let (world, id) = founded();
        let before = serde_json::to_string(&world).unwrap();
        let village = &world.settlements[&id];
        assert!(village.forecast_abstract_days(0).is_empty());
        let three = village.forecast_abstract_days(COUNCIL_FORECAST_DAYS);
        assert_eq!(three.len(), 3);
        assert_eq!(three[0].offset, 1);
        assert_eq!(three[2].offset, 3);
        assert_eq!(three, village.forecast_abstract_days(3));
        assert_eq!(village.forecast_abstract_days(u8::MAX).len(), 7);
        assert!(three[0].houses >= village.houses);
        assert_eq!(serde_json::to_string(&world).unwrap(), before);
    }

    #[test]
    fn forecast_detects_no_bakers_before_actual_starvation() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.stock.insert(Good::Bread, 0);
        for person in &mut village.residents {
            if person.job == Job::Baker { person.alive = false; }
        }
        let priorities = village.council_priorities();
        let food = priorities.iter().find(|p| p.need == CivicNeed::Food).unwrap();
        assert_eq!(food.urgency, 90);
        assert!(food.reason.contains("day +1"));
        assert_eq!(village.quantity(Good::Bread), 0);
    }

    #[test]
    fn war_priority_is_abstract_security_only_no_automatic_combat() {
        let (mut world, a) = founded();
        let b = world.found("Brightwell", "minecraft:overworld", [300, 70, 0], "op2").unwrap();
        world.declare_war(a, b).unwrap();
        let priorities = world.settlements[&a].council_priorities();
        assert_eq!(priorities[0].need, CivicNeed::Security);
        assert_eq!(priorities[0].urgency, 95);
        assert!(priorities[0].next_step.contains("operator-controlled"));
    }

    #[test]
    fn materials_are_for_future_abstract_houses_not_physical_stock() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.stock.insert(Good::Wood, 1);
        village.stock.insert(Good::Stone, 2);
        let priorities = village.council_priorities();
        let materials = priorities.iter().find(|p| p.need == CivicNeed::Materials).unwrap();
        assert_eq!(materials.urgency, 60);
        assert!(materials.reason.contains("8 wood/6 stone"));
        village.houses = village.population().div_ceil(3) + 1;
        assert!(!village.council_priorities().iter().any(|p| p.need == CivicNeed::Materials));
    }

    #[test]
    fn partial_spawn_reports_identity_repair_not_a_new_spawn() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.actors_spawned = true;
        village.residents[0].entity_uuid = Some(9);
        let priorities = village.council_priorities();
        let identity = priorities.iter().find(|p| p.need == CivicNeed::Identity).unwrap();
        assert_eq!(identity.urgency, 88);
        assert!(identity.next_step.contains("never spawn blindly"));
        world.validate().unwrap();
    }

    #[test]
    fn fixed_tie_order_and_no_needs_are_deterministic_without_save_fields() {
        let (mut world, id) = founded();
        let village = world.settlements.get_mut(&id).unwrap();
        village.houses = 5;
        village.visual_sites = vec![[20, 70, 0], [-20, 70, 0], [0, 70, 20], [0, 70, -20]];
        village.actors_spawned = true;
        for (index, resident) in village.residents.iter_mut().enumerate() {
            resident.entity_uuid = Some(index as u128 + 1);
        }
        assert_eq!(village.physical_housing(), (12, 10, 0));
        assert!(village.council_priorities().is_empty());
        let original = serde_json::to_string(&world).unwrap();
        assert!(!original.contains("council_priorities"));
        assert!(!original.contains("forecast_abstract_days"));
        world.validate().unwrap();
        assert_eq!(serde_json::to_string(&world).unwrap(), original);
        let recovered: World = serde_json::from_str(&original).unwrap();
        assert!(recovered.settlements[&id].council_priorities().is_empty());
    }
