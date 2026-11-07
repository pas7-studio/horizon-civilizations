// Horizon Civilizations domain module, re-exported by core/mod.rs.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn autonomous_by_default() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settlement {
    pub id: u64,
    pub name: String,
    pub culture: String,
    pub dimension: String,
    pub center: [i32; 3],
    pub founder: String,
    pub residents: Vec<Resident>,
    pub stock: BTreeMap<Good, u32>,
    /// Actual player deliveries and confirmed native harvests pending
    /// placement in the visible hall barrels. NOT virtual daily stock and
    /// NOT duplicated by a successful barrel deposit.
    #[serde(default)]
    pub warehouse: BTreeMap<Good, u32>,
    /// Goods already physically removed from a barrel for the current
    /// construction request. No native items remain in the container.
    #[serde(default)]
    pub build_escrow: BTreeMap<Good, u32>,
    /// Single journalled native barrel operation. Write-ahead state prevents
    /// repeated external insertion/extraction on crash or reload.
    #[serde(default)]
    pub barrel_transfer: Option<BarrelTransfer>,
    /// One direct native-barrel player purchase, precharged in world.json.
    #[serde(default)]
    pub market_purchase: Option<MarketPurchase>,
    /// Confirmed physical wheat harvested but still being carried by
    /// UUID-linked farmers toward the central village receiving area.
    #[serde(default)]
    pub wheat_in_transit: u32,
    /// One physical bakery, built and operated by actual NPC workers.
    #[serde(default)]
    pub bakery: Option<BakeryWorkshop>,
    /// Active physical civic hall construction; never inferred from a JSON
    /// house counter, and can resume after restarting with observed blocks.
    #[serde(default)]
    pub hall_job: Option<VisualBuildJob>,
    /// The civic center is proven complete only after all plan blocks read.
    #[serde(default)]
    pub hall_complete: bool,
    /// Pending actual tree/stone harvest; reconciled against native blocks.
    #[serde(default)]
    pub gather_job: Option<GatherJob>,
    /// Optional continuation for a proven short native oak trunk.
    /// Each log is still a separate issued gather and delivered receipt.
    #[serde(default)]
    pub oak_stem: Option<OakStem>,
    /// Player civic coin payments are transferred here, not silently burned.
    #[serde(default)]
    pub treasury_coins: u64,
    pub houses: u32,
    pub building_progress: u32,
    pub starving_days: u32,
    pub morale: i32,
    pub relationships: BTreeMap<u64, i16>,
    pub active_wars: BTreeSet<u64>,
    pub pending_visual_buildings: u32,
    pub actors_spawned: bool,
    /// One-shot, durably reserved OP recovery attempts after partial native
    /// actor spawning. A crash can occur AFTER the entity is created but
    /// BEFORE its UUID is persisted: never automatically reissue this spawn.
    /// The OP must first inspect/reconcile any already-existing villager.
    #[serde(default)]
    pub actor_recovery_issued: BTreeSet<u64>,
    /// Villages develop by default. OP can pause selected settlements.
    #[serde(default = "autonomous_by_default")]
    pub autonomy_enabled: bool,
    /// Land purchased by players; no village physical job may use these
    /// parcels unless its owner explicitly commissions a building.
    #[serde(default)]
    pub player_parcels: Vec<PlayerParcel>,
    /// Empty for existing v0.1.1 saves; these are *physical* build sites only.
    #[serde(default)]
    pub visual_sites: Vec<[i32; 3]>,
    #[serde(default)]
    pub visual_job: Option<VisualBuildJob>,
    /// Physical bill-of-materials deducted once per active hut project.
    #[serde(default)]
    pub hut_supplies_reserved: bool,
    /// Optional one-field living crop simulation, retained across saves.
    #[serde(default)]
    pub farm: Option<FarmPlot>,
    /// Lumber camps and quarries. Old saves load with none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub posts: Vec<WorkPost>,
    /// How many staircase steps of the quarry pit are already opened. A pit
    /// is real excavation on real ground, never a virtual stone source.
    #[serde(default)]
    pub quarry_step: u8,
}

/// Compute *combined* outstanding construction demand using verified native
/// barrel counts and confirmed inbound receipts. Each project's escrow is
/// deducted from its OWN bill, never reused to satisfy another project.
/// Caller must pass a fresh complete snapshot of BOTH native hall barrels.
/// No virtual daily stock is involved.
pub fn physical_construction_shortfall(
    village: &Settlement, native_stock: &[(Good, u32)],
) -> (u32, u32) {
    let bakery = village.bakery.as_ref().filter(|b| {
        !b.supplies_reserved
            && b.building.as_ref().is_some_and(|job| !job.blocked)
    });
    let hut_needed = village.visual_job.as_ref().is_some_and(|job| {
        !job.blocked && !village.hut_supplies_reserved
    }) || (village.visual_job.is_none()
        && village.pending_visual_buildings > 0
        && village.physical_housing().2 > 0);

    let unpaid = |good: Good| {
        let bakery_bill = if let Some(b) = bakery {
            let total = if good == Good::Wood { BAKERY_WOOD_LOGS }
                else { BAKERY_COBBLESTONE };
            total.saturating_sub(b.construction_escrow.get(&good).copied().unwrap_or(0))
        } else { 0 };
        let hut_bill = if hut_needed {
            let total = if good == Good::Wood { HUT_WOOD_LOGS }
                else { HUT_COBBLESTONE };
            total.saturating_sub(village.build_escrow.get(&good).copied().unwrap_or(0))
        } else { 0 };
        bakery_bill.saturating_add(hut_bill)
    };
    let available = |good: Good| {
        let stored = native_stock.iter().find(|(g, _)| *g == good)
            .map(|(_, count)| *count).unwrap_or(0);
        stored.saturating_add(village.warehouse_quantity(good))
    };
    (unpaid(Good::Wood).saturating_sub(available(Good::Wood)),
     unpaid(Good::Stone).saturating_sub(available(Good::Stone)))
}

/// Bounded village bread buffer, backed only by actual native hall barrels,
/// confirmed inbound receipts and the baker's already-crafted delivery.
pub fn physical_bread_target(village: &Settlement) -> u32 {
    village.population().saturating_mul(2).min(64)
}

/// Prevent infinite physical baking and storage-filling after a settlement
/// has enough real bread. The observer MUST read both native hall barrels;
/// an unavailable block entity pauses production instead of treating it as
/// an empty container. This pure function never credits abstract stock.
pub fn needs_real_bread(village: &Settlement, observed_barrel_bread: u32) -> bool {
    let pending = village.warehouse_quantity(Good::Bread)
        .saturating_add(village.bakery.as_ref()
            .map(|b| b.bread_in_transit).unwrap_or(0));
    observed_barrel_bread.saturating_add(pending) < physical_bread_target(village)
}

impl Settlement {
    pub fn count(&self, job: Job) -> u32 {
        self.residents.iter().filter(|r| r.alive && r.job == job).count() as u32
    }

    pub fn population(&self) -> u32 {
        self.residents.iter().filter(|r| r.alive).count() as u32
    }

    /// How many residents this village can actually employ, by role. Every
    /// entry is backed by a site that exists in the save: no farmer without a
    /// farm plot, no baker without a bakery, no lumberjack or miner without a
    /// camp or a real material shortage. Guards patrol the settlement and the
    /// builder works the construction the orchestrator keeps open, so both
    /// keep one seat.
    pub fn workplace_demand(&self) -> BTreeMap<Job, usize> {
        let mut demand: BTreeMap<Job, usize> = BTreeMap::new();
        let mut want = |job: Job, seats: usize| {
            if seats > 0 {
                *demand.entry(job).or_insert(0) += seats;
            }
        };
        want(Job::Guard, 1);
        want(Job::Builder, 1);
        if self.hall_complete || self.hall_job.is_some() {
            want(Job::Merchant, 1);
        }
        if self.bakery.is_some() {
            want(Job::Baker, 1);
        }
        if self.farm.is_some() {
            want(Job::Farmer, 1);
        }
        let lumber_camps = self.posts.iter()
            .filter(|post| post.kind == PostKind::Lumber).count();
        let quarries = self.posts.iter()
            .filter(|post| post.kind == PostKind::Quarry).count();
        let (wood_missing, stone_missing) = self.hall_materials_missing();
        // An active building programme is real work even before the saved
        // warehouse shows a shortfall: huts, the hall and the bakery are
        // paid from real barrels, so one camp seat each is justified while
        // anything is being raised.
        let building = !self.hall_complete || self.hall_job.is_some()
            || self.visual_job.is_some()
            || (self.pending_visual_buildings > 0 && self.physical_housing().2 > 0)
            || self.bakery.as_ref().is_some_and(|bakery| !bakery.complete);
        // A shortage alone justifies one seat even before a camp stands: the
        // orchestrator opens the camp at the first real source the worker
        // finds. Camps themselves are permanent workplaces.
        let wood_work = wood_missing > 0 || building || self.oak_stem.is_some()
            || self.gather_job.as_ref().is_some_and(|job| job.good == Good::Wood);
        let stone_work = stone_missing > 0 || building
            || self.gather_job.as_ref().is_some_and(|job| job.good == Good::Stone);
        want(Job::Lumberjack, lumber_camps.max(usize::from(wood_work)));
        want(Job::Miner, quarries.max(usize::from(stone_work)));
        // Sand and dirt are only demanded by a pit the player or operator
        // actually placed: nothing consumes them yet, so no seat is invented.
        let pits = self.posts.iter()
            .filter(|post| matches!(post.kind, PostKind::Sand | PostKind::Dirt))
            .count();
        want(Job::Digger, pits.min(1));
        demand
    }

    /// Recompute every resident's role from the workplaces this village really
    /// has. Stable across turns: a resident keeps a role while its demand
    /// lasts, camp workers keep their camp, and only the leftovers become
    /// laborers. Returns the residents whose role changed so the caller can
    /// persist once and update their visible name tags. No world edit here.
    pub fn staff_workplaces(&mut self) -> Vec<(u64, Job)> {
        let alive: Vec<u64> = self.residents.iter()
            .filter(|resident| resident.alive).map(|resident| resident.id).collect();
        let mut remaining = self.workplace_demand();
        let mut chosen: BTreeMap<u64, Job> = BTreeMap::new();
        // A harvest already issued to one resident keeps that resident in its
        // role: re-rolling mid-job would orphan the durable journal entry that
        // authorizes the native block edit.
        let harvest_pin = self.gather_job.as_ref().and_then(|job| {
            job.worker_id.map(|worker| {
                (worker, job.good.job().unwrap_or(Job::Miner))
            })
        });
        if let Some((worker, job)) = harvest_pin {
            if alive.contains(&worker) && !chosen.contains_key(&worker) {
                if let Some(seats) = remaining.get_mut(&job) {
                    *seats = seats.saturating_sub(1);
                }
                chosen.insert(worker, job);
            }
        }
        // 1. A camp keeps the same worker so receipts stay attributable.
        for post in &self.posts {
            let Some(worker) = post.worker_id else { continue };
            if !alive.contains(&worker) || chosen.contains_key(&worker) {
                continue;
            }
            let job = post.kind.job();
            if let Some(seats) = remaining.get_mut(&job) {
                if *seats > 0 {
                    *seats -= 1;
                    chosen.insert(worker, job);
                }
            }
        }
        // 2. Keep an existing role while a seat for it remains.
        for resident in self.residents.iter().filter(|resident| resident.alive) {
            if chosen.contains_key(&resident.id) || resident.job == Job::Laborer {
                continue;
            }
            if let Some(seats) = remaining.get_mut(&resident.job) {
                if *seats > 0 {
                    *seats -= 1;
                    chosen.insert(resident.id, resident.job);
                }
            }
        }
        // 3. Fill the remaining seats with unassigned residents, in a fixed
        // priority order so the same save always produces the same roster.
        for job in [Job::Guard, Job::Builder, Job::Merchant, Job::Baker,
                    Job::Farmer, Job::Lumberjack, Job::Miner, Job::Digger] {
            while remaining.get(&job).copied().unwrap_or(0) > 0 {
                let Some(id) = self.residents.iter()
                    .filter(|resident| resident.alive && !chosen.contains_key(&resident.id))
                    .map(|resident| resident.id)
                    .min() else { break };
                if let Some(seats) = remaining.get_mut(&job) {
                    *seats -= 1;
                }
                chosen.insert(id, job);
            }
        }
        // 4. Everyone else is honestly unemployed, not a fake specialist.
        let mut changed = Vec::new();
        for resident in self.residents.iter_mut() {
            if !resident.alive {
                continue;
            }
            let job = chosen.get(&resident.id).copied().unwrap_or(Job::Laborer);
            if resident.job != job {
                resident.job = job;
                changed.push((resident.id, job));
            }
        }
        // 5. Never keep a camp bound to a dead or re-roled worker.
        for post in self.posts.iter_mut() {
            let Some(worker) = post.worker_id else { continue };
            let keep = self.residents.iter().any(|resident| {
                resident.id == worker && resident.alive && resident.job == post.kind.job()
            });
            if !keep {
                post.worker_id = None;
            }
        }
        changed
    }

    /// Derived physical housing capacity. Abstract houses are NOT included.
    /// Allocations are recalculated from the current living roster and ordered
    /// completed sites; no new data is serialized to legacy world saves.
    /// The player's commissioned houses are not public NPC lodging. The
    /// physical_housing and home_slot computations share this same filter.
    pub fn public_huts(&self) -> usize {
        self.visual_sites.iter().filter(|site| !self.player_parcels.iter()
            .any(|plot| plot.commissioned && plot.origin == **site)).count()
    }

    pub fn physical_housing(&self) -> (usize, usize, usize) {
        let capacity = self.public_huts() * PHYSICAL_HOME_CAPACITY;
        let residents = self.residents.iter().filter(|resident| resident.alive).count();
        (capacity, residents.min(capacity), residents.saturating_sub(capacity))
    }

    /// Stable for the same roster/sites, independent of resident vector order.
    /// The first three living IDs get the first completed hut, next three the
    /// second hut; no spot is invented in an abstract or unfinished house.
    pub fn home_slot(&self, resident_id: u64) -> Option<HomeSlot> {
        let mut ids: Vec<u64> = self.residents.iter()
            .filter(|resident| resident.alive).map(|resident| resident.id).collect();
        ids.sort_unstable();
        let position = ids.binary_search(&resident_id).ok()?;
        let site = self.visual_sites.iter()
            .filter(|site| !self.player_parcels.iter().any(|plot| plot.commissioned && plot.origin == **site))
            .nth(position / PHYSICAL_HOME_CAPACITY).copied()?;
        let lane = (position % PHYSICAL_HOME_CAPACITY) as u8;
        Some(HomeSlot { site, lane })
    }

    /// Run the existing daily economy on an in-memory clone. The forecast
    /// cannot place blocks, grant items, save data or append world history.
    /// Only the next 1..=7 abstract days are accepted for presentation.
    pub fn forecast_abstract_days(&self, days: u8) -> Vec<AbstractDayForecast> {
        let mut hypothetical = self.clone();
        let mut report = Vec::new();
        for offset in 1..=days.min(MAX_COUNCIL_FORECAST_DAYS) {
            // Events belong to the hypothetical clone, not World.chronicle.
            let _discarded_events = hypothetical.day();
            report.push(AbstractDayForecast {
                offset,
                wheat: hypothetical.quantity(Good::Wheat),
                bread: hypothetical.quantity(Good::Bread),
                morale: hypothetical.morale,
                starving_days: hypothetical.starving_days,
                houses: hypothetical.houses,
                pending_visual_buildings: hypothetical.pending_visual_buildings,
            });
        }
        report
    }

    /// Rank *current* settlement risks from its own already-saved economy,
    /// population, physical housing and identity state. No AI inference,
    /// no random actions, and no implicit permission to trade/build/fight.
    pub fn council_priorities(&self) -> Vec<CivicPriority> {
        let mut priorities = Vec::new();
        let population = self.population();
        if population == 0 {
            return priorities;
        }
        let food_per_day = population.div_ceil(3);
        let predicted = self.forecast_abstract_days(COUNCIL_FORECAST_DAYS);
        let starvation_forecast = predicted.iter().find(|d| d.starving_days > 0)
            .map(|d| d.offset);
        let bread = self.quantity(Good::Bread);
        if self.starving_days > 0 {
            priorities.push(CivicPriority {
                need: CivicNeed::Food, urgency: 100,
                reason: format!(
                    "food shortage has lasted {} day(s); {} bread/day needed",
                    self.starving_days, food_per_day
                ),
                next_step: format!("Inspect /civ info {} and food production before advancing time", self.name),
            });
        } else if let Some(day) = starvation_forecast {
            priorities.push(CivicPriority {
                need: CivicNeed::Food, urgency: 90,
                reason: format!(
                    "abstract economy forecasts starvation on day +{}; current bread {}",
                    day, bread
                ),
                next_step: format!("Inspect /civ forecast {} and bread supply; physical harvests do not credit stock", self.name),
            });
        } else if bread < food_per_day.saturating_mul(3) {
            priorities.push(CivicPriority {
                need: CivicNeed::Food, urgency: 55,
                reason: format!(
                    "bread reserve {} is below three-day consumption {} (forecast currently fed)",
                    bread, food_per_day.saturating_mul(3)
                ),
                next_step: format!("Inspect /civ forecast {} before changing the economy", self.name),
            });
        }

        if !self.active_wars.is_empty() {
            priorities.push(CivicPriority {
                need: CivicNeed::Security, urgency: 95,
                reason: format!(
                    "{} active war(s), {} living guard(s); warfare is abstract",
                    self.active_wars.len(), self.count(Job::Guard)
                ),
                next_step: format!("Review /civ history and /civ info {}; diplomacy remains player/operator-controlled", self.name),
            });
        }
        if self.morale < 30 {
            priorities.push(CivicPriority {
                need: CivicNeed::Stability,
                urgency: if self.morale < 0 { 85 } else { 60 },
                reason: format!(
                    "morale {} below 30; recent shortage/war may be contributors",
                    self.morale
                ),
                next_step: format!("Inspect /civ info {} and /civ history before making policy changes", self.name),
            });
        }
        if !self.actors_spawned {
            priorities.push(CivicPriority {
                need: CivicNeed::Identity, urgency: 70,
                reason: format!(
                    "{} residents exist only in the abstract simulation; spawn not reserved",
                    population
                ),
                next_step: format!("OP may use /civ actors {} once in a disposable world, then /civ roster {}", self.name, self.name),
            });
        } else {
            let missing = self.residents.iter()
                .filter(|r| r.alive && r.entity_uuid.is_none()).count();
            if missing > 0 {
                priorities.push(CivicPriority {
                    need: CivicNeed::Identity, urgency: 88,
                    reason: format!(
                        "{} living resident(s) lack a durable entity UUID; no automatic respawn",
                        missing
                    ),
                    next_step: format!("Inspect /civ roster {} and reconcile only known legacy actors; never spawn blindly", self.name),
                });
            }
        }

        let (_, _, homeless) = self.physical_housing();
        if homeless > 0 {
            let (urgency, reason, next_step) = if let Some(job) = &self.visual_job {
                (45, format!(
                    "{} residents lack completed physical hut slots; active build at {:?}{}",
                    homeless, job.origin, if job.blocked { " is blocked" } else { "" }
                ), format!("Inspect /civ construction {}; do not approve another site while a job is active", self.name))
            } else if self.pending_visual_buildings > 0 {
                (65, format!(
                    "{} residents lack physical housing; {} abstract house(s) ready for visual construction",
                    homeless, self.pending_visual_buildings
                ), format!("Inspect /civ survey {}; OP approval is required before building", self.name))
            } else {
                (40, format!(
                    "{} residents lack completed physical hut slots; no abstract house awaits visualization",
                    homeless
                ), format!("Inspect /civ homes {}; abstract houses cannot be auto-converted to physical huts", self.name))
            };
            priorities.push(CivicPriority {
                need: CivicNeed::Housing, urgency, reason, next_step,
            });
        }

        let desired_houses = population.div_ceil(3).saturating_add(1);
        let wood = self.quantity(Good::Wood);
        let stone = self.quantity(Good::Stone);
        if self.houses < desired_houses && self.count(Job::Builder) > 0
            && (wood < 8 || stone < 6) {
            priorities.push(CivicPriority {
                need: CivicNeed::Materials, urgency: 60,
                reason: format!(
                    "future abstract house needs 8 wood/6 stone; available {wood} wood/{stone} stone"
                ),
                next_step: format!(
                    "Inspect /civ info {}; existing physical job materials were already paid in the abstract simulation",
                    self.name
                ),
            });
        }
        priorities.sort_by(|a, b| b.urgency.cmp(&a.urgency).then(a.need.cmp(&b.need)));
        priorities.truncate(5);
        priorities
    }

    pub fn wheat_delivery_pending(&self) -> u32 { self.wheat_in_transit }

    /// Called only after the native server sees an actual UUID-bound
    /// farmer return to the hall. Same world-JSON save commits the pending
    /// receipt and delivered stock; no second crop receipt is created.
    pub fn warehouse_quantity(&self, good: Good) -> u32 {
        self.warehouse.get(&good).copied().unwrap_or(0)
    }

    /// Physical receipt; reject overflow rather than silently mint or lose items.
    pub fn warehouse_credit(&mut self, good: Good, count: u32) -> Result<(), String> {
        let next = self.warehouse_quantity(good).checked_add(count)
            .ok_or("physical warehouse capacity overflow")?;
        if next > 10_000_000 { return Err("physical warehouse capacity exceeded".into()); }
        self.warehouse.insert(good, next);
        Ok(())
    }

    pub fn warehouse_debit(&mut self, good: Good, count: u32) -> Result<(), String> {
        let current = self.warehouse_quantity(good);
        if current < count { return Err(format!(
            "warehouse lacks {}: {} available, {} required",
            good.as_str(), current, count
        )); }
        self.warehouse.insert(good, current - count);
        Ok(())
    }

    pub fn hall_materials_missing(&self) -> (u32, u32) {
        (HALL_WOOD_LOGS.saturating_sub(self.warehouse_quantity(Good::Wood)),
         HALL_COBBLESTONE.saturating_sub(self.warehouse_quantity(Good::Stone)))
    }

    pub fn quantity(&self, good: Good) -> u32 {
        self.stock.get(&good).copied().unwrap_or(0)
    }

    /// Unique, stable vanilla entity name for non-destructive actor reattachment.
    /// This is a convenience label, not an authenticated NPC identity.
    pub fn actor_name(&self, resident: &Resident) -> String {
        format!("{} ({}) [civ:{}:{}]", resident.name, resident.job.name(), self.id, resident.id)
    }

    fn add(&mut self, good: Good, amount: u32) {
        let old = self.quantity(good);
        self.stock.insert(good, old.saturating_add(amount));
    }

    pub(crate) fn take(&mut self, good: Good, amount: u32) -> bool {
        if self.quantity(good) < amount {
            return false;
        }
        *self.stock.entry(good).or_default() -= amount;
        true
    }

    pub(crate) fn day(&mut self) -> Vec<String> {
        let mut events = Vec::new();
        self.add(Good::Wheat, self.count(Job::Farmer).saturating_mul(4));
        self.add(Good::Wood, self.count(Job::Lumberjack).saturating_mul(3));
        self.add(Good::Stone, self.count(Job::Miner).saturating_mul(2));
        for _ in 0..self.count(Job::Baker) {
            if self.take(Good::Wheat, 3) {
                self.add(Good::Bread, 2);
            }
        }
        let food = self.population().div_ceil(3);
        if self.take(Good::Bread, food) {
            self.starving_days = 0;
            self.morale = (self.morale + 1).min(100);
        } else {
            self.starving_days = self.starving_days.saturating_add(1);
            self.morale = (self.morale - 8).max(-100);
            events.push(format!("{}: food shortage ({} days)", self.name, self.starving_days));
        }
        // Avoid growing infinitely many abstract houses with a fixed population.
        let desired_houses = self.population().div_ceil(3).saturating_add(1);
        // Homes sold to players or commissioned privately are no longer
        // public stock. The abstract economy must eventually replace them
        // using its normal wood/stone costs rather than mint free houses.
        let private_houses = self.player_parcels.iter()
            .filter(|plot| plot.commissioned).count() as u32;
        let public_houses = self.houses.saturating_sub(private_houses);
        if self.count(Job::Builder) > 0 && self.population() > 0 && public_houses < desired_houses {
            // Buildings are simulated here. Physical blocks are not placed until a
            // construction adapter explicitly commits a structure in the level.
            if self.take(Good::Wood, 8) {
                if self.take(Good::Stone, 6) {
                    self.houses = self.houses.saturating_add(1);
                    self.pending_visual_buildings = self.pending_visual_buildings.saturating_add(1);
                    events.push(format!("{}: completed house #{} (simulation)", self.name, self.houses));
                } else {
                    self.add(Good::Wood, 8);
                }
            }
        }
        events
    }
}
