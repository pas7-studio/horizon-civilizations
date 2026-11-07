// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    pub fn relation(&self, a: u64, b: u64) -> i16 {
        self.settlements.get(&a).and_then(|x| x.relationships.get(&b).copied()).unwrap_or(0)
    }

    pub fn declare_war(&mut self, a: u64, b: u64) -> Result<(), String> {
        if a == b { return Err("a settlement cannot declare war on itself".into()); }
        if !self.settlements.contains_key(&a) || !self.settlements.contains_key(&b) {
            return Err("unknown settlement".into());
        }
        if self.settlements[&a].active_wars.contains(&b) { return Err("already at war".into()); }
        self.settlements.get_mut(&a).unwrap().active_wars.insert(b);
        self.settlements.get_mut(&b).unwrap().active_wars.insert(a);
        self.settlements.get_mut(&a).unwrap().relationships.insert(b, -100);
        self.settlements.get_mut(&b).unwrap().relationships.insert(a, -100);
        self.event(format!("war declared between {a} and {b}"));
        Ok(())
    }

    pub fn make_peace(&mut self, a: u64, b: u64) -> Result<(), String> {
        if !self.settlements.contains_key(&a) || !self.settlements.contains_key(&b) { return Err("unknown settlement".into()); }
        if !self.settlements[&a].active_wars.contains(&b) {
            return Err("settlements are not at war".into());
        }
        self.settlements.get_mut(&a).unwrap().active_wars.remove(&b);
        self.settlements.get_mut(&b).unwrap().active_wars.remove(&a);
        self.settlements.get_mut(&a).unwrap().relationships.insert(b, -40);
        self.settlements.get_mut(&b).unwrap().relationships.insert(a, -40);
        self.event(format!("peace between {a} and {b}"));
        Ok(())
    }
}
