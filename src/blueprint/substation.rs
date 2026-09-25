use crate::blueprint::{constants::*, macros::*, models::*};
use glam::{dvec2, DVec2};
use std::sync::Arc;

/// Tracks cells occupied by substations.
pub struct SubstationOccupied {
    pub start_pos: DVec2,
    quality: Option<Arc<Quality>>,
    requests: Vec<DVec2>,
}
impl SubstationOccupied {
    pub fn new(start_pos: DVec2, quality: Option<Arc<Quality>>) -> Self {
        Self {
            start_pos,
            requests: Vec::new(),
            quality,
        }
    }

    pub fn coverage(&self) -> f64 {
        2.0 * self.quality.as_ref().map_or(0, |q| match q.as_ref() {
            Quality::Normal => 9,
            Quality::Uncommon => 10,
            Quality::Rare => 11,
            Quality::Epic => 12,
            Quality::Legendary => 14,
            _ => 0,
        }) as f64
    }

    /// Requests for a substation to be placed to power the given point.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the substation would NOT occupy the given point.
    pub fn request(&mut self, mut point: DVec2) -> bool {
        if self.coverage() == 0.0 {
            return true;
        }
        point -= self.start_pos;

        let center = (point / self.coverage()).round() * self.coverage();
        let delta = (point - center).abs();

        if !self.requests.contains(&center) {
            self.requests.push(center);
        }

        delta.x >= 1.0 || delta.y >= 1.0
    }
}

/// Generates substation entities and wires for powering the blueprint.
///
/// # Arguments
///
/// * `lamp_dim` - Width and height of the lamp grid.
/// * `n_frames` - Number of frames (affects vertical coverage).
/// * `base_ent_n` - Starting entity number.
/// * `args` - Args for generating the blueprint.
///
/// # Returns
///
/// A tuple with substation entities, their wires, occupied grid cells, and the next entity number.
pub fn generate_substations(
    occupied: &mut SubstationOccupied,
    base_ent_n: u32,
) -> (Vec<Entity>, Vec<Wire>, u32) {
    let mut subs: Vec<Entity> = Vec::new();
    let mut wires: Vec<[u32; 4]> = Vec::new();
    let mut curr_ent_n: u32 = base_ent_n;
    let cov = occupied.coverage();

    if cov == 0.0 {
        return (subs, wires, base_ent_n);
    }

    // f64 doesn't implement Ord, so we use total_cmp (which also sorts NaN)
    occupied.requests.sort_by(|a, b| a.y.total_cmp(&b.y)); // Group by sorted y
    occupied.requests.sort_by(|a, b| a.x.total_cmp(&b.x)); // Preserves previous order

    let get_tag = |pos: DVec2| format!("sub-({},{})", pos.x, pos.y);

    for req in &occupied.requests {
        let pos = occupied.start_pos + req;
        let tag = get_tag(pos);
        let mut en = Entity::new(curr_ent_n, Arc::clone(&SUBSTATION), pos);
        en.quality = occupied.quality.clone();
        subs.push(en.with_tag(&tag));

        // Connect wires to substations above / substations to the left
        for prev_pos in [pos - dvec2(cov, 0.0), pos - dvec2(0.0, cov)] {
            if let Some(prev) = subs.iter().find(|e| e.position.abs_diff_eq(prev_pos, 0.01)) {
                wires.push(mkwires!(C subs; IN get_tag(prev.position.with_y(pos.y)) => IN tag));
            }
            if let Some(prev) = subs.iter().find(|e| e.position.abs_diff_eq(prev_pos, 0.01)) {
                wires.push(mkwires!(C subs; IN get_tag(prev.position.with_x(pos.x)) => IN tag));
            }
        }
        curr_ent_n += 1;
    }

    return (subs, wires, curr_ent_n);
}
