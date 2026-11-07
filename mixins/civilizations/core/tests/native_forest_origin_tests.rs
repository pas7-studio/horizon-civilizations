use std::collections::{BTreeMap};
    use super::super::*;

    fn growing_tree() -> BTreeMap<[i32; 3], String> {
        let mut blocks = BTreeMap::new();
        blocks.insert([0, 69, 0], "minecraft:grass_block".into());
        for y in 70..=73 {
            blocks.insert([0, y, 0], "minecraft:oak_log".into());
        }
        blocks.insert([1, 73, 0], "minecraft:oak_leaves".into());
        blocks
    }

    #[test]
    fn real_rooted_oak_qualifies_at_base_not_upper_trunk() {
        let b = growing_tree();
        let read = |p| Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()));
        assert!(is_rooted_oak_harvest_origin([0, 70, 0], read));
        assert!(!is_rooted_oak_harvest_origin([0, 71, 0], |p|
            Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()))));
    }

    #[test]
    fn decorated_timber_and_disconnected_leaves_are_not_forestry_receipts() {
        let mut b = growing_tree();
        b.remove(&[0, 72, 0]);
        assert!(!is_rooted_oak_harvest_origin([0, 70, 0], |p|
            Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()))));
        b.insert([0, 72, 0], "minecraft:oak_log".into());
        b.insert([0, 69, 0], "minecraft:cobblestone".into());
        assert!(!is_rooted_oak_harvest_origin([0, 70, 0], |p|
            Some(b.get(&p).cloned().unwrap_or("minecraft:air".into()))));
    }

    #[test]
    fn unloaded_crown_is_not_proof_of_harvestable_wood() {
        let b = growing_tree();
        assert!(!is_rooted_oak_harvest_origin([0, 70, 0], |p|
            if p == [1, 72, 0] { None }
            else { Some(b.get(&p).cloned().unwrap_or("minecraft:air".into())) }));
    }


    #[test]
    fn resource_job_queued_before_lot_purchase_loses_permission() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        let origin = [24, 70, 0];
        let stand = [25, 70, 0];
        assert!(world.gather_site_unclaimed(id, origin, stand));
        world.plan_gather(id, Good::Wood, origin, stand).unwrap();
        world.settlements.get_mut(&id).unwrap().player_parcels.push(PlayerParcel {
            origin, owner: "owner".into(), commissioned: false,
        });
        assert!(!world.gather_site_unclaimed(id, origin, stand));
        world.validate().unwrap();
    }

    #[test]
    fn worksite_cannot_approach_occupied_housing_or_bakery_from_side() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        let origin = [32,70,0];
        assert!(world.gather_site_unclaimed(id, origin, [33,70,0]));
        world.settlements.get_mut(&id).unwrap().player_parcels.push(PlayerParcel {
            origin: [42,70,0], owner: "owner".into(), commissioned: false,
        });
        // The harvesting BLOCK is distant, but the worker stand touches
        // somebody else's registered parcel exclusion radius.
        assert!(!world.gather_site_unclaimed(id, origin, [34,70,0]));
    }

    #[test]
    fn legacy_world_roundtrip_needs_no_new_forestry_save_fields() {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.validate().unwrap();
        let restored: World = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert!(restored.settlements[&id].gather_job.is_none());
        restored.validate().unwrap();
    }
