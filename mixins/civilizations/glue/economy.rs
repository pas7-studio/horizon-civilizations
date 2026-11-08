// Horizon adapter: see glue/mod.rs for the module map.
use super::*;

/// One bounded planning step per 100 loaded level ticks. NPCs choose free
/// natural plots *inside their settlement radius* and start actual physical
/// jobs. Paid player parcels are hard exclusions in BOTH domain paths.
pub(super) fn autonomous_village_step(game: &mut Game<'_>, state: &mut ActiveWorld) -> Result<(), String> {
    let dimension = game.dim.id().to_owned();
    let enabled: Vec<u64> = state.world.as_ref().unwrap().settlements.values()
        .filter(|v| v.dimension == dimension && v.autonomy_enabled)
        .map(|v| v.id).collect();
    if enabled.is_empty() { return Ok(()); }
    let index = (state.physical_ticks / 100) as usize % enabled.len();
    let id = enabled[index];
    // Roles follow real workplaces. Do this before any planner decision or
    // the first spawn so residents never hold jobs the village cannot give.
    staff_village(game, state, id)?;
    if !state.world.as_ref().unwrap().settlements[&id].actors_spawned {
        return auto_spawn_villagers(game, state, id);
    }
    let prior = state.world.as_ref().unwrap().clone();
    if state.world.as_mut().unwrap().prepare_village_housing(id)? {
        save_autonomous(state, prior)?;
    }
    let village = state.world.as_ref().unwrap().settlements[&id].clone();
    let actors = loaded_actors(game, state.world.as_ref().unwrap(), id);
    let has_builder = village.residents.iter().any(|r| r.alive && r.job == Job::Builder
        && actors.iter().any(|(known, _)| *known == r.id));
    // Recover from a refused/modified extraction without paying any wood or
    // stone; the next planning cycle may choose another untouched source.
    if village.gather_job.as_ref().is_some_and(|job| job.blocked && !job.issued) {
        // No Minecraft edit was dispatched. Discard this stale Planned
        // candidate without a receipt and survey a different source later.
        let before = state.world.as_ref().unwrap().clone();
        state.world.as_mut().unwrap().discard_blocked_gather(id)?;
        save_autonomous(state, before)?;
        return Ok(());
    }
    // A blocked *Issued* gather might already have changed native blocks.
    // Keep its durable receipt quarantined until OP inspection. Planning
    // other civic tasks may continue, but no further gather is scheduled
    // while the uncertain source remains recorded.

    // Resource demand MUST include real native barrel contents, saved
    // incoming deliveries and physically extracted, project-specific
    // reserves. In v0.1.13 looking at only the incoming queue caused
    // villagers to gather indefinitely even with filled real barrels.
    // An unloaded/missing barrel cannot be interpreted as zero stock:
    // pause surveying until both native block entities can be observed.
    let (wood_needed, stone_needed) = if !village.hall_complete {
        village.hall_materials_missing()
    } else {
        // A journalled mutation may already have changed the native barrel
        // without updating saved escrows/receipts. Do not count goods twice
        // or send miners to replace them until the receipt is reconciled.
        if village.barrel_transfer.is_some() || village.market_purchase.is_some() {
            return Ok(());
        }
        let Some(native_stock) = physical_barrel_stock(game, &dimension, village.center)
        else { return Ok(()); };
        crate::game::civilizations_core::physical_construction_shortfall(
            &village, &native_stock,
        )
    };

    if village.gather_job.is_none() {
        // Stone is rarely exposed at the surface of a forest or plains
        // village. A quarry starts as a pit on natural ground, so open one
        // when cobble is short and the village has none yet.
        if stone_needed > 0 && village.posts.iter().all(|post| post.kind != PostKind::Quarry) {
            if let Some(spot) = quarry_start(game, &dimension, village.center) {
                // Never open a pit on top of an existing plot or claim.
                let free = state.world.as_ref().unwrap()
                    .gather_site_unclaimed(id, spot, spot);
                let before = state.world.as_ref().unwrap().clone();
                if free && state.world.as_mut().unwrap()
                    .add_post(id, PostKind::Quarry, spot).is_ok()
                {
                    save_autonomous(state, before)?;
                    log::info!("[civilizations] {} opened a quarry pit at {:?}", village.name, spot);
                    return Ok(());
                }
            }
        }
        let mut work: Vec<(Good, u32, Job)> = vec![
            (Good::Wood, wood_needed, Job::Lumberjack),
            (Good::Stone, stone_needed, Job::Miner),
        ];
        // A sand or dirt pit the player placed is a real workplace, so it is
        // serviced on its own. Nothing builds with sand or dirt yet, so it
        // only keeps a small, explicit buffer in the hall.
        const PIT_TARGET: u32 = 32;
        for pit in [PostKind::Sand, PostKind::Dirt] {
            if !village.posts.iter().any(|post| post.kind == pit) { continue; }
            let stored = if village.hall_complete {
                physical_barrel_stock(game, &dimension, village.center)
                    .and_then(|stock| stock.iter()
                        .find(|(good, _)| *good == pit.good())
                        .map(|(_, count)| *count)).unwrap_or(0)
            } else { 0 };
            let have = village.warehouse_quantity(pit.good()).saturating_add(stored);
            if have < PIT_TARGET {
                work.push((pit.good(), PIT_TARGET - have, pit.job()));
            }
        }
        for (good, need, profession) in work {
            if need == 0 || !village.residents.iter().any(|r| {
                r.alive && r.job == profession
                    && actors.iter().any(|(rid, _)| *rid == r.id)
            }) {
                continue;
            }
            // Inspect at most 320 of 1,681 X/Z candidates per planner
            // turn, with up to 6 nearby Y levels. Never scan thousands of
            // chunks at once; rotate to visit new places on later turns.
            // A work camp turns ad-hoc gathering into a fixed, visible post.
            // Without one, the first found natural source reserves a camp at
            // the worker stand; later scans stay inside its radius.
            let camp_kind = PostKind::for_good(good);
            let camp = village.posts.iter().find(|post| post.kind == camp_kind).map(|post| post.origin);
            let (base_x, base_y, base_z) = {
                let base = camp.unwrap_or(village.center);
                (base[0], base[1], base[2])
            };
            let candidates = 41_usize * 41;
            let start = ((state.physical_ticks / 100) as usize * 101) % candidates;
            for step in 0..320_usize {
                let index = (start + step) % candidates;
                let dx = (index / 41) as i32 * 2 - 40;
                let dz = (index % 41) as i32 * 2 - 40;
                if camp.is_some() && (dx.abs() > POST_RADIUS || dz.abs() > POST_RADIUS) {
                    continue;
                }
                    let (Some(x), Some(z)) = (
                        base_x.checked_add(dx),
                        base_z.checked_add(dz)
                    ) else { continue; };
                    for dy in [-2_i32, -1, 0, 1, 2, 3] {
                        let Some(y) = base_y.checked_add(dy) else { continue; };
                        let position = [x, y, z];
                        if !seems_natural_resource(game, &dimension, position, good) { continue; }
                        let Some(stand) = gather_stand(game, &dimension, position) else { continue; };
                        // No camp yet: reserve this stand as the camp and let
                        // the next planner turn pick the actual resource.
                        if camp.is_none() {
                            let prior = state.world.as_ref().unwrap().clone();
                            if state.world.as_mut().unwrap().add_post(id, camp_kind, stand).is_ok() {
                                save_autonomous(state, prior)?;
                                log::info!("[civilizations] {} opened a {} camp at {:?}",
                                    village.name, camp_kind.name(), stand);
                                return Ok(());
                            }
                        }
                        // Preserve the full pre-image, not merely the
                        // former one-block GatherJob. A stem plan also
                        // mutates oak_stem; both fields must roll back if
                        // persistence fails before the physical job starts.
                        let prior = state.world.as_ref().unwrap().clone();
                        let planned = if good == Good::Wood {
                            // Each naturally rooted short oak is ONE durable
                            // job with a bounded sequence of separate actual
                            // log receipts. Taller trees remain untouched
                            // until scaffolding/real forestry is implemented.
                            let height = short_oak_stem_height(position, |pos|
                                world_block(game, &dimension, pos));
                            height.is_some_and(|height|
                                state.world.as_mut().unwrap()
                                    .plan_short_oak_stem(id, position, stand, height).is_ok())
                        } else {
                            state.world.as_mut().unwrap()
                                .plan_gather(id, good, position, stand).is_ok()
                        };
                        if planned {
                            // The entire old settlement, including the
                            // optional tree continuation, is restored on
                            // failed save. No native block command is sent.
                            save_autonomous(state, prior)?;
                            log::info!("[civilizations] {} sent a {} to {:?}", village.name,
                                profession.name(), position);
                            return Ok(());
                        }
                    }
            }
        }
    }

    // Town hall is physically assembled first from the verified warehouse
    // bill of materials; it may remain pending while players deliver goods.
    // The planner does NOT consume abstract simulation wood/stone.
    let mut clearing = 4_u32;
    if !village.hall_complete && village.hall_job.is_none() && has_builder {
        let hall_site = village.center;
        if open_build_plot(game, &dimension, hall_site, 2, 3, &mut clearing,
            |game: &Game<'_>| check_empty_hut_site(game, &dimension, hall_site)).is_ok() {
            let before = state.world.as_ref().unwrap().clone();
            if state.world.as_mut().unwrap().reserve_town_hall(id)? {
                save_autonomous(state, before)?;
                log::info!("[civilizations] {} started material-backed civic hall", village.name);
                return Ok(());
            }
        }
    }
    // The field and the bakery are the workplaces: reserving one is what
    // creates its worker seat, so neither waits for a farmer or baker that
    // could only exist after the site. Roles are staffed every turn.
    let need_field = village.farm.is_none();
    let need_bakery = has_builder && village.hall_complete
        && village.bakery.is_none();
    let need_hut = has_builder && village.hall_complete && village.visual_job.is_none()
        && village.pending_visual_buildings > 0 && village.physical_housing().2 > 0;
    if !need_field && !need_hut && !need_bakery { return Ok(()); }

    // The first plot the planner had to refuse, so a stalled village can say
    // why in the log instead of silently standing still.
    let mut refusal: Option<String> = None;
    for [dx, dz] in crate::game::civilizations_core::HOUSING_PLOT_OFFSETS {
        let Some(x) = village.center[0].checked_add(dx) else { continue; };
        let Some(z) = village.center[2].checked_add(dz) else { continue; };
        for dy in [0_i32, 1, -1, 2, -2] {
            let Some(y) = village.center[1].checked_add(dy) else { continue; };
            let site = [x, y, z];
            // First establish a functional field. Farms must be harvested
            // by a loaded nearby farmer; their cycles do NOT mint stock.
            if need_field {
                let mut trial = state.world.as_ref().unwrap().clone();
                if trial.start_farm(id, site).is_ok() {
                    match open_build_plot(game, &dimension, site, 1, 1, &mut clearing,
                        |game: &Game<'_>| check_empty_farm_site(game, &dimension, site)) {
                        Ok(cleared) => {
                            let prior = state.world.as_ref().unwrap().clone();
                            state.world.as_mut().unwrap().start_farm(id, site)?;
                            save_autonomous(state, prior)?;
                            log::info!("[civilizations] {} autonomously reserved crop field at {:?} ({cleared} growth cleared)",
                                village.name, site);
                            return Ok(());
                        }
                        Err(reason) => {
                            if refusal.is_none() { refusal = Some(format!("field at {site:?}: {reason}")); }
                        }
                    }
                }
            }
            // After the field and civic hall exist, build the first
            // functional workshop on the same loaded, physically open,
            // non-player-owned plot policy as ordinary housing.
            if need_bakery {
                let allowed = state.world.as_ref().unwrap()
                    .check_visual_house_site(id, site).is_ok();
                let mut trial = state.world.as_ref().unwrap().clone();
                if allowed && trial.start_bakery(id, site).is_ok() {
                    match open_build_plot(game, &dimension, site, 2, 3, &mut clearing,
                        |game: &Game<'_>| check_empty_hut_site(game, &dimension, site)) {
                        Ok(cleared) => {
                            let old = state.world.as_ref().unwrap().clone();
                            state.world.as_mut().unwrap().start_bakery(id, site)?;
                            save_autonomous(state, old)?;
                            log::info!("[civilizations] {} autonomously reserved real bakery at {:?} ({cleared} growth cleared)",
                                village.name, site);
                            return Ok(());
                        }
                        Err(reason) => {
                            if refusal.is_none() { refusal = Some(format!("bakery at {site:?}: {reason}")); }
                        }
                    }
                }
            }
            if need_hut {
                let can_build = state.world.as_ref().unwrap()
                    .check_visual_house_site(id, site).is_ok();
                if can_build {
                    match open_build_plot(game, &dimension, site, 2, 3, &mut clearing,
                        |game: &Game<'_>| check_empty_hut_site(game, &dimension, site)) {
                        Ok(cleared) => {
                            let prior = state.world.as_ref().unwrap().clone();
                            state.world.as_mut().unwrap().start_visual_house(id, site)?;
                            save_autonomous(state, prior)?;
                            log::info!("[civilizations] {} autonomously reserved real hut at {:?} ({cleared} growth cleared)",
                                village.name, site);
                            return Ok(());
                        }
                        Err(reason) => {
                            if refusal.is_none() { refusal = Some(format!("hut at {site:?}: {reason}")); }
                        }
                    }
                }
            }
        }
    }
    // No clear loaded plot; never clear players' trees, fences or walls.
    if state.physical_ticks % 1200 == 0 {
        if let Some(reason) = refusal {
            log::info!(
                "[civilizations] {} started no build and no field: {reason}",
                village.name
            );
        }
    }
    Ok(())
}
