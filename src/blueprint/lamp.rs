use crate::blueprint::{
    constants::*,
    models::{ImageRotation::*, *},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Generates a grid of lamp entities for the blueprint.
///
/// # Arguments
///
/// * `signals` - Signals to assign to each lamp.
/// * `grid_dim` - Number of lamps horizontally and vertically.
/// * `occupied_cells` - Set of grid cells already occupied by substations.
/// * `base_ent_n` - Starting entity number for lamps.
/// * `start_pos` - Starting X and Y coordinates.
/// * `args` - Args for generating the blueprint.
///
/// # Returns
///
/// A tuple with lamp entities, lamp wires, the next entity number, and the top-right lamp entity.
pub fn generate_lamps(
    signals: &[Arc<Signal>],
    grid_dim: (u32, u32),
    occupied_cells: &HashSet<(i32, i32)>,
    base_ent_n: u32,
    start_pos: (i32, i32),
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32, u32) {
    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut curr_ent_n = base_ent_n;
    let mut prev_ents_n: HashMap<i32, u32> = HashMap::new();
    let mut top_right_lamp: u32 = 0;

    // Auto-rotate wires based on image rotation
    let prefer_horizontal_wires =
        matches!(args.image_rotation, Deg90 | Deg270) ^ args.prefer_horizontal_wires;

    let get_ent_num = |r: u32, c: u32| -> u32 { base_ent_n + r * grid_dim.0 + c };

    for r in 0..grid_dim.1 {
        let mut prev_ent_n: Option<u32> = None; // Used for horizontal wires
        for c in 0..grid_dim.0 {
            curr_ent_n = get_ent_num(r, c);

            let (x, y) = (start_pos.0 + c as i32, start_pos.1 + r as i32);
            if occupied_cells.contains(&(x, y)) {
                continue;
            }
            let index = (r * grid_dim.0 + c) as usize;
            let signal = Arc::clone(&signals[index]);
            let colors = if args.grayscale_bits > 0 {
                ControlBehavior::GrayLamp {
                    use_colors: true,
                    color_mode: 1,
                    red_signal: Arc::clone(&signal),
                    green_signal: Arc::clone(&signal),
                    blue_signal: Arc::clone(&signal),
                }
            } else {
                ControlBehavior::ColorLamp {
                    use_colors: true,
                    color_mode: 2,
                    rgb_signal: signal,
                }
            };
            let lamp = Entity::new(curr_ent_n, Arc::clone(&LAMP), (x as f64, y as f64));
            ents.push(lamp.with_control_behavior(colors).with_always_on(true));

            if r == 0 && c > 0 {
                if let Some(prev) = prev_ent_n {
                    wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]); // Always a data wire here
                    wires.push([curr_ent_n, WIRE_R, prev, WIRE_R]); // Timing signals wire
                }
                top_right_lamp = curr_ent_n;
            } else {
                if prefer_horizontal_wires {
                    if let Some(prev) = prev_ent_n {
                        wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]); // horizontal data wire
                    }
                }

                // Detect if we're on the right edge, and connect vertical wires
                if !prefer_horizontal_wires || (c + 1 == grid_dim.0) {
                    if let Some(&prev) = prev_ents_n.get(&x) {
                        // Always connect vertical wire to last entity, to handle rotation
                        wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]); // vertical data wire

                        // Detect if we're on the edge and a substation is right above us
                        if grid_dim.0 >= 2 // height of a substation
                            && occupied_cells.contains(&(x, y - 1))
                            && occupied_cells.contains(&(x, y - 2))
                            && occupied_cells.contains(&(x - 1, y - 1))
                            && occupied_cells.contains(&(x - 1, y - 2))
                        {
                            let lamps_1 = (get_ent_num(r - 0, c - 2), get_ent_num(r - 1, c - 2));
                            let lamps_2 = (get_ent_num(r - 2, c - 2), get_ent_num(r - 3, c - 2));

                            wires.push([lamps_1.0, WIRE_G, lamps_1.1, WIRE_G]);
                            wires.push([lamps_2.0, WIRE_G, lamps_2.1, WIRE_G]);
                        }
                    }
                }
            }

            prev_ent_n = Some(curr_ent_n);
            prev_ents_n.insert(x, curr_ent_n);
        }
    }
    curr_ent_n += 1;
    return (ents, wires, curr_ent_n, top_right_lamp);
}
