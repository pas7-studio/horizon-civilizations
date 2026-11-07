    use super::super::*;

    fn ready() -> (World, u64, [i32; 3]) {
        let mut world = World::default();
        let id = world.found("Ashford", "minecraft:overworld", [0, 70, 0], "owner").unwrap();
        world.settlements.get_mut(&id).unwrap().hall_complete = true;
        let pos = hall_storage_positions([0, 70, 0])[0];
        (world, id, pos)
    }

    #[test]
    fn unissued_native_intent_can_be_canceled_once() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 8).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Wood,
            BarrelDirection::ToBarrel, 0, 8, None).unwrap();
        world.cancel_planned_barrel_transfer(id).unwrap();
        assert!(world.settlements[&id].barrel_transfer.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 8);
        assert!(world.cancel_planned_barrel_transfer(id).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn issued_and_blocked_transactions_cannot_be_canceled_without_evidence() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Stone, 3).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Stone,
            BarrelDirection::ToBarrel, 0, 3, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        assert!(world.cancel_planned_barrel_transfer(id).is_err());
        world.block_barrel_transfer(id).unwrap();
        assert!(world.cancel_planned_barrel_transfer(id).is_err());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Stone), 3);
    }

    #[test]
    fn blocked_receipt_matches_exact_physical_after_count_only() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 4).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Wood,
            BarrelDirection::ToBarrel, 10, 4, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.block_barrel_transfer(id).unwrap();
        let prior = serde_json::to_value(&world).unwrap();
        assert!(world.reconcile_blocked_barrel_transfer(id, 10).is_err());
        assert_eq!(serde_json::to_value(&world).unwrap(), prior);
        world.reconcile_blocked_barrel_transfer(id, 14).unwrap();
        assert!(world.settlements[&id].barrel_transfer.is_none());
        assert_eq!(world.settlements[&id].warehouse_quantity(Good::Wood), 0);
        assert!(world.reconcile_blocked_barrel_transfer(id, 14).is_err());
        world.validate().unwrap();
    }

    #[test]
    fn reconciliation_rolls_back_when_inbound_receipt_was_lost() {
        let (mut world, id, pos) = ready();
        world.settlements.get_mut(&id).unwrap().warehouse_credit(Good::Wood, 4).unwrap();
        world.plan_barrel_transfer(id, pos, Good::Wood,
            BarrelDirection::ToBarrel, 0, 4, None).unwrap();
        world.issue_barrel_transfer(id).unwrap();
        world.block_barrel_transfer(id).unwrap();
        world.settlements.get_mut(&id).unwrap().warehouse_debit(Good::Wood, 4).unwrap();
        let prior = serde_json::to_value(&world).unwrap();
        assert!(world.reconcile_blocked_barrel_transfer(id, 4).is_err());
        assert_eq!(serde_json::to_value(&world).unwrap(), prior);
        assert_eq!(world.settlements[&id].barrel_transfer.as_ref().unwrap().phase,
            BarrelPhase::Blocked);
    }
