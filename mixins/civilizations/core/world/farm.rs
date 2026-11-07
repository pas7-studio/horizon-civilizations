// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Open the next step of the quarry staircase. The depth is bounded: a
    /// pit is a real hole in real ground, never a virtual stone source.
    pub fn advance_quarry(&mut self, village_id: u64) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if village.posts.iter().all(|post| post.kind != PostKind::Quarry) {
            return Err("this village has no quarry".into());
        }
        if village.quarry_step >= MAX_QUARRY_STEPS {
            return Err("quarry pit is deep enough".into());
        }
        village.quarry_step += 1;
        Ok(())
    }

    /// Reserve a real 3x3 growing patch without manufacturing economic stock.
    /// The Horizon adapter must preflight the terrain before calling this.
    pub fn start_farm(&mut self, village_id: u64, origin: [i32; 3]) -> Result<(), String> {
        let village = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if village.farm.is_some() { return Err("farm already assigned".into()); }
        // No farmer is required to reserve the plot: the field is the real
        // workplace, and the orchestrator assigns a farmer once it exists.
        // Requiring one first would deadlock a village that has none.
        if i64::from(origin[0]).abs() > 29_999_988 || i64::from(origin[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&origin[1]) {
            return Err("farm outside safe build coordinates".into());
        }
        let distance = |a: [i32; 3], b: [i32; 3]| {
            let dx = i128::from(a[0]) - i128::from(b[0]);
            let dz = i128::from(a[2]) - i128::from(b[2]);
            dx * dx + dz * dz
        };
        if !(12_i128 * 12..=80_i128 * 80).contains(&distance(village.center, origin)) {
            return Err("farm must be 12..80 blocks from the village center".into());
        }
        for s in self.settlements.values().filter(|s| s.dimension == village.dimension) {
            if s.player_parcels.iter().any(|plot| near_plot(plot.origin, origin)) {
                return Err("farm site is owned by a player".into());
            }
            if s.farm.as_ref().is_some_and(|p| distance(p.origin, origin) < 9 * 9)
                || s.visual_job.as_ref().is_some_and(|p| distance(p.origin, origin) < 9 * 9)
                || s.visual_sites.iter().any(|&site| distance(site, origin) < 9 * 9)
                || s.bakery.as_ref().is_some_and(|b| distance(b.origin, origin) < 9 * 9) {
                return Err("farm overlaps an existing civilization structure".into());
            }
        }
        self.settlements.get_mut(&village_id).unwrap().farm = Some(FarmPlot {
            origin, stage: FarmStage::Preparing, cursor: 0, blocked: false, completed_cycles: 0,
            harvest_sequence: 0, harvest_receipts: Vec::new(),
        });
        Ok(())
    }

    /// The adapter must first inspect the unblocked plot against the real
    /// loaded blocks before issuing an operator-approved recovery.
    pub fn recover_farm(&mut self, village_id: u64) -> Result<(), String> {
        let farm = self.settlements.get_mut(&village_id)
            .ok_or("unknown settlement")?.farm.as_mut().ok_or("no farm")?;
        if !farm.blocked { return Err("farm is not blocked".into()); }
        farm.blocked = false;
        Ok(())
    }

    pub fn advance_farm(&mut self, village_id: u64, decision: FarmDecision) -> Result<bool, String> {
        let world_day = self.day;
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        // Harvest credit must be validated BEFORE mutating its receipt/cursor.
        // Each receipt and warehouse credit commit in the same world JSON save.
        let credit_harvest = matches!(&decision, FarmDecision::StageComplete)
            && village.farm.as_ref().is_some_and(|farm| farm.stage == FarmStage::Harvesting);
        let new_wheat = if credit_harvest {
            let pending = village.wheat_in_transit.checked_add(8)
                .ok_or("wheat farm delivery overflow")?;
            if pending > 10_000_000 { return Err("wheat delivery queue exceeded".into()); }
            Some(pending)
        } else { None };
        let farm = village.farm.as_mut().ok_or("no farm")?;
        match decision {
            FarmDecision::Unloaded | FarmDecision::Paused | FarmDecision::Place { .. } => Ok(false),
            FarmDecision::InvalidCursor => Err("farm has invalid cursor".into()),
            FarmDecision::Blocked { .. } => {
                farm.blocked = true;
                Ok(true)
            }
            FarmDecision::Rewind(cursor) => {
                farm.cursor = cursor;
                Ok(true)
            }
            FarmDecision::Advance => {
                let limit = if farm.stage == FarmStage::Preparing { farm_plan(farm.origin).len() } else { 8 };
                if farm.cursor >= limit {
                    return Err("cannot advance farm beyond phase step count".into());
                }
                farm.cursor += 1;
                Ok(true)
            }
            FarmDecision::StageComplete => {
                let steps = if farm.stage == FarmStage::Preparing { farm_plan(farm.origin).len() } else { 8 };
                if farm.stage != FarmStage::Growing && farm.cursor != steps {
                    return Err("farm phase cannot complete before all blocks are verified".into());
                }
                farm.stage = match farm.stage {
                    FarmStage::Preparing => FarmStage::Growing,
                    FarmStage::Growing => FarmStage::Harvesting,
                    FarmStage::Harvesting => {
                        let sequence = farm.harvest_sequence.checked_add(1)
                            .ok_or("harvest sequence overflow")?;
                        farm.harvest_sequence = sequence;
                        farm.harvest_receipts.push(HarvestReceipt {
                            sequence, world_day, crops: 8,
                        });
                        if farm.harvest_receipts.len() > MAX_HARVEST_RECEIPTS {
                            farm.harvest_receipts.remove(0);
                        }
                        // Receipt + phase + physical warehouse wheat credit
                        // are committed in ONE JSON save. No abstract stock,
                        // native item drops or player coins are credited.
                        FarmStage::Replanting
                    },
                    FarmStage::Replanting => {
                        farm.completed_cycles = farm.completed_cycles.saturating_add(1);
                        FarmStage::Growing
                    }
                };
                farm.cursor = 0;
                // Harvest is physically observed and has exactly one
                // receipt. The stock remains in the farmer's transit queue
                // until a live UUID-bound farmer returns to the hall.
                if let Some(count) = new_wheat {
                    village.wheat_in_transit = count;
                }
                Ok(true)
            }
        }
    }

    /// Each selected plot is considered only when its chunk is already loaded.
    pub fn next_farm(&self, dimension: &str, sequence: usize,
                     mut loaded: impl FnMut([i32; 3]) -> bool) -> Option<(u64, FarmPlot)> {
        let eligible: Vec<_> = self.settlements.values()
            .filter(|s| s.dimension == dimension)
            .filter_map(|s| {
                let farm = s.farm.as_ref()?;
                (!farm.blocked && loaded(farm.origin)).then(|| (s.id, farm.clone()))
            })
            .collect();
        if eligible.is_empty() { None } else { Some(eligible[sequence % eligible.len()].clone()) }
    }
}
