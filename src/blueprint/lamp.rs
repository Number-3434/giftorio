use crate::blueprint::{
    constants::*,
    models::{ImageRotation::*, *},
    substation::*,
};
use glam::{dvec2, DVec2};
use std::sync::Arc;

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
    occupied: &mut SubstationOccupied,
    base_ent_n: u32,
    start_pos: DVec2,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32, u32) {
    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut curr_ent_n = base_ent_n;
    let mut top_right_lamp: u32 = 0;

    // Auto-rotate wires based on image rotation
    let prefer_horizontal_wires =
        matches!(args.image_rotation, Deg90 | Deg270) ^ args.prefer_horizontal_wires;

    let base_pos = start_pos + dvec2(0.5, 0.5); // Lamps are centered on the middle of a tile
    let get_ent_num = |r: u32, c: u32| -> u32 { base_ent_n + r * grid_dim.0 + c };
    let mut prev_ents_n: Vec<Option<u32>> = vec![None; grid_dim.0 as usize];

    for r in 0..grid_dim.1 {
        let mut prev_ent_n: Option<u32> = None; // Used for horizontal wires
        let mut did_connect = false;

        for c in 0..grid_dim.0 {
            curr_ent_n = get_ent_num(r, c);

            let pos = base_pos + DVec2::new(c as f64, r as f64);
            if !occupied.request(pos) {
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
            let lamp = Entity::new(curr_ent_n, Arc::clone(&LAMP), pos);
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
                // Also connect early if a substation would block the final column on this row
                if !prefer_horizontal_wires
                    || (c + 1 == grid_dim.0)
                    || (c + 2 == grid_dim.0 && !occupied.request(pos + dvec2(1.0, 0.0)))
                    || (c + 3 == grid_dim.0
                        && !occupied.request(pos + dvec2(1.0, 0.0))
                        && !occupied.request(pos + dvec2(2.0, 0.0)))
                {
                    if let Some(prev) = prev_ents_n[c as usize] {
                        // Always connect vertical wire to last entity, to handle rotation
                        wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]); // vertical data wire
                        did_connect = true;
                    }
                }
            }

            prev_ent_n = Some(curr_ent_n);
            prev_ents_n[c as usize] = Some(curr_ent_n);
        }

        // Sometimes a substation may block the vertical connections between rows.
        // This occurs if a substation occupies any tiles on the rightmost column.
        if !did_connect {
            for c in (0..grid_dim.0).rev() {
                let pos = base_pos + DVec2::new(c as f64, r as f64);
                if !occupied.request(pos) {
                    continue;
                }
                break;
            }
        }
    }
    curr_ent_n += 1;
    return (ents, wires, curr_ent_n, top_right_lamp);
}
