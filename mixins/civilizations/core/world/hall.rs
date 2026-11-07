// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Reserve the actual resource budget for the village's one central hall.
    /// No free town hall spawns in a new or migrated world.
    pub fn reserve_town_hall(&mut self, village_id: u64) -> Result<bool, String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if !v.autonomy_enabled || v.hall_complete || v.hall_job.is_some() { return Ok(false); }
        if v.hall_materials_missing() != (0, 0) {
            return Ok(false);
        }
        if i64::from(v.center[0]).abs() > 29_999_988
            || i64::from(v.center[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&v.center[1]) {
            return Err("civic center outside safe build bounds".into());
        }
        // Domain claims are only a first pass; native preflight checks all
        // world blocks, loaded chunks and a safe location BEFORE this call.
        if self.settlements.values().filter(|s| s.dimension == v.dimension).any(|s| {
            s.player_parcels.iter().any(|plot| near_plot(plot.origin, v.center))
            || s.visual_sites.iter().any(|site| near_plot(*site, v.center))
            || s.visual_job.as_ref().is_some_and(|job| near_plot(job.origin, v.center))
            || s.farm.as_ref().is_some_and(|farm| near_plot(farm.origin, v.center))
            || s.bakery.as_ref().is_some_and(|b| near_plot(b.origin, v.center))
        }) {
            return Err("civic hall site overlaps a registered building or player parcel".into());
        }
        let v = self.settlements.get_mut(&village_id).unwrap();
        v.warehouse_debit(Good::Wood, HALL_WOOD_LOGS)?;
        v.warehouse_debit(Good::Stone, HALL_COBBLESTONE)?;
        let center = v.center;
        v.hall_job = Some(VisualBuildJob { origin: center, cursor: 0, blocked: false });
        let name = v.name.clone();
        self.event(format!("{name}: civic hall construction funded and started at {center:?}"));
        Ok(true)
    }

    /// A full native block read is mandatory before recording completion.
    pub fn finish_town_hall(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.hall_job.as_ref().ok_or("no civic hall construction")?;
        if job.cursor != town_hall_plan(job.origin).len() {
            return Err("civic hall building plan incomplete".into());
        }
        let name = v.name.clone();
        v.hall_job = None;
        v.hall_complete = true;
        self.event(format!("{name}: physical civic hall completed"));
        Ok(())
    }
}
