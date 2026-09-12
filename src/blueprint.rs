use crate::constants::*;
use crate::image_processing::{rgb_to_int, FrameData};
use crate::macros::*;
use crate::models::*;
use crate::progress::{report_progress, set_progress};
use crate::signals::get_signals_with_quality;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wasm_bindgen::JsValue;

const ENCODE_CHUNK_SIZE: usize = 100;

#[derive(serde::Deserialize)]
pub struct BlueprintArgs {
    pub name: String,
    #[serde(rename = "imageType")]
    pub image_type: String,
    #[serde(rename = "temporalCompressionBufferMs")]
    pub temporal_compression_buffer_ms: u32,
    #[serde(rename = "includeLastFrame")]
    pub include_last_frame: bool,
    #[serde(rename = "useDLC")]
    pub use_dlc: bool,
    #[serde(rename = "targetFps")]
    pub target_fps: u32,
    #[serde(rename = "maxSize")]
    pub max_size: u32,
    #[serde(rename = "substationQuality")]
    pub substation_quality: String,
    #[serde(rename = "grayscaleBits")]
    pub grayscale_bits: u32,
    #[serde(rename = "resamplingFilter")]
    pub resampling_filter: String,
    #[serde(rename = "useGreenLampWires")]
    pub use_green_lamp_wires: bool,
    #[serde(rename = "useHorizontalLampWires")]
    pub use_horizontal_lamp_wires: bool,
}

pub struct BlueprintEncoder {
    blueprint: Blueprint,
    current_chunk_idx: usize,
    state: BlueprintEncoderState,
}
enum BlueprintEncoderState {
    Start,
    Entities,
    Wires,
    Finished,
}

impl BlueprintEncoder {
    pub fn new(blueprint: Blueprint) -> Self {
        Self {
            blueprint: blueprint,
            current_chunk_idx: 0,
            state: BlueprintEncoderState::Start,
        }
    }

    pub fn done(&self) -> bool {
        match self.state {
            BlueprintEncoderState::Finished => true,
            _ => false,
        }
    }

    pub fn next_chunk(&mut self, buf: &mut Vec<u8>) -> Result<(), JsValue> {
        let entities = &self.blueprint.blueprint.entities;
        let wires = &self.blueprint.blueprint.wires;
        let entity_chunk_count = entities.len().div_ceil(ENCODE_CHUNK_SIZE);

        report_progress(0.50, "Encoding blueprint...");

        match self.state {
            BlueprintEncoderState::Start => {
                buf.extend_from_slice(b"{\"blueprint\":");
                buf.extend_from_slice(b"{\"entities\":[");
                self.state = BlueprintEncoderState::Entities;
            }
            BlueprintEncoderState::Entities => {
                let i = self.current_chunk_idx;
                let n = ENCODE_CHUNK_SIZE;
                let chunk = &entities[(i * n)..((i + 1) * n).min(entities.len())];
                let global_index = i * chunk.len();

                set_progress(
                    0.50,
                    0.95,
                    global_index as f64 / entities.len() as f64,
                    &format!("Converting to JSON {}/{}...", global_index, entities.len()),
                );

                if i > 0 {
                    buf.extend_from_slice(b","); // Delimiter
                }

                let json_bytes = serde_json::to_vec(&chunk)
                    .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))?;

                // Extend from JSON bytes + remove surrounding brackets.
                buf.extend_from_slice(&json_bytes[1..json_bytes.len() - 1]);

                self.current_chunk_idx += 1;

                if self.current_chunk_idx >= entity_chunk_count {
                    buf.extend_from_slice(b"]");

                    self.state = BlueprintEncoderState::Wires;
                }
            }
            BlueprintEncoderState::Wires => {
                buf.extend_from_slice(b",\"wires\":");
                serde_json::to_writer(&mut *buf, &wires)
                    .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))?;
                buf.extend_from_slice(b"}");

                report_progress(0.99, "Finishing...");
                buf.extend_from_slice(b"}");
                self.state = BlueprintEncoderState::Finished;
            }
            BlueprintEncoderState::Finished => {
                return Err(JsValue::from_str(&format!("Already finished")))?
            }
        }

        Ok(())
    }
}

/// Generates timer entities and wires for the blueprint's timing mechanism.
///
/// # Arguments
///
/// * `stop` - Maximum tick value before reset.
/// * `grayscale_bits` - Number of grayscale bits (adds extra timer entities if > 0).
/// * `ticks_per_frame` - Ticks per frame (derived from FPS).
/// * `frames_per_combinator` - Number of frames processed per combinator.
///
/// # Returns
///
/// A tuple with timer entities and wires.
pub fn generate_timer(
    stop: u32,
    grayscale_bits: u32,
    ticks_per_frame: u32,
    frames_per_combinator: u32,
) -> (Vec<Entity>, Vec<Wire>) {
    let mut entities = Vec::new();
    let mut wires = Vec::new();

    entities.push(
        Entity::new(1, CONSTANT_COMB, Position::from(TIMER1_POS))
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::Constant {
                sections: Sections {
                    sections: vec![Section {
                        index: 1,
                        filters: vec![Filter {
                            index: 1,
                            type_: SIG_TYPE_VIRTUAL,
                            name: SIG_T,
                            quality: Some(QUAL_NORMAL),
                            comparator: Some(COMP_EQ),
                            count: Some(1),
                        }],
                    }],
                },
            }),
    );

    entities.push(
        Entity::new(2, DECIDER_COMB, Position::from(TIMER2_POS))
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::Decider {
                decider_conditions: DeciderConditions {
                    conditions: vec![Condition {
                        first_signal: Signal::new_virtual(SIG_T),
                        constant: stop as i32,
                        comparator: COMP_LT,
                        compare_type: None,
                    }],
                    outputs: vec![CombinatorOutput::new(
                        Arc::from(Signal::new_virtual(SIG_T)),
                        None,
                    )],
                },
            }).with_description("[virtual-signal=signal-T] is our timer that ticks up 60 times per second up to the max ticks for the entire gif. \
            When it reaches the max, it will start over, resetting the gif. This timer is used to know which frames to render.")
    );

    entities.push(
        Entity::new(3, ARITHMETIC_COMB, Position::from(TIMER3_POS))
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                arithmetic_virtual!(SIG_T - 1 => SIG_T),
            )),
    );

    wires.push([1, WIRE_R, 2, WIRE_R]);
    wires.push([2, WIRE_R, 2, WIRE_OUT_R]);
    wires.push([2, WIRE_R, 3, WIRE_R]);

    if grayscale_bits > 0 {
        entities.push(
            Entity::new(4, ARITHMETIC_COMB, Position::from(TIMER4_POS))
                .with_direction(DIR_L)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_T % (ticks_per_frame * frames_per_combinator) => SIG_S),
                )),
        );
        entities.push(
            Entity::new(5, ARITHMETIC_COMB, Position::from(TIMER5_POS)).with_control_behavior(
                ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_S / ticks_per_frame => SIG_F),
                ),
            ),
        );
        entities.push(
            Entity::new(6, ARITHMETIC_COMB, Position::from(TIMER6_POS))
                .with_direction(DIR_R)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_EACH * grayscale_bits => SIG_EACH),
                ))
                .with_description(
                    "Calculates the bit shift necessary for the frame we should be rendering.",
                ),
        );

        wires.push([3, WIRE_R, 4, WIRE_R]);
        wires.push([4, WIRE_OUT_R, 5, WIRE_R]);
        wires.push([5, WIRE_OUT_R, 6, WIRE_R]);
        wires.push([6, WIRE_OUT_R, 3, WIRE_OUT_R]);
    }

    return (entities, wires);
}

/// Generates substation entities and wires for powering the blueprint.
///
/// # Arguments
///
/// * `substation_quality` - Quality level ("none", "normal", "uncommon", "rare", "epic", "legendary").
/// * `lamp_width` - Width of the lamp grid.
/// * `lamp_height` - Height of the lamp grid.
/// * `frame_count` - Number of frames (affects vertical coverage).
/// * `start_entity_number` - Starting entity number.
///
/// # Returns
///
/// A tuple with substation entities, their wires, occupied grid cells, and the next entity number.
pub fn generate_substations(
    substation_quality: String,
    lamp_width: u32,
    lamp_height: u32,
    frame_count: u32,
    start_entity_number: u32,
) -> (Vec<Entity>, Vec<Wire>, HashSet<(i32, i32)>, u32) {
    if substation_quality == QUAL_NONE {
        return (Vec::new(), Vec::new(), HashSet::new(), start_entity_number);
    }
    let coverage = match substation_quality.as_str() {
        QUAL_UNCOMMON => 20,
        QUAL_RARE => 22,
        QUAL_EPIC => 24,
        QUAL_LEGENDARY => 28,
        _ => 18,
    };
    let mut substation_entities = Vec::new();
    let mut substation_wires = Vec::new();
    let mut occupied_cells = HashSet::new();
    let mut current_entity = start_entity_number;
    let half_coverage = ((coverage as f64) - 2.0) / 2.0;
    let mut frame_coverage_count =
        (((frame_count as f64) - half_coverage) / (coverage as f64)).ceil() as u32;
    while ((frame_count as f64) - half_coverage + (frame_coverage_count as f64 * 2.0))
        > (frame_coverage_count as f64 * coverage as f64)
    {
        frame_coverage_count += 1;
    }
    let num_substations_width =
        (((lamp_width as f64) - half_coverage) / (coverage as f64)).ceil() as u32 + 1;
    let num_substations_height = (((lamp_height as f64) - half_coverage) / (coverage as f64)).ceil()
        as u32
        + 1
        + frame_coverage_count;
    let start_x = -1;
    let start_y = -1 - (frame_coverage_count as i32 * coverage as i32);
    for i in 0..num_substations_height {
        for j in 0..num_substations_width {
            let x = start_x + (j as i32 * coverage as i32);
            let y = start_y + (i as i32 * coverage as i32);
            let mut entity = Entity::new(
                current_entity,
                SUBSTATION,
                Position::new(x as f64, y as f64),
            );
            entity.quality = if substation_quality.as_str() != QUAL_NORMAL {
                Some(substation_quality.clone())
            } else {
                None
            };
            substation_entities.push(entity);

            // Mark occupied cells.
            occupied_cells.insert((x - 1, y - 1));
            occupied_cells.insert((x - 1, y));
            occupied_cells.insert((x, y - 1));
            occupied_cells.insert((x, y));
            if i > 0 {
                substation_wires.push([
                    current_entity,
                    WIRE_C,
                    current_entity - num_substations_width,
                    WIRE_C,
                ]);
            }
            if j > 0 {
                substation_wires.push([current_entity, WIRE_C, current_entity - 1, WIRE_C]);
            }
            current_entity += 1;
        }
    }

    return (
        substation_entities,
        substation_wires,
        occupied_cells,
        current_entity,
    );
}

/// Generates combinator entities and wiring for each frame group.
///
/// # Arguments
///
/// * `frame_outputs` - A vector of all the outputs for a frame.
/// * `occupied_y` - Set of Y coordinates occupied by substations.
/// * `ticks_per_group` - Ticks per group.
/// * `base_entity_number` - Starting entity number.
/// * `base_decider_x` - Base X coordinate for decider combinators.
/// * `base_y` - Base Y coordinate for placement.
/// * `max_rows_per_group` - Maximum rows per group.
/// * `grayscale_bits` - Number of grayscale bits (affects extra combinators).
///
/// # Returns
///
/// A tuple containing combinator entities, their wires, and the next entity number.
/// Does not populate combinators with data.
pub fn generate_frame_combinators(
    n_chunks: u64,
    occupied_y: &HashSet<i32>,
    base_entity_number: u32,
    base_decider_x: f64,
    base_y: f64,
    max_rows_per_group: u32,
    grayscale_bits: u32,
) -> (Vec<Entity>, Vec<Entity>, Vec<Wire>, u32) {
    let mut curr_entity_idx = base_entity_number;
    let mut other_entities = Vec::with_capacity(3); // shifters / etc
    let mut new_entities = Vec::with_capacity(n_chunks as usize * 2); // data entities
    let mut wires = Vec::with_capacity(n_chunks as usize * 3 + 4);

    let comp1_x = base_decider_x;
    let shifter1_x = base_decider_x + (1 as f64) * 2.0;
    let shifter2_x = base_decider_x + (2 as f64) * 2.0;

    // if true {
    //     other_entities.push(
    //         Entity::new(
    //             curr_entity_idx,
    //             DECIDER_COMB,
    //             Position::new(comp1_x, base_y + 1.0),
    //         )
    //         .with_direction(DIR_R)
    //         .with_control_behavior(ControlBehavior::from_decider_conditions(
    //             DeciderConditions {
    //                 conditions: vec![Condition {
    //                     first_signal: Signal::new_virtual(SIG_EACH),
    //                     constant: 0,
    //                     comparator: COMP_NE,
    //                     compare_type: None,
    //                 }],
    //                 outputs: vec![CombinatorOutput::new(
    //                     Arc::from(Signal::new_virtual(SIG_EACH)),
    //                     None,
    //                 )],
    //             },
    //         ))
    //         .with_description("test_x"),
    //     );
    //     curr_entity_idx += 1;
    // }

    if grayscale_bits > 0 {
        other_entities.push(
            Entity::new(
                curr_entity_idx,
                ARITHMETIC_COMB,
                Position::new(shifter1_x, base_y + 1.0),
            )
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                ArithmeticConditions {
                    first_signal: Signal::new_virtual(SIG_EACH),
                    second_signal: Some(Signal::new_virtual(SIG_F)),
                    second_constant: None,
                    operation: OP_RSHIFT,
                    output_signal: Signal::new_virtual(SIG_EACH),
                },
            ))
            .with_description("shifter1_x"),
        );

        let first_decider_id = curr_entity_idx
            + if grayscale_bits == 1 || grayscale_bits == 4 {
                4
            } else {
                3
            };
        wires.push([curr_entity_idx, WIRE_R, first_decider_id, WIRE_R]);
        wires.push([curr_entity_idx, WIRE_G, first_decider_id, WIRE_OUT_G]);
        curr_entity_idx += 1;

        other_entities.push(
            Entity::new(
                curr_entity_idx,
                ARITHMETIC_COMB,
                Position::new(shifter2_x, base_y + 1.0),
            )
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                arithmetic_virtual!(SIG_EACH AND match grayscale_bits {
                    1 => 1,
                    4 => 15,
                    _ => 255,
                } => SIG_EACH),
            ))
            .with_description("shifter2_x"),
        );

        wires.push([curr_entity_idx - 1, WIRE_OUT_R, curr_entity_idx, WIRE_R]);
        curr_entity_idx += 1;
        if grayscale_bits == 1 || grayscale_bits == 4 {
            other_entities.push(
                Entity::new(
                    curr_entity_idx,
                    ARITHMETIC_COMB,
                    Position::new(shifter2_x + 1.0, base_y + 2.0),
                )
                .with_direction(DIR_L)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_EACH * if grayscale_bits == 1 { 255 } else { 17 } => SIG_EACH),
                )),
            );
            wires.push([curr_entity_idx - 1, WIRE_OUT_R, curr_entity_idx, WIRE_R]);
            curr_entity_idx += 1;
        }
    }

    let mut first_decider = true;
    let (mut x_offset, mut y_offset) = (0.0, 0.0);
    let mut row_in_this_column = 0;
    let mut prev_first_decider: Option<u32> = None;

    // Generates combinators up to down, then left ro right.
    for _ in 0..n_chunks as usize {
        let mut curr_y = base_y - (row_in_this_column as f64) - y_offset;
        if occupied_y.contains(&(curr_y.floor() as i32)) {
            y_offset += 2.0;
            curr_y -= 2.0;
        }

        new_entities.push(
            Entity::new(
                curr_entity_idx,
                DECIDER_COMB,
                Position::new(base_decider_x + x_offset, curr_y),
            )
            .with_direction(DIR_R),
        );

        if !first_decider {
            // Wire to previous decider
            let prev_decider_id = curr_entity_idx - 1;
            wires.push([prev_decider_id, WIRE_R, curr_entity_idx, WIRE_R]);
            wires.push([prev_decider_id, WIRE_OUT_G, curr_entity_idx, WIRE_OUT_G]);
        } else {
            if let Some(prev) = prev_first_decider {
                wires.push([prev, WIRE_R, curr_entity_idx, WIRE_R]);
                wires.push([prev, WIRE_OUT_G, curr_entity_idx, WIRE_OUT_G]);
            }
            prev_first_decider = Some(curr_entity_idx);
        }

        first_decider = false;
        curr_entity_idx += 1;
        row_in_this_column += 1;

        if row_in_this_column >= max_rows_per_group {
            row_in_this_column = 0;
            first_decider = true;
            y_offset = 0.0;
            x_offset += 2.0;
        }
    }

    return (other_entities, new_entities, wires, curr_entity_idx);
}

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
    grid_width: u32,
    grid_height: u32,
    occupied_cells: &HashSet<(i32, i32)>,
    start_entity_number: u32,
    start_x: i32,
    start_y: i32,
    use_grayscale: bool,
    use_horizontal_lamp_wires: bool,
) -> (Vec<Entity>, Vec<Wire>, u32, u32) {
    let mut lamp_entities = Vec::new();
    let mut lamp_wires = Vec::new();
    let mut current_entity = start_entity_number;
    let mut previous_entities: HashMap<i32, u32> = HashMap::new();
    let mut top_right_lamp: u32 = 0;
    let mut previous_entity: Option<u32>;

    for r in 0..grid_height as i32 {
        previous_entity = None;
        for c in 0..grid_width as i32 {
            let x = start_x + c;
            let y = start_y + r;
            if occupied_cells.contains(&(x, y)) {
                continue;
            }
            let index = (r as u32 * grid_width + c as u32) as usize;
            let signal = Arc::clone(&signals[index]);
            let colors = if use_grayscale {
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
            let lamp = Entity::new(current_entity, LAMP, Position::new(x as f64, y as f64))
                .with_control_behavior(colors)
                .with_always_on(true);
            lamp_entities.push(lamp);

            if r == 0 && c > 0 {
                lamp_wires.push([current_entity, WIRE_G, current_entity - 1, WIRE_G]);
                lamp_wires.push([current_entity, WIRE_R, current_entity - 1, WIRE_R]);
                top_right_lamp = current_entity;
            } else if use_horizontal_lamp_wires {
                if let Some(prev) = previous_entity {
                    lamp_wires.push([current_entity, WIRE_G, prev, WIRE_G]);
                }
                previous_entity = Some(current_entity);
            }

            if r > 0 {
                if !use_horizontal_lamp_wires || (c + 1 == grid_width as i32) {
                    if let Some(&prev_entity) = previous_entities.get(&x) {
                        lamp_wires.push([current_entity, WIRE_G, prev_entity, WIRE_G]);
                    }
                }
            }
            previous_entities.insert(x, current_entity);
            current_entity += 1;
        }
    }

    return (lamp_entities, lamp_wires, current_entity, top_right_lamp);
}

/// Builds the complete blueprint JSON by combining all components.
///
/// EDITS:
/// 1. Added streaming. The pipeline is now:
/// - Generate the locations of all entities + wires (combinators, lamps, etc.)
/// - Iterate through each frame and THEN populate the data of the already-generated entities.
/// 2. Added compression. The pipeline is now:
/// - Same as above, but after we accumulate `n` frames we "compress" the data.
/// - Our compression currently finds any pixels that haven't changed between two frames,
///   and stores them using a singular combinator instead of two. The larger the compression size,
///   the more frames we look over. Note that this drastically increases the number of combinators
///   required, but can also greatly reduce the size of the blueprint.
///
/// # Arguments
///
/// * `fps` - Effective frames per second.
/// * `sampled_frames` - Processed image frames.
/// * `use_dlc` - Whether to use DLC signals.
/// * `grayscale_bits` - Number of grayscale bits (0 means color mode).
/// * `signals` - Available signals vector.
/// * `substation_quality` - Quality level for substations.
///
/// # Returns
///
/// The final blueprint as a struct.
pub fn generate_blueprint(
    frame_data: &mut FrameData,
    args: &BlueprintArgs,
) -> Result<Blueprint, JsValue> {
    report_progress(0, "Starting blueprint update");

    // Get signals internally.
    let signals: Vec<Arc<Signal>> = get_signals_with_quality(args.use_dlc);

    if frame_data.total_frames() == 0 {
        return Err(JsValue::from_str("No sampled frames"));
    }

    let use_grayscale = args.grayscale_bits > 0;
    let n_frames = frame_data.total_frames();
    let n_scaled_frames = frame_data.total_frames()
        * if args.temporal_compression_buffer_ms > 0 {
            2
        } else {
            1
        };
    let frames_per_comb = if args.grayscale_bits > 0 {
        32 / args.grayscale_bits
    } else {
        1
    };
    let n_comp_buf_frames = if args.temporal_compression_buffer_ms > 0 {
        (args.temporal_compression_buffer_ms as u64 * frame_data.fps() as u64)
            .div_ceil(1000 * frames_per_comb as u64) as usize
    } else {
        1
    };
    let n_comp_frames_per_chunk = if n_comp_buf_frames > 1 { 1 } else { 0 } as usize;
    let (full_width, full_height) = frame_data.dimensions();
    let max_columns_per_group = ((signals.len() as u32) / full_height).min(full_width);
    let num_groups = (full_width as f64 / max_columns_per_group as f64).ceil() as u32;
    let max_columns_per_group = full_width / num_groups;
    if max_columns_per_group < 1 {
        return Err(JsValue::from_str(
            "Not enough signals for even one column of lamps!",
        ));
    }
    let max_rows_per_group =
        (((n_scaled_frames as f64 / ((max_columns_per_group as f64 / 2.0).floor())).ceil())
            / frames_per_comb as f64)
            .ceil() as u32;

    let ticks_per_frame = (60.0 / frame_data.fps() as f64) as u32;
    let stop = n_frames * ticks_per_frame;
    let (timer_entities, timer_wires) =
        generate_timer(stop, args.grayscale_bits, ticks_per_frame, frames_per_comb);

    let mut all_entities = timer_entities;
    let mut all_wires: Vec<Wire> = timer_wires;
    let mut next_entity = all_entities
        .iter()
        .map(|e| e.entity_number)
        .max()
        .unwrap_or(0)
        + 1;
    report_progress(0.10, "Generating power grid");
    let (substation_entities, substation_wires, occupied_cells, next_entity_new) =
        generate_substations(
            args.substation_quality.clone(),
            full_width,
            full_height,
            max_rows_per_group
                + match args.grayscale_bits {
                    1 | 4 => 2,
                    8 => 1,
                    _ => 0,
                },
            next_entity,
        );

    next_entity = next_entity_new;
    all_entities.extend(substation_entities);
    all_wires.extend(substation_wires);

    let substation_occupied_y: HashSet<i32> = occupied_cells.iter().map(|(_, y)| *y).collect();
    let mut previous_top_right_lamp: Option<u32> = None;
    let mut all_data_comb_entity_indexes: Vec<Vec<u32>> = Vec::new();

    for group_index in 0..num_groups {
        let group_left = group_index * max_columns_per_group;
        let group_right = ((group_index + 1) * max_columns_per_group).min(full_width);
        let group_width = group_right - group_left;

        let group_offset_x = group_index * max_columns_per_group;
        let first_connection_entity = if use_grayscale {
            next_entity
        } else {
            next_entity + 1
        };

        let (other_entities, data_combinators, mut group_comb_wires, new_next_entity) =
            generate_frame_combinators(
                (n_scaled_frames as u64).div_ceil(frames_per_comb as u64),
                &substation_occupied_y,
                next_entity,
                group_offset_x as f64 + 0.5,
                match args.grayscale_bits {
                    1 | 4 => -5.0,
                    8 => -4.0,
                    _ => -3.0,
                },
                max_rows_per_group,
                args.grayscale_bits,
            );
        if group_index == 0 {
            group_comb_wires.push([3, WIRE_OUT_R, first_connection_entity, WIRE_R]);
        }
        next_entity = new_next_entity;

        let (group_lamps, mut group_lamp_wires, new_next_entity, top_right_lamp) = generate_lamps(
            &signals,
            group_width,
            full_height,
            &occupied_cells,
            next_entity,
            group_offset_x as i32,
            0,
            use_grayscale,
            args.use_horizontal_lamp_wires,
        );
        next_entity = new_next_entity;

        let first_lamp_entity = group_lamps[0].entity_number;
        if use_grayscale {
            group_comb_wires.push([first_lamp_entity, WIRE_R, first_connection_entity, WIRE_R]);
            let last_shifter = if args.grayscale_bits == 1 || args.grayscale_bits == 4 {
                first_connection_entity + 2
            } else {
                first_connection_entity + 1
            };
            group_comb_wires.push([first_lamp_entity, 1, last_shifter, WIRE_OUT_G]);
        } else {
            group_comb_wires.push([
                first_lamp_entity,
                WIRE_G,
                first_connection_entity,
                WIRE_OUT_G,
            ]);
            group_comb_wires.push([first_lamp_entity, WIRE_R, first_connection_entity, WIRE_R]);
        }

        if let Some(prev) = previous_top_right_lamp {
            group_lamp_wires.push([group_lamps[0].entity_number, WIRE_R, prev, WIRE_R]);
        }
        previous_top_right_lamp = Some(top_right_lamp);

        // Track the entity indexes of the data combinators for each group
        all_data_comb_entity_indexes
            .push(data_combinators.iter().map(|e| e.entity_number).collect());

        all_entities.extend(other_entities);
        all_entities.extend(data_combinators);
        all_entities.extend(group_lamps);
        all_wires.extend(group_comb_wires);
        all_wires.extend(group_lamp_wires);

        set_progress(
            0.30,
            0.40,
            group_index as f64 / num_groups as f64,
            &format!("Processed chunk {}/{}", group_index, num_groups),
        );
    }

    struct GroupPatchState {
        curr_chunk_idx: usize, // which *chunk* we’re patching (not frame)
        grayscale_buf: Vec<image::DynamicImage>,
        output_buf: Vec<Vec<i32>>,
    }

    let ticks_per_group = ticks_per_frame * frames_per_comb;

    let mut patch_states: Vec<GroupPatchState> = (0..num_groups)
        .map(|_| GroupPatchState {
            curr_chunk_idx: 0,
            grayscale_buf: Vec::with_capacity(frames_per_comb as usize),
            output_buf: Vec::with_capacity(n_comp_buf_frames),
        })
        .collect();

    let mk_control_behaviour =
        |range: (i32, i32), outputs: Vec<CombinatorOutput>| ControlBehavior::Decider {
            decider_conditions: DeciderConditions {
                conditions: vec![
                    Condition {
                        first_signal: Signal::new_virtual(SIG_T),
                        constant: range.0,
                        comparator: COMP_GE,
                        compare_type: None,
                    },
                    Condition {
                        first_signal: Signal::new_virtual(SIG_T),
                        constant: range.1,
                        comparator: COMP_LT,
                        compare_type: Some(COMP_AND),
                    },
                ],
                outputs,
            },
        };

    // Swap all wires if requested (default uses red wires, so swap all for green)
    if args.use_green_lamp_wires {
        fn get_swap(x: u32) -> u32 {
            match x {
                1 => 2, // circuit_green / combinator_input_green -> circuit_red / combinator_input_red
                2 => 1, // circuit_red / combinator_input_red -> circuit_green / combinator_input_green
                3 => 4, // combinator_output_green -> combinator_output_red
                4 => 3, // combinator_output_red -> combinator_output_green
                _ => x, // usually just copper
            }
        }

        for w in all_wires.iter_mut() {
            w[1] = get_swap(w[1]);
            w[3] = get_swap(w[3]);
        }
    }

    let mut frame = frame_data.next().unwrap()?;
    let mut next_frame: Option<image::DynamicImage>;

    all_entities.sort_by_key(|entity| entity.entity_number);

    loop {
        next_frame = match frame_data.next() {
            Some(frame) => Some(frame?),
            None => None,
        };
        let is_last_frame = next_frame.is_none();

        for group_i in 0..num_groups as usize {
            let state = &mut patch_states[group_i];
            let outputs: Vec<i32>;

            let group_left = group_i as u32 * max_columns_per_group;
            let group_right = ((group_i as u32 + 1) * max_columns_per_group).min(full_width);
            let group_width = group_right - group_left;

            let cropped = frame.crop_imm(group_left, 0, group_width, full_height);
            let expected_outputs_len = (cropped.width() * cropped.height()) as usize;

            if use_grayscale {
                state.grayscale_buf.push(cropped);
                if state.grayscale_buf.len() < frames_per_comb as usize && !is_last_frame {
                    continue; // wait until we have a full chunk for this group
                }
                outputs = grayscale_frames_to_outputs(&state.grayscale_buf, args.grayscale_bits)?;
                state.grayscale_buf.clear();
            } else {
                outputs = color_frame_to_outputs(&cropped)?;
            }

            if outputs.len() != expected_outputs_len {
                return Err(JsValue::from_str(&format!(
                    "Outputs length ({}) does not match frame size ({}).",
                    outputs.len(),
                    expected_outputs_len
                )));
            }

            state.output_buf.push(outputs);

            if state.output_buf.len() < n_comp_buf_frames && !is_last_frame {
                continue; // Accumulate until we have `compression_level` outputs,
            }

            fn find_entity_by_entity_number(
                all_entities: &Vec<Entity>,
                target_entity_number: u32,
            ) -> usize {
                all_entities
                    .binary_search_by_key(&target_entity_number, |e| e.entity_number)
                    .expect("target entity number not found")
            }

            let mut changed_mask: Vec<bool> = vec![false; expected_outputs_len];
            let first_outputs = &state.output_buf[0];

            if state.output_buf.len() > 1 {
                for comb_outputs in state.output_buf[1..].iter() {
                    for i in 0..expected_outputs_len {
                        if !changed_mask[i] && (&comb_outputs[i] != &first_outputs[i]) {
                            changed_mask[i] = true;
                        }
                    }
                }
            } else {
                for i in 0..expected_outputs_len {
                    changed_mask[i] = true;
                }
            }

            let base_chunk_i = state.curr_chunk_idx * n_comp_buf_frames;

            // Update data for all non-static frames (frames with differing pixels)
            for (comb_i, comb_outputs) in state.output_buf.iter().enumerate() {
                let target_i = base_chunk_i + comb_i;
                let target_entity_comb_i = state.curr_chunk_idx
                    * (n_comp_buf_frames + n_comp_frames_per_chunk)
                    + comb_i
                    + n_comp_frames_per_chunk;
                let curr_entity_idx = find_entity_by_entity_number(
                    &all_entities,
                    all_data_comb_entity_indexes[group_i][target_entity_comb_i],
                );
                let start_frame_i = (target_i as u32 * ticks_per_group) as i32;
                let end_frame_i = ((target_i as u32 + 1) * ticks_per_group) as i32;

                // Only contains non-changed pixels (i.e., the ones we want to store)
                let mut target_comb_outputs = Vec::with_capacity(signals.len());

                // Accumulate only the changed outputs
                for (i, v) in comb_outputs.iter().enumerate() {
                    if changed_mask[i] && *v != 0 {
                        target_comb_outputs
                            .push(CombinatorOutput::new(Arc::clone(&signals[i]), Some(*v)));
                    }
                }

                // Update the data combinator. Note this uses the real frame index at all times.
                all_entities[curr_entity_idx] =
                    all_entities[curr_entity_idx].clone().with_control_behavior(
                        mk_control_behaviour((start_frame_i, end_frame_i), target_comb_outputs),
                    );
            }

            // Make unchanged pixels data combinators
            if state.output_buf.len() > 1 {
                let start_frame_i = (base_chunk_i as u32 * ticks_per_group) as i32;
                let end_frame_i = ((base_chunk_i as u32 + state.output_buf.len() as u32)
                    * ticks_per_group) as i32;
                let mut target_comb_outputs = Vec::with_capacity(signals.len());

                for i in 0..expected_outputs_len {
                    if !changed_mask[i] && first_outputs[i] != 0 {
                        target_comb_outputs.push(CombinatorOutput::new(
                            Arc::clone(&signals[i]),
                            Some(first_outputs[i]),
                        ));
                    }
                }

                let target_entity_comb_i =
                    state.curr_chunk_idx * (n_comp_buf_frames + n_comp_frames_per_chunk);
                let curr_entity_idx = find_entity_by_entity_number(
                    &all_entities,
                    all_data_comb_entity_indexes[group_i][target_entity_comb_i],
                );

                all_entities[curr_entity_idx] =
                    all_entities[curr_entity_idx].clone().with_control_behavior(
                        mk_control_behaviour((start_frame_i, end_frame_i), target_comb_outputs),
                    );
            }

            state.curr_chunk_idx += 1;
            state.output_buf.clear();
        }

        if next_frame.is_none() {
            break;
        }
        frame = next_frame.take().unwrap();
    }

    let blueprint = Blueprint {
        blueprint: BlueprintInner {
            icons: vec![Icon {
                signal: Signal::new_virtual(DECIDER_COMB),
                index: 1,
            }],
            entities: all_entities,
            wires: all_wires,
            item: BLUEPRINT,
            label: args.name.clone(),
            version: BLUEPRINT_VERSION,
        },
    };

    Ok(blueprint)
}

/// Converts an RGB pixel to an integer using a utility function.
///
/// # Arguments
///
/// * `r` - Red channel.
/// * `g` - Green channel.
/// * `b` - Blue channel.
///
/// # Returns
///
/// A vector of CombinatorOutputs for the frame
pub fn color_frame_to_outputs(frame: &image::DynamicImage) -> Result<Vec<i32>, JsValue> {
    let mut outputs: Vec<i32> = Vec::with_capacity((frame.width() * frame.height()) as usize);
    for chunk in frame.to_rgb8().into_raw().chunks(3) {
        if chunk.len() < 3 {
            continue;
        }
        outputs.push(rgb_to_int(chunk[0], chunk[1], chunk[2]) as i32);
    }
    Ok(outputs)
}

/// Packs grayscale frames into output signals by bit-packing pixel values.
///
/// # Arguments
///
/// * `frames` - A slice of grayscale image frames.
/// * `signals` - The signals to map to each pixel.
/// * `grayscale_bits` - Number of bits for grayscale conversion.
///
/// # Returns
///
/// A vector of JSON objects representing output filters.
pub fn grayscale_frames_to_outputs(
    frames: &[image::DynamicImage],
    grayscale_bits: u32,
) -> Result<Vec<i32>, JsValue> {
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames provided for packing"));
    }
    let num_pixels = (frames[0].width() * frames[0].height()) as usize;
    let luma_images: Vec<_> = frames.iter().map(|x| x.to_luma8()).collect();
    let mut outputs: Vec<i32> = Vec::with_capacity(num_pixels);

    for i in 0..num_pixels {
        let mut packed_value = 0u32;

        for (j, img) in luma_images.iter().enumerate() {
            let pixel_value = img.as_raw()[i];
            packed_value |= match grayscale_bits {
                1 => (pixel_value >= GRAYSCALE_THRESH) as u32,
                4 => (pixel_value >> 4) as u32,
                8 => pixel_value as u32,
                _ => {
                    return Err(JsValue::from_str("Unsupported grayscale bit depth"));
                }
            } << (grayscale_bits * j as u32);
        }
        outputs.push(packed_value as i32);
    }
    Ok(outputs)
}
