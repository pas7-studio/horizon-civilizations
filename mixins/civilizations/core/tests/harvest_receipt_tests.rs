    use super::super::*;

    fn harvest_world() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        (world, id)
    }

    #[test]
    fn full_harvest_records_once_and_never_mints_stock() {
        let (mut world, id) = harvest_world();
        let prior_stock = world.settlements[&id].stock.clone();
        {
            let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
            farm.stage = FarmStage::Harvesting;
            farm.cursor = 8;
        }
        world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Replanting);
        assert_eq!(farm.harvest_sequence, 1);
        assert_eq!(farm.harvest_receipts, vec![HarvestReceipt {
            sequence: 1, world_day: 0, crops: 8,
        }]);
        assert_eq!(world.settlements[&id].stock, prior_stock);
        // Another callback cannot record the same completed harvest again.
        assert!(world.advance_farm(id, FarmDecision::StageComplete).is_err());
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().harvest_sequence, 1);
        world.validate().unwrap();
        let decoded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.settlements[&id].farm.as_ref().unwrap().harvest_receipts.len(), 1);
    }

    #[test]
    fn receipt_ring_is_bounded_and_rejects_duplicate_or_invalid_evidence() {
        let (mut world, id) = harvest_world();
        for _ in 0..MAX_HARVEST_RECEIPTS + 3 {
            let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
            farm.stage = FarmStage::Harvesting;
            farm.cursor = 8;
            world.advance_farm(id, FarmDecision::StageComplete).unwrap();
        }
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.harvest_sequence, (MAX_HARVEST_RECEIPTS + 3) as u64);
        assert_eq!(farm.harvest_receipts.len(), MAX_HARVEST_RECEIPTS);
        assert_eq!(farm.harvest_receipts[0].sequence, 4);
        world.validate().unwrap();
        let farm = world.settlements.get_mut(&id).unwrap().farm.as_mut().unwrap();
        farm.harvest_receipts[1].sequence = farm.harvest_receipts[0].sequence;
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_farm_missing_audit_fields_loads_with_no_invented_harvests() {
        let (world, id) = harvest_world();
        let mut raw: serde_json::Value = serde_json::to_value(&world).unwrap();
        let map = raw["settlements"][id.to_string()]["farm"].as_object_mut().unwrap();
        map.remove("harvest_sequence");
        map.remove("harvest_receipts");
        let loaded: World = serde_json::from_value(raw).unwrap();
        loaded.validate().unwrap();
        let farm = loaded.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.harvest_sequence, 0);
        assert!(farm.harvest_receipts.is_empty());
    }
