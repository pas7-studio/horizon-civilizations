use std::collections::{BTreeMap, BTreeSet};
    use super::super::*;

    fn world() -> (World, u64) {
        let mut w = World::default();
        let id = w.found("Ashford", "minecraft:overworld", [0, 64, 0], "player-1").unwrap();
        (w, id)
    }

    #[test]
    fn founding_is_bounded_unique_and_grants_lordship() {
        let (mut w, id) = world();
        assert_eq!(w.settlements[&id].population(), 10);
        assert_eq!(w.players["player-1"].rank, Rank::Lord);
        assert!(w.found("Ashford", "minecraft:overworld", [400, 64, 0], "other").is_err());
        assert!(w.found("Village_2", "minecraft:overworld", [20, 64, 0], "other").is_err());
        assert!(w.found("Village_2", "minecraft:the_nether", [20, 64, 0], "other").is_ok());
    }

    #[test]
    fn economy_is_nonnegative_and_stocks_only_move_on_success() {
        let (mut w, id) = world();
        let wheat = w.settlements[&id].quantity(Good::Wheat);
        let coins = w.players["player-1"].coins;
        assert!(w.buy("player-1", id, Good::Wheat, 64).is_err());
        assert_eq!(w.players["player-1"].coins, coins);
        assert_eq!(w.settlements[&id].quantity(Good::Wheat), wheat);
        assert_eq!(w.sell("player-1", id, Good::Wheat, 4), Ok(8));
        assert_eq!(w.buy("player-1", id, Good::Wheat, 4), Ok(8));
        assert_eq!(w.settlements[&id].quantity(Good::Wheat), wheat);
        assert_eq!(w.players["player-1"].coins, coins);
    }

    #[test]
    fn offline_simulation_produces_food_and_houses() {
        let (mut w, id) = world();
        w.advance(12).unwrap();
        assert_eq!(w.day, 12);
        assert!(w.settlements[&id].houses > 2);
        assert_eq!(w.settlements[&id].population(), 10);
        assert_eq!(w.settlements[&id].starving_days, 0);
        assert!(w.advance(31).is_err());
    }

    #[test]
    fn wars_are_symmetric_and_peace_can_be_declared() {
        let (mut w, a) = world();
        let b = w.found("Brighthold", "minecraft:overworld", [450, 65, 0], "player-2").unwrap();
        w.declare_war(a, b).unwrap();
        assert!(w.settlements[&a].active_wars.contains(&b));
        assert!(w.settlements[&b].active_wars.contains(&a));
        w.advance(4).unwrap();
        w.make_peace(a, b).unwrap();
        assert!(w.settlements[&a].active_wars.is_empty());
        assert!(w.settlements[&b].active_wars.is_empty());
    }

    #[test]
    fn settlement_does_not_build_infinite_empty_houses() {
        let (mut w, id) = world();
        w.advance(30).unwrap();
        let village = &w.settlements[&id];
        assert_eq!(village.houses, village.population().div_ceil(3) + 1);
        assert_eq!(village.pending_visual_buildings, village.houses - 2);
    }

    #[test]
    fn workplace_and_entity_labels_are_stable() {
        let (w, id) = world();
        let village = &w.settlements[&id];
        let resident = &village.residents[0];
        let tag = village.actor_name(resident);
        assert!(tag.contains("[civ:1:1000]"));
        assert_eq!(resident.job.workplace(resident.id), resident.job.workplace(resident.id));
    }

    #[test]
    fn loads_reject_asymmetric_war_and_dangling_player() {
        let (mut w, a) = world();
        let b = w.found("Brighthold", "minecraft:overworld", [450, 65, 0], "player-2").unwrap();
        w.declare_war(a, b).unwrap();
        assert!(w.validate().is_ok());
        w.settlements.get_mut(&b).unwrap().active_wars.clear();
        assert!(w.validate().unwrap_err().contains("war"));
        w.settlements.get_mut(&b).unwrap().active_wars.insert(a);
        w.players.get_mut("player-1").unwrap().settlement = Some(999);
        assert!(w.validate().unwrap_err().contains("missing settlement"));
    }

    #[test]
    fn peace_requires_a_real_war() {
        let (mut w, a) = world();
        let b = w.found("Brighthold", "minecraft:overworld", [450, 65, 0], "player-2").unwrap();
        assert!(w.make_peace(a, b).is_err());
        assert!(w.settlements[&a].active_wars.is_empty());
    }

    #[test]
    fn twenty_settlements_simulate_without_broken_references() {
        let mut w = World::default();
        for i in 0..20 {
            let name = format!("Village_{i}");
            let founder = format!("p{i}");
            let id = w.found(&name, "minecraft:overworld", [i * 400, 64, 0], &founder).unwrap();
            assert_eq!(id, i as u64 + 1);
        }
        w.advance(30).unwrap();
        assert_eq!(w.day, 30);
        assert_eq!(w.settlements.len(), 20);
        assert!(w.validate().is_ok());
        for s in w.settlements.values() {
            assert!(s.houses <= s.population().div_ceil(3) + 1);
            assert!(s.quantity(Good::Bread) < 10000);
        }
    }

    #[test]
    fn duplicate_villager_names_within_village_do_not_share_tags() {
        let (w, id) = world();
        let s = &w.settlements[&id];
        let tags: BTreeSet<_> = s.residents.iter().map(|r| s.actor_name(r)).collect();
        assert_eq!(tags.len(), s.residents.len());
    }

    #[test]
    fn hut_is_small_deterministic_and_has_clear_doorway() {
        let a = hut_plan([100, 70, -30]);
        assert_eq!(a, hut_plan([100, 70, -30]));
        assert_eq!(a.iter().filter(|b| b.floor).count(), 25);
        assert!(!a.iter().any(|b| b.pos == [100, 70, -32] || b.pos == [100, 71, -32]));
        let unique: BTreeSet<_> = a.iter().map(|b| b.pos).collect();
        assert_eq!(a.len(), unique.len());
        assert!(a.len() < 110);
    }

    #[test]
    fn visual_house_requires_abstract_project_and_is_resume_safe() {
        let (mut w, id) = world();
        assert!(w.start_visual_house(id, [20, 64, 0]).is_err());
        w.advance(1).unwrap();
        w.start_visual_house(id, [20, 64, 0]).unwrap();
        assert!(w.start_visual_house(id, [30, 64, 0]).is_err());
        assert!(w.finish_visual_house(id).is_err());
        w.settlements.get_mut(&id).unwrap().visual_job.as_mut().unwrap().cursor = hut_plan([20, 64, 0]).len();
        let encoded = serde_json::to_string(&w).unwrap();
        let mut reloaded: World = serde_json::from_str(&encoded).unwrap();
        reloaded.validate().unwrap();
        // This older fixture bypasses native block placement; explicitly
        // emulate a previously persisted warehouse materials reservation.
        reloaded.settlements.get_mut(&id).unwrap().hut_supplies_reserved = true;
        reloaded.finish_visual_house(id).unwrap();
        assert_eq!(reloaded.settlements[&id].visual_sites, vec![[20, 64, 0]]);
        assert_eq!(reloaded.settlements[&id].pending_visual_buildings, 0);
        assert!(reloaded.start_visual_house(id, [21, 64, 0]).is_err());
        reloaded.validate().unwrap();
    }


    #[test]
    fn physical_builder_never_overwrites_an_unexpected_block() {
        let job = VisualBuildJob { origin: [32, 70, -16], cursor: 0, blocked: false };
        let first = hut_plan(job.origin)[0].pos;
        assert_eq!(
            inspect_hut(&job, |p| (p == first).then(|| "minecraft:chest".to_owned())),
            BuildInspection::Blocked { position: first, found: "minecraft:chest".to_owned() }
        );
    }

    #[test]
    fn physical_builder_places_only_a_natural_foundation_or_air() {
        let job = VisualBuildJob { origin: [32, 70, -16], cursor: 0, blocked: false };
        let first = hut_plan(job.origin)[0].pos;
        assert_eq!(
            inspect_hut(&job, |_| Some("minecraft:dirt".to_owned())),
            BuildInspection::Place {
                position: first,
                old_block: "minecraft:dirt".to_owned(),
                new_block: "minecraft:cobblestone",
            }
        );
        assert_eq!(
            inspect_hut(&job, |_| Some("minecraft:cobblestone".to_owned())),
            BuildInspection::Advance
        );
    }

    #[test]
    fn physical_builder_repairs_a_cursor_ahead_of_chunk_persistence() {
        let job = VisualBuildJob { origin: [32, 70, -16], cursor: 3, blocked: false };
        let plan = hut_plan(job.origin);
        let first = plan[0].pos;
        assert_eq!(
            inspect_hut(&job, |p| Some(if p == first { "minecraft:grass_block" } else { "minecraft:cobblestone" }.into())),
            BuildInspection::Rewind(0)
        );
        let second = plan[1].pos;
        assert_eq!(
            inspect_hut(&job, |p| Some(if p == second { "minecraft:bedrock" } else { "minecraft:cobblestone" }.into())),
            BuildInspection::Blocked { position: second, found: "minecraft:bedrock".to_owned() }
        );
    }

    #[test]
    fn physical_builder_pauses_on_unloaded_chunk_and_finishes_only_after_recheck() {
        let origin = [32, 70, -16];
        let mut job = VisualBuildJob { origin, cursor: 0, blocked: false };
        assert_eq!(inspect_hut(&job, |_| None), BuildInspection::Unloaded);
        job.blocked = true;
        assert_eq!(inspect_hut(&job, |_| Some("minecraft:dirt".into())), BuildInspection::Paused);
        job.blocked = false;
        job.cursor = hut_plan(origin).len();
        let by_pos: BTreeMap<[i32; 3], String> = hut_plan(origin)
            .into_iter().map(|step| (step.pos, step.block.to_owned())).collect();
        assert_eq!(inspect_hut(&job, |p| by_pos.get(&p).cloned()), BuildInspection::Complete);
        let missing = hut_plan(origin)[5].pos;
        assert_eq!(inspect_hut(&job, |p| Some(if p == missing { "minecraft:dirt".to_owned() }
            else { by_pos[&p].clone() })), BuildInspection::Rewind(5));
        job.cursor += 1;
        assert_eq!(inspect_hut(&job, |p| by_pos.get(&p).cloned()), BuildInspection::InvalidCursor);
    }

    #[test]
    fn loaded_construction_jobs_take_fair_turns_and_ignore_unloaded_worlds() {
        let (mut sim, a) = world();
        let b = sim.found("Brightfield", "minecraft:overworld", [450, 70, 0], "other").unwrap();
        sim.advance(1).unwrap();
        sim.start_visual_house(a, [20, 64, 0]).unwrap();
        sim.start_visual_house(b, [470, 70, 0]).unwrap();
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |_| true).unwrap().0, a);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 1, |_| true).unwrap().0, b);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 2, |_| true).unwrap().0, a);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |p| p[0] > 400).unwrap().0, b);
        assert_eq!(sim.next_visual_job("minecraft:the_nether", 0, |_| true), None);
        sim.settlements.get_mut(&a).unwrap().visual_job.as_mut().unwrap().blocked = true;
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |_| true).unwrap().0, b);
        assert_eq!(sim.next_visual_job("minecraft:overworld", 0, |_| false), None);
    }

    #[test]
    fn old_save_without_visual_fields_loads_cleanly() {
        let (w, id) = world();
        let mut j: serde_json::Value = serde_json::to_value(&w).unwrap();
        let key = id.to_string();
        let s = j["settlements"].get_mut(key.as_str()).unwrap().as_object_mut().unwrap();
        s.remove("visual_sites");
        s.remove("visual_job");
        let legacy: World = serde_json::from_value(j).unwrap();
        assert!(legacy.settlements[&id].visual_sites.is_empty());
        assert!(legacy.settlements[&id].visual_job.is_none());
        legacy.validate().unwrap();
    }

    #[test]
    fn saved_world_roundtrips_and_preserves_owners() {
        let (mut w, id) = world();
        w.advance(3).unwrap();
        let json = serde_json::to_string(&w).unwrap();
        let loaded: World = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.schema, SCHEMA);
        assert_eq!(loaded.day, 3);
        assert_eq!(loaded.settlements[&id].founder, "player-1");
    }
