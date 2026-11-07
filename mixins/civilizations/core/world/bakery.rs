// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;
use std::collections::{BTreeMap};

impl World {
    /// Called only after the native server sees an actual UUID-bound
    /// farmer return to the hall. Same world-JSON save commits the pending
    /// receipt and delivered stock; no second crop receipt is created.
    pub fn deliver_farm_wheat(&mut self, village_id: u64) -> Result<bool, String> {
        let (name, delivered) = {
            let v = self.settlements.get_mut(&village_id).ok_or("unknown village")?;
            if v.wheat_in_transit == 0 {
                return Ok(false);
            }
            let next = v
                .warehouse_quantity(Good::Wheat)
                .checked_add(v.wheat_in_transit)
                .ok_or("wheat delivery overflow")?;
            if next > 10_000_000 {
                return Err("village wheat receiving capacity exceeded".into());
            }
            let delivered = v.wheat_in_transit;
            v.warehouse.insert(Good::Wheat, next);
            v.wheat_in_transit = 0;
            (v.name.clone(), delivered)
        };
        self.event(format!(
            "{name}: farmer brought {delivered} verified wheat to the hall"
        ));
        Ok(true)
    }

    /// The default-on village reserves a 5x5 public bakery plot after its
    /// hall is constructed. The native server must independently preflight
    /// loaded real ground and clear air before calling this domain method.
    pub fn start_bakery(&mut self, id: u64, origin: [i32; 3]) -> Result<(), String> {
        let village = self.settlements.get(&id).ok_or("unknown village")?;
        if !village.autonomy_enabled || !village.hall_complete {
            return Err("village needs a completed hall and active autonomy".into());
        }
        if village.bakery.is_some() { return Err("bakery already reserved or constructed".into()); }
        // The builder raises the workshop; the baker seat follows the
        // reserved bakery. Requiring a baker first would deadlock a village
        // that has never had a bakery, which is exactly the honest state.
        if village.count(Job::Builder) == 0 {
            return Err("bakery requires a living builder".into());
        }
        if i64::from(origin[0]).abs() > 29_999_988
            || i64::from(origin[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&origin[1]) {
            return Err("bakery plot outside safe construction bounds".into());
        }
        let dx = i128::from(origin[0]) - i128::from(village.center[0]);
        let dz = i128::from(origin[2]) - i128::from(village.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("bakery must be 12..80 blocks from its hall".into());
        }
        for other in self.settlements.values().filter(|s| s.dimension == village.dimension) {
            if near_plot(other.center, origin)
                || other.player_parcels.iter().any(|p| near_plot(p.origin, origin))
                || other.visual_sites.iter().any(|s| near_plot(*s, origin))
                || other.visual_job.as_ref().is_some_and(|job| near_plot(job.origin, origin))
                || other.farm.as_ref().is_some_and(|p| near_plot(p.origin, origin))
                || other.bakery.as_ref().is_some_and(|b| near_plot(b.origin, origin)) {
                return Err("bakery would overlap an owned plot or another physical structure".into());
            }
        }
        let v = self.settlements.get_mut(&id).unwrap();
        v.bakery = Some(BakeryWorkshop {
            origin,
            building: Some(VisualBuildJob { origin, cursor: 0, blocked: false }),
            complete: false, supplies_reserved: false,
            construction_escrow: BTreeMap::new(),
            wheat_escrow: 0, bread_in_transit: 0, batches: 0,
        });
        let name = v.name.clone();
        self.event(format!("{name}: planned real public bakery at {origin:?}"));
        Ok(())
    }

    /// Materials are physically extracted from real hall barrels by the
    /// native bridge BEFORE this call, into the bakery-specific escrow.
    pub fn reserve_bakery_supplies(&mut self, id: u64) -> Result<bool, String> {
        let bakery = self.settlements.get_mut(&id).ok_or("unknown village")?
            .bakery.as_mut().ok_or("no bakery project")?;
        if bakery.complete || bakery.supplies_reserved { return Ok(false); }
        let job = bakery.building.as_ref().ok_or("no bakery construction job")?;
        if job.blocked || job.cursor != 0 {
            return Err("unfunded partial bakery job must be inspected".into());
        }
        if bakery.construction_escrow.get(&Good::Wood).copied().unwrap_or(0) < BAKERY_WOOD_LOGS
            || bakery.construction_escrow.get(&Good::Stone).copied().unwrap_or(0) < BAKERY_COBBLESTONE {
            return Ok(false);
        }
        *bakery.construction_escrow.get_mut(&Good::Wood).unwrap() -= BAKERY_WOOD_LOGS;
        *bakery.construction_escrow.get_mut(&Good::Stone).unwrap() -= BAKERY_COBBLESTONE;
        bakery.supplies_reserved = true;
        Ok(true)
    }

    pub fn finish_bakery(&mut self, id: u64) -> Result<(), String> {
        let (name, origin) = {
            let v = self.settlements.get_mut(&id).ok_or("unknown village")?;
            let bakery = v.bakery.as_mut().ok_or("no bakery")?;
            let job = bakery.building.as_ref().ok_or("no bakery construction job")?;
            if bakery.complete || !bakery.supplies_reserved
                || job.cursor != bakery_plan(job.origin).len() {
                return Err("unfunded or physically incomplete bakery cannot open".into());
            }
            let origin = bakery.origin;
            bakery.building = None;
            bakery.complete = true;
            (v.name.clone(), origin)
        };
        self.event(format!("{name}: verified bakery opened at {origin:?}"));
        Ok(())
    }

    /// At the physically present crafting table, turn three actually
    /// extracted native wheat items into ONE bread. Output is a saved
    /// delivery receipt, not an abstract daily-production bonus.
    pub fn craft_bread(&mut self, id: u64) -> Result<bool, String> {
        let village = self.settlements.get_mut(&id).ok_or("unknown village")?;
        if village.count(Job::Baker) == 0 { return Ok(false); }
        let b = village.bakery.as_mut().ok_or("no bakery")?;
        if !b.complete || b.wheat_escrow < BREAD_WHEAT_RECIPE
            || b.bread_in_transit >= 8 { return Ok(false); }
        let next = b.batches.checked_add(1).ok_or("bread production counter overflow")?;
        b.wheat_escrow -= BREAD_WHEAT_RECIPE;
        b.bread_in_transit += 1;
        b.batches = next;
        let name = village.name.clone();
        self.event(format!("{name}: real baker made one bread from three native wheat items"));
        Ok(true)
    }

    /// A UUID-bound baker must first return to the hall; then their
    /// completed bread enters the steward's bounded delivery queue.
    pub fn deliver_bakery_bread(&mut self, id: u64) -> Result<bool, String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown village")?;
        let pending = v.bakery.as_ref().ok_or("no bakery")?.bread_in_transit;
        if pending == 0 { return Ok(false); }
        v.warehouse_credit(Good::Bread, pending)?;
        let b = v.bakery.as_mut().unwrap();
        b.bread_in_transit = 0;
        let name = v.name.clone();
        self.event(format!("{name}: baker delivered {pending} crafted bread to the hall"));
        Ok(true)
    }
}
