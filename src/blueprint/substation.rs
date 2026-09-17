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
    let coverage = match args.substation_quality {
        SubstationQuality::Normal => 18.0,
        SubstationQuality::Uncommon => 20.0,
        SubstationQuality::Rare => 22.0,
        SubstationQuality::Epic => 24.0,
        SubstationQuality::Legendary => 28.0,
        _ => 18.0,
    };
    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut occupied = HashSet::new();
    let mut curr_ent_n = base_ent_n;

    let half_cov = (coverage - 2.0) / 2.0;
    let n_frame_cov = ((n_frames as f64 - half_cov) / (coverage - 2.0)).floor() + 1.0;
    let n_subs_width = lamp_dim.0.div_ceil(coverage as u32);
    let n_subs_height =
        (((lamp_dim.1 as f64 - half_cov) / coverage).ceil() + n_frame_cov) as u32 + 1;

    let start_x = -1;
    let start_y = -1 - (n_frame_cov * coverage) as i32;

    for i in 0..n_subs_height as i32 {
        for j in 0..n_subs_width as i32 {
            let (x, y) = (start_x + j * coverage as i32, start_y + i * coverage as i32);
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
