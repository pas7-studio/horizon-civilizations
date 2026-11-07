    use super::super::*;

    fn ready_world() -> (World, u64, u64) {
        let mut world = World::default();
        let settlement = world.found("Ashford", "minecraft:overworld", [0, 64, 0], "player").unwrap();
        world.settlements.get_mut(&settlement).unwrap().actors_spawned = true;
        let resident = world.settlements[&settlement].residents[0].id;
        (world, settlement, resident)
    }

    #[test]
    fn uuid_persists_as_32_hex_characters_with_no_json_precision_loss() {
        let (mut world, settlement, resident) = ready_world();
        let uuid = 0xdeadbeef_01234567_89abcdef_01234567_u128;
        assert!(world.bind_actor(settlement, resident, uuid).unwrap());
        let encoded = serde_json::to_string(&world).unwrap();
        assert!(encoded.contains("deadbeef0123456789abcdef01234567"));
        let recovered: World = serde_json::from_str(&encoded).unwrap();
        recovered.validate().unwrap();
        assert_eq!(recovered.settlements[&settlement].residents[0].entity_uuid, Some(uuid));
        assert!(!world.bind_actor(settlement, resident, uuid).unwrap());
    }

    #[test]
    fn uuid_binding_rejects_reassignment_collisions_and_unreserved_spawning() {
        let (mut world, settlement, resident) = ready_world();
        assert!(world.bind_actor(settlement, resident, 0).is_err());
        assert!(world.bind_actor(settlement, resident, 123).unwrap());
        assert!(world.bind_actor(settlement, resident, 124).is_err());
        let second = world.settlements[&settlement].residents[1].id;
        assert!(world.bind_actor(settlement, second, 123).is_err());
        world.settlements.get_mut(&settlement).unwrap().actors_spawned = false;
        assert!(world.bind_actor(settlement, second, 987).is_err());
        assert!(world.validate().is_err());
    }

    #[test]
    fn legacy_world_without_entity_uuid_remains_loadable() {
        let (world, settlement, _) = ready_world();
        let serialized = serde_json::to_string(&world).unwrap();
        assert!(!serialized.contains("entity_uuid"));
        let old: World = serde_json::from_str(&serialized).unwrap();
        old.validate().unwrap();
        assert!(old.settlements[&settlement].residents.iter().all(|r| r.entity_uuid.is_none()));
    }

    #[test]
    fn malformed_or_repeated_uuids_fail_closed_on_load() {
        let (mut world, settlement, resident) = ready_world();
        world.bind_actor(settlement, resident, 0x1111).unwrap();
        let mut data: serde_json::Value = serde_json::to_value(&world).unwrap();
        data["settlements"][settlement.to_string()]["residents"][0]["entity_uuid"] =
            serde_json::Value::String("INVALID".into());
        assert!(serde_json::from_value::<World>(data).is_err());
        let second = world.settlements[&settlement].residents[1].id;
        world.settlements.get_mut(&settlement).unwrap().residents[1].entity_uuid = Some(0x1111);
        assert_eq!(second, world.settlements[&settlement].residents[1].id);
        assert!(world.validate().is_err());
    }
