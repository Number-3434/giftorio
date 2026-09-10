use crate::constants::*;
use crate::image_processing::{rgb_to_int, FrameData};
use crate::models::*;
use crate::progress::{report_progress, set_progress};
use crate::signals::get_signals_with_quality;
use image::GenericImageView;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wasm_bindgen::JsValue;

const ENCODE_CHUNK_SIZE: usize = 500;
macro_rules! log {
    ($($arg:tt)*) => {
        web_sys::console::log_1(&format!($($arg)*).into());
    };
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
        Entity::new(1, CONSTANT_COMB, Position::from(TIMER_ENTITY1_POSITION))
            .with_direction(DIR_RIGHT)
            .with_control_behavior(ControlBehavior::Constant {
                sections: Sections {
                    sections: vec![Section {
                        index: 1,
                        filters: vec![Filter {
                            index: 1,
                            type_: SIGNAL_TYPE_VIRTUAL,
                            name: SIGNAL_T,
                            quality: Some(QUAL_NORMAL),
                            comparator: Some(COMPARATOR_EQ),
                            count: Some(1),
                        }],
                    }],
                },
            }),
    );

    entities.push(
        Entity::new(2, DECIDER_COMB, Position::from(TIMER_ENTITY2_POSITION))
            .with_direction(DIR_RIGHT)
            .with_control_behavior(ControlBehavior::Decider {
                decider_conditions: DeciderConditions {
                    conditions: vec![Condition {
                        first_signal: Signal::new_virtual(SIGNAL_T),
                        constant: stop as i32,
                        comparator: COMPARATOR_LT,
                        compare_type: None,
                    }],
                    outputs: vec![CombinatorOutput::new(
                        Arc::from(Signal::new_virtual(SIGNAL_T)),
                        None,
                    )],
                },
            }).with_description("[virtual-signal=signal-T] is our timer that ticks up 60 times per second up to the max ticks for the entire gif. \
            When it reaches the max, it will start over, resetting the gif. This timer is used to know which frames to render.")
    );

    entities.push(
        Entity::new(3, ARITHMETIC_COMB, Position::from(TIMER_ENTITY3_POSITION))
            .with_direction(DIR_RIGHT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal::new_virtual(SIGNAL_T),
                    second_signal: None,
                    second_constant: Some(1),
                    operation: OPERATION_SUB,
                    output_signal: Signal::new_virtual(SIGNAL_T),
                },
            }),
    );

    wires.push([1, 2, 2, 2]);
    wires.push([2, 2, 2, 4]);
    wires.push([2, 2, 3, 2]);

    if grayscale_bits > 0 {
        entities.push(
            Entity::new(4, ARITHMETIC_COMB, Position::from(TIMER_ENTITY4_POSITION))
                .with_direction(DIR_LEFT)
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: Signal::new_virtual(SIGNAL_T),
                        second_signal: None,
                        second_constant: Some((ticks_per_frame * frames_per_combinator) as i32),
                        operation: OPERATION_MOD,
                        output_signal: Signal::new_virtual(SIGNAL_S),
                    },
                }),
        );

        entities.push(
            Entity::new(5, ARITHMETIC_COMB, Position::from(TIMER_ENTITY5_POSITION))
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: Signal::new_virtual(SIGNAL_S),
                        second_signal: None,
                        second_constant: Some(ticks_per_frame as i32),
                        operation: OPERATION_DIV,
                        output_signal: Signal::new_virtual(SIGNAL_F),
                    },
                }),
        );

        entities.push(
            Entity::new(6, ARITHMETIC_COMB, Position::from(TIMER_ENTITY6_POSITION))
                .with_direction(DIR_RIGHT)
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: Signal::new_virtual(SIGNAL_EACH),
                        second_signal: None,
                        second_constant: Some(grayscale_bits as i32),
                        operation: OPERATION_MUL,
                        output_signal: Signal::new_virtual(SIGNAL_EACH),
                    },
                })
                .with_description(
                    "Calculates the bit shift necessary for the frame we should be rendering.",
                ),
        );

        wires.push([3, 2, 4, 2]);
        wires.push([4, 4, 5, 2]);
        wires.push([5, 4, 6, 2]);
        wires.push([6, 4, 3, 4]);
    }

    (entities, wires)
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
                    5,
                    current_entity - num_substations_width,
                    5,
                ]);
            }
            if j > 0 {
                substation_wires.push([current_entity, 5, current_entity - 1, 5]);
            }
            current_entity += 1;
        }
    }
    (
        substation_entities,
        substation_wires,
        occupied_cells,
        current_entity,
    )
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
pub fn generate_frame_combinators(
    frame_outputs: &[Vec<CombinatorOutput>],
    occupied_y: &HashSet<i32>,
    ticks_per_group: u32,
    base_entity_number: u32,
    base_decider_x: f64,
    base_y: f64,
    max_rows_per_group: u32,
    grayscale_bits: u32,
) -> (Vec<Entity>, Vec<Wire>, u32) {
    let mut curr_entity_idx = base_entity_number;
    let num_frames = frame_outputs.len();
    let mut new_entities = Vec::with_capacity(num_frames * 2 + 3);
    let mut wires = Vec::with_capacity(num_frames * 3 + 4);

    if grayscale_bits > 0 {
        let shifter1_x = base_decider_x;
        let shifter2_x = shifter1_x + 2.0;
        new_entities.push(
            Entity::new(
                curr_entity_idx,
                ARITHMETIC_COMB,
                Position::new(shifter1_x, base_y + 1.0),
            )
            .with_direction(DIR_RIGHT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal::new_virtual(SIGNAL_EACH),
                    second_signal: Some(Signal::new_virtual(SIGNAL_F)),
                    second_constant: None,
                    operation: OPERATION_SHIFT_R,
                    output_signal: Signal::new_virtual(SIGNAL_EACH),
                },
            }),
        );

        let first_decider_id = curr_entity_idx
            + if grayscale_bits == 1 || grayscale_bits == 4 {
                4
            } else {
                3
            };
        wires.push([curr_entity_idx, 2, first_decider_id, 2]);
        wires.push([curr_entity_idx, 1, first_decider_id, 3]);
        curr_entity_idx += 1;

        new_entities.push(
            Entity::new(
                curr_entity_idx,
                ARITHMETIC_COMB,
                Position::new(shifter2_x, base_y + 1.0),
            )
            .with_direction(DIR_RIGHT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal::new_virtual(SIGNAL_EACH),
                    second_signal: None,
                    second_constant: Some(match grayscale_bits {
                        1 => 1,
                        4 => 15,
                        _ => 255,
                    }),
                    operation: OPERATION_AND,
                    output_signal: Signal::new_virtual(SIGNAL_EACH),
                },
            }),
        );
        wires.push([curr_entity_idx - 1, 4, curr_entity_idx, 2]);
        curr_entity_idx += 1;
        if grayscale_bits == 1 || grayscale_bits == 4 {
            new_entities.push(
                Entity::new(
                    curr_entity_idx,
                    ARITHMETIC_COMB,
                    Position::new(shifter2_x + 1.0, base_y + 2.0),
                )
                .with_direction(DIR_LEFT)
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: Signal::new_virtual(SIGNAL_EACH),
                        second_signal: None,
                        second_constant: Some(if grayscale_bits == 1 { 255 } else { 17 }),
                        operation: OPERATION_MUL,
                        output_signal: Signal::new_virtual(SIGNAL_EACH),
                    },
                }),
            );
            wires.push([curr_entity_idx - 1, 4, curr_entity_idx, 2]);
            curr_entity_idx += 1;
        }
    }

    let mut first_decider = true;
    let mut x_offset = 0.0;
    let mut y_offset = 0.0;
    let mut row_in_this_column = 0;
    let mut previous_first_decider: Option<u32> = None;
    for (i, outputs) in frame_outputs.iter().enumerate() {
        let mut current_y = base_y - row_in_this_column as f64 - y_offset;
        if occupied_y.contains(&(current_y.floor() as i32)) {
            y_offset += 2.0;
            current_y -= 2.0;
        }
        let decider_num = curr_entity_idx + 1;
        let lower_bound = (i as u32 * ticks_per_group) as i32;
        let upper_bound = ((i as u32 + 1) * ticks_per_group) as i32;
        let decider_entity = Entity::new(
            decider_num,
            DECIDER_COMB,
            Position::new(base_decider_x + x_offset, current_y),
        )
        .with_direction(DIR_RIGHT)
        .with_control_behavior(ControlBehavior::Decider {
            decider_conditions: DeciderConditions {
                conditions: vec![
                    Condition {
                        first_signal: Signal::new_virtual(SIGNAL_T),
                        constant: lower_bound,
                        comparator: COMPARATOR_GE,
                        compare_type: None,
                    },
                    Condition {
                        first_signal: Signal::new_virtual(SIGNAL_T),
                        constant: upper_bound,
                        comparator: COMPARATOR_LT,
                        compare_type: Some(COMPARE_AND),
                    },
                ],
                outputs: outputs.clone(), // Cloning the outputs once per entity.
            },
        });
        new_entities.push(decider_entity);

        if !first_decider {
            let previous_decider_id = decider_num - 2;
            wires.push([previous_decider_id, 2, decider_num, 2]);
            wires.push([previous_decider_id, 3, decider_num, 3]);
        } else {
            if let Some(prev) = previous_first_decider {
                wires.push([prev, 2, decider_num, 2]);
                wires.push([prev, 3, decider_num, 3]);
            }
            previous_first_decider = Some(decider_num);
        }

        first_decider = false;
        curr_entity_idx += 2;
        row_in_this_column += 1;

        if row_in_this_column >= max_rows_per_group {
            row_in_this_column = 0;
            first_decider = true;
            y_offset = 0.0;
            x_offset += 2.0;
        }
    }
    (new_entities, wires, curr_entity_idx)
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
    signals: Vec<Arc<Signal>>,
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
                lamp_wires.push([current_entity, 1, current_entity - 1, 1]);
                lamp_wires.push([current_entity, 2, current_entity - 1, 2]);
                top_right_lamp = current_entity;
            } else if use_horizontal_lamp_wires {
                if let Some(prev) = previous_entity {
                    lamp_wires.push([current_entity, 1, prev, 1]);
                }
                previous_entity = Some(current_entity);
            }

            if r > 0 {
                if !use_horizontal_lamp_wires || (c + 1 == grid_width as i32) {
                    if let Some(&prev_entity) = previous_entities.get(&x) {
                        lamp_wires.push([current_entity, 1, prev_entity, 1]);
                    }
                }
            }
            previous_entities.insert(x, current_entity);
            current_entity += 1;
        }
    }
    (lamp_entities, lamp_wires, current_entity, top_right_lamp)
}

/// Builds the complete blueprint JSON by combining all components.
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
    name: String,
    frame_data: &mut FrameData,
    use_dlc: bool,
    grayscale_bits: u32,
    substation_quality: String,
    use_green_lamp_wires: bool,
    use_horizontal_lamp_wires: bool,
) -> Result<Blueprint, JsValue> {
    report_progress(0, "Starting blueprint update");

    // Get signals internally.
    let signals: Vec<Arc<Signal>> = get_signals_with_quality(use_dlc);

    if frame_data.total_frames() == 0 {
        return Err(JsValue::from_str("No sampled frames"));
    }

    let use_grayscale = grayscale_bits > 0;
    let total_frames = frame_data.total_frames() as u32;
    let frames_per_combinator = if grayscale_bits > 0 {
        32 / grayscale_bits
    } else {
        1
    };
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
        (((total_frames as f64 / ((max_columns_per_group as f64 / 2.0).floor())).ceil())
            / frames_per_combinator as f64)
            .ceil() as u32;

    let ticks_per_frame = (60.0 / frame_data.fps() as f64) as u32;
    let stop = total_frames * ticks_per_frame;
    let (timer_entities, timer_wires) =
        generate_timer(stop, grayscale_bits, ticks_per_frame, frames_per_combinator);

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
            substation_quality,
            full_width,
            full_height,
            max_rows_per_group
                + match grayscale_bits {
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

    for group_index in 0..num_groups {
        let group_left = group_index * max_columns_per_group;
        let group_right = ((group_index + 1) * max_columns_per_group).min(full_width);
        let group_width = group_right - group_left;

        let group_frames_outputs = if use_grayscale {
            let mut outputs = Vec::new();

            loop {
                let mut frames = Vec::with_capacity(frames_per_combinator as usize);

                for _ in 0..frames_per_combinator {
                    match frame_data.next() {
                        Some(frame) => frames.push(frame),
                        None => break,
                    }
                }

                if frames.is_empty() {
                    break;
                }

                let cropped_frames = frames
                    .into_iter()
                    .map(|frame| {
                        frame.map(|frame| frame.crop_imm(group_left, 0, group_width, full_height))
                    })
                    .collect::<Result<Vec<_>, JsValue>>()?;

                outputs.push(pack_grayscale_frames_to_outputs(
                    &cropped_frames,
                    signals.clone(),
                    grayscale_bits,
                )?);
            }

            outputs
        } else {
            let mut outputs = Vec::new();
            for frame in frame_data.by_ref() {
                let frame = frame?;
                let cropped = frame.crop_imm(group_left, 0, group_width, full_height);
                outputs.push(frame_to_outputs(&cropped, signals.clone())?);
            }
            outputs
        };

        let group_offset_x = group_index * max_columns_per_group;
        let first_connection_entity = if use_grayscale {
            next_entity
        } else {
            next_entity + 1
        };

        let (group_combinators, mut group_comb_wires, new_next_entity) = generate_frame_combinators(
            &group_frames_outputs,
            &substation_occupied_y,
            ticks_per_frame * frames_per_combinator,
            next_entity,
            group_offset_x as f64 + 0.5,
            match grayscale_bits {
                1 | 4 => -5.0,
                8 => -4.0,
                _ => -3.0,
            },
            max_rows_per_group,
            grayscale_bits,
        );
        if group_index == 0 {
            group_comb_wires.push([3, 4, first_connection_entity, 2]);
        }
        next_entity = new_next_entity;

        let (group_lamps, mut group_lamp_wires, new_next_entity, top_right_lamp) = generate_lamps(
            signals.clone(),
            group_width,
            full_height,
            &occupied_cells,
            next_entity,
            group_offset_x as i32,
            0,
            use_grayscale,
            use_horizontal_lamp_wires,
        );
        next_entity = new_next_entity;

        let first_lamp_entity = group_lamps[0].entity_number;
        if use_grayscale {
            group_comb_wires.push([first_lamp_entity, 2, first_connection_entity, 2]);
            let last_shifter = if grayscale_bits == 1 || grayscale_bits == 4 {
                first_connection_entity + 2
            } else {
                first_connection_entity + 1
            };
            group_comb_wires.push([first_lamp_entity, 1, last_shifter, 3]);
        } else {
            group_comb_wires.push([first_lamp_entity, 1, first_connection_entity, 3]);
            group_comb_wires.push([first_lamp_entity, 2, first_connection_entity, 2]);
        }

        if let Some(prev) = previous_top_right_lamp {
            group_lamp_wires.push([group_lamps[0].entity_number, 2, prev, 2]);
        }
        previous_top_right_lamp = Some(top_right_lamp);

        all_entities.extend(group_combinators);
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

    // Swap all wires if requested (default uses red wires, so swap all for green)
    if use_green_lamp_wires {
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

    let blueprint = Blueprint {
        blueprint: BlueprintInner {
            icons: vec![Icon {
                signal: Signal::new_virtual(DECIDER_COMB),
                index: 1,
            }],
            entities: all_entities,
            wires: all_wires,
            item: BLUEPRINT,
            label: name.clone(),
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
pub fn frame_to_outputs(
    frame: &image::DynamicImage,
    signals: Vec<Arc<Signal>>,
) -> Result<Vec<CombinatorOutput>, JsValue> {
    let (width, height) = frame.dimensions();
    let num_pixels = (width * height) as usize;
    if num_pixels > signals.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels,
            signals.len()
        )));
    }
    let rgb_image = frame.to_rgb8();
    let pixels = rgb_image.into_raw();
    let mut outputs = Vec::with_capacity(num_pixels);
    for (i, chunk) in pixels.chunks(3).enumerate() {
        if chunk.len() < 3 {
            continue;
        }
        let value = rgb_to_int(chunk[0], chunk[1], chunk[2]) as i32;
        let signal = Arc::clone(&signals[i]);
        outputs.push(CombinatorOutput::new(signal, Some(value)));
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
pub fn pack_grayscale_frames_to_outputs(
    frames: &[image::DynamicImage],
    signals: Vec<Arc<Signal>>,
    grayscale_bits: u32,
) -> Result<Vec<CombinatorOutput>, JsValue> {
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames provided for packing"));
    }

    let (width, height) = frames[0].dimensions();
    let num_pixels = (width * height) as usize;

    if num_pixels > signals.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels,
            signals.len()
        )));
    }

    let luma_images: Vec<_> = frames.iter().map(|frame| frame.to_luma8()).collect();
    let mut outputs = Vec::with_capacity(num_pixels);

    for i in 0..num_pixels {
        let mut packed_value = 0u32;

        for (j, img) in luma_images.iter().enumerate() {
            let pixel_value = img.as_raw()[i];
            let value = match grayscale_bits {
                1 => (pixel_value >= GRAYSCALE_THRESHOLD) as u32,
                4 => (pixel_value >> 4) as u32,
                8 => pixel_value as u32,
                _ => {
                    return Err(JsValue::from_str("Unsupported grayscale bit depth"));
                }
            };

            packed_value |= value << (grayscale_bits * j as u32);
        }

        outputs.push(CombinatorOutput::new(
            Arc::clone(&signals[i]),
            Some(packed_value as i32),
        ));
    }

    Ok(outputs)
}
