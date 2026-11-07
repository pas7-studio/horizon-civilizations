use std::collections::{BTreeSet};
    use super::super::*;

    fn village() -> (World,u64) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0,70,0], "owner").unwrap();
        (world,id)
    }
    fn hall() -> (World,u64) {
        let (mut world,id) = village();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        (world,id)
    }
    fn ready_bakery() -> (World,u64) {
        let (mut world,id) = hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        {
            let b = world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
            b.construction_escrow.insert(Good::Wood, BAKERY_WOOD_LOGS);
            b.construction_escrow.insert(Good::Stone, BAKERY_COBBLESTONE);
        }
        assert!(world.reserve_bakery_supplies(id).unwrap());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor = bakery_plan([24,70,0]).len();
        world.finish_bakery(id).unwrap();
        world.validate().unwrap();
        (world,id)
    }

    #[test]
    fn native_bakery_has_one_crafting_table_and_real_workstation() {
        let blueprint = bakery_plan([24,70,0]);
        let positions: BTreeSet<_> = blueprint.iter().map(|b| b.pos).collect();
        assert_eq!(positions.len(), blueprint.len());
        assert_eq!(blueprint.iter().filter(|b| b.block == "minecraft:crafting_table").count(),1);
        assert_eq!(blueprint.iter().filter(|b| b.block == "minecraft:smoker").count(),1);
        assert_eq!(blueprint.iter().filter(|b| b.floor).count(),25);
        assert_eq!(blueprint,bakery_plan([24,70,0]));
    }

    #[test]
    fn legacy_saves_do_not_forge_a_free_existing_bakery() {
        let (world,id)=village();
        let json=serde_json::to_string(&world).unwrap();
        let mut value: serde_json::Value=serde_json::from_str(&json).unwrap();
        value["settlements"].as_object_mut().unwrap()
            .get_mut(&id.to_string()).unwrap().as_object_mut().unwrap().remove("bakery");
        let migrated: World=serde_json::from_value(value).unwrap();
        migrated.validate().unwrap();
        assert!(migrated.settlements[&id].bakery.is_none());
    }

    #[test]
    fn bakery_requires_real_hall_and_an_unclaimed_safe_plot() {
        let (mut world,id)=village();
        assert!(world.start_bakery(id,[24,70,0]).is_err());
        world.settlements.get_mut(&id).unwrap().hall_complete=true;
        assert!(world.start_bakery(id,[2,70,0]).is_err());
        world.buy_plot("owner",id,[24,70,0]).unwrap();
        assert!(world.start_bakery(id,[24,70,0]).is_err());
        world.start_bakery(id,[40,70,0]).unwrap();
        assert!(world.start_bakery(id,[60,70,0]).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn unbacked_bakery_never_constructs_and_funding_is_once_only() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        let v=world.settlements.get_mut(&id).unwrap();
        v.warehouse_credit(Good::Wood,100).unwrap();
        v.warehouse_credit(Good::Stone,100).unwrap();
        assert!(!world.reserve_bakery_supplies(id).unwrap()); // queue is not a real barrel
        assert!(world.finish_bakery(id).is_err());
        {
            let b=world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
            b.construction_escrow.insert(Good::Wood,BAKERY_WOOD_LOGS);
            b.construction_escrow.insert(Good::Stone,BAKERY_COBBLESTONE-1);
        }
        assert!(!world.reserve_bakery_supplies(id).unwrap());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .construction_escrow.insert(Good::Stone,BAKERY_COBBLESTONE);
        assert!(world.reserve_bakery_supplies(id).unwrap());
        assert!(!world.reserve_bakery_supplies(id).unwrap());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood),100);
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap()
            .construction_escrow[&Good::Wood],0);
        world.validate().unwrap();
    }

    #[test]
    fn bakery_only_finishes_after_all_workstations_physically_verified() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().complete,false);
        assert!(world.finish_bakery(id).is_err());
        let job=world.settlements[&id].bakery.as_ref().unwrap().building.as_ref().unwrap().clone();
        assert!(matches!(inspect_bakery(&job, |_|None),BuildInspection::Unloaded));
        let station=bakery_plan(job.origin).last().unwrap().pos;
        assert!(matches!(inspect_bakery(&job, |p| if p==station {
            Some("minecraft:player_block".into())
        } else { Some("minecraft:air".into()) }),BuildInspection::Blocked {..}
            | BuildInspection::Place {..}));
        // Simulate the native world's complete observation, not merely
        // declaring a structure built with abstract house counters.
        let b=world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap();
        b.construction_escrow.insert(Good::Wood,BAKERY_WOOD_LOGS);
        b.construction_escrow.insert(Good::Stone,BAKERY_COBBLESTONE);
        world.reserve_bakery_supplies(id).unwrap();
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor = bakery_plan([24,70,0]).len();
        world.finish_bakery(id).unwrap();
        assert!(world.settlements[&id].bakery.as_ref().unwrap().complete);
        world.validate().unwrap();
    }

    #[test]
    fn real_barrel_materials_move_to_bakery_escrow_not_virtual_stock() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        let pos=hall_storage_positions([0,70,0])[0];
        assert!(world.plan_barrel_transfer(id,pos,Good::Bread,
            BarrelDirection::ToBakeryBuilder,8,8,Some(0)).is_err());
        world.plan_barrel_transfer(id,pos,Good::Wood,
            BarrelDirection::ToBakeryBuilder,8,8,Some(0)).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        assert!(world.confirm_barrel_transfer(id,8).is_err());
        world.confirm_barrel_transfer(id,0).unwrap();
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap()
            .construction_escrow[&Good::Wood],8);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood),0);
        world.validate().unwrap();
    }

    #[test]
    fn a_baker_never_uses_virtual_stock_and_crafts_exact_recipe() {
        let (mut world,id)=ready_bakery();
        assert!(world.settlements[&id].quantity(Good::Wheat)>0);
        assert_eq!(world.craft_bread(id),Ok(false));
        let pos=hall_storage_positions([0,70,0])[1];
        world.plan_barrel_transfer(id,pos,Good::Wheat,
            BarrelDirection::ToBakerIngredients,3,3,Some(0)).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id,0).unwrap();
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().wheat_escrow,3);
        assert_eq!(world.craft_bread(id),Ok(true));
        assert_eq!(world.craft_bread(id),Ok(false));
        let bakery=world.settlements[&id].bakery.as_ref().unwrap();
        assert_eq!(bakery.wheat_escrow,0);
        assert_eq!(bakery.bread_in_transit,1);
        assert_eq!(bakery.batches,1);
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Bread),0);
        world.validate().unwrap();
    }

    #[test]
    fn baker_delivers_one_receipt_then_steward_inserts_actual_bread() {
        let (mut world,id)=ready_bakery();
        let pos=hall_storage_positions([0,70,0])[1];
        world.plan_barrel_transfer(id,pos,Good::Wheat,
            BarrelDirection::ToBakerIngredients,3,3,Some(0)).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id,0).unwrap();
        world.craft_bread(id).unwrap();
        assert_eq!(world.deliver_bakery_bread(id),Ok(true));
        assert_eq!(world.deliver_bakery_bread(id),Ok(false));
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Bread),1);
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().bread_in_transit,0);
        world.plan_barrel_transfer(id,pos,Good::Bread,
            BarrelDirection::ToBarrel,0,1,None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.confirm_barrel_transfer(id,1).unwrap();
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Bread),0);
        assert_eq!(world.settlements[&id].bakery.as_ref().unwrap().batches,1);
        let restored: World=serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        restored.validate().unwrap();
    }

    #[test]
    fn native_block_ownership_protects_public_work_and_private_lots() {
        let (mut world,id)=ready_bakery();
        assert!(!world.player_may_break("stranger","minecraft:overworld",[0,70,0]));
        assert!(!world.player_may_break("stranger","minecraft:overworld",[24,70,0]));
        assert!(world.player_may_break("stranger","minecraft:the_nether",[24,70,0]));
        assert!(world.player_may_break("stranger","minecraft:overworld",[100,70,0]));
        world.buy_plot("owner",id,[40,70,0]).unwrap();
        assert!(world.player_may_break("owner","minecraft:overworld",[40,70,0]));
        assert!(!world.player_may_break("stranger","minecraft:overworld",[40,70,0]));
        assert!(world.player_may_break("stranger","minecraft:overworld",[40,60,0]));
        world.validate().unwrap();
    }

    #[test]
    fn bakery_job_rejects_blocked_or_overflowed_saved_work() {
        let (mut world,id)=hall();
        world.start_bakery(id,[24,70,0]).unwrap();
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor=bakery_plan([24,70,0]).len()+1;
        assert!(world.validate().is_err());
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap()
            .building.as_mut().unwrap().cursor=0;
        world.settlements.get_mut(&id).unwrap().bakery.as_mut().unwrap().wheat_escrow=13;
        assert!(world.validate().is_err());
    }
