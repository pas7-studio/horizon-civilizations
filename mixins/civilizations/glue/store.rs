// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

pub(super) struct ActiveWorld {
    pub(super) path: PathBuf,
    pub(super) world: Option<World>,
    pub(super) problem: Option<String>,
    pub(super) physical_ticks: u64,
}

pub(super) static ACTIVE: Mutex<Option<ActiveWorld>> = Mutex::new(None);

pub(super) fn load(path: PathBuf) -> ActiveWorld {
    let file = path.join(SAVE);
    // Either artifact signals an interrupted transaction. In particular,
    // world.json.tmp may be the ONLY persisted bytes of a new settlement
    // when the very first world save was interrupted before rename. Never
    // mistake that for an empty world and overwrite the recovery evidence.
    for remnant in ["world.json.swap", "world.json.tmp"] {
        if path.join(remnant).exists() {
            let message = format!(
                "interrupted save detected ({remnant}). Back up the world folder and inspect primary/backup/temporary save files before restarting"
            );
            return ActiveWorld { path, world: None, problem: Some(message), physical_ticks: 0 };
        }
    }
    let result = match fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str::<World>(&text)
            .map_err(|e| format!("{}: invalid save: {e}", file.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(World::default()),
        Err(e) => Err(format!("{}: {e}", file.display())),
    }.and_then(|world| {
        world.validate().map(|()| world)
    });
    let (world, problem) = match result {
        Ok(world) => (Some(world), None),
        Err(error) => {
            log::error!("[civilizations] refusing to touch damaged data: {error}");
            (None, Some(error))
        }
    };
    ActiveWorld { path, world, problem, physical_ticks: 0 }
}

pub(super) fn active(path: PathBuf) -> MutexGuard<'static, Option<ActiveWorld>> {
    let mut locked = ACTIVE.lock().unwrap_or_else(|poison| poison.into_inner());
    match locked.as_ref() {
        None => {
            *locked = Some(load(path));
        }
        Some(old) if old.path == path => {}
        Some(old) if old.world.is_none() => {
            // No in-memory state to lose; invalid on-disk world remains untouched.
            *locked = Some(load(path));
        }
        Some(old) => {
            // Retry saving even if a previous operation had failed. Never
            // discard a valid mutated world unless persistence succeeds.
            if let Err(error) = save(old) {
                log::error!(
                    "[civilizations] refusing world switch; outgoing save failed: {error}"
                );
                // All hooks for the requested new world stop on this problem.
                if let Some(state) = locked.as_mut() {
                    state.problem = Some(format!(
                        "world switch prevented; outgoing save failed: {error}"
                    ));
                }
                return locked;
            }
            *locked = Some(load(path));
        }
    }
    locked
}

pub(super) fn save(active: &ActiveWorld) -> Result<(), String> {
    let world = active.world.as_ref().ok_or("world data unavailable")?;
    world.validate().map_err(|e| format!("refusing to save invalid world: {e}"))?;
    fs::create_dir_all(&active.path).map_err(|e| e.to_string())?;
    let output = serde_json::to_vec_pretty(world).map_err(|e| e.to_string())?;
    let dest = active.path.join(SAVE);
    // A stale swap marks an interrupted save. Refuse it before touching
    // either the previous backup or the next temporary file.
    let swap = active.path.join("world.json.swap");
    let temp = active.path.join("world.json.tmp");
    if swap.exists() || temp.exists() {
        return Err("pending world.json.swap or world.json.tmp from interrupted save; inspect and recover before continuing".into());
    }
    let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&output).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    // Keep a recoverable older save. A failed backup aborts the write rather
    // than destroying the only known-good state.
    if dest.exists() {
        fs::copy(&dest, active.path.join("world.json.bak")).map_err(|e| e.to_string())?;
    }
    // Windows cannot rename a file over an existing destination. Keep a
    // recoverable swap file between the two renames; never overwrite a stale
    // swap left by a crash (that must be inspected manually).
    if dest.exists() { fs::rename(&dest, &swap).map_err(|e| e.to_string())?; }
    if let Err(e) = fs::rename(&temp, &dest) {
        if swap.exists() { let _ = fs::rename(&swap, &dest); }
        return Err(e.to_string());
    }
    // The new primary is already committed. A cleanup failure must not be
    // reported as an uncommitted trade after inventory items have moved.
    if swap.exists() {
        if let Err(e) = fs::remove_file(&swap) {
            log::warn!("[civilizations] committed world save, but stale swap needs cleanup: {e}");
        }
    }
    Ok(())
}

pub fn closed() {
    let mut guard = ACTIVE.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(state) = guard.as_ref() {
        if state.world.is_some() {
            // Keep in-memory data on write failure until the disk is fixed,
            // instead of silently discarding it on close.
            if let Err(error) = save(state) {
                log::error!(
                    "[civilizations] shutdown save failed; retaining in-memory world: {error}"
                );
                if let Some(state) = guard.as_mut() {
                    state.problem = Some(format!("shutdown save failed: {error}"));
                }
                return;
            }
        }
    }
    *guard = None;
}

/// World-JSON operations commit before any external physical side effect.
pub(super) fn save_autonomous(state: &mut ActiveWorld, prior: World) -> Result<(), String> {
    if let Err(error) = save(state) {
        state.world = Some(prior);
        return Err(format!("persistence: {error}"));
    }
    Ok(())
}
