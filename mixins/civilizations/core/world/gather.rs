// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Re-evaluate live settlement ownership before executing an already
    /// planned physical resource action. Land may be bought or new buildings
    /// reserved between the initial plan and the worker reaching its target.
    /// This is shared with planning so neither path can bypass a new parcel.
    pub fn gather_site_unclaimed(
        &self, village_id: u64, position: [i32; 3], stand: [i32; 3],
    ) -> bool {
        let Some(village) = self.settlements.get(&village_id) else { return false; };
        let protected = |other: &Settlement| {
            near_plot(other.center, position) || near_plot(other.center, stand)
                || other.player_parcels.iter().any(|lot|
                    near_plot(lot.origin, position) || near_plot(lot.origin, stand))
                || other.visual_sites.iter().any(|site|
                    near_plot(*site, position) || near_plot(*site, stand))
                || other.visual_job.as_ref().is_some_and(|job|
                    near_plot(job.origin, position) || near_plot(job.origin, stand))
                || other.farm.as_ref().is_some_and(|farm|
                    near_plot(farm.origin, position) || near_plot(farm.origin, stand))
                || other.bakery.as_ref().is_some_and(|b|
                    near_plot(b.origin, position) || near_plot(b.origin, stand))
        };
        !self.settlements.values().filter(|other| other.dimension == village.dimension)
            .any(protected)
    }

    /// Planning creates no items. Both native and domain checks forbid
    /// extraction inside the town hall, another structure or player lot.
    pub fn plan_gather(
        &mut self, village_id: u64, good: Good, position: [i32; 3], stand: [i32; 3]
    ) -> Result<(), String> {
        let Some(worker_job) = good.job() else {
            return Err("only natural wood, stone, sand or dirt may be gathered".into());
        };
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if v.gather_job.is_some() { return Err("another physical gather is pending".into()); }
        if !v.autonomy_enabled { return Err("village autonomy paused".into()); }
        if v.count(worker_job) == 0 { return Err("no qualified living worker".into()); }
        if i64::from(position[0]).abs() > 29_999_988
            || i64::from(position[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&position[1]) {
            return Err("gather point outside safe world bounds".into());
        }
        let wx = i128::from(stand[0]) - i128::from(position[0]);
        let wy = i128::from(stand[1]) - i128::from(position[1]);
        let wz = i128::from(stand[2]) - i128::from(position[2]);
        if wx * wx + wy * wy + wz * wz > 16 {
            return Err("worker stand is not reachable from resource within 4 blocks".into());
        }
        let dx = i128::from(position[0]) - i128::from(v.center[0]);
        let dz = i128::from(position[2]) - i128::from(v.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("gather only in the village outer work area".into());
        }
        if !self.gather_site_unclaimed(village_id, position, stand) {
            return Err("natural resource overlaps a building, town hall or player claim".into());
        }
        self.settlements.get_mut(&village_id).unwrap().gather_job = Some(GatherJob {
            position, stand, good, worker_id: None, issued: false,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: false,
        });
        Ok(())
    }

    /// The claim rule for an existing plan, identical to the planner's: a
    /// quarry's own pit is exempt from the plot-proximity half of the rule
    /// (its camp stands there by design), everything else is not.
    pub fn gather_claim_ok(
        &self, village_id: u64, good: Good, position: [i32; 3], stand: [i32; 3],
    ) -> bool {
        if self.gather_site_unclaimed(village_id, position, stand) { return true; }
        if good != Good::Stone { return false; }
        let Some(v) = self.settlements.get(&village_id) else { return false; };
        v.posts.iter().any(|post| post.kind == PostKind::Quarry
            && (i64::from(position[0]) - i64::from(post.origin[0])).abs()
                <= i64::from(POST_RADIUS)
            && (i64::from(position[2]) - i64::from(post.origin[2])).abs()
                <= i64::from(POST_RADIUS))
    }

    /// Plan one stone block inside a village's own quarry yard. A pit is dug
    /// where the camp stands, so the ordinary claim check (which protects the
    /// village's own plots) would refuse its own excavation. Only the
    /// yard-radius check replaces it; every other validation is identical.
    pub fn plan_quarry_gather(
        &mut self, village_id: u64, position: [i32; 3], stand: [i32; 3]
    ) -> Result<(), String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        let quarry = v.posts.iter().find(|post| post.kind == PostKind::Quarry)
            .ok_or("this village has no quarry")?;
        if (i64::from(position[0]) - i64::from(quarry.origin[0])).abs() > i64::from(POST_RADIUS)
            || (i64::from(position[2]) - i64::from(quarry.origin[2])).abs() > i64::from(POST_RADIUS)
        {
            return Err("stone is outside the quarry yard".into());
        }
        if v.gather_job.is_some() { return Err("another physical gather is pending".into()); }
        if !v.autonomy_enabled { return Err("village autonomy paused".into()); }
        if v.count(Job::Miner) == 0 { return Err("no qualified living worker".into()); }
        if i64::from(position[0]).abs() > 29_999_988
            || i64::from(position[2]).abs() > 29_999_988
            || !(-2046..=2046).contains(&position[1]) {
            return Err("gather point outside safe world bounds".into());
        }
        let wx = i128::from(stand[0]) - i128::from(position[0]);
        let wy = i128::from(stand[1]) - i128::from(position[1]);
        let wz = i128::from(stand[2]) - i128::from(position[2]);
        if wx * wx + wy * wy + wz * wz > 16 {
            return Err("worker stand is not reachable from resource within 4 blocks".into());
        }
        let dx = i128::from(position[0]) - i128::from(v.center[0]);
        let dz = i128::from(position[2]) - i128::from(v.center[2]);
        if !(12_i128 * 12..=80_i128 * 80).contains(&(dx * dx + dz * dz)) {
            return Err("gather only in the village outer work area".into());
        }
        self.settlements.get_mut(&village_id).unwrap().gather_job = Some(GatherJob {
            position, stand, good: Good::Stone, worker_id: None, issued: false,
            native_edit_confirmed: false, pickup_witnessed: false, blocked: false,
        });
        Ok(())
    }

    /// Start a whole SHORT tree only after the adapter observed all native
    /// trunk and crown blocks. No wood is credited until individual segments
    /// have actually disappeared and the bound lumberjack returns to hall.
    pub fn plan_short_oak_stem(
        &mut self, village_id: u64, root: [i32; 3], stand: [i32; 3], height: u8,
    ) -> Result<(), String> {
        if !(3..=4).contains(&height) {
            return Err("only 3-4-log naturally reachable oak stems are supported".into());
        }
        let last_y = root[1].checked_add(i32::from(height) - 1)
            .ok_or("oak tree height overflows native world coordinate")?;
        if !(-2046..=2046).contains(&last_y) {
            return Err("tree reaches outside safe native build height".into());
        }
        if self.settlements.get(&village_id)
            .ok_or("unknown village")?.oak_stem.is_some() {
            return Err("previous oak stem still pending".into());
        }
        self.plan_gather(village_id, Good::Wood, root, stand)?;
        self.settlements.get_mut(&village_id).unwrap().oak_stem =
            Some(OakStem { root, stand, height, cursor: 0 });
        Ok(())
    }

    /// Bind the selected UUID-linked citizen's stable resident ID to the
    /// native job and save before any external block mutation. The adapter
    /// MUST independently verify that this resident's UUID-bound entity is
    /// loaded, at the recorded work stand and within genuine 3D block reach.
    /// An already assigned job can never be silently stolen by another NPC.
    pub fn assign_gather_worker(
        &mut self, village_id: u64, resident_id: u64,
    ) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id)
            .ok_or("unknown settlement")?;
        let job = village.gather_job.as_ref().ok_or("no gather job")?;
        if job.issued || job.blocked {
            return Err("cannot assign an already issued or blocked gather".into());
        }
        let required = if job.good == Good::Wood { Job::Lumberjack }
            else if job.good == Good::Stone { Job::Miner }
            else { return Err("unsupported native gather good".into()); };
        if !village.residents.iter().any(|resident|
            resident.alive && resident.id == resident_id && resident.job == required) {
            return Err("worker is not a living qualified settlement resident".into());
        }
        if job.worker_id.is_some_and(|assigned| assigned != resident_id) {
            return Err("gather already assigned to a different worker".into());
        }
        village.gather_job.as_mut().unwrap().worker_id = Some(resident_id);
        Ok(())
    }

    pub fn mark_gather_issued(&mut self, village_id: u64) -> Result<(), String> {
        let job = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?
            .gather_job.as_mut().ok_or("no planned gather")?;
        if job.blocked || job.issued { return Err("gather may not be re-issued".into()); }
        if job.worker_id.is_none() {
            return Err("native gather must have a durably assigned citizen before issue".into());
        }
        job.issued = true;
        Ok(())
    }

    /// The adapter must call this ONLY after its native conditional
    /// command returned success and a FRESH read of that loaded source
    /// block returned air. Save this acknowledgement before pickup.
    /// Domain code cannot itself attest to native engine observations.
    pub fn mark_gather_native_edit_confirmed(&mut self, village_id: u64)
        -> Result<(), String> {
        let job = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?
            .gather_job.as_mut().ok_or("no gather job")?;
        if !job.issued || job.blocked || job.worker_id.is_none()
            || job.native_edit_confirmed {
            return Err("native edit receipt requires a new issued assigned gather".into());
        }
        job.native_edit_confirmed = true;
        Ok(())
    }

    /// The bridge has just observed the issued source as air AND the same
    /// UUID-linked worker at the saved loaded source work stand, inside
    /// actual 3D reach. Do not mark pickup before the native block edit.
    /// This journal step is saved before the worker gets a return route.
    pub fn witness_gather_pickup(
        &mut self, village_id: u64, resident_id: u64,
    ) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.gather_job.as_mut().ok_or("no gather job")?;
        if !job.issued || !job.native_edit_confirmed
            || job.blocked || job.pickup_witnessed {
            return Err("gather not awaiting a first source pickup witness".into());
        }
        if job.worker_id != Some(resident_id) {
            return Err("source pickup identity does not match assigned worker".into());
        }
        job.pickup_witnessed = true;
        Ok(())
    }

    /// A planned job whose block was removed externally is NOT a harvest:
    /// either player intervention or a crash before issued-state persistence
    /// must fail closed, never credit phantom inventory.
    pub fn block_gather(&mut self, village_id: u64) -> Result<(), String> {
        let job = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?
            .gather_job.as_mut().ok_or("no gather job")?;
        job.blocked = true;
        Ok(())
    }

    /// Discard an obstructed gather without any material credit, then retry
    /// another site on the next planning cycle; no foreign blocks are edited.
    pub fn discard_blocked_gather(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if !v.gather_job.as_ref().is_some_and(|job| job.blocked) {
            return Err("only a blocked gather can be discarded".into());
        }
        v.gather_job = None;
        v.oak_stem = None;
        Ok(())
    }

    /// Explicit operator decision after checking native block evidence.
    /// The adapter MUST require an OP in the correct dimension and a loaded,
    /// observed source block (or air). This method only abandons a blocked,
    /// previously issued receipt: it never credits wood/stone or issues edits.
    pub fn abandon_blocked_issued_gather(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get(&village_id).ok_or("unknown settlement")?;
        if !v.gather_job.as_ref().is_some_and(|job| job.issued && job.blocked) {
            return Err("only a blocked issued gather may be manually abandoned".into());
        }
        let name = v.name.clone();
        self.discard_blocked_gather(village_id)?;
        self.event(format!("{name}: operator abandoned an uncertain gather; zero resource credit"));
        Ok(())
    }

    pub fn confirm_gather(&mut self, village_id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let job = v.gather_job.as_ref().ok_or("no gather job")?;
        if !job.issued || !job.native_edit_confirmed
            || !job.pickup_witnessed || job.blocked {
            return Err("unproven pickup or gather cannot be credited".into());
        }
        let good = job.good;
        let pos = job.position;
        let worker_id = job.worker_id.ok_or("unassigned gather cannot be credited")?;
        // Compute the exact next source before committing inventory, so a
        // corrupted height cannot partially credit or advance this stem.
        let continuation = if let Some(stem) = &v.oak_stem {
            let next = stem.cursor.checked_add(1).ok_or("oak cursor overflow")?;
            if next < stem.height {
                let y = stem.root[1].checked_add(i32::from(next))
                    .ok_or("oak height overflow")?;
                Some((next, GatherJob {
                    position: [stem.root[0], y, stem.root[2]],
                    stand: stem.stand, good: Good::Wood,
                    worker_id: Some(worker_id), issued: false,
                    native_edit_confirmed: false, pickup_witnessed: false, blocked: false,
                }))
            } else {
                None
            }
        } else { None };
        v.warehouse_credit(good, 1)?;
        if let Some((cursor, next_job)) = continuation {
            v.oak_stem.as_mut().unwrap().cursor = cursor;
            v.gather_job = Some(next_job);
        } else {
            v.oak_stem = None;
            v.gather_job = None;
        }
        let name = v.name.clone();
        self.event(format!("{name}: physically gathered one {} at {pos:?}", good.as_str()));
        Ok(())
    }
}
