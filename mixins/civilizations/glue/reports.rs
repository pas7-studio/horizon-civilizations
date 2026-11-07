// Horizon adapter: read-only /civ report commands, split out of `perform`.
use super::*;

/// Prints one /civ report. Returns false when no report matched, so
/// `perform` can fall through to the mutating commands.
pub(super) fn report(
    game: &mut Game<'_>,
    player: &NativeServerPlayer,
    world: &World,
    action: &str,
    actor: &str,
    args: &mut std::str::SplitWhitespace<'_>,
) -> bool {
    if action == "help" {
        reply(game, "Civilizations: /civ list | info [name] | roster <name> | schedule <name> | homes <name> | survey <name> | council <name> | forecast <name> | plots <name> | development <name> | hall <name> | warehouse <name> | posts <name> | roles <name> | status <name> | chunks <name> | diagnose <name> | buyplot <name> | buyhouse <name> | commission <name> | construction <name> | farming <name> | harvests <name> | join <name> | title | promote | history | buy/sell <good> <1..64> | (OP) found <name>, actors <name>, reconcile <name>, recoveractor <name> <resident-id> attempt, barrelcheck <name>, resolvebarrel <name> confirm|cancel <observed_count>, gathercheck <name>, discardgather <name> intact|empty, day [days], construct <name>, post <name> lumber|quarry|sand|dirt, approvehouse <name> <slot> <y>, autonomy <name> on/off, farm <name>, recoverfarm <name>, war/peace <from> <to>. Goods: wheat, bread, wood (oak logs), stone (cobble), iron.");
        return true;
    }
    if action == "list" {
        let mut items = Vec::new();
        for s in world.settlements.values().take(12) {
            items.push(format!("#{} {} [{}] pop={} houses={}", s.id, s.name, s.culture, s.population(), s.houses));
        }
        reply(game, if items.is_empty() { "No settlements yet. OP: /civ found Ashford".into() } else { items.join(" | ") });
        return true;
    }
    if action == "hall" || action == "warehouse" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ hall <name>");
            return true;
        };
        let mayor = village.residents.iter()
            .find(|r| r.alive && r.job == Job::Merchant)
            .map(|r| r.name.as_str()).unwrap_or("unassigned");
        let goods = [Good::Wheat, Good::Bread, Good::Wood, Good::Stone, Good::Iron,
                     Good::Sand, Good::Dirt];
        let incoming = goods.iter().map(|good| format!(
            "{}:{}", good.as_str(), village.warehouse_quantity(*good)
        )).collect::<Vec<_>>().join(" / ");
        let stored = if village.hall_complete {
            physical_barrel_stock(game, &village.dimension, village.center)
        } else { None };
        let physical = stored.as_ref().map(|items| items.iter()
            .map(|(good, count)| format!("{}:{}", good.as_str(), count))
            .collect::<Vec<_>>().join(" / "))
            .unwrap_or_else(|| "unloaded / not constructed".to_owned());
        let escrow = goods.iter().map(|good| format!(
            "{}:{}", good.as_str(), village.build_escrow.get(good).copied().unwrap_or(0)
        )).collect::<Vec<_>>().join(" / ");
        let (hall_wood, hall_stone) = village.hall_materials_missing();
        let journal_pending = village.barrel_transfer.is_some()
            || village.market_purchase.is_some();
        let demand = if journal_pending { None } else {
            stored.as_ref().map(|items|
                crate::game::civilizations_core::physical_construction_shortfall(village, items)
            )
        };
        let outstanding = if journal_pending {
            "unknown (native transaction must reconcile first)".to_owned()
        } else {
            demand.map(|(wood, stone)| format!("{wood} logs / {stone} cobble"))
                .unwrap_or_else(|| "unknown (native barrels unloaded/unavailable)".to_owned())
        };
        let transfer = village.barrel_transfer.as_ref().map(|t| format!(
            "{:?} {:?} {} x{} at {:?}, before {}, phase {:?}",
            t.direction, t.good, t.good.item(), t.count, t.position,
            t.before, t.phase
        )).unwrap_or_else(|| "none".to_owned());
        let purchase = village.market_purchase.as_ref().map(|order| format!(
            "{} bought {} x{} for {} coins, source {:?}, phase {:?}; no automatic retries",
            order.buyer, order.good.as_str(), order.count, order.price,
            order.position, order.phase
        )).unwrap_or_else(|| "none".to_owned());
        if action == "warehouse" {
            reply(game, format!(
                "{} ACTUAL BARRELS [{}]. Awaiting steward delivery [{}]. Farmer carrying {} wheat. Physically extracted hut escrow [{}]. Combined pending construction deficit: {}. Durable transfer: {}. Market order: {}. Native barrels are authoritative after hall completion; abstract stock is separate. Unresolved journal operations suspend new gathering.",
                village.name, physical, incoming, village.wheat_delivery_pending(), escrow,
                outstanding, transfer, purchase
            ));
        } else {
            let building = village.hall_job.as_ref().map(|job| format!(
                "{}/{} blocks verified at {:?}, blocked={}",
                job.cursor, town_hall_plan(job.origin).len(), job.origin, job.blocked
            )).unwrap_or_else(|| "waiting for funded construction".to_owned());
            let condition = if village.hall_complete {
                "COMPLETED: native barrel warehouse, mayor desk, 2 information signs"
            } else { "UNDER CONSTRUCTION / UNFUNDED" };
            reply(game, format!(
                "{} civic center: {}. Mayor: {}. {}. Native stored items [{}], incoming [{}]. Hall funding deficit: {} logs / {} cobble. {}",
                village.name, condition, mayor, building, physical, incoming,
                hall_wood, hall_stone,
                if village.barrel_transfer.is_some() {
                    "See /civ warehouse for pending or blocked transfer"
                } else { "Warehouse transfer journal clear" }
            ));
        }
        return true;
    }
    if action == "diagnose" {
        // Read-only first-play troubleshooting. No world save, no player
        // inventory access, no native mutations and no inferred offscreen NPC.
        let name = args.next().unwrap_or("");
        let Some(v) = world.by_name(name) else {
            error(game, "unknown settlement: /civ diagnose <name>");
            return true;
        };
        if v.dimension != player.dimension {
            reply(game, format!(
                "{}: visit dimension {} for native actor, block and barrel checks. No cross-world state is assumed.",
                v.name, v.dimension
            ));
            return true;
        }
        let linked = loaded_actors(game, world, v.id);
        let loaded_role = |role: Job| v.residents.iter().any(|resident| {
            resident.alive && resident.job == role
                && linked.iter().any(|(id, _)| *id == resident.id)
        });
        let phase = DailyPhase::at(game.world.day_time);
        let hall_status = if v.hall_complete {
            match physical_barrel_stock(game, &v.dimension, v.center) {
                Some(stock) if v.barrel_transfer.is_none() && v.market_purchase.is_none() => {
                    let (wood, stone) =
                        crate::game::civilizations_core::physical_construction_shortfall(v, &stock);
                    format!("COMPLETE; native barrel snapshot available; current construction gap {wood} logs / {stone} cobble")
                }
                Some(_) => "COMPLETE; native journals unresolved, stock needs reconciliation".to_owned(),
                None => "COMPLETE in save; real native barrels missing or unloaded".to_owned(),
            }
        } else if let Some(job) = &v.hall_job {
            format!("BUILDING {}/{} blocks, blocked={}",
                job.cursor, town_hall_plan(job.origin).len(), job.blocked)
        } else {
            let (wood, stone) = v.hall_materials_missing();
            format!("UNBUILT; confirmed incoming hall deficit {wood} logs / {stone} cobble")
        };
        let bakery_status = match &v.bakery {
            Some(b) if b.complete => format!(
                "COMPLETE; wheat escrow {}, bread in transit {}, batches {}",
                b.wheat_escrow, b.bread_in_transit, b.batches
            ),
            Some(b) => format!(
                "BUILDING; funded={}, cursor={}, blocked={}",
                b.supplies_reserved,
                b.building.as_ref().map(|job| job.cursor).unwrap_or(0),
                b.building.as_ref().is_some_and(|job| job.blocked),
            ),
            None => "NOT PLANNED".to_owned(),
        };
        let blocker = if !v.autonomy_enabled {
            "autonomy is PAUSED; OP can enable /civ autonomy NAME on".to_owned()
        } else if !v.actors_spawned {
            "villager identity reservation not started; keep center loaded".to_owned()
        } else if linked.is_empty() {
            "no UUID-bound resident is currently loaded; inspect /civ roster NAME and chunks".to_owned()
        } else if v.market_purchase.is_some() {
            "native player purchase unresolved; inspect /civ warehouse NAME (NO automatic refund)".to_owned()
        } else if v.barrel_transfer.is_some() {
            "native barrel journal pending; OP: /civ barrelcheck NAME; never blindly retry".to_owned()
        } else if v.hall_job.as_ref().is_some_and(|job| job.blocked) {
            "civic hall construction blocked by modified ground; inspect blocks".to_owned()
        } else if v.bakery.as_ref().and_then(|b| b.building.as_ref())
            .is_some_and(|job| job.blocked) {
            "bakery construction blocked by modified ground; inspect blocks".to_owned()
        } else if v.gather_job.as_ref().is_some_and(|job| job.blocked) {
            if v.gather_job.as_ref().is_some_and(|job| job.issued) {
                "issued wood/stone gather QUARANTINED; OP: /civ gathercheck NAME, then inspect before /civ discardgather NAME intact|empty".to_owned()
            } else {
                "never-issued gather candidate blocked; planner may discard without credit".to_owned()
            }
        } else if !loaded_role(Job::Builder) {
            "no loaded UUID-bound builder; required for real buildings".to_owned()
        } else if !phase.on_shift(Job::Builder) {
            "workers are off shift; wait for Minecraft daytime".to_owned()
        } else if v.hall_complete && physical_barrel_stock(game, &v.dimension, v.center).is_none() {
            "official hall barrel block entities unavailable; load and inspect both barrels".to_owned()
        } else if !v.hall_complete && v.hall_materials_missing() != (0, 0) {
            "hall lacks confirmed physical wood/stone; deliver items or let UUID-bound workers gather".to_owned()
        } else if v.hall_complete && v.bakery.is_none() && !loaded_role(Job::Baker) {
            "no loaded UUID-bound baker for first real production workshop".to_owned()
        } else {
            "no explicit saved-state blocker; check natural plots, NPC routes and native game logs".to_owned()
        };
        let gather_state = if let Some(stem) = &v.oak_stem {
            format!("oak log {}/{} at {:?}, issued={}, native={}, pickup={}, blocked={}",
                stem.cursor + 1, stem.height,
                v.gather_job.as_ref().map(|job| (job.position, job.worker_id)),
                v.gather_job.as_ref().is_some_and(|job| job.issued),
                v.gather_job.as_ref().is_some_and(|job| job.native_edit_confirmed),
                v.gather_job.as_ref().is_some_and(|job| job.pickup_witnessed),
                v.gather_job.as_ref().is_some_and(|job| job.blocked))
        } else {
            v.gather_job.as_ref().map(|job| format!(
                "{:?} at {:?}, worker={:?}, issued={}, native={}, pickup={}, blocked={}",
                job.good, job.position, job.worker_id, job.issued,
                job.native_edit_confirmed, job.pickup_witnessed, job.blocked
            )).unwrap_or_else(|| "idle".to_owned())
        };
        reply(game, format!(
            "{} DIAG: autonomous={} | loaded UUID actors {}/{} | builder={} farmer={} lumberjack={} miner={} baker={} steward={} | shift={} | hall=[{}] | bakery=[{}] | farm={} | gather={} | blocker: {}. This is a read-only snapshot, not a gameplay pass.",
            v.name, v.autonomy_enabled, linked.len(), v.population(),
            loaded_role(Job::Builder), loaded_role(Job::Farmer),
            loaded_role(Job::Lumberjack), loaded_role(Job::Miner),
            loaded_role(Job::Baker), loaded_role(Job::Merchant),
            phase.name(), hall_status, bakery_status, v.farm.is_some(),
            gather_state, blocker
        ));
        return true;
    }
    if action == "gathercheck" {
        if player.permission < 2 {
            error(game, "resource gathering audit requires server operator level 2");
            return true;
        }
        let name = args.next().unwrap_or("");
        let Some(v) = world.by_name(name) else {
            error(game, "unknown settlement: /civ gathercheck <name>");
            return true;
        };
        if v.dimension != player.dimension {
            error(game, "visit the settlement dimension for native block inspection");
            return true;
        }
        let Some(job) = &v.gather_job else {
            reply(game, format!("{}: no pending natural wood/stone gather", v.name));
            return true;
        };
        let observed = world_block(game, &v.dimension, job.position);
        let stem_progress = v.oak_stem.as_ref().map(|stem| format!(
            " Short oak stem log {}/{} rooted at {:?}; one durable delivery per segment.",
            stem.cursor + 1, stem.height, stem.root
        )).unwrap_or_default();
        reply(game, format!(
            "{} gather audit: {:?} at {:?}, worker stand {:?}, assigned resident {:?}, planned native block {}, issued={}, native edit confirmed={}, source pickup witnessed={}, blocked={}, live native block {:?}.{} Blocked ISSUED receipts are quarantined. Inspect world save and original block before OP /civ discardgather {} intact|empty; abandonment provides ZERO materials.",
            v.name, job.good, job.position, job.stand, job.worker_id, job.source_block(),
            job.issued, job.native_edit_confirmed, job.pickup_witnessed,
            job.blocked, observed, stem_progress, v.name
        ));
        return true;
    }
    if action == "barrelcheck" {
        if player.permission < 2 {
            error(game, "warehouse recovery inspection requires server operator level 2");
            return true;
        }
        let name = args.next().unwrap_or("");
        let Some(v) = world.by_name(name) else {
            error(game, "unknown settlement: /civ barrelcheck <name>");
            return true;
        };
        if v.dimension != player.dimension {
            error(game, "enter the settlement dimension before inspecting its native barrel");
            return true;
        }
        if v.market_purchase.is_some() {
            error(game, "an unresolved player purchase exists; barrel recovery is not safe");
            return true;
        }
        let Some(t) = &v.barrel_transfer else {
            reply(game, format!("{}: no pending physical barrel transfer", v.name));
            return true;
        };
        let observed = plain_barrel_count(game, &v.dimension, t.position, t.good);
        let expected = t.expected_after();
        reply(game, format!(
            "{} native barrel audit: {:?} {} x{} at {:?}, phase {:?}, saved before {}, expected after {:?}, current native count {:?}. Use /civ resolvebarrel {} confirm|cancel <observed_count> ONLY after manually examining real barrel inventory and logs. This command changes nothing.",
            v.name, t.direction, t.good.as_str(), t.count, t.position,
            t.phase, t.before, expected, observed, v.name
        ));
        return true;
    }
    if action == "bakery" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ bakery <name>");
            return true;
        };
        let report = if let Some(b) = &village.bakery {
            let construction = b.building.as_ref().map(|job| format!(
                "BUILDING {}/{} blocks, blocked={}, funded={}",
                job.cursor, bakery_plan(job.origin).len(), job.blocked,
                b.supplies_reserved
            )).unwrap_or_else(|| if b.complete {
                "COMPLETED native crafting table and smoker".to_owned()
            } else { "UNFUNDED construction".to_owned() });
            let wood = b.construction_escrow.get(&Good::Wood).copied().unwrap_or(0);
            let stone = b.construction_escrow.get(&Good::Stone).copied().unwrap_or(0);
            let still_need_wood = crate::game::civilizations_core::BAKERY_WOOD_LOGS
                .saturating_sub(wood);
            let still_need_stone = crate::game::civilizations_core::BAKERY_COBBLESTONE
                .saturating_sub(stone);
            let native_ready = if b.complete {
                bakery_plan(b.origin).iter().all(|step|
                    world_block(game, &village.dimension, step.pos).as_deref() == Some(step.block))
            } else { false };
            format!(
                "{} at {:?}. {}. Intact native blocks: {}. Materials carried for bakery: wood={}, stone={}; still needed: {} logs / {} cobblestone. Real wheat removed from hall: {}. Crafted bread on baker: {}. Completed 3-wheat-to-1-bread batches: {}.",
                village.name, b.origin, construction, native_ready, wood, stone,
                if b.supplies_reserved { 0 } else { still_need_wood },
                if b.supplies_reserved { 0 } else { still_need_stone },
                b.wheat_escrow, b.bread_in_transit, b.batches,
            )
        } else {
            format!(
                "{}: bakery not yet reserved. After the real hall and UUID-bound Baker/Builder are loaded, village autonomously chooses an unclaimed natural plot. Construction then needs {} genuine oak logs and {} genuine cobblestone.",
                village.name, crate::game::civilizations_core::BAKERY_WOOD_LOGS,
                crate::game::civilizations_core::BAKERY_COBBLESTONE
            )
        };
        reply(game, report);
        return true;
    }
    if action == "development" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ development <village>");
            return true;
        };
        let linked = loaded_actors(game, world, village.id);
        let (_, _, homeless) = village.physical_housing();
        let note = if !village.actors_spawned {
            "waiting for ten safe loaded natural-ground villager spawn sites"
        } else if linked.is_empty() {
            "no UUID-linked villagers loaded; return to village or inspect /civ roster, never blindly respawn"
        } else if village.farm.is_none() {
            "field not yet reserved; the loaded farmer and a clear natural 3x3 plot are required"
        } else if village.visual_job.is_none() && village.pending_visual_buildings == 0 && homeless > 0 {
            "waiting for paid abstract housing inventory; future economic days may prepare houses"
        } else {
            "workers act during daylight on loaded clear plots; follow construction/farming progress"
        };
        let job = village.visual_job.as_ref().map(|j| format!(
            "building at {:?}: {}/{} blocks, blocked={}", j.origin,
            j.cursor, hut_plan(j.origin).len(), j.blocked
        )).unwrap_or_else(|| "no current build site".to_owned());
        let field = village.farm.as_ref().map(|farm| format!(
            "field at {:?}: {:?}, blocked={}, cycles={}", farm.origin,
            farm.stage, farm.blocked, farm.completed_cycles
        )).unwrap_or_else(|| "no physical crop field".to_owned());
        reply(game, format!(
            "{} development: autonomy={} | spawn reserved={} | UUID-linked actors loaded {}/{} | {} finished PUBLIC huts, {} still without housing, {} abstract huts ready | {} | {} | {} player-owned parcels | treasury {} civic coins. Diagnostic: {}.",
            village.name,
            if village.autonomy_enabled { "ON" } else { "OFF" },
            village.actors_spawned, linked.len(), village.population(), village.public_huts(), homeless,
            village.pending_visual_buildings, job, field, village.player_parcels.len(),
            village.treasury_coins, note
        ));
        return true;
    }
    if action == "plots" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ plots <name>");
            return true;
        };
        let plots: Vec<String> = village.player_parcels.iter()
            .filter(|plot| plot.owner == actor).take(32)
            .map(|plot| format!(
                "{:?} ({})", plot.origin, if plot.commissioned { "private hut owned or commissioned" } else { "empty private parcel; build freely" }
            )).collect();
        reply(game, format!(
            "{} owns {} purchased plots; {} parcels registered in {}. {}. These protect against CIVILIZATIONS NPC edits only, not external players or mods.",
            actor, plots.len(), village.player_parcels.len(), village.name,
            if plots.is_empty() { "No purchased parcels".to_owned() } else { plots.join(" | ") }
        ));
        return true;
    }
    if action == "council" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ council <name>");
            return true;
        };
        let priorities = village.council_priorities();
        if priorities.is_empty() {
            reply(game, format!(
                "{} council, day {}: no urgent current needs under the abstract model. No decisions auto-executed.",
                village.name, world.day
            ));
        } else {
            let details: Vec<String> = priorities.iter().take(3).enumerate()
                .map(|(i, priority)| format!(
                    "#{} {} [{}/100]: {}. Next: {}",
                    i + 1, priority.need.label(), priority.urgency,
                    priority.reason, priority.next_step
                ))
                .collect();
            reply(game, format!(
                "{} council, day {}: {}. Read-only advice; no items, land claims, orders or block edits.",
                village.name, world.day, details.join(" | ")
            ));
        }
        return true;
    }
    if action == "forecast" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ forecast <name>");
            return true;
        };
        let output: Vec<String> = village.forecast_abstract_days(
            crate::game::civilizations_core::COUNCIL_FORECAST_DAYS
        ).iter().map(|day| format!(
            "+{}d bread {} wheat {} morale {} hunger {} houses {} pending {}",
            day.offset, day.bread, day.wheat, day.morale, day.starving_days,
            day.houses, day.pending_visual_buildings
        )).collect();
        reply(game, format!(
            "{} ABSTRACT forecast (not saved): {}. Excludes World war attrition, player trades, inventory, physical crops and real Minecraft time jumps.",
            village.name, output.join(" | ")
        ));
        return true;
    }
    if action == "schedule" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ schedule <name>");
            return true;
        };
        let phase = DailyPhase::at(game.world.day_time);
        let on_duty = village.residents.iter()
            .filter(|resident| resident.alive && phase.on_shift(resident.job)).count();
        reply(game, format!(
            "{} routine: {} (Horizon tick {}) | on duty {} / {} | guards watch overnight | physical farm/build edits only on work shift 2000..10999 | navigation only",
            village.name, phase.name(), game.world.day_time.rem_euclid(24_000),
            on_duty, village.population()
        ));
        return true;
    }
    if action == "survey" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ survey <name>");
            return true;
        };
        if village.dimension != player.dimension {
            error(game, "visit the settlement dimension to inspect live terrain");
            return true;
        }
        let (capacity, _, waiting) = village.physical_housing();
        if waiting == 0 {
            reply(game, format!("{} has enough completed physical hut capacity ({capacity}) for its living residents.", village.name));
            return true;
        }
        if village.pending_visual_buildings == 0 {
            reply(game, format!(
                "{} needs {} more physical spaces, but no simulated house is ready to build. Advance the economy first; no terrain is reserved.",
                village.name, waiting
            ));
            return true;
        }
        if village.visual_job.is_some() {
            reply(game, format!(
                "{} already has a physical construction project. Finish or recover it before planning another.",
                village.name
            ));
            return true;
        }
        let mut proposals = Vec::new();
        for slot in 1..=HOUSING_PLOT_OFFSETS.len() {
            // The X/Z slot index is immutable. Probe only a bounded +/-2 Y
            // window, using the same live terrain check as manual approval.
            for delta in [0_i32, 1, -1, 2, -2] {
                let Some(y) = village.center[1].checked_add(delta) else { continue; };
                let Ok(site) = world.proposed_hut(village.id, slot, y) else { continue; };
                if check_empty_hut_site(game, &village.dimension, site).is_ok() {
                    proposals.push(format!(
                        "#{slot} at {},{},{}",
                        site[0], site[1], site[2]
                    ));
                    break;
                }
            }
            if proposals.len() >= MAX_HOUSING_SURVEY_RESULTS { break; }
        }
        if proposals.is_empty() {
            reply(game, format!(
                "{} needs {} physical spaces; {} simulated houses await visualization, but no safe flat loaded plot was found among {} fixed candidates. Move to the area or prepare natural ground. Nothing was reserved.",
                village.name, waiting, village.pending_visual_buildings, HOUSING_PLOT_OFFSETS.len()
            ));
        } else {
            reply(game, format!(
                "{} can use these optional safe building slots: {} spaces missing, {} abstract houses awaiting build. {}. Autonomous builders select free plots by default after the funded town hall; OP /civ approvehouse {} <slot> <y> is an optional manual override. Real blocks and player plots are always rechecked.",
                village.name, waiting, village.pending_visual_buildings,
                proposals.join(" | "), village.name
            ));
        }
        return true;
    }
    if action == "homes" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement: /civ homes <name>");
            return true;
        };
        let (capacity, assigned, waiting) = village.physical_housing();
        let roster: Vec<String> = village.residents.iter()
            .filter(|resident| resident.alive)
            .take(10)
            .map(|resident| match village.home_slot(resident.id) {
                Some(home) => format!(
                    "{}: hut at {:?}, lane {}",
                    resident.name, home.site, home.lane + 1
                ),
                None => format!("{}: commons (no built hut slot)", resident.name),
            })
            .collect();
        reply(game, format!(
            "{} physical homes: {} completed huts, capacity {}, assigned {}, waiting {}. {}. No beds are claimed; night routes check real blocks first.",
            village.name, village.visual_sites.len(), capacity, assigned, waiting,
            roster.join(" | ")
        ));
        return true;
    }
    if action == "roster" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement: /civ roster <name>"); return true; };
        let linked = loaded_actors(game, world, s.id);
        let listing: Vec<_> = s.residents.iter().map(|r| format!(
            "{}#{}:{} [{}]", r.name, r.id, r.job.name(),
            if s.actor_recovery_issued.contains(&r.id) { "RECOVERY PENDING" }
            else if r.entity_uuid.is_some() { "UUID saved" } else { "UNBOUND" }
        )).collect();
        reply(game, format!(
            "{} actors: {}/{} nearby and loaded | {}. OP partial spawn: /civ reconcile {}, then /civ recoveractor {} <resident-id> attempt ONLY after native inspection.",
            s.name, linked.len(), s.population(), listing.join(", "), s.name, s.name
        ));
        return true;
    }
    if action == "harvests" {
        let name = args.next().unwrap_or("");
        let Some(village) = world.by_name(name) else {
            error(game, "unknown settlement");
            return true;
        };
        let Some(farm) = &village.farm else {
            reply(game, format!("{}: no physical farm", village.name));
            return true;
        };
        let details: Vec<String> = farm.harvest_receipts.iter().rev().take(5)
            .map(|receipt| format!("#{} day {} ({} crops)",
                receipt.sequence, receipt.world_day, receipt.crops)).collect();
        reply(game, format!(
            "{} observed physical harvest evidence: {} recorded batches, {} kept in bounded log. {}. Confirmed crops remain with the farmer until hall delivery; steward then inserts real wheat into the public barrels. No abstract or player inventory credit.",
            village.name, farm.harvest_sequence, farm.harvest_receipts.len(),
            if details.is_empty() { "No harvest receipts".to_owned() } else { details.join(" | ") },
        ));
        return true;
    }
    if action == "farming" {
        let name = args.next().unwrap_or("");
        let Some(settlement) = world.by_name(name) else {
            error(game, "unknown settlement");
            return true;
        };
        let message = if let Some(farm) = &settlement.farm {
            format!(
                "{} 3x3 crop field at {:?}: {:?}, {}/{} steps, blocked={}, completed visual cycles={}, observed harvest batches={}. No farm stock is minted.",
                settlement.name, farm.origin, farm.stage, farm.cursor,
                if farm.stage == FarmStage::Preparing { farm_plan(farm.origin).len() } else { 8 },
                farm.blocked, farm.completed_cycles, farm.harvest_sequence,
            )
        } else {
            format!("{} has no physical farm yet; UUID-bound farmers automatically select a loaded natural 3x3 plot. OP /civ farm {} remains an optional manual fallback.", settlement.name, settlement.name)
        };
        reply(game, message);
        return true;
    }
    if action == "chunks" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return true; };
        let sites: Vec<[i32; 3]> = settlement_work_sites(s);
        let wanted = crate::game::civilizations_core::village_chunk_footprint(
            s.center, &sites,
        );
        reply(game, format!(
            "{} holds {} chunks ({:?} + {} workplaces/corridors) around {:?} while autonomy is on ({})",
            s.name, wanted.len(), wanted.first().copied().unwrap_or([0, 0]),
            sites.len(), s.center,
            if s.autonomy_enabled { "held now" } else { "paused: released" }
        ));
        return true;
    }
    if action == "status" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return true; };
        let (missing_wood, missing_stone) = s.hall_materials_missing();
        let camps = if s.posts.is_empty() {
            "none".to_owned()
        } else {
            s.posts.iter().map(|post| {
                let worker = post.worker_id
                    .and_then(|id| s.residents.iter().find(|r| r.id == id && r.alive))
                    .map(|r| r.name.as_str()).unwrap_or("-");
                format!("{} {:?} ({worker})", post.kind.name(), post.origin)
            }).collect::<Vec<_>>().join(", ")
        };
        let harvest = s.gather_job.as_ref().map(|job| format!(
            "{} {:?}", job.good.as_str(),
            if job.issued { "issued" } else { "planned" }
        )).unwrap_or_else(|| "none".to_owned());
        reply(game, format!(
            "{}: pop {} (laborers {}), hall {}, homeless {}, next builds {}, missing {} logs/{} cobble, camps [{}], harvest {}",
            s.name, s.population(), s.count(Job::Laborer),
            if s.hall_complete { "complete" } else if s.hall_job.is_some() { "building" } else { "pending" },
            s.physical_housing().2, s.pending_visual_buildings,
            missing_wood, missing_stone, camps, harvest
        ));
        return true;
    }
    if action == "roles" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return true; };
        let seats: Vec<String> = s.workplace_demand().iter()
            .map(|(job, seats)| format!("{} {}/{}", job.name(), s.count(*job), seats))
            .collect();
        reply(game, format!(
            "{} roles (filled/workplaces): {} | laborers {}",
            s.name, seats.join(", "), s.count(Job::Laborer)
        ));
        return true;
    }
    if action == "posts" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return true; };
        let text = if s.posts.is_empty() {
            format!("{} has no work camps yet. OP: stand at a clear spot 8..80 blocks from center and /civ post {} lumber|quarry|sand|dirt", s.name, s.name)
        } else {
            let list: Vec<String> = s.posts.iter().map(|post| {
                let worker = post.worker_id
                    .and_then(|id| s.residents.iter().find(|r| r.id == id && r.alive))
                    .map(|r| r.name.clone()).unwrap_or_else(|| "unassigned".into());
                format!("{} at {:?} ({worker})", post.kind.name(), post.origin)
            }).collect();
            format!("{} camps: {}", s.name, list.join(" | "))
        };
        reply(game, text);
        return true;
    }
    if action == "construction" {
        let name = args.next().unwrap_or("");
        let Some(s) = world.by_name(name) else { error(game, "unknown settlement"); return true; };
        let text = if let Some(j) = &s.visual_job {
            format!("{} physical hut: {}/{} blocks observed, blocked={}, origin {:?}",
                    s.name, j.cursor, hut_plan(j.origin).len(), j.blocked, j.origin)
        } else {
            format!("{}: {} completed physical huts, {} abstract houses awaiting construction. OP: stand on flat empty ground 12..80 blocks from center; /civ construct {}",
                    s.name, s.visual_sites.len(), s.pending_visual_buildings, s.name)
        };
        reply(game, text);
        return true;
    }
    if action == "history" {
        let text = world.chronicle.iter().rev().take(6).cloned().collect::<Vec<_>>().join(" | ");
        reply(game, if text.is_empty() { "No history yet".into() } else { text });
        return true;
    }
    if action == "info" {
        let s = match args.next() {
            Some(name) => world.by_name(name),
            None => world.players.get(actor).and_then(|p| p.settlement).and_then(|id| world.settlements.get(&id)),
        };
        if let Some(s) = s {
            reply(game, format!(
                "{} #{} | day {} | people {} | houses {} ({} awaiting visual build) | morale {} | wheat {} bread {} wood {} stone {} iron {} | actors {} | wars {}",
                s.name, s.id, world.day, s.population(), s.houses, s.pending_visual_buildings,
                s.morale, s.quantity(Good::Wheat), s.quantity(Good::Bread), s.quantity(Good::Wood),
                s.quantity(Good::Stone), s.quantity(Good::Iron), if s.actors_spawned { "summoned" } else { "not summoned" }, s.active_wars.len()
            ));
        } else { error(game, "settlement not found (or join one first)"); }
        return true;
    }
    if action == "title" {
        if let Some(p) = world.players.get(actor) {
            reply(game, format!("Your rank: {} | coins: {} | reputation: {:?}", p.rank.name(), p.coins, p.reputation));
        } else { reply(game, "Visitor. Join a settlement for 25 starting coins."); }
        return true;
    }
    false
}
