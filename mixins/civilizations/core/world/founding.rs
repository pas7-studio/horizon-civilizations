// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;
use std::collections::{BTreeMap, BTreeSet};

impl World {
    pub fn found(
        &mut self,
        name: &str,
        dimension: &str,
        center: [i32; 3],
        founder: &str,
    ) -> Result<u64, String> {
        if self.settlements.len() >= MAX_SETTLEMENTS {
            return Err("settlement cap reached".into());
        }
        if name.len() < 3 || name.len() > 24 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("name must be 3..24 ASCII letters, numbers or underscores".into());
        }
        if self.settlements.values().any(|v| v.name.eq_ignore_ascii_case(name)) {
            return Err("settlement name already exists".into());
        }
        if self.settlements.values().any(|v| {
            if v.dimension != dimension { return false; }
            let dx = i64::from(v.center[0]) - i64::from(center[0]);
            let dz = i64::from(v.center[2]) - i64::from(center[2]);
            dx * dx + dz * dz < MIN_SETTLEMENT_DISTANCE * MIN_SETTLEMENT_DISTANCE
        }) {
            return Err("another settlement is within 192 blocks".into());
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("ID counter exhausted")?;
        let roles = [
            Job::Farmer, Job::Farmer, Job::Farmer, Job::Baker, Job::Baker,
            Job::Lumberjack, Job::Miner, Job::Builder, Job::Guard, Job::Merchant,
        ];
        let first_names = ["Ada", "Bryn", "Cora", "Daren", "Eira", "Finn", "Gwen", "Hugo", "Iris", "Joren"];
        let residents = roles.iter().enumerate().map(|(i, job)| Resident {
            id: id.saturating_mul(1000) + i as u64,
            name: first_names[i].to_owned(),
            job: *job,
            age_days: 20 * 365 + i as u32 * 220,
            alive: true,
            entity_uuid: None,
        }).collect();
        let settlement = Settlement {
            id,
            name: name.to_owned(),
            culture: "frontier".into(),
            dimension: dimension.to_owned(),
            center,
            founder: founder.to_owned(),
            residents,
            stock: BTreeMap::from([
                (Good::Wheat, 20), (Good::Bread, 12), (Good::Wood, 12),
                (Good::Stone, 8), (Good::Iron, 2),
            ]),
            // Founding abstract stock is NOT physical inventory.
            warehouse: BTreeMap::new(),
            build_escrow: BTreeMap::new(),
            barrel_transfer: None,
            market_purchase: None,
            wheat_in_transit: 0,
            bakery: None,
            hall_job: None,
            hall_complete: false,
            gather_job: None,
            oak_stem: None,
            treasury_coins: 0,
            houses: 2,
            building_progress: 0,
            starving_days: 0,
            morale: 50,
            relationships: BTreeMap::new(),
            active_wars: BTreeSet::new(),
            pending_visual_buildings: 0,
            actors_spawned: false,
            actor_recovery_issued: BTreeSet::new(),
            autonomy_enabled: true,
            player_parcels: Vec::new(),
            visual_sites: Vec::new(),
            visual_job: None,
            hut_supplies_reserved: false,
            farm: None,
            posts: Vec::new(),
            quarry_step: 0,
        };
        self.settlements.insert(id, settlement);
        let p = self.players.entry(founder.to_owned()).or_default();
        p.settlement = Some(id);
        p.rank = Rank::Lord;
        p.coins = p.coins.saturating_add(25);
        p.reputation.insert(id, 50);
        self.event(format!("{name} founded"));
        Ok(id)
    }
}
