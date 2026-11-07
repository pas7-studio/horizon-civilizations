// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;
use std::collections::{BTreeSet};

impl World {
    /// Validate semantic invariants on load as well as JSON syntax. On corruption
    /// the adapter fails closed and never overwrites the damaged save.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA { return Err(format!("unknown schema {}", self.schema)); }
        if self.settlements.len() > MAX_SETTLEMENTS { return Err("too many settlements".into()); }
        if self.chronicle.len() > 200 { return Err("chronicle too large".into()); }
        if self.next_id <= self.settlements.keys().next_back().copied().unwrap_or(0) {
            return Err("settlement ID counter is not monotonic".into());
        }
        let mut seen = BTreeSet::new();
        let mut all_entity_uuids = BTreeSet::new();
        let mut all_parcels: Vec<(String, [i32; 3])> = Vec::new();
        for (&id, village) in &self.settlements {
            if village.id != id { return Err("settlement id/key mismatch".into()); }
            let normalized = village.name.to_ascii_lowercase();
            if !seen.insert(normalized) { return Err("duplicate settlement names".into()); }
            if let Some(stem) = &village.oak_stem {
                let gather = village.gather_job.as_ref()
                    .ok_or("oak stem saved without an active physical gather")?;
                let expected_y = stem.root[1].checked_add(i32::from(stem.cursor))
                    .ok_or("oak stem Y coordinate overflow")?;
                if !(3..=4).contains(&stem.height) || stem.cursor >= stem.height
                    || gather.good != Good::Wood
                    || gather.position != [stem.root[0], expected_y, stem.root[2]]
                    || gather.stand != stem.stand {
                    return Err("invalid saved short oak stem continuation".into());
                }
            }
            if let Some(gather) = &village.gather_job {
                if let Some(worker_id) = gather.worker_id {
                    let role = if gather.good == Good::Wood { Job::Lumberjack }
                        else { Job::Miner };
                    if !village.residents.iter().any(|r|
                        r.id == worker_id && r.job == role) {
                        return Err("physical gather worker ID is invalid".into());
                    }
                }
                if gather.native_edit_confirmed
                    && (!gather.issued || gather.worker_id.is_none()) {
                    return Err("native edit receipt without issued worker identity".into());
                }
                // Historic PR #30 saves can contain a witnessed pickup
                // without the new post-command acknowledgement field.
                // Accept that save structurally so the whole settlement
                // remains loadable; the native adapter QUARANTINES this
                // ambiguous Issued job before processing any more receipts.
                // Never infer native_edit_confirmed from pickup_witnessed.
                if gather.pickup_witnessed
                    && (!gather.issued || gather.worker_id.is_none()) {
                    return Err("pickup witness without issued worker identity".into());
                }
                let [x, y, z] = gather.position;
                let [sx, sy, sz] = gather.stand;
                let walk_dx = i128::from(sx) - i128::from(x);
                let walk_dy = i128::from(sy) - i128::from(y);
                let walk_dz = i128::from(sz) - i128::from(z);
                let village_dx = i128::from(x) - i128::from(village.center[0]);
                let village_dz = i128::from(z) - i128::from(village.center[2]);
                if gather.good.job().is_none()
                    || i64::from(x).abs() > 29_999_988
                    || i64::from(z).abs() > 29_999_988
                    || !(-2046..=2046).contains(&y)
                    || i64::from(sx).abs() > 29_999_988
                    || i64::from(sz).abs() > 29_999_988
                    || !(-2046..=2046).contains(&sy)
                    || walk_dx * walk_dx + walk_dy * walk_dy + walk_dz * walk_dz > 16
                    || !(12_i128 * 12..=80_i128 * 80)
                        .contains(&(village_dx * village_dx + village_dz * village_dz)) {
                    return Err("invalid physical gather record".into());
                }
            }
            if let Some(purchase) = &village.market_purchase {
                if !village.hall_complete
                    || !hall_storage_positions(village.center).contains(&purchase.position)
                    || !(1..=64).contains(&purchase.count) || purchase.count > purchase.before
                    || purchase.price != u64::from(purchase.good.price()) * u64::from(purchase.count)
                    || !self.players.contains_key(&purchase.buyer) {
                    return Err("invalid pending native market purchase".into());
                }
            }
            if let Some(b) = &village.bakery {
                if !village.hall_complete
                    || i64::from(b.origin[0]).abs() > 29_999_988
                    || i64::from(b.origin[2]).abs() > 29_999_988
                    || !(-2046..=2046).contains(&b.origin[1])
                    || b.wheat_escrow > BREAD_WHEAT_RECIPE || b.bread_in_transit > 8
                    || b.construction_escrow.values().any(|&amount| amount > BAKERY_COBBLESTONE)
                    || (b.complete && (b.building.is_some() || !b.supplies_reserved))
                    || (!b.complete && b.building.is_none())
                    || b.building.as_ref().is_some_and(|job|
                        job.origin != b.origin || job.cursor > bakery_plan(b.origin).len()) {
                    return Err("invalid saved physical bakery state".into());
                }
            }
            if village.wheat_in_transit > 10_000_000 {
                return Err("wheat in physical transit exceeds cap".into());
            }
            if village.build_escrow.values().any(|&amount| amount > 10_000_000) {
                return Err("physical builder escrow entry too large".into());
            }
            if let Some(t) = &village.barrel_transfer {
                if !village.hall_complete || !hall_storage_positions(village.center).contains(&t.position)
                    || t.before > 10_000_000 || !(1..=8).contains(&t.count)
                    || t.expected_after().is_none() {
                    return Err("invalid official barrel transfer record".into());
                }
                match t.direction {
                    BarrelDirection::ToBuilder if t.slot.is_none()
                        || !matches!(t.good, Good::Wood | Good::Stone) =>
                        return Err("invalid builder barrel extraction slot".into()),
                    BarrelDirection::ToBarrel if t.slot.is_some() =>
                        return Err("native barrel deposit cannot name a slot".into()),
                    _ => {}
                }
                match t.direction {
                    BarrelDirection::ToBakeryBuilder if t.slot.is_none()
                        || !matches!(t.good, Good::Wood | Good::Stone)
                        || village.bakery.as_ref().is_none_or(|b|
                            b.supplies_reserved || b.complete
                                || b.building.as_ref().is_none_or(|job| job.blocked)) =>
                        return Err("invalid native bakery material extraction".into()),
                    BarrelDirection::ToBakerIngredients if t.slot.is_none()
                        || t.good != Good::Wheat
                        || village.bakery.as_ref().is_none_or(|b|
                            !b.complete || b.bread_in_transit != 0
                                || b.wheat_escrow.checked_add(t.count)
                                    .is_none_or(|n| n > BREAD_WHEAT_RECIPE)) =>
                        return Err("invalid native baker wheat extraction".into()),
                    _ => {}
                }
            }
            if village.hall_complete && village.hall_job.is_some() {
                return Err("civic hall cannot be simultaneously complete and under construction".into());
            }
            if let Some(job) = &village.hall_job {
                if job.origin != village.center
                    || job.cursor > town_hall_plan(job.origin).len() {
                    return Err("civic hall cursor/site invalid".into());
                }
            }
            for &amount in village.warehouse.values() {
                if amount > 10_000_000 { return Err("physical warehouse entry too large".into()); }
            }
            if village.player_parcels.len() > MAX_PLAYER_PARCELS { return Err("too many player parcels".into()); }
            for parcel in &village.player_parcels {
                if parcel.owner.is_empty() || !self.players.contains_key(&parcel.owner) {
                    return Err("parcel owner has no player record".into());
                }
                let [x, y, z] = parcel.origin;
                if i64::from(x).abs() > 29_999_990 || i64::from(z).abs() > 29_999_990
                    || !(-2048..=2048).contains(&y) {
                    return Err("parcel outside world bounds".into());
                }
                if all_parcels.iter().any(|(dimension, point)| {
                    dimension == &village.dimension && near_plot(*point, parcel.origin)
                }) {
                    return Err("overlapping player land parcels".into());
                }
                if parcel.commissioned
                    && !village.visual_sites.contains(&parcel.origin)
                    && !village.visual_job.as_ref().is_some_and(|job| job.origin == parcel.origin) {
                    return Err("commissioned parcel has no recorded house job or completed hut".into());
                }
                all_parcels.push((village.dimension.clone(), parcel.origin));
            }
            if village.residents.len() > 128 { return Err("resident cap exceeded".into()); }
            if !village.actor_recovery_issued.is_empty() && !village.actors_spawned {
                return Err("recovery attempts exist before actor spawn reservation".into());
            }
            for recovery_id in &village.actor_recovery_issued {
                // A resident may legitimately die AFTER one native spawn
                // attempt was reserved. Keep that one-shot tombstone on disk
                // (do not revive them), but never reject the entire village
                // save merely because their life ended.
                if !village.residents.iter().any(|r|
                    r.id == *recovery_id && r.entity_uuid.is_none()) {
                    return Err("invalid pending actor recovery ID or already bound actor".into());
                }
            }
            let mut resident_ids = BTreeSet::new();
            for resident in &village.residents {
                if !resident_ids.insert(resident.id) {
                    return Err("duplicate resident ID".into());
                }
                if let Some(uuid) = resident.entity_uuid {
                    if uuid == 0 || !village.actors_spawned {
                        return Err("entity UUID is invalid for this settlement".into());
                    }
                    if !all_entity_uuids.insert(uuid) {
                        return Err("duplicate actor entity UUID across settlements".into());
                    }
                }
            }
            if let Some(plot) = &village.farm {
                let [x, y, z] = plot.origin;
                if i64::from(x).abs() > 29_999_988 || i64::from(z).abs() > 29_999_988
                    || !(-2046..=2046).contains(&y) {
                    return Err("farm outside world bounds".into());
                }
                let count = if plot.stage == FarmStage::Preparing { farm_plan(plot.origin).len() } else { 8 };
                if plot.cursor > count { return Err("farm cursor beyond phase".into()); }
                if plot.completed_cycles > 1_000_000 { return Err("farm cycle counter invalid".into()); }
                if plot.harvest_receipts.len() > MAX_HARVEST_RECEIPTS {
                    return Err("farm harvest receipt history too large".into());
                }
                let mut previous = 0_u64;
                for receipt in &plot.harvest_receipts {
                    if receipt.sequence <= previous
                        || receipt.sequence > plot.harvest_sequence
                        || receipt.crops != 8
                        || receipt.world_day > self.day {
                        return Err("farm receipt invalid or out of order".into());
                    }
                    previous = receipt.sequence;
                }
                if plot.harvest_receipts.last().is_some_and(|r| r.sequence != plot.harvest_sequence) {
                    return Err("farm receipt tail does not match sequence".into());
                }
            }
            if village.visual_sites.len() > 32 { return Err("visual building site cap exceeded".into()); }
            if village.visual_sites.len() as u32 > village.houses { return Err("visual buildings exceed houses".into()); }
            if village.pending_visual_buildings > village.houses { return Err("pending physical houses exceed housing".into()); }
            if village.visual_sites.len() as u32 > village.houses.saturating_sub(village.pending_visual_buildings) {
                return Err("visual building count exceeds housing".into());
            }
            if village.posts.len() > MAX_POSTS {
                return Err("camp limit exceeded".into());
            }
            if village.quarry_step > MAX_QUARRY_STEPS {
                return Err("quarry pit depth out of range".into());
            }
            let mut camp_sites = BTreeSet::new();
            for post in &village.posts {
                if i64::from(post.origin[0]).abs() > 29_999_990
                    || i64::from(post.origin[2]).abs() > 29_999_990
                    || !(-2048..=2048).contains(&post.origin[1])
                {
                    return Err("camp outside world bounds".into());
                }
                if !camp_sites.insert(post.origin) {
                    return Err("duplicate camp site".into());
                }
                if let Some(worker) = post.worker_id {
                    let resident = village.residents.iter()
                        .find(|r| r.id == worker && r.alive)
                        .ok_or("camp worker is not an active resident")?;
                    if resident.job != post.kind.job() {
                        return Err("camp worker has the wrong job".into());
                    }
                }
            }
            let mut unique_sites = BTreeSet::new();
            for site in &village.visual_sites {
                if i64::from(site[0]).abs() > 29_999_990 || i64::from(site[2]).abs() > 29_999_990
                    || !(-2048..=2048).contains(&site[1]) {
                    return Err("visual site outside world bounds".into());
                }
                if !unique_sites.insert((site[0], site[1], site[2])) { return Err("duplicate visual build site".into()); }
            }
            if village.hut_supplies_reserved && village.visual_job.is_none() {
                return Err("reserved hut materials without an active job".into());
            }
            if let Some(job) = &village.visual_job {
                if i64::from(job.origin[0]).abs() > 29_999_990 || i64::from(job.origin[2]).abs() > 29_999_990
                    || !(-2048..=2048).contains(&job.origin[1]) {
                    return Err("construction site outside world bounds".into());
                }
                if job.cursor > hut_plan(job.origin).len() { return Err("construction cursor beyond plan".into()); }
                if village.pending_visual_buildings == 0 { return Err("visual job without pending house".into()); }
                if unique_sites.contains(&(job.origin[0], job.origin[1], job.origin[2])) {
                    return Err("building is already completed at the job site".into());
                }
            }
            if !(-100..=100).contains(&village.morale) { return Err("morale out of bounds".into()); }
            for (&other, &value) in &village.relationships {
                if other == id || !self.settlements.contains_key(&other) || !(-100..=100).contains(&value) {
                    return Err("invalid diplomatic relation".into());
                }
            }
            for &enemy in &village.active_wars {
                if enemy == id || !self.settlements.get(&enemy).is_some_and(|v| v.active_wars.contains(&id)) {
                    return Err("asymmetric or dangling war".into());
                }
            }
        }
        for p in self.players.values() {
            if p.settlement.is_some_and(|id| !self.settlements.contains_key(&id)) {
                return Err("player references missing settlement".into());
            }
            if p.reputation.keys().any(|id| !self.settlements.contains_key(id)) {
                return Err("reputation references missing settlement".into());
            }
        }
        Ok(())
    }

    pub(crate) fn event(&mut self, message: String) {
        self.chronicle.push(format!("day {}: {message}", self.day));
        if self.chronicle.len() > 200 {
            self.chronicle.drain(..self.chronicle.len() - 200);
        }
    }

    /// Bind a resident to the actual saved Horizon entity UUID, once.
    /// The operation is idempotent for the same identity and rejects reassignment.
    /// The OP explicitly requested ONE missing villager after inspecting
    /// loaded native actors. Save this reservation before sending any spawn
    /// command, so a crash or native refusal does not cause a second entity
    /// to be spawned by automated logic on a later tick.
    pub fn reserve_actor_recovery(
        &mut self, settlement_id: u64, resident_id: u64,
    ) -> Result<(), String> {
        let settlement = self.settlements.get_mut(&settlement_id)
            .ok_or("unknown settlement")?;
        if !settlement.actors_spawned {
            return Err("initial villager batch has not been reserved".into());
        }
        let missing = settlement.residents.iter().any(|r|
            r.id == resident_id && r.alive && r.entity_uuid.is_none());
        if !missing {
            return Err("resident is dead, missing or already UUID-linked".into());
        }
        if !settlement.actor_recovery_issued.insert(resident_id) {
            return Err("actor recovery already attempted; inspect/reconcile, never spawn twice".into());
        }
        Ok(())
    }

    pub fn bind_actor(&mut self, settlement_id: u64, resident_id: u64, uuid: u128) -> Result<bool, String> {
        if uuid == 0 {
            return Err("cannot bind zero entity UUID".into());
        }
        if self.settlements.iter().any(|(&sid, s)| {
            s.residents.iter().any(|r| {
                r.entity_uuid == Some(uuid) && (sid != settlement_id || r.id != resident_id)
            })
        }) {
            return Err("entity UUID is already owned by another resident".into());
        }
        let settlement = self.settlements.get_mut(&settlement_id).ok_or("unknown settlement")?;
        if !settlement.actors_spawned {
            return Err("cannot bind before actor spawning has been reserved".into());
        }
        let resident = settlement.residents.iter_mut()
            .find(|r| r.id == resident_id && r.alive)
            .ok_or("unknown or inactive resident")?;
        match resident.entity_uuid {
            Some(existing) if existing == uuid => Ok(false),
            Some(_) => Err("resident already bound to a different entity UUID".into()),
            None => {
                resident.entity_uuid = Some(uuid);
                // Atomic in-memory state saved by the bridge. If the save
                // fails, the disk retains the pending one-shot reservation.
                settlement.actor_recovery_issued.remove(&resident_id);
                Ok(true)
            }
        }
    }

    pub fn by_name(&self, name: &str) -> Option<&Settlement> {
        self.settlements.values().find(|v| v.name.eq_ignore_ascii_case(name))
    }

    pub fn join(&mut self, player_id: &str, village_id: u64) -> Result<(), String> {
        if !self.settlements.contains_key(&village_id) {
            return Err("no such settlement".into());
        }
        let p = self.players.entry(player_id.to_owned()).or_default();
        if p.rank == Rank::Lord && p.settlement != Some(village_id) {
            return Err("a lord cannot abandon their settlement".into());
        }
        p.settlement = Some(village_id);
        if p.rank == Rank::Visitor {
            p.rank = Rank::Citizen;
            p.coins = p.coins.saturating_add(25);
        }
        p.reputation.entry(village_id).or_insert(5);
        Ok(())
    }

    pub fn promote(&mut self, player_id: &str) -> Result<Rank, String> {
        let p = self.players.get_mut(player_id).ok_or("join a settlement first")?;
        let id = p.settlement.ok_or("join a settlement first")?;
        let reputation = *p.reputation.get(&id).unwrap_or(&0);
        p.rank = match (p.rank, reputation) {
            (Rank::Citizen, n) if n >= 15 => Rank::Freeman,
            (Rank::Freeman, n) if n >= 40 => Rank::Knight,
            (Rank::Knight, n) if n >= 90 => Rank::Lord,
            _ => return Err("not enough reputation for the next title".into()),
        };
        Ok(p.rank)
    }

    /// Synchronize the abstract economy to the real Horizon day counter.
    /// On first run/reload from an old save, anchor without manufacturing
    /// historical output. Time rewinds never replay already accounted days.
    /// A giant forward jump catches up only MAX_HOST_CATCHUP_DAYS once.
    pub fn observe_host_day(&mut self, host_day: i64) -> Result<bool, String> {
        match self.last_host_day {
            None => {
                self.last_host_day = Some(host_day);
                Ok(true)
            }
            Some(previous) if host_day <= previous => Ok(false),
            Some(previous) => {
                // i128 prevents overflow across the entire signed i64 range.
                let elapsed = (i128::from(host_day) - i128::from(previous))
                    .min(i128::from(MAX_HOST_CATCHUP_DAYS)) as u32;
                self.advance(elapsed)?;
                self.last_host_day = Some(host_day);
                Ok(true)
            }
        }
    }

    pub fn advance(&mut self, days: u32) -> Result<(), String> {
        if days == 0 || days > MAX_SIM_DAYS { return Err("simulate 1..30 days at a time".into()); }
        for _ in 0..days {
            self.day = self.day.checked_add(1).ok_or("simulation day overflow")?;
            let mut events = Vec::new();
            for settlement in self.settlements.values_mut() {
                events.extend(settlement.day());
            }
            // Abstract warfare at strategic scale. There is no combat NPC AI yet.
            let pairs: Vec<(u64, u64)> = self.settlements.iter()
                .flat_map(|(&id, s)| s.active_wars.iter().filter(move |&&enemy| id < enemy).map(move |&enemy| (id, enemy)))
                .collect();
            for (a, b) in pairs {
                let a_power = self.settlements[&a].count(Job::Guard) + self.settlements[&a].quantity(Good::Iron) / 8;
                let b_power = self.settlements[&b].count(Job::Guard) + self.settlements[&b].quantity(Good::Iron) / 8;
                if self.day.is_multiple_of(4) {
                    let target = if a_power >= b_power { b } else { a };
                    let s = self.settlements.get_mut(&target).expect("known settlement");
                    s.morale = (s.morale - 3).max(-100);
                    s.take(Good::Bread, 2);
                    events.push(format!("{} suffered war attrition (abstract)", s.name));
                }
            }
            for event in events { self.event(event); }
        }
        Ok(())
    }
}
