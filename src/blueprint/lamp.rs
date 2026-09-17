use crate::blueprint::models::*;
use crate::constants::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Generates a grid of lamp entities for the blueprint.
///
/// # Arguments
///
/// * `lamp_signals` - Signals to assign to each lamp.
/// * `grid_width` - Number of lamps horizontally.
/// * `grid_height` - Number of lamps vertically.
/// * `occupied_cells` - Set of grid cells already occupied by substations.
/// * `start_entity_number` - Starting entity number for lamps.
/// * `start_x` - Starting X coordinate.
/// * `start_y` - Starting Y coordinate.
/// * `use_grayscale` - If true, configure lamps for grayscale mode.
/// * `use_horizontal_lamp_wires` - If true, configure lamps to connect horizontally instead of vertically.
///
/// # Returns
///
/// A tuple with lamp entities, lamp wires, the next entity number, and the top-right lamp entity.
pub fn generate_lamps(
    signals: &[Arc<Signal>],
    grid_dim: (u32, u32),
    occupied_cells: &HashSet<(i32, i32)>,
    base_ent_num: u32,
    start_pos: (i32, i32),
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32, u32) {
    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut curr_ent_n = base_ent_num;
    let mut prev_ents: HashMap<i32, u32> = HashMap::new();
    let mut top_right_lamp: u32 = 0;
    let mut prev_ent_n: Option<u32>;

    // Auto-rotate wires based on image rotation
    let prefer_horizontal_wires = matches!(
        args.image_rotation,
        ImageRotation::Deg90 | ImageRotation::Deg270
    ) ^ args.prefer_horizontal_wires;

    for r in 0..grid_dim.1 as i32 {
        prev_ent_n = None;
        for c in 0..grid_dim.0 as i32 {
            let x = start_pos.0 + c;
            let y = start_pos.1 + r;
            if occupied_cells.contains(&(x, y)) {
                continue;
            }
            let index = (r as u32 * grid_dim.0 + c as u32) as usize;
            let signal = Arc::clone(&signals[index]);
            let colors = if args.grayscale_bits > 0 {
                ControlBehavior::GrayLamp {
                    use_colors: true,
                    color_mode: 1,
                    red_signal: signal.clone(),
                    green_signal: signal.clone(),
                    blue_signal: signal.clone(),
                }
            } else {
                ControlBehavior::ColorLamp {
                    use_colors: true,
                    color_mode: 2,
                    rgb_signal: signal,
                }
            };
            let lamp = Entity::new(curr_ent_n, LAMP, (x as f64, y as f64))
                .with_control_behavior(colors)
                .with_always_on(true);
            ents.push(lamp);

            if r == 0 && c > 0 {
                wires.push([curr_ent_n, WIRE_G, curr_ent_n - 1, WIRE_G]);
                wires.push([curr_ent_n, WIRE_R, curr_ent_n - 1, WIRE_R]);
                top_right_lamp = curr_ent_n;
            } else if prefer_horizontal_wires {
                if let Some(prev) = prev_ent_n {
                    wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]);
                }
                prev_ent_n = Some(curr_ent_n);
            }

            if r > 0 && (!prefer_horizontal_wires || (c + 1 == grid_dim.0 as i32)) {
                if let Some(&prev_entity) = prev_ents.get(&x) {
                    wires.push([curr_ent_n, WIRE_G, prev_entity, WIRE_G]);
                }
            }
            prev_ents.insert(x, curr_ent_n);
            curr_ent_n += 1;
        }
    }

    return (ents, wires, curr_ent_n, top_right_lamp);
}
