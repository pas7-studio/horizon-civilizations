    use super::super::*;

    #[test]
    fn incomplete_phase_cannot_commit_or_overrun_cursor() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        assert!(world.advance_farm(id, FarmDecision::StageComplete).is_err());
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().stage, FarmStage::Preparing);
        world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap().cursor = 17;
        assert!(world.advance_farm(id, FarmDecision::Advance).is_err());
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.cursor, 0);
        world.validate().unwrap();
    }
