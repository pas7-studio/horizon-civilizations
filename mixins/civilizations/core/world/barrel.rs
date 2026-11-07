// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Prepare one and only one actual vanilla barrel mutation; native code
    /// must read the exact barrel count/slot before calling. Planning itself
    /// creates no item and deducts no warehouse quantity.
    pub fn plan_barrel_transfer(
        &mut self, id: u64, position: [i32; 3], good: Good,
        direction: BarrelDirection, before: u32, count: u32, slot: Option<usize>,
    ) -> Result<(), String> {
        let v = self.settlements.get(&id).ok_or("unknown settlement")?;
        if !v.hall_complete { return Err("physical town hall not complete".into()); }
        if v.barrel_transfer.is_some() || v.market_purchase.is_some() {
            return Err("barrel transfer or player purchase already pending".into());
        }
        if !hall_storage_positions(v.center).contains(&position) {
            return Err("barrel is not an official village hall warehouse".into());
        }
        if before > 10_000_000 || !(1..=8).contains(&count) {
            return Err("invalid native barrel batch or item count".into());
        }
        match direction {
            BarrelDirection::ToBarrel => {
                if v.warehouse_quantity(good) < count {
                    return Err("insufficient confirmed goods awaiting barrel delivery".into());
                }
                if slot.is_some() { return Err("deposit is not a slot extraction".into()); }
                before.checked_add(count).ok_or("barrel count overflow")?;
            }
            BarrelDirection::ToBuilder => {
                if v.visual_job.is_none() || v.hut_supplies_reserved {
                    return Err("no unfunded physical hut work order".into());
                }
                let required = match good {
                    Good::Wood => HUT_WOOD_LOGS,
                    Good::Stone => HUT_COBBLESTONE,
                    _ => return Err("builders only reserve verified wood and cobblestone".into()),
                };
                if v.build_escrow.get(&good).copied().unwrap_or(0)
                    .checked_add(count).is_none_or(|n| n > required) {
                    return Err("building material batch exceeds project bill".into());
                }
                if before < count || slot.is_none() {
                    return Err("missing native barrel item/slot for extraction".into());
                }
            }
            BarrelDirection::ToBakeryBuilder => {
                let bakery = v.bakery.as_ref().ok_or("no planned bakery")?;
                if bakery.building.as_ref().is_none_or(|job| job.blocked)
                    || bakery.supplies_reserved || bakery.complete {
                    return Err("no valid unfunded physical bakery project".into());
                }
                let required = match good {
                    Good::Wood => BAKERY_WOOD_LOGS,
                    Good::Stone => BAKERY_COBBLESTONE,
                    _ => return Err("bakery construction requires wood or cobblestone".into()),
                };
                if bakery.construction_escrow.get(&good).copied().unwrap_or(0)
                    .checked_add(count).is_none_or(|n| n > required) {
                    return Err("bakery materials exceed the real construction bill".into());
                }
                if before < count || slot.is_none() {
                    return Err("no verified native barrel slot for bakery supplies".into());
                }
            }
            BarrelDirection::ToBakerIngredients => {
                let bakery = v.bakery.as_ref().ok_or("no bakery")?;
                if !bakery.complete || good != Good::Wheat || slot.is_none()
                    || before < count || bakery.wheat_escrow
                        .checked_add(count).is_none_or(|n| n > BREAD_WHEAT_RECIPE)
                    || bakery.bread_in_transit != 0 {
                    return Err("baker cannot take additional unverified wheat".into());
                }
            }

        }
        self.settlements.get_mut(&id).unwrap().barrel_transfer = Some(BarrelTransfer {
            position, good, direction, phase: BarrelPhase::Planned, before, count, slot,
        });
        Ok(())
    }

    /// Persist this state BEFORE inserting or extracting from native barrels.
    /// Issued transactions must NEVER be dispatched again after a reload.
    pub fn issue_barrel_transfer(&mut self, id: u64) -> Result<(), String> {
        let transfer = self.settlements.get_mut(&id).ok_or("unknown settlement")?
            .barrel_transfer.as_mut().ok_or("no barrel transfer")?;
        if transfer.phase != BarrelPhase::Planned {
            return Err("barrel transfer already issued/blocked".into());
        }
        transfer.phase = BarrelPhase::Issued;
        Ok(())
    }

    /// Compare a later native snapshot of precisely the same barrel.
    pub fn confirm_barrel_transfer(&mut self, id: u64, after: u32) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        let t = v.barrel_transfer.as_ref().ok_or("no barrel transfer")?;
        if t.phase != BarrelPhase::Issued || t.expected_after() != Some(after) {
            return Err("barrel item count does not match the issued transfer".into());
        }
        let good = t.good;
        let direction = t.direction;
        let count = t.count;
        match direction {
            BarrelDirection::ToBarrel => v.warehouse_debit(good, count)?,
            BarrelDirection::ToBuilder => {
                let previous = v.build_escrow.get(&good).copied().unwrap_or(0);
                let next = previous.checked_add(count).ok_or("building escrow overflow")?;
                v.build_escrow.insert(good, next);
            }
            BarrelDirection::ToBakeryBuilder => {
                let b = v.bakery.as_mut().ok_or("no bakery")?;
                let prev = b.construction_escrow.get(&good).copied().unwrap_or(0);
                b.construction_escrow.insert(good, prev.checked_add(count)
                    .ok_or("bakery materials overflow")?);
            }
            BarrelDirection::ToBakerIngredients => {
                let b = v.bakery.as_mut().ok_or("no bakery")?;
                b.wheat_escrow = b.wheat_escrow.checked_add(count)
                    .ok_or("baker wheat carry overflow")?;
            }
        }
        v.barrel_transfer = None;
        Ok(())
    }

    /// A refused/ambiguous external mutation never retries automatically.
    pub fn block_barrel_transfer(&mut self, id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        let t = v.barrel_transfer.as_mut().ok_or("no barrel transfer")?;
        t.phase = BarrelPhase::Blocked;
        Ok(())
    }

    /// An operator may cancel ONLY a never-issued Planned intent, after the
    /// adapter compares a loaded official native barrel's exact item count to
    /// the saved before-count. No external mutation is possible before Issued
    /// is durably persisted by the normal bridge.
    pub fn cancel_planned_barrel_transfer(&mut self, id: u64) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        if v.market_purchase.is_some() {
            return Err("cannot cancel while a player purchase is unresolved".into());
        }
        let t = v.barrel_transfer.as_ref().ok_or("no warehouse transfer")?;
        if t.phase != BarrelPhase::Planned {
            return Err("only a never-issued planned transfer can be canceled".into());
        }
        v.barrel_transfer = None;
        let name = v.name.clone();
        self.event(format!("{name}: operator canceled an unissued native barrel plan"));
        Ok(())
    }

    /// An operator may explicitly reconcile an already BLOCKED receipt only
    /// after confirming the exact expected native after-count via a fresh
    /// loaded snapshot. No item is inserted/extracted by this operation.
    /// We reuse the normal once-only receipt transition and rollback on any
    /// domain error; this NEVER retries an uncertain native API call.
    pub fn reconcile_blocked_barrel_transfer(
        &mut self, id: u64, observed_after: u32,
    ) -> Result<(), String> {
        let v = self.settlements.get(&id).ok_or("unknown settlement")?;
        if v.market_purchase.is_some() {
            return Err("cannot reconcile while player market purchase is unresolved".into());
        }
        let t = v.barrel_transfer.as_ref().ok_or("no warehouse transfer")?;
        if t.phase != BarrelPhase::Blocked
            || t.expected_after() != Some(observed_after) {
            return Err("blocked receipt does not match the verified native count".into());
        }
        let backup = self.clone();
        self.settlements.get_mut(&id).unwrap().barrel_transfer.as_mut().unwrap().phase =
            BarrelPhase::Issued;
        if let Err(error) = self.confirm_barrel_transfer(id, observed_after) {
            *self = backup;
            return Err(error);
        }
        let name = self.settlements[&id].name.clone();
        self.event(format!(
            "{name}: operator reconciled one blocked barrel receipt against actual native inventory"
        ));
        Ok(())
    }

    /// Called only after the adapter has checked a live barrel slot, the
    /// authenticated player identity, and available inventory space.
    /// Coins are durably debited BEFORE the native item extraction.
    pub fn begin_market_purchase(
        &mut self, buyer: &str, id: u64, good: Good, count: u32,
        position: [i32; 3], slot: usize, before: u32,
    ) -> Result<u64, String> {
        if !(1..=64).contains(&count) || count > before {
            return Err("purchase exceeds one visible native stack".into());
        }
        let v = self.settlements.get(&id).ok_or("unknown settlement")?;
        if !v.hall_complete { return Err("real warehouse is not constructed".into()); }
        if !hall_storage_positions(v.center).contains(&position) {
            return Err("purchase source is not an official town hall barrel".into());
        }
        if v.barrel_transfer.is_some() || v.market_purchase.is_some() {
            return Err("warehouse has an outstanding transfer or purchase".into());
        }
        let person = self.players.get(buyer).ok_or("join village before buying")?;
        if person.settlement != Some(id) { return Err("join this village before buying".into()); }
        let price = u64::from(good.price()) * u64::from(count);
        if person.coins < price { return Err("insufficient civic coins".into()); }
        let treasury = v.treasury_coins.checked_add(price).ok_or("treasury coin overflow")?;
        self.players.get_mut(buyer).unwrap().coins -= price;
        let v = self.settlements.get_mut(&id).unwrap();
        v.treasury_coins = treasury;
        v.market_purchase = Some(MarketPurchase {
            buyer: buyer.to_owned(), good, count, price, position,
            slot, before, phase: MarketPhase::Issued,
        });
        Ok(price)
    }

    /// Only clear a pending purchase after the external native barrel item
    /// was removed and the exact item was handed to the authenticated buyer.
    pub fn complete_market_purchase(&mut self, id: u64, buyer: &str) -> Result<(), String> {
        let v = self.settlements.get_mut(&id).ok_or("unknown settlement")?;
        let order = v.market_purchase.as_ref().ok_or("no physical market purchase")?;
        if order.buyer != buyer || order.phase != MarketPhase::Issued {
            return Err("buyer identity or market state mismatch".into());
        }
        let good = order.good;
        let count = order.count;
        v.market_purchase = None;
        let name = v.name.clone();
        self.event(format!("{name}: player acquired {count} real {} from a hall barrel",
            good.as_str()));
        Ok(())
    }

    pub fn block_market_purchase(&mut self, id: u64) -> Result<(), String> {
        let order = self.settlements.get_mut(&id).ok_or("unknown settlement")?
            .market_purchase.as_mut().ok_or("no market order")?;
        order.phase = MarketPhase::Blocked;
        Ok(())
    }
}
