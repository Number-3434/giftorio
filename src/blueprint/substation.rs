use crate::blueprint::models::*;
use crate::constants::*;
use std::collections::HashSet;

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
    lamp_dim: (u32, u32),
    n_frames: u32,
    base_ent_n: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, HashSet<(i32, i32)>, u32) {
    if args.substation_quality == SubstationQuality::None {
        return (Vec::new(), Vec::new(), HashSet::new(), base_ent_n);
    }
    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut occupied = HashSet::new();
    let mut curr_ent_n = base_ent_n;

    const SUB_WIDTH: u32 = 2;
    let half_cov: u32 = match args.substation_quality {
        SubstationQuality::Normal => 9,
        SubstationQuality::Uncommon => 10,
        SubstationQuality::Rare => 11,
        SubstationQuality::Epic => 12,
        SubstationQuality::Legendary => 14,
        _ => unreachable!(),
    };
    let cov: u32 = half_cov * 2;

    // Frame combs don't go on same y as subs
    let comb_cov = cov.strict_sub(2);

    // Coverage for combinators.
    let n_comb_subs = ((n_frames + comb_cov).strict_sub(half_cov)) / comb_cov;
    let cov_margin = cov / 2 - SUB_WIDTH / 2; // they're both multiples of 2
    let n_subs_width = 1 + (lamp_dim.0 - cov_margin).div_ceil(cov);
    let n_subs_height = 1 + (lamp_dim.1 - cov_margin).div_ceil(cov) + n_comb_subs;

    let start_x = -1;
    let start_y = -1 - (n_comb_subs * cov) as i32;

    for i in 0..n_subs_height as i32 {
        for j in 0..n_subs_width as i32 {
            let (x, y) = (start_x + j * cov as i32, start_y + i * cov as i32);
            let mut ent = Entity::new(curr_ent_n, SUBSTATION, (x as f64, y as f64));
            ent.quality = Some(args.substation_quality.to_string());
            ents.push(ent);

            // Mark occupied cells.
            occupied.insert((x - 1, y - 1));
            occupied.insert((x - 1, y));
            occupied.insert((x, y - 1));
            occupied.insert((x, y));

            if i > 0 {
                wires.push([curr_ent_n, WIRE_C, curr_ent_n - n_subs_width, WIRE_C]);
            }
            if j > 0 {
                wires.push([curr_ent_n, WIRE_C, curr_ent_n - 1, WIRE_C]);
            }
            curr_ent_n += 1;
        }
    }
    return (ents, wires, occupied, curr_ent_n);
}
