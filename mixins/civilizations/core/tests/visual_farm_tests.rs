use std::collections::{BTreeSet};
    use super::super::*;

    fn plot() -> FarmPlot {
        FarmPlot { origin: [25, 72, -20], stage: FarmStage::Preparing, cursor: 0, blocked: false,
            completed_cycles: 0, harvest_sequence: 0, harvest_receipts: Vec::new() }
    }
    fn block(name: &str, age: Option<u8>) -> Option<FarmBlock> {
        Some(FarmBlock { name: name.into(), age })
    }
    fn farm_in_world() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 72, 0], "founder").unwrap();
        world.start_farm(id, [25, 72, 0]).unwrap();
        (world, id)
    }
    #[test]
    fn field_is_nine_soil_tiles_and_eight_wheat_crops() {
        let plan = farm_plan([25, 72, -20]);
        assert_eq!(plan.len(), 17);
        assert_eq!(plan.iter().filter(|s| s.block == "minecraft:farmland").count(), 8);
        assert_eq!(plan.iter().filter(|s| s.block == "minecraft:water").count(), 1);
        assert_eq!(plan.iter().filter(|s| s.block == "minecraft:wheat").count(), 8);
        let positions: BTreeSet<_> = plan.iter().map(|s| s.pos).collect();
        assert_eq!(positions.len(), plan.len());
    }
    #[test]
    fn preparing_only_replaces_natural_ground_and_air() {
        let plot = plot();
        let first = farm_plan(plot.origin)[0].pos;
        assert_eq!(inspect_farm(&plot, |_| block("minecraft:grass_block", None)),
            FarmDecision::Place { position: first, old_block: "minecraft:grass_block".into(), new_block: "minecraft:farmland" });
        assert_eq!(inspect_farm(&plot, |_| block("minecraft:chest", None)),
            FarmDecision::Blocked { position: first, found: "minecraft:chest".into() });
        assert_eq!(inspect_farm(&plot, |_| None), FarmDecision::Unloaded);
    }
    #[test]
    fn phase_changes_only_after_all_confirmed_blocks() {
        let (mut world, id) = farm_in_world();
        world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap().cursor = 17;
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.cursor, 0);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn crop_growth_uses_real_maturity_and_never_credits_stock() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Growing;
        let origin = farm.origin;
        let mature = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:wheat", Some(7)) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(mature, FarmDecision::StageComplete);
        let immature = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:wheat", Some(4)) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(immature, FarmDecision::Paused);
        world.advance_farm(id, mature).unwrap();
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().stage, FarmStage::Harvesting);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn harvesting_is_conditional_and_never_drops_items_or_rewinds_stock() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Harvesting;
        let origin = farm.origin;
        let expected = farm_plan(origin)[9].pos;
        let result = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:wheat", Some(7)) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(result, FarmDecision::Place {
            position: expected, old_block: "minecraft:wheat[age=7]".into(), new_block: "minecraft:air",
        });
        assert!(!world.advance_farm(id, result).unwrap());
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn interrupted_harvest_rewinds_without_minting_wheat() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Harvesting;
        farm.cursor = 3;
        let origin = farm.origin;
        let first = farm_plan(origin)[9].pos;
        let decision = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] {
                if pos == first { block("minecraft:wheat", Some(7)) }
                else { block("minecraft:air", None) }
            } else if pos[0] == origin[0] && pos[2] == origin[2] {
                block("minecraft:water", None)
            } else { block("minecraft:farmland", None) }
        });
        assert_eq!(decision, FarmDecision::Rewind(0));
        world.advance_farm(id, decision).unwrap();
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().cursor, 0);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn replants_then_restarts_growing_idempotently() {
        let (mut world, id) = farm_in_world();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.stage = FarmStage::Replanting;
        let origin = farm.origin;
        let first = farm_plan(origin)[9].pos;
        let decision = inspect_farm(farm, |pos| {
            if pos[1] == origin[1] { block("minecraft:air", None) }
            else if pos[0] == origin[0] && pos[2] == origin[2] { block("minecraft:water", None) }
            else { block("minecraft:farmland", None) }
        });
        assert_eq!(decision, FarmDecision::Place {
            position: first, old_block: "minecraft:air".into(), new_block: "minecraft:wheat",
        });
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.cursor = 8;
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.completed_cycles, 1);
        assert_eq!(world.settlements[&id].quantity(Good::Wheat), 20);
    }
    #[test]
    fn farm_is_persisted_and_legacy_without_farm_loads() {
        let (world, id) = farm_in_world();
        let json = serde_json::to_string(&world).unwrap();
        let decoded: World = serde_json::from_str(&json).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.settlements[&id].farm.as_ref().unwrap().origin, [25, 72, 0]);
        let mut data: serde_json::Value = serde_json::from_str(&json).unwrap();
        data["settlements"][id.to_string()].as_object_mut().unwrap().remove("farm");
        let legacy: World = serde_json::from_value(data).unwrap();
        legacy.validate().unwrap();
        assert!(legacy.settlements[&id].farm.is_none());
    }
    #[test]
    fn site_collision_and_wrong_dimensions_are_rejected() {
        let (mut world, id) = farm_in_world();
        assert!(world.start_farm(id, [60, 72, 0]).is_err());
        let second = world.found("Brighthold", "minecraft:overworld", [300, 72, 0], "other").unwrap();
        assert!(world.start_farm(second, [25, 72, 0]).is_err());
        world.start_farm(second, [325, 72, 0]).unwrap();
        assert!(world.validate().is_ok());
    }
