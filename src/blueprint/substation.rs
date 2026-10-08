use crate::{
    blueprint::{constants::*, macros::*, models::*},
    macros::log,
};
use glam::{dvec2, DVec2};
use std::sync::Arc;

/// Tracks cells occupied by substations.
pub struct SubstationOccupied {
    pub start_pos: DVec2,
    include_substations: bool,
    quality: Option<Arc<str>>,
    requests: Vec<DVec2>,
}
impl SubstationOccupied {
    pub fn new(start_pos: DVec2, args: &BlueprintArgs) -> Self {
        let quality = args.substation_quality.clone();
        let quality = quality.filter(|q| *q != *DEFAULT_QUAL_KEY);

        Self {
            start_pos,
            requests: Vec::new(),
            include_substations: args.include_substations,
            quality,
        }
    }

    pub fn coverage(&self) -> f64 {
        const DEFAULT: i32 = 9;
        2.0 * self.quality.as_ref().map_or(DEFAULT, |q| match q.as_ref() {
            "normal" => DEFAULT,
            "uncommon" => 10,
            "rare" => 11,
            "epic" => 12,
            "legendary" => 14,
            _ => DEFAULT,
        }) as f64
    }

    /// Tests if it is possible to place an entity at the given point, without overlapping with a substation.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the substation would NOT occupy the given point.
    pub fn test(&mut self, mut point: DVec2) -> bool {
        if !self.include_substations || self.coverage() == 0.0 {
            return true;
        }
        point -= self.start_pos;

        let center = (point / self.coverage()).round() * self.coverage();
        let delta = (point - center).abs();

        delta.x >= 1.0 || delta.y >= 1.0
    }

    /// Requests that a substation be placed to power the given point.
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
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32) {
    let mut subs: Vec<Entity> = Vec::new();
    let mut wires: Vec<[u32; 4]> = Vec::new();
    let mut curr_ent_n: u32 = base_ent_n;
    let cov = occupied.coverage();

    if !occupied.include_substations || cov == 0.0 {
        return (subs, wires, base_ent_n);
    }

    log!(
        "Sub qual: {}",
        occupied.quality.as_ref().unwrap_or(&"...".into())
    );

    // f64 doesn't implement Ord, so we use total_cmp (which also sorts NaN)
    occupied.requests.sort_by(|a, b| a.y.total_cmp(&b.y)); // Group by sorted y
    occupied.requests.sort_by(|a, b| a.x.total_cmp(&b.x)); // Preserves previous order

    let get_tag = |pos: DVec2| format!("sub-({},{})", pos.x, pos.y);
    let mut prev_y: Option<f64> = None;
    let mut did_connect = false;

    for req in &occupied.requests {
        let pos = occupied.start_pos + req;
        let tag = get_tag(pos);
        let mut en = Entity::new(curr_ent_n, Arc::clone(&SUBSTATION), pos);
        en.quality = occupied.quality.clone();
        subs.push(en.with_tag(&tag));

        // Connect wires to substations above / substations to the left
        for prev_pos in [pos - dvec2(cov, 0.0), pos - dvec2(0.0, cov)] {
            if let Some(prev) = subs.iter().find(|e| e.position.abs_diff_eq(prev_pos, 0.01)) {
                if !did_connect {
                    wires.push(mkwires!(C subs; IN get_tag(prev.position.with_y(pos.y)) => IN tag));
                }
                if true {
                    wires.push(mkwires!(C subs; IN get_tag(prev.position.with_x(pos.x)) => IN tag));
                }
            }
        }
        if let Some(prev_y) = prev_y {
            if (prev_y - pos.y).abs() > 0.001 {
                did_connect = false;
            }
        }
        curr_ent_n += 1;
        prev_y = Some(pos.y);
    }

    return (subs, wires, curr_ent_n);
}
