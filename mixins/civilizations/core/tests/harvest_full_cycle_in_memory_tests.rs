use std::collections::{BTreeMap};
    use super::super::*;

    fn plot_world() -> (World, u64, BTreeMap<[i32; 3], FarmBlock>) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        world.start_farm(id, [20, 70, 0]).unwrap();
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        let mut blocks = BTreeMap::new();
        for step in farm_plan(farm.origin) {
            let name = if step.floor { "minecraft:dirt" } else { "minecraft:air" };
            blocks.insert(step.pos, FarmBlock { name: name.into(), age: None });
        }
        (world, id, blocks)
    }

    fn run_until_waiting(
        world: &mut World,
        village_id: u64,
        blocks: &mut BTreeMap<[i32; 3], FarmBlock>,
    ) {
        for _ in 0..200 {
            let snapshot = world.settlements[&village_id].farm.as_ref().unwrap().clone();
            let decision = inspect_farm(&snapshot, |pos| blocks.get(&pos).cloned());
            match decision {
                FarmDecision::Place { position, new_block, .. } => {
                    // Mock the Horizon command without immediately advancing JSON.
                    blocks.insert(position, FarmBlock {
                        name: new_block.into(),
                        age: (new_block == "minecraft:wheat").then_some(0),
                    });
                }
                FarmDecision::Paused => return,
                FarmDecision::Unloaded | FarmDecision::Blocked { .. }
                | FarmDecision::InvalidCursor => panic!("valid farm unexpectedly stopped: {:?}", decision),
                other => {
                    world.advance_farm(village_id, other).unwrap();
                }
            }
        }
        panic!("farm did not become idle within bounded work");
    }

    #[test]
    fn complete_wheat_cycle_tracks_real_blocks_and_one_audit_receipt() {
        let (mut world, id, mut blocks) = plot_world();
        let original_stock = world.settlements[&id].stock.clone();

        // Simulate a block committed to a chunk, followed by a crash before
        // the world.json cursor is committed. On reload it must be observed.
        let snapshot = world.settlements[&id].farm.as_ref().unwrap().clone();
        let first = inspect_farm(&snapshot, |pos| blocks.get(&pos).cloned());
        let FarmDecision::Place { position, new_block, .. } = first else {
            panic!("expected first terrain step");
        };
        blocks.insert(position, FarmBlock { name: new_block.into(), age: None });
        world = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();

        run_until_waiting(&mut world, id, &mut blocks);
        assert_eq!(world.settlements[&id].farm.as_ref().unwrap().stage, FarmStage::Growing);
        let plan = farm_plan(world.settlements[&id].farm.as_ref().unwrap().origin);
        for step in plan.iter().skip(9) {
            let wheat = blocks.get_mut(&step.pos).unwrap();
            assert_eq!(wheat.name, "minecraft:wheat");
            assert_eq!(wheat.age, Some(0));
            wheat.age = Some(7);
        }
        run_until_waiting(&mut world, id, &mut blocks);
        let farm = world.settlements[&id].farm.as_ref().unwrap();
        assert_eq!(farm.stage, FarmStage::Growing);
        assert_eq!(farm.completed_cycles, 1);
        assert_eq!(farm.harvest_sequence, 1);
        assert_eq!(farm.harvest_receipts.len(), 1);
        assert_eq!(world.settlements[&id].stock, original_stock);
        for step in plan.iter().skip(9) {
            let planted = blocks.get(&step.pos).unwrap();
            assert_eq!(planted.name, "minecraft:wheat");
            assert_eq!(planted.age, Some(0));
        }
        world.validate().unwrap();
        let loaded: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.settlements[&id].farm.as_ref().unwrap().harvest_sequence, 1);
    }
