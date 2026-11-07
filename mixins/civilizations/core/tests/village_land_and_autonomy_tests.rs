    use super::super::*;

    fn founded() -> (World, u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        (world, id)
    }

    #[test]
    fn town_autonomy_is_on_in_new_and_legacy_saves() {
        let (world, id) = founded();
        assert!(world.settlements[&id].autonomy_enabled);
        let mut legacy = serde_json::to_value(&world).unwrap();
        let village = legacy["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap();
        village.remove("autonomy_enabled");
        village.remove("player_parcels");
        village.remove("treasury_coins");
        let loaded: World = serde_json::from_value(legacy).unwrap();
        loaded.validate().unwrap();
        assert!(loaded.settlements[&id].autonomy_enabled);
        assert!(loaded.settlements[&id].player_parcels.is_empty());
    }

    #[test]
    fn already_paid_abstract_houses_become_real_work_backlog_once() {
        let (mut world, id) = founded();
        assert_eq!(world.settlements[&id].houses, 2);
        assert_eq!(world.settlements[&id].pending_visual_buildings, 0);
        assert!(world.prepare_village_housing(id).unwrap());
        assert_eq!(world.settlements[&id].pending_visual_buildings, 2);
        assert!(!world.prepare_village_housing(id).unwrap());
        assert_eq!(world.settlements[&id].houses, 2);
        world.validate().unwrap();
        world.set_autonomy(id, false).unwrap();
        assert!(!world.settlements[&id].autonomy_enabled);
        assert!(!world.prepare_village_housing(id).unwrap());
        world.set_autonomy(id, true).unwrap();
        assert!(!world.prepare_village_housing(id).unwrap());
    }

    #[test]
    fn land_purchase_is_paid_and_blocks_ordinary_village_construction() {
        let (mut world, id) = founded();
        let before = world.players["owner"].coins;
        assert_eq!(world.buy_plot("owner", id, [20, 70, 0]), Ok(LAND_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, before - LAND_PRICE_COINS);
        assert_eq!(world.settlements[&id].treasury_coins, LAND_PRICE_COINS);
        assert_eq!(world.settlements[&id].player_parcels.len(), 1);
        assert!(world.buy_plot("owner", id, [20, 70, 0]).is_err());
        assert!(world.buy_plot("owner", id, [24, 70, 0]).is_err());
        assert_eq!(world.players["owner"].coins, before - LAND_PRICE_COINS);
        world.prepare_village_housing(id).unwrap();
        assert!(world.start_visual_house(id, [20, 70, 0]).is_err());
        assert!(world.start_farm(id, [20, 70, 0]).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn commission_is_paid_once_built_in_player_parcel_and_not_public_housing() {
        let (mut world, id) = founded();
        world.buy_plot("owner", id, [20, 70, 0]).unwrap();
        assert_eq!(world.players["owner"].coins, 15);
        assert_eq!(world.commission_house("owner", id, [20, 70, 0]), Ok(COMMISSION_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, 0);
        assert_eq!(world.settlements[&id].treasury_coins, LAND_PRICE_COINS + COMMISSION_PRICE_COINS);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), 12);
        assert_eq!(world.settlements[&id].quantity(Good::Stone), 8);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.commission_house("owner", id, [20, 70, 0]).is_err());
        assert_eq!(world.settlements[&id].houses, 3);
        assert_eq!(world.settlements[&id].pending_visual_buildings, 1);
        world.validate().unwrap();
        world.settlements.get_mut(&id).unwrap().visual_job.as_mut().unwrap().cursor =
            hut_plan([20, 70, 0]).len();
        world.settlements.get_mut(&id).unwrap().hut_supplies_reserved = true;
        world.finish_visual_house(id).unwrap();
        assert_eq!(world.settlements[&id].public_huts(), 0);
        assert_eq!(world.settlements[&id].physical_housing(), (0, 0, 10));
        let saved = serde_json::to_string(&world).unwrap();
        let recovered: World = serde_json::from_str(&saved).unwrap();
        recovered.validate().unwrap();
        assert_eq!(recovered.settlements[&id].public_huts(), 0);
    }

    #[test]
    fn another_player_cannot_commission_on_owned_plot() {
        let (mut world, id) = founded();
        world.join("other", id).unwrap();
        world.buy_plot("owner", id, [20, 70, 0]).unwrap();
        let before = world.players["other"].coins;
        assert!(world.commission_house("other", id, [20, 70, 0]).is_err());
        assert_eq!(world.players["other"].coins, before);
        assert!(!world.settlements[&id].player_parcels[0].commissioned);
        world.validate().unwrap();
    }

    #[test]
    fn no_implicit_claims_and_no_buildings_in_other_registered_land() {
        let (mut world, id) = founded();
        world.join("other", id).unwrap();
        world.buy_plot("other", id, [20, 70, 0]).unwrap();
        let before = serde_json::to_string(&world).unwrap();
        assert!(world.check_visual_house_site(id, [20, 70, 0]).is_err());
        assert!(world.start_farm(id, [20, 70, 0]).is_err());
        assert!(world.commission_house("owner", id, [20, 70, 0]).is_err());
        assert_eq!(serde_json::to_string(&world).unwrap(), before);
    }

    #[test]
    fn commission_queues_without_virtual_materials_but_cannot_build_unfunded() {
        let (mut world, id) = founded();
        world.buy_plot("owner", id, [20, 70, 0]).unwrap();
        world.settlements.get_mut(&id).unwrap().stock.insert(Good::Wood, 0);
        world.settlements.get_mut(&id).unwrap().stock.insert(Good::Stone, 0);
        assert_eq!(world.commission_house("owner", id, [20, 70, 0]), Ok(COMMISSION_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, 0);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), 0);
        assert_eq!(world.settlements[&id].quantity(Good::Stone), 0);
        assert!(!world.reserve_hut_supplies(id).unwrap());
        assert!(!world.settlements[&id].hut_supplies_reserved);
        world.validate().unwrap();
    }

    #[test]
    fn buying_finished_hut_transfers_title_and_funds_replacement() {
        let (mut world, id) = founded();
        world.players.get_mut("owner").unwrap().coins += 20;
        world.settlements.get_mut(&id).unwrap().visual_sites.push([20, 70, 0]);
        assert_eq!(world.settlements[&id].public_huts(), 1);
        assert_eq!(world.buy_finished_house("owner", id, [20,70,0]),
                   Ok(FINISHED_HUT_PRICE_COINS));
        assert_eq!(world.players["owner"].coins, 10);
        assert_eq!(world.settlements[&id].treasury_coins, FINISHED_HUT_PRICE_COINS);
        assert_eq!(world.settlements[&id].public_huts(), 0);
        assert_eq!(world.settlements[&id].physical_housing(), (0, 0, 10));
        assert_eq!(world.settlements[&id].player_parcels[0].owner, "owner");
        assert!(world.settlements[&id].player_parcels[0].commissioned);
        let saved = serde_json::to_string(&world).unwrap();
        assert!(world.buy_finished_house("owner", id, [20,70,0]).is_err());
        assert_eq!(world.players["owner"].coins, 10);
        assert_eq!(world.settlements[&id].treasury_coins, FINISHED_HUT_PRICE_COINS);
        world.validate().unwrap();
        let loaded: World = serde_json::from_str(&saved).unwrap();
        loaded.validate().unwrap();

        // A sold hut is NOT a public house. Existing abstract day() must
        // replenish civic housing via normal resource consumption.
        world.advance(1).unwrap();
        assert_eq!(world.settlements[&id].houses, 3);
        assert_eq!(world.settlements[&id].pending_visual_buildings, 1);
        assert_eq!(world.settlements[&id].quantity(Good::Wood), 7);
        assert_eq!(world.settlements[&id].quantity(Good::Stone), 4);
        world.validate().unwrap();
    }

    #[test]
    fn physical_housing_counts_only_public_huts() {
        let (mut world, id) = founded();
        let v = world.settlements.get_mut(&id).unwrap();
        v.houses = 2;
        v.player_parcels.push(PlayerParcel { origin: [20,70,0], owner: "owner".into(), commissioned: true });
        v.visual_sites.push([20,70,0]);
        v.visual_sites.push([40,70,0]);
        assert_eq!(v.public_huts(), 1);
        assert_eq!(v.physical_housing(), (3, 3, 7));
        let mut ids: Vec<u64> = v.residents.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        assert_eq!(v.home_slot(ids[0]).unwrap().site, [40,70,0]);
        world.validate().unwrap();
    }
