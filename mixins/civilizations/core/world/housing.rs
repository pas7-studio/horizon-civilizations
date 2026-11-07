// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Domain-only collision policy; physical construction still separately
    /// requires clear loaded terrain. Player parcels are hard exclusions.
    pub fn check_visual_house_site(&self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        self.check_visual_house_site_for(village_id, origin, None, true)
    }

    /// A commission may use *only its authenticated owner's* bought lot.
    /// All other protections and structure separation remain in force.
    fn check_visual_house_site_for(
        &self, village_id: u64, origin: [i32; 3],
        owner: Option<&str>, require_budget: bool
    ) -> Result<(), String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if require_budget && village.pending_visual_buildings == 0 {
            return Err("no completed abstract house awaits visualization".into());
        }
        if village.visual_job.is_some() {
            return Err("construction already in progress".into());
        }
        if village.visual_sites.len() >= 32 {
            return Err("physical housing site limit reached".into());
        }
        if i64::from(origin[0]).abs() > 29_999_990 || i64::from(origin[2]).abs() > 29_999_990
            || !(-2048..=2048).contains(&origin[1]) {
            return Err("site too close to world border".into());
        }
        // i32 coordinates may differ by 2^32-1. Squaring two such deltas
        // overflows i64; use i128 even for corrupted or untrusted save data.
        let distance = |a: [i32; 3], b: [i32; 3]| {
            let dx = i128::from(a[0]) - i128::from(b[0]);
            let dz = i128::from(a[2]) - i128::from(b[2]);
            dx * dx + dz * dz
        };
        if !(12_i128 * 12..=80_i128 * 80).contains(&distance(village.center, origin)) {
            return Err("build 12..80 blocks from village center".into());
        }
        for other in self.settlements.values().filter(|s| s.dimension == village.dimension) {
            if other.player_parcels.iter().any(|plot| {
                near_plot(plot.origin, origin) && owner != Some(plot.owner.as_str())
            }) {
                return Err("plot is owned by a player; construction requires its owner's commission".into());
            }
            if other.visual_sites.iter().any(|&site| distance(site, origin) < 9_i128 * 9)
                || other.visual_job.as_ref().is_some_and(|job| distance(job.origin, origin) < 9_i128 * 9)
                || other.farm.as_ref().is_some_and(|farm| distance(farm.origin, origin) < 9_i128 * 9)
                || other.bakery.as_ref().is_some_and(|b| distance(b.origin, origin) < 9_i128 * 9) {
                return Err("plot overlaps an existing civilization structure or farm".into());
            }
        }
        Ok(())
    }

    /// A read-only village proposal. Slot is 1-based and always resolves to
    /// the same X/Z for this settlement. The operator supplies the exact Y
    /// level after a *live* survey; nothing is reserved by this method.
    pub fn proposed_hut(&self, village_id: u64, slot: usize, y: i32) -> Result<[i32; 3], String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if village.physical_housing().2 == 0 {
            return Err("all residents already have physical hut capacity".into());
        }
        let [dx, dz] = *HOUSING_PLOT_OFFSETS.get(slot.checked_sub(1).ok_or("invalid housing slot")?)
            .ok_or("invalid housing slot")?;
        let x = i64::from(village.center[0]) + i64::from(dx);
        let z = i64::from(village.center[2]) + i64::from(dz);
        let x = i32::try_from(x).map_err(|_| "housing candidate outside world bounds")?;
        let z = i32::try_from(z).map_err(|_| "housing candidate outside world bounds")?;
        let origin = [x, y, z];
        self.check_visual_house_site(village_id, origin)?;
        Ok(origin)
    }

    /// Already-paid abstract houses become physical jobs automatically. This
    /// never changes the abstract number of houses or manufactures resources.
    /// Player-commissioned homes are separately counted in visual_sites.
    pub fn prepare_village_housing(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if !v.autonomy_enabled { return Ok(false); }
        let completed = v.visual_sites.len() as u32;
        let covered = completed.saturating_add(v.pending_visual_buildings);
        let physical_needed = v.population().div_ceil(PHYSICAL_HOME_CAPACITY as u32);
        let missing = physical_needed.saturating_sub(v.public_huts() as u32)
            .saturating_sub(v.pending_visual_buildings);
        let already_paid = v.houses.saturating_sub(covered);
        let queue = missing.min(already_paid);
        if queue == 0 { return Ok(false); }
        v.pending_visual_buildings = v.pending_visual_buildings
            .checked_add(queue).ok_or("visual backlog overflow")?;
        Ok(true)
    }

    /// Simple authorization control. Autonomy is ON by default even in old
    /// saves. Off pauses new site selection, but preserves existing jobs.
    pub fn set_autonomy(&mut self, village_id: u64, enabled: bool) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        village.autonomy_enabled = enabled;
        let name = village.name.clone();
        self.event(format!("{name}: autonomous construction {}", if enabled { "enabled" } else { "paused" }));
        Ok(())
    }

    /// A purchased 5x5 parcel is the player's local build right in this mod.
    /// Terrain and the actor UUID are checked by the native command adapter.
    pub fn buy_plot(&mut self, player_id: &str, village_id: u64, origin: [i32; 3]) -> Result<u64, String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        let p = self.players.get(player_id).ok_or("join the village first")?;
        if p.settlement != Some(village_id) { return Err("join this village before buying land".into()); }
        if p.coins < LAND_PRICE_COINS { return Err("not enough coins to buy a parcel".into()); }
        if v.player_parcels.len() >= MAX_PLAYER_PARCELS { return Err("plot registry is full".into()); }
        let treasury_after = v.treasury_coins.checked_add(LAND_PRICE_COINS)
            .ok_or("village treasury overflow")?;
        if i64::from(origin[0]).abs() > 29_999_990 || i64::from(origin[2]).abs() > 29_999_990
            || !(-2048..=2048).contains(&origin[1]) {
            return Err("land outside safe world bounds".into());
        }
        let dx = i128::from(origin[0]) - i128::from(v.center[0]);
        let dz = i128::from(origin[2]) - i128::from(v.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("buy land 12..80 blocks from village center".into());
        }
        for other in self.settlements.values().filter(|s| s.dimension == v.dimension) {
            if other.player_parcels.iter().any(|plot| near_plot(plot.origin, origin))
                || other.visual_sites.iter().any(|site| near_plot(*site, origin))
                || other.visual_job.as_ref().is_some_and(|job| near_plot(job.origin, origin))
                || other.farm.as_ref().is_some_and(|farm| near_plot(farm.origin, origin))
                || other.bakery.as_ref().is_some_and(|b| near_plot(b.origin, origin)) {
                return Err("land overlaps a registered building, field, bakery or another player's parcel".into());
            }
        }
        self.players.get_mut(player_id).unwrap().coins -= LAND_PRICE_COINS;
        let v = self.settlements.get_mut(&village_id).unwrap();
        v.player_parcels.push(PlayerParcel {
            origin, owner: player_id.to_owned(), commissioned: false
        });
        v.treasury_coins = treasury_after;
        let name = self.settlements[&village_id].name.clone();
        self.event(format!("{name}: player-owned 5x5 parcel sold at {},{},{}", origin[0], origin[1], origin[2]));
        Ok(LAND_PRICE_COINS)
    }

    /// Buy a finished, genuinely recorded public hut, not an abstract house.
    /// Native command verifies its real loaded Minecraft blocks first.
    /// Residents leave the sold building; future daily economy may pay for
    /// replacement public housing using normal material consumption.
    pub fn buy_finished_house(
        &mut self, player_id: &str, village_id: u64, origin: [i32; 3]
    ) -> Result<u64, String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if !v.visual_sites.contains(&origin) {
            return Err("no completed physical village hut at this site".into());
        }
        let p = self.players.get(player_id).ok_or("join the village first")?;
        if p.settlement != Some(village_id) { return Err("join the village first".into()); }
        if p.coins < FINISHED_HUT_PRICE_COINS { return Err("not enough civic coins for this hut".into()); }
        if v.player_parcels.len() >= MAX_PLAYER_PARCELS { return Err("plot registry is full".into()); }
        let treasury_after = v.treasury_coins.checked_add(FINISHED_HUT_PRICE_COINS)
            .ok_or("village treasury overflow")?;
        if self.settlements.values().filter(|s| s.dimension == v.dimension).any(|s| {
            s.player_parcels.iter().any(|plot| near_plot(plot.origin, origin))
        }) {
            return Err("this physical hut is already privately owned".into());
        }
        self.players.get_mut(player_id).unwrap().coins -= FINISHED_HUT_PRICE_COINS;
        let v = self.settlements.get_mut(&village_id).unwrap();
        v.player_parcels.push(PlayerParcel {
            origin, owner: player_id.to_owned(), commissioned: true
        });
        v.treasury_coins = treasury_after;
        let name = v.name.clone();
        self.event(format!(
            "{name}: finished hut sold into private ownership at {},{},{}",
            origin[0], origin[1], origin[2]
        ));
        Ok(FINISHED_HUT_PRICE_COINS)
    }

    /// Commission a physical hut on a bought plot. This is a WORLD-JSON
    /// transaction: coin labor fee and abstract stock materials are consumed
    /// together with the construction job. Block writes occur later.
    pub fn commission_house(&mut self, player_id: &str, village_id: u64, origin: [i32; 3]) -> Result<u64, String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        let parcel_index = village.player_parcels.iter()
            .position(|plot| plot.origin == origin && plot.owner == player_id && !plot.commissioned)
            .ok_or("buy this empty plot first; already commissioned lots cannot be reused")?;
        let p = self.players.get(player_id).ok_or("join the village first")?;
        if p.settlement != Some(village_id) { return Err("join this village before commissioning".into()); }
        if p.coins < COMMISSION_PRICE_COINS { return Err("not enough coins for a building commission".into()); }
        let houses = village.houses.checked_add(1).ok_or("house count overflow")?;
        let pending = village.pending_visual_buildings.checked_add(1).ok_or("pending job overflow")?;
        let treasury_after = village.treasury_coins.checked_add(COMMISSION_PRICE_COINS)
            .ok_or("village treasury overflow")?;
        self.check_visual_house_site_for(village_id, origin, Some(player_id), false)?;
        // All validations above are read-only; all mutations below are in
        // the same save object. The command adapter rolls back on save error.
        self.players.get_mut(player_id).unwrap().coins -= COMMISSION_PRICE_COINS;
        let village = self.settlements.get_mut(&village_id).unwrap();
        // The owner's labor fee pays the civic treasury. The physical
        // bill-of-materials is charged exactly once later, immediately
        // before the first world block, by reserve_hut_supplies().
        // No virtual abstract stone/wood may pay for that construction.
        village.houses = houses;
        village.pending_visual_buildings = pending;
        village.treasury_coins = treasury_after;
        village.player_parcels[parcel_index].commissioned = true;
        village.visual_job = Some(VisualBuildJob { origin, cursor: 0, blocked: false });
        village.hut_supplies_reserved = false;
        let name = village.name.clone();
        self.event(format!("{name}: commissioned player-owned hut at {},{},{}",origin[0],origin[1],origin[2]));
        Ok(COMMISSION_PRICE_COINS)
    }

    /// Reserve one operator-approved house; a later native level tick performs
    /// one conditional placement at a time after checking live world blocks.
    pub fn start_visual_house(&mut self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        self.check_visual_house_site(village_id, origin)?;
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let name = village.name.clone();
        village.visual_job = Some(VisualBuildJob { origin, cursor: 0, blocked: false });
        village.hut_supplies_reserved = false;
        self.event(format!(
            "{name}: physical construction project reserved at {},{},{}",
            origin[0], origin[1], origin[2]
        ));
        Ok(())
    }

    /// Backed by actual items delivered by players or confirmed physically
    /// collected by the village. An existing abstract house is NOT a supply.
    pub fn reserve_hut_supplies(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.visual_job.as_ref().ok_or("no active hut construction")?;
        if v.hut_supplies_reserved { return Ok(false); }
        if job.cursor != 0 || job.blocked {
            return Err("unfunded partial/blocked hut needs inspection before any physical work".into());
        }
        // A completed town hall changes the authoritative storage source:
        // only goods physically extracted from its real vanilla barrels
        // into the write-ahead builder escrow can pay this construction bill.
        let available = if v.hall_complete {
            v.build_escrow.get(&Good::Wood).copied().unwrap_or(0) >= HUT_WOOD_LOGS
                && v.build_escrow.get(&Good::Stone).copied().unwrap_or(0) >= HUT_COBBLESTONE
        } else {
            // Historical pre-hall work orders may finish from their already
            // accounted delivered ledger; new automatic huts wait for the
            // real completed civic hall.
            v.warehouse_quantity(Good::Wood) >= HUT_WOOD_LOGS
                && v.warehouse_quantity(Good::Stone) >= HUT_COBBLESTONE
        };
        if !available { return Ok(false); }
        if v.hall_complete {
            *v.build_escrow.get_mut(&Good::Wood).ok_or("missing wood escrow")? -= HUT_WOOD_LOGS;
            *v.build_escrow.get_mut(&Good::Stone).ok_or("missing stone escrow")? -= HUT_COBBLESTONE;
        } else {
            v.warehouse_debit(Good::Wood, HUT_WOOD_LOGS)?;
            v.warehouse_debit(Good::Stone, HUT_COBBLESTONE)?;
        }
        v.hut_supplies_reserved = true;
        Ok(true)
    }

    /// Rotate across loaded, unblocked physical jobs in a deterministic order.
    /// Never let an unloaded first village stall all other civilizations.
    pub fn next_visual_job(
        &self,
        dimension: &str,
        sequence: usize,
        mut loaded: impl FnMut([i32; 3]) -> bool,
    ) -> Option<(u64, VisualBuildJob)> {
        let eligible: Vec<_> = self.settlements.values()
            .filter(|s| s.dimension == dimension)
            .filter_map(|s| {
                let job = s.visual_job.as_ref()?;
                (!job.blocked && loaded(job.origin)).then(|| (s.id, job.clone()))
            })
            .collect();
        if eligible.is_empty() {
            None
        } else {
            Some(eligible[sequence % eligible.len()].clone())
        }
    }

    /// Called only after all block states have been observed at their targets.
    pub fn finish_visual_house(&mut self, village_id: u64) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = village.visual_job.as_ref().ok_or("no construction project")?;
        if job.cursor != hut_plan(job.origin).len() { return Err("construction incomplete".into()); }
        let origin = job.origin;
        if village.pending_visual_buildings == 0 { return Err("house counter underflow".into()); }
        if !village.hut_supplies_reserved {
            return Err("hut cannot complete without proven physical material reservation".into());
        }
        village.visual_job = None;
        village.hut_supplies_reserved = false;
        village.visual_sites.push(origin);
        village.pending_visual_buildings -= 1;
        let name = village.name.clone();
        self.event(format!("{name}: house constructed at {},{},{}", origin[0], origin[1], origin[2]));
        Ok(())
    }
}
