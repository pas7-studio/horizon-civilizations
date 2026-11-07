// Inherent impl block of World, split by domain (same crate/module root).
use super::super::*;

impl World {
    /// Pure authorisation for native player block-breaking hook: purchased
    /// land belongs to its authenticated owner, while shared civic plots,
    /// native farms and physical workshop footprints are protected.
    /// OP bypass is handled separately by the native host's permission.
    /// True when `pos` is a sign this mod owns: a civic hall board (current or
    /// pre-wall legacy position), a construction or finished-site board, or a
    /// work-camp board. Their text comes only from saved village state, so
    /// players are never allowed to open or edit them.
    pub fn is_civic_sign(&self, dimension: &str, pos: [i32; 3]) -> bool {
        self.settlements.values().filter(|v| v.dimension == dimension).any(|v| {
            let hall = v.hall_complete || v.hall_job.is_some();
            (hall && (hall_all_board_positions(v.center).contains(&pos)
                || legacy_hall_board_positions(v.center).contains(&pos)
                || site_board_position(v.center) == pos))
                || v.visual_sites.iter().any(|site| site_board_position(*site) == pos)
                || v.visual_job.as_ref().is_some_and(|j| site_board_position(j.origin) == pos)
                || v.bakery.as_ref().is_some_and(|b| site_board_position(b.origin) == pos)
                || v.posts.iter().any(|p| {
                    p.origin[0].checked_add(1).is_some_and(|x| [x, p.origin[1], p.origin[2]] == pos)
                })
        })
    }

    pub fn player_may_break(&self, player: &str, dimension: &str, pos: [i32; 3]) -> bool {
        let in_hut = |origin: [i32;3]| {
            (i64::from(pos[0]) - i64::from(origin[0])).abs() <= 2
                && (i64::from(pos[2]) - i64::from(origin[2])).abs() <= 2
                && (-1..=3).contains(&(i64::from(pos[1]) - i64::from(origin[1])))
        };
        let in_field = |origin: [i32;3]| {
            (i64::from(pos[0]) - i64::from(origin[0])).abs() <= 1
                && (i64::from(pos[2]) - i64::from(origin[2])).abs() <= 1
                && (i64::from(pos[1]) - i64::from(origin[1])).abs() <= 2
        };
        for v in self.settlements.values().filter(|v| v.dimension == dimension) {
            // Private property is protected from other players; the owner
            // retains the right to edit bought plots and completed houses.
            if let Some(lot) = v.player_parcels.iter().find(|lot| in_hut(lot.origin)) {
                if lot.owner != player { return false; }
                continue;
            }
            if (v.hall_complete || v.hall_job.is_some()) && in_hut(v.center) {
                return false;
            }
            if v.bakery.as_ref().is_some_and(|b| in_hut(b.origin))
                || v.farm.as_ref().is_some_and(|f| in_field(f.origin))
                || v.visual_sites.iter().any(|site| in_hut(*site))
                || v.visual_job.as_ref().is_some_and(|job| in_hut(job.origin)) {
                return false;
            }
        }
        true
    }
}
