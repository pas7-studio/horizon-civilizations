// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    pub fn sell(&mut self, player_id: &str, village_id: u64, good: Good, count: u32) -> Result<u64, String> {
        if count == 0 || count > 64 { return Err("amount must be 1..64".into()); }
        let p = self.players.get(player_id).ok_or("join a settlement first")?;
        if p.settlement != Some(village_id) { return Err("join this settlement first".into()); }
        let payout = u64::from(good.price()) * u64::from(count);
        if p.coins.checked_add(payout).is_none() { return Err("coin balance overflow".into()); }
        let s = self.settlements.get_mut(&village_id).ok_or("no settlement")?;
        // Real inventory transfer is checked and committed in the adapter.
        // Do NOT also credit abstract stock: that would count the same item
        // as both a Minecraft-derived resource and virtual daily production.
        s.warehouse_credit(good, count)?;
        let p = self.players.get_mut(player_id).ok_or("no player")?;
        p.coins += payout;
        let rep = p.reputation.entry(village_id).or_default();
        *rep = rep.saturating_add(1).min(100);
        Ok(payout)
    }

    pub fn buy(&mut self, player_id: &str, village_id: u64, good: Good, count: u32) -> Result<u64, String> {
        if count == 0 || count > 64 { return Err("amount must be 1..64".into()); }
        let cost = u64::from(good.price()) * u64::from(count);
        let p = self.players.get(player_id).ok_or("join a settlement first")?;
        if p.settlement != Some(village_id) { return Err("join this settlement first".into()); }
        if p.coins < cost { return Err("not enough coins".into()); }
        let s = self.settlements.get_mut(&village_id).ok_or("no settlement")?;
        s.warehouse_debit(good, count)?;
        self.players.get_mut(player_id).ok_or("no player")?.coins -= cost;
        Ok(cost)
    }
}
