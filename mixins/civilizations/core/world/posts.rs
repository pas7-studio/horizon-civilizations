// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Reserve a permanent work camp. Sites are validated against the world
    /// by the native adapter before this is called; this only keeps the
    /// domain bounded and unique.
    pub fn add_post(
        &mut self, village_id: u64, kind: PostKind, origin: [i32; 3],
    ) -> Result<(), String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        if village.posts.len() >= MAX_POSTS {
            return Err("camp limit reached".into());
        }
        if village.posts.iter().any(|post| post.kind == kind) {
            return Err("this village already has that camp".into());
        }
        if village.posts.iter().any(|post| post.origin == origin) {
            return Err("a camp already stands here".into());
        }
        let dx = i64::from(origin[0]) - i64::from(village.center[0]);
        let dz = i64::from(origin[2]) - i64::from(village.center[2]);
        let distance = dx * dx + dz * dz;
        if !(8 * 8..=80 * 80).contains(&distance) {
            return Err("camp must be 8..80 blocks from the village center".into());
        }
        if i64::from(origin[0]).abs() > 29_999_990
            || i64::from(origin[2]).abs() > 29_999_990
            || !(-2048..=2048).contains(&origin[1])
        {
            return Err("camp outside safe world bounds".into());
        }
        village.posts.push(WorkPost { kind, origin, worker_id: None });
        Ok(())
    }

    pub fn post_origin(&self, village_id: u64, kind: PostKind) -> Option<[i32; 3]> {        self.settlements.get(&village_id)?
            .posts.iter().find(|post| post.kind == kind).map(|post| post.origin)
    }

    /// Bind the camp to one resident, once. A different worker is refused so
    /// delivery evidence stays attributable to a single UUID-bound actor.
    pub fn set_post_worker(
        &mut self, village_id: u64, kind: PostKind, worker: u64,
    ) -> Result<bool, String> {
        let village = self.settlements.get_mut(&village_id).ok_or("unknown settlement")?;
        let resident = village.residents.iter()
            .find(|r| r.id == worker && r.alive)
            .ok_or("unknown or inactive resident")?;
        if resident.job != kind.job() {
            return Err("resident does not hold that camp's job".into());
        }
        let post = village.posts.iter_mut()
            .find(|post| post.kind == kind).ok_or("no such camp")?;
        match post.worker_id {
            Some(existing) if existing == worker => Ok(false),
            Some(_) => Err("camp is already assigned to another worker".into()),
            None => { post.worker_id = Some(worker); Ok(true) }
        }
    }
}
