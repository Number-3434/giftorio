use crate::constants::*;
use crate::image_processing::{rgb_to_int, FrameData};
use crate::macros::*;
use crate::models::*;
use crate::signals::get_signals_with_quality;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use wasm_bindgen::JsValue;

/// High-level utility macro for making wire connections.
macro_rules! get_wires {
    ($wire_type:ident $ents:ident; $src_type:ident $src_name:expr => $dst_type:ident $dst_name:expr) => {
        [
            entity_idx_by_tag!($ents, $src_name).expect("No entity found with tag"),
            get_wires!(@wire_type $wire_type, $src_type),
            entity_idx_by_tag!($ents, $dst_name).expect("No entity found with tag"),
            get_wires!(@wire_type $wire_type, $dst_type),
        ]
    };

    (@wire_type R, IN) => { 1 };
    (@wire_type G, IN) => { 2 };
    (@wire_type R, OUT) => { 3 };
    (@wire_type G, OUT) => { 4 };

    (@wire_type C, IN) => { 5 };
    (@wire_type C, OUT) => { 6 };
}
macro_rules! entity_idx_by_tag {
    ($ents:ident, $tag:expr) => {
        $ents
            .iter()
            .find(|e| e.has_tag($tag))
            .map(|e| e.entity_number)
    };
}

#[derive(serde::Deserialize)]
pub struct BlueprintArgs {
    pub name: String,
    #[serde(rename = "imageType")]
    pub image_type: String,

    #[serde(rename = "combinatorCompression")]
    pub combinator_compression: Option<BlueprintCombinatorCompression>,

    #[serde(rename = "includeLastFrame")]
    pub last_frame: bool,

    #[serde(rename = "useDLC")]
    pub use_dlc: bool,

    #[serde(rename = "targetFps")]
    pub target_fps: u32,

    #[serde(rename = "maxSize")]
    pub max_size: u32,

    #[serde(rename = "substationQuality")]
    pub substation_quality: BlueprintSubstationQuality,

    #[serde(rename = "grayscaleBits")]
    pub grayscale_bits: u32,

    #[serde(rename = "resamplingFilter")]
    pub sampling_filter: BlueprintResamplingFilter,

    #[serde(rename = "useGreenLampWires")]
    pub prefer_green_wires: bool,

    #[serde(rename = "useHorizontalLampWires")]
    pub prefer_horizontal_wires: bool,

    #[serde(rename = "sortSignals")]
    pub sort_signals: bool,

    #[serde(rename = "outputFormat")]
    pub output_format: BlueprintOutputFormat,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
pub enum BlueprintSubstationQuality {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "uncommon")]
    Uncommon,
    #[serde(rename = "rare")]
    Rare,
    #[serde(rename = "epic")]
    Epic,
    #[serde(rename = "legendary")]
    Legendary,
}
impl Display for BlueprintSubstationQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Normal => write!(f, "normal"),
            Self::Uncommon => write!(f, "uncommon"),
            Self::Rare => write!(f, "rare"),
            Self::Epic => write!(f, "epic"),
            Self::Legendary => write!(f, "legendary"),
        }
    }
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
pub enum BlueprintResamplingFilter {
    #[serde(rename = "catrom")]
    Catrom,
    #[serde(rename = "gaussian")]
    Gaussian,
    #[serde(rename = "lanczos3")]
    Lanczos3,
    #[serde(rename = "nearest")]
    Nearest,
    #[serde(rename = "triangle")]
    Triangle,
}

#[derive(serde::Deserialize, Clone, PartialEq)]
pub enum BlueprintCombinatorCompression {
    #[serde(rename = "temporal")]
    Temporal { window: u32 },
    #[serde(rename = "delta")]
    Delta,
}

#[derive(serde::Deserialize, Clone, PartialEq)]
pub enum BlueprintOutputFormat {
    #[serde(rename = "blueprint")]
    Blueprint,
    #[serde(rename = "json")]
    Json,
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
    gray_bits: u32,
    ticks_per_frame: u32,
    frames_per_comb: u32,
) -> (Vec<Entity>, Vec<Wire>) {
    let mut ents = Vec::new();
    let mut wires = Vec::new();

    ents.push(
        Entity::new(1, CONSTANT_COMB, TIMER1_POS)
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
    ents.push(
        Entity::new(2, DECIDER_COMB, TIMER2_POS)
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::Decider {
                decider_conditions: DeciderConditions {
                    conditions: vec![Condition {
                        first_signal: Signal::new_virtual(SIG_T),
                        constant: stop as i32,
                        comparator: COMP_LT,
                        compare_type: None,
                        first_signal_networks: None,
                    }],
                    outputs: vec![CombinatorOutput::new(
                        Arc::from(Signal::new_virtual(SIG_T)),
                        None,
                    )],
                },
            }).with_description("[virtual-signal=signal-T] is our timer that ticks up 60 times per second up to the max ticks for the entire gif. \
            When it reaches the max, it will start over, resetting the gif. This timer is used to know which frames to render.")
    );
    ents.push(
        Entity::new(3, ARITHMETIC_COMB, TIMER3_POS)
            .with_direction(DIR_R)
            .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                arithmetic_virtual!(SIG_T - 1 => SIG_T),
            )),
    );

    wires.push([1, WIRE_R, 2, WIRE_R]);
    wires.push([2, WIRE_R, 2, WIRE_OUT_R]);
    wires.push([2, WIRE_R, 3, WIRE_R]);

    if gray_bits > 0 {
        ents.push(
            Entity::new(4, ARITHMETIC_COMB, TIMER4_POS)
                .with_direction(DIR_L)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_T % (ticks_per_frame * frames_per_comb) => SIG_S),
                )),
        );
        ents.push(
            Entity::new(5, ARITHMETIC_COMB, TIMER5_POS).with_control_behavior(
                ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_S / ticks_per_frame => SIG_F),
                ),
            ),
        );
        ents.push(
            Entity::new(6, ARITHMETIC_COMB, TIMER6_POS)
                .with_direction(DIR_R)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_EACH * gray_bits => SIG_EACH),
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

    return (ents, wires);
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
    lamp_width: u32,
    lamp_height: u32,
    n_frames: u32,
    base_ent_num: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, HashSet<(i32, i32)>, u32) {
    if args.substation_quality == BlueprintSubstationQuality::None {
        return (Vec::new(), Vec::new(), HashSet::new(), base_ent_num);
    }
    let coverage = match args.substation_quality {
        BlueprintSubstationQuality::Normal => 18,
        BlueprintSubstationQuality::Uncommon => 20,
        BlueprintSubstationQuality::Rare => 22,
        BlueprintSubstationQuality::Epic => 24,
        BlueprintSubstationQuality::Legendary => 28,
        _ => 18,
    };

    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut occupied = HashSet::new();
    let mut curr_ent_num = base_ent_num;
    let half_cov = ((coverage as f64) - 2.0) / 2.0;
    let mut n_frame_cov = (((n_frames as f64) - half_cov) / (coverage as f64)).ceil() as u32;
    while ((n_frames as f64) - half_cov + (n_frame_cov as f64 * 2.0))
        > (n_frame_cov as f64 * coverage as f64)
    {
        n_frame_cov += 1;
    }
    let n_subs_width = (((lamp_width as f64) - half_cov) / (coverage as f64)).ceil() as u32 + 1;
    let n_subs_height =
        (((lamp_height as f64) - half_cov) / (coverage as f64)).ceil() as u32 + 1 + n_frame_cov;
    let start_x = -1;
    let start_y = -1 - (n_frame_cov as i32 * coverage as i32);

    for i in 0..n_subs_height {
        for j in 0..n_subs_width {
            let x = start_x + (j as i32 * coverage as i32);
            let y = start_y + (i as i32 * coverage as i32);
            let mut ent = Entity::new(curr_ent_num, SUBSTATION, (x as f64, y as f64));
            ent.quality = (args.substation_quality != BlueprintSubstationQuality::Normal)
                .then_some(args.substation_quality.to_string());
            ents.push(ent);

            // Mark occupied cells.
            occupied.insert((x - 1, y - 1));
            occupied.insert((x - 1, y));
            occupied.insert((x, y - 1));
            occupied.insert((x, y));
            if i > 0 {
                wires.push([curr_ent_num, WIRE_C, curr_ent_num - n_subs_width, WIRE_C]);
            }
            if j > 0 {
                wires.push([curr_ent_num, WIRE_C, curr_ent_num - 1, WIRE_C]);
            }
            curr_ent_num += 1;
        }
    }

    return (ents, wires, occupied, curr_ent_num);
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
/// A tuple containing combinator entities, their wires, and
/// (first connection entity, target output entity, next entity number).
///
/// Does not populate combinators with data.
pub fn mk_frame_combs(
    n_chunks: u64,
    occupied_y: &HashSet<i32>,
    base_ent_num: u32,
    base_dc_x: f64,
    base_y: f64,
    max_rows_per_group: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Entity>, Vec<Wire>, (u32, u32, u32)) {
    let mut first_connection_ent_num: Option<u32> = None;
    let mut curr_ent_num = base_ent_num;
    let mut other_ents = Vec::with_capacity(3); // shifters / etc
    let mut new_ent = Vec::with_capacity(n_chunks as usize * 2); // data entities
    let mut wires = Vec::with_capacity(n_chunks as usize * 3 + 4);
    let use_delta_comp = args
        .combinator_compression
        .as_ref()
        .is_some_and(|c| *c == BlueprintCombinatorCompression::Delta);

    let comp1_x = base_dc_x;
    let shifter1_x = base_dc_x + (1 as f64) * 2.0;
    let shifter2_x = base_dc_x + (2 as f64) * 2.0;

    if use_delta_comp {
        other_ents.push(
            Entity::new(curr_ent_num, DECIDER_COMB, (comp1_x + 1.0, base_y + 2.0))
                .with_tag("delay comb")
                .with_direction(DIR_R)
                .with_control_behavior(ControlBehavior::from_decider_conditions(
                    DeciderConditions {
                        conditions: vec![Condition {
                            first_signal: Signal::new_virtual(SIG_EACH),
                            constant: 0,
                            comparator: COMP_NE,
                            compare_type: None,
                            first_signal_networks: None,
                        }],
                        outputs: vec![CombinatorOutput::new(
                            Arc::from(Signal::new_virtual(SIG_EACH)),
                            None,
                        )],
                    },
                ))
                .with_description("Utility 1-tick delay combinator."),
        );
        first_connection_ent_num = Some(curr_ent_num);
        curr_ent_num += 1;

        let desc = "Memory cell for delta compression. This holds all the signal values, then allows us to set any signal value by applying a delta. We can reach any number in one tick using overflow logic.";
        other_ents.push(
            Entity::new(curr_ent_num, DECIDER_COMB, (comp1_x + 3.0, base_y + 2.0))
                .with_tag("memory comb")
                .with_direction(DIR_R)
                .with_control_behavior(ControlBehavior::from_decider_conditions(
                    DeciderConditions {
                        conditions: vec![
                            Condition {
                                first_signal: Signal::new_virtual(SIG_EACH),
                                constant: 0,
                                comparator: COMP_NE,
                                compare_type: None,
                                first_signal_networks: Some(NetworkFilters::green()),
                            },
                            Condition {
                                first_signal: Signal::new_virtual(SIG_T),
                                constant: 0,
                                comparator: COMP_NE,
                                compare_type: Some(COMP_AND),
                                first_signal_networks: Some(NetworkFilters::red()),
                            },
                        ],
                        outputs: vec![CombinatorOutput::new(
                            Arc::from(Signal::new_virtual(SIG_EACH)),
                            None,
                        )
                        .with_networks(NetworkFilters::green())],
                    },
                ))
                .with_description(desc),
        );

        // Self connection for memory
        wires.push([curr_ent_num, WIRE_G, curr_ent_num, WIRE_OUT_G]);
        wires.push(get_wires!(R other_ents; OUT "delay comb" => IN "memory comb"));
        curr_ent_num += 1;
    }

    if args.grayscale_bits > 0 {
        other_ents.push(
            Entity::new(curr_ent_num, ARITHMETIC_COMB, (shifter1_x, base_y + 1.0))
                .with_tag(">> comb")
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
                .with_description(
                    "Shifts the input numbers until they are in the range of the current frame.",
                ),
        );

        if first_connection_ent_num.is_none() {
            first_connection_ent_num = Some(curr_ent_num);
        }
        curr_ent_num += 1;

        other_ents.push(
            Entity::new(curr_ent_num, ARITHMETIC_COMB, (shifter2_x, base_y + 1.0))
                .with_tag("AND comb")
                .with_direction(DIR_R)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_EACH AND match args.grayscale_bits {
                    1 => 1,
                    4 => 15,
                    _ => 255,
                } => SIG_EACH),
                ))
                .with_description("Filters out the bits that are not relevant for the current frame, after bit-shifting. (The value of each signal encodes multiple frames)"),
        );
        wires.push(get_wires!(R other_ents; OUT ">> comb" => IN "AND comb"));

        if use_delta_comp {
            wires.push(get_wires!(G other_ents; OUT "memory comb" => IN ">> comb"));
            wires.push(get_wires!(R other_ents; OUT "delay comb" => IN ">> comb"));
        }

        // RSHIFT -> AND
        curr_ent_num += 1;

        if args.grayscale_bits == 1 || args.grayscale_bits == 4 {
            other_ents.push(
                Entity::new(
                    curr_ent_num,
                    ARITHMETIC_COMB,
                    (shifter2_x + 1.0, base_y + 2.0),
                )
                .with_tag("* comb")
                .with_direction(DIR_L)
                .with_control_behavior(ControlBehavior::from_arithmetic_conditions(
                    arithmetic_virtual!(SIG_EACH * if args.grayscale_bits == 1 { 255 } else { 17 } => SIG_EACH),
                )),
            );
            wires.push(get_wires!(R other_ents; OUT "AND comb" => IN "* comb"));
            curr_ent_num += 1;
        } else {
        }
    } else if first_connection_ent_num.is_none() {
        first_connection_ent_num = Some(curr_ent_num);
    }

    let first_connection_ent = first_connection_ent_num.unwrap();
    let comb_out_ent_num = if args.grayscale_bits > 0 {
        (curr_ent_num - 1).max(base_ent_num)
    } else {
        entity_idx_by_tag!(other_ents, "memory comb")
            .unwrap_or((curr_ent_num - 1).max(base_ent_num))
    };

    let mut is_first_dc = true;
    let (mut x_offset, mut y_offset) = (0.0, 0.0);
    let mut row_in_this_col = 0;
    let mut prev_first_dc: Option<u32> = None;

    // Generates combinators up to down, then left ro right.
    for chunk_i in 0..n_chunks as usize {
        let mut curr_y = base_y - (row_in_this_col as f64) - y_offset;
        if occupied_y.contains(&(curr_y.floor() as i32)) {
            y_offset += 2.0;
            curr_y -= 2.0;
        }

        let en = Entity::new(curr_ent_num, DECIDER_COMB, (base_dc_x + x_offset, curr_y))
            .with_direction(DIR_R);

        if chunk_i == 0 {
            new_ent.push(en.with_tag("first data comb"));
            if use_delta_comp {
                wires.push([
                    curr_ent_num,
                    WIRE_OUT_G,
                    entity_idx_by_tag!(other_ents, "memory comb").expect("No memory combinator!"),
                    WIRE_G,
                ]);
            } else if args.grayscale_bits > 0 {
                wires.push([
                    curr_ent_num,
                    WIRE_OUT_G,
                    entity_idx_by_tag!(other_ents, ">> comb").expect("No bitshift combinator!"),
                    WIRE_G,
                ]);
            }
        } else {
            new_ent.push(en);
        }

        if !is_first_dc {
            // Wire to previous decider
            let prev_dc_num = curr_ent_num - 1;
            wires.push([prev_dc_num, WIRE_R, curr_ent_num, WIRE_R]);
            wires.push([prev_dc_num, WIRE_OUT_G, curr_ent_num, WIRE_OUT_G]);
        } else {
            if let Some(prev) = prev_first_dc {
                wires.push([prev, WIRE_R, curr_ent_num, WIRE_R]);
                wires.push([prev, WIRE_OUT_G, curr_ent_num, WIRE_OUT_G]);
            }
            prev_first_dc = Some(curr_ent_num);
        }

        is_first_dc = false;
        curr_ent_num += 1;
        row_in_this_col += 1;

        if row_in_this_col >= max_rows_per_group {
            row_in_this_col = 0;
            is_first_dc = true;
            y_offset = 0.0;
            x_offset += 2.0;
        }
    }

    wires.push([
        first_connection_ent,
        WIRE_R,
        entity_idx_by_tag!(new_ent, "first data comb").expect("No first data comb!"),
        WIRE_R,
    ]);

    return (
        other_ents,
        new_ent,
        wires,
        (first_connection_ent, comb_out_ent_num, curr_ent_num),
    );
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
    grid_dim: (u32, u32),
    occupied_cells: &HashSet<(i32, i32)>,
    base_ent_num: u32,
    start_pos: (i32, i32),
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32, u32) {
    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut curr_ent_num = base_ent_num;
    let mut prev_ents: HashMap<i32, u32> = HashMap::new();
    let mut top_right_lamp: u32 = 0;
    let mut prev_ent_num: Option<u32>;

    for r in 0..grid_dim.1 as i32 {
        prev_ent_num = None;
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
            let lamp = Entity::new(curr_ent_num, LAMP, (x as f64, y as f64))
                .with_control_behavior(colors)
                .with_always_on(true);
            ents.push(lamp);

            if r == 0 && c > 0 {
                wires.push([curr_ent_num, WIRE_G, curr_ent_num - 1, WIRE_G]);
                wires.push([curr_ent_num, WIRE_R, curr_ent_num - 1, WIRE_R]);
                top_right_lamp = curr_ent_num;
            } else if args.prefer_horizontal_wires {
                if let Some(prev) = prev_ent_num {
                    wires.push([curr_ent_num, WIRE_G, prev, WIRE_G]);
                }
                prev_ent_num = Some(curr_ent_num);
            }

            if r > 0 {
                if !args.prefer_horizontal_wires || (c + 1 == grid_dim.0 as i32) {
                    if let Some(&prev_entity) = prev_ents.get(&x) {
                        wires.push([curr_ent_num, WIRE_G, prev_entity, WIRE_G]);
                    }
                }
            }
            prev_ents.insert(x, curr_ent_num);
            curr_ent_num += 1;
        }
    }

    return (ents, wires, curr_ent_num, top_right_lamp);
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
    // Get signals internally.
    let signals: Vec<Arc<Signal>> = get_signals_with_quality(args.use_dlc, args.sort_signals);

    if frame_data.total_frames() == 0 {
        return Err(JsValue::from_str("No sampled frames"));
    }

    let combinator_compression = args.combinator_compression.as_ref();
    let time_comp_win = combinator_compression
        .map(|c| match c {
            BlueprintCombinatorCompression::Temporal { window } => *window,
            _ => 0,
        })
        .unwrap_or(0);
    let use_delta_comp =
        combinator_compression.is_some_and(|c| *c == BlueprintCombinatorCompression::Delta);
    let n_frames = frame_data.total_frames();
    let n_scaled_frames = frame_data.total_frames() * if time_comp_win > 0 { 2 } else { 1 };
    let frames_per_cb = if args.grayscale_bits > 0 {
        32 / args.grayscale_bits
    } else {
        1
    };
    let n_comp_buf_frames = if time_comp_win > 0 {
        (time_comp_win as u64 * frame_data.fps() as u64).div_ceil(1000 * frames_per_cb as u64)
            as usize
    } else {
        1
    };
    let n_comp_frames_per_chunk = if n_comp_buf_frames > 1 { 1 } else { 0 } as usize;
    let (full_width, full_height) = frame_data.dimensions();
    let max_cols_per_grp = ((signals.len() as u32) / full_height).min(full_width);
    let n_groups = (full_width as f64 / max_cols_per_grp as f64).ceil() as u32;
    let max_cols_per_grp = full_width / n_groups;
    if max_cols_per_grp < 1 {
        return Err(JsValue::from_str(
            "Not enough signals for even one column of lamps!",
        ));
    }
    let max_rows_per_grp = (((n_scaled_frames as f64 / ((max_cols_per_grp as f64 / 2.0).floor()))
        .ceil())
        / frames_per_cb as f64)
        .ceil() as u32;

    let ticks_per_frame = (60.0 / frame_data.fps() as f64) as u32;
    let stop = n_frames * ticks_per_frame;
    let (timer_ent, timer_wires) =
        generate_timer(stop, args.grayscale_bits, ticks_per_frame, frames_per_cb);
    let mut all_ents = timer_ent;
    let mut all_wires: Vec<Wire> = timer_wires;
    let occupied_cells: HashSet<(i32, i32)>;
    let mut next_ent_num = all_ents.iter().map(|e| e.entity_number).max().unwrap_or(0) + 1;

    {
        let (ents, wires, cells, new_next) = generate_substations(
            full_width,
            full_height,
            max_rows_per_grp
                + match args.grayscale_bits {
                    1 | 4 => 2,
                    8 => 1,
                    _ => 0,
                },
            next_ent_num,
            &args,
        );
        occupied_cells = cells;
        next_ent_num = new_next;
        all_ents.extend(ents);
        all_wires.extend(wires);
    }

    let substation_occupied_y: HashSet<i32> = occupied_cells.iter().map(|(_, y)| *y).collect();
    let mut prev_top_right_lamp: Option<u32> = None;
    let mut all_data_combs: Vec<Vec<u32>> = Vec::new();

    for group_i in 0..n_groups {
        let grp_left = group_i * max_cols_per_grp;
        let grp_right = ((group_i + 1) * max_cols_per_grp).min(full_width);
        let grp_width = grp_right - grp_left;
        let grp_offset_x = group_i * max_cols_per_grp;
        let (
            other_ents,
            data_cbs,
            mut grp_cb_wires,
            (cb_in_ent_num, cb_out_ent_num, new_base_ent_num),
        ) = mk_frame_combs(
            (n_scaled_frames as u64).div_ceil(frames_per_cb as u64),
            &substation_occupied_y,
            next_ent_num,
            grp_offset_x as f64 + 0.5,
            match args.grayscale_bits {
                1 | 4 => -5.0,
                8 => -4.0,
                _ => -3.0,
            },
            max_rows_per_grp,
            args,
        );
        next_ent_num = new_base_ent_num;
        if group_i == 0 {
            // Connect first wire to comb in
            grp_cb_wires.push([3, WIRE_OUT_R, cb_in_ent_num, WIRE_R]);
        }

        #[allow(unused_variables)]
        let (grp_lamps, mut grp_lamp_wires, new_next, top_right_lamp) = generate_lamps(
            &signals,
            (grp_width, full_height),
            &occupied_cells,
            next_ent_num,
            (grp_offset_x as i32, 0),
            &args,
        );
        next_ent_num = new_next;
        let first_lamp = grp_lamps[0].entity_number;

        grp_cb_wires.push([first_lamp, WIRE_R, cb_in_ent_num, WIRE_R]);
        grp_cb_wires.push([first_lamp, WIRE_G, cb_out_ent_num, WIRE_OUT_G]);

        // Connect previous lamps together
        if let Some(prev) = prev_top_right_lamp {
            grp_lamp_wires.push([grp_lamps[0].entity_number, WIRE_R, prev, WIRE_R]);
        }
        prev_top_right_lamp = Some(top_right_lamp);

        // Track the entity indexes of the data combinators for each group
        all_data_combs.push(data_cbs.iter().map(|e| e.entity_number).collect());

        all_ents.extend(other_ents);
        all_ents.extend(data_cbs);
        all_ents.extend(grp_lamps);
        all_wires.extend(grp_cb_wires);
        all_wires.extend(grp_lamp_wires);
    }

    struct GroupPatchState {
        chunk_i: usize, // which *chunk* we’re patching (not frame)
        grayscale_buf: Vec<image::DynamicImage>,
        output_buf: Vec<Vec<i32>>,
    }

    let ticks_per_group = ticks_per_frame * frames_per_cb;
    let mut patch_states: Vec<GroupPatchState> = (0..n_groups)
        .map(|_| GroupPatchState {
            chunk_i: 0,
            grayscale_buf: Vec::with_capacity(frames_per_cb as usize),
            output_buf: Vec::with_capacity(n_comp_buf_frames),
        })
        .collect();

    let mk_cb = |start: u32, end: u32, outputs: Vec<CombinatorOutput>| {
        ControlBehavior::from_decider_conditions(DeciderConditions {
            conditions: vec![
                Condition {
                    first_signal: Signal::new_virtual(SIG_T),
                    constant: start as i32,
                    comparator: COMP_GE,
                    compare_type: None,
                    first_signal_networks: None,
                },
                Condition {
                    first_signal: Signal::new_virtual(SIG_T),
                    constant: end as i32,
                    comparator: COMP_LT,
                    compare_type: Some(COMP_AND),
                    first_signal_networks: None,
                },
            ],
            outputs,
        })
    };

    // Swap all wires if requested (default uses red wires, so swap all for green).
    // Circuit network filters are already handled.
    if !args.prefer_green_wires {
        invert_wires(&mut all_ents, &mut all_wires);
    }

    let mut frame = frame_data.next().unwrap()?;
    let mut next_frame: Option<image::DynamicImage>;

    all_ents.sort_by_key(|entity| entity.entity_number);

    loop {
        next_frame = match frame_data.next() {
            Some(frame) => Some(frame?),
            None => None,
        };
        let is_last_frame = next_frame.is_none();

        for group_i in 0..n_groups as usize {
            let state = &mut patch_states[group_i];
            let outputs: Vec<i32>;

            let grp_left = group_i as u32 * max_cols_per_grp;
            let grp_right = ((group_i as u32 + 1) * max_cols_per_grp).min(full_width);
            let grp_width = grp_right - grp_left;

            let cropped = frame.crop_imm(grp_left, 0, grp_width, full_height);
            let expected_outputs_len = (cropped.width() * cropped.height()) as usize;

            if use_delta_comp && state.output_buf.len() == 0 {
                // Pre-populate first frame with zeros
                state.output_buf.push(vec![0i32; expected_outputs_len]);
            }

            if args.grayscale_bits > 0 {
                state.grayscale_buf.push(cropped);
                if state.grayscale_buf.len() < frames_per_cb as usize && !is_last_frame {
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

            if !use_delta_comp {
                // If we're using delta compression we only compare aginst the previous frame
                state.output_buf.push(outputs.clone());
            }
            if state.output_buf.len() < n_comp_buf_frames && !is_last_frame {
                continue; // Accumulate until we have `compression_level` outputs,
            }

            // Utility to help us find the target entity number so we can populate the data
            // combinator with the correct data. (Data combinators already placed, but no data)
            fn find_entity(all_ent: &Vec<Entity>, target_number: u32) -> usize {
                all_ent
                    .binary_search_by_key(&target_number, |e| e.entity_number)
                    .expect("target entity number not found")
            }

            // If using delta compression, only store the difference between the current frame
            // and the previous frame
            if use_delta_comp {
                let prev_outputs = &state.output_buf[0];
                let mut target_outputs = Vec::with_capacity(expected_outputs_len);

                for i in 0..expected_outputs_len {
                    // In Factorio we will use overflow to reach any target value.
                    // E.g. if we need to get from -100 to max_positive_i32, we will instead subtract
                    // instead of adding
                    let v = outputs[i].wrapping_sub(prev_outputs[i]);
                    if v != 0 {
                        target_outputs
                            .push(CombinatorOutput::new(Arc::clone(&signals[i]), Some(v)));
                    }
                }

                // Update the data combinator. Note this uses the real frame index at all times.
                let ent_i = find_entity(&all_ents, all_data_combs[group_i][state.chunk_i]);
                all_ents[ent_i] = all_ents[ent_i].clone().with_control_behavior(mk_cb(
                    1 + ((state.chunk_i as u32) * ticks_per_group),
                    2 + ((state.chunk_i as u32) * ticks_per_group),
                    target_outputs,
                ));
                state.output_buf[0] = outputs;
            } else {
                let mut changed_mask: Vec<bool> = vec![false; expected_outputs_len];
                let outputs_0 = &state.output_buf[0];

                if state.output_buf.len() > 1 {
                    for comb_outputs in state.output_buf.iter().skip(1) {
                        for i in 0..expected_outputs_len {
                            if !changed_mask[i] && (&comb_outputs[i] != &outputs_0[i]) {
                                changed_mask[i] = true;
                            }
                        }
                    }
                } else {
                    for i in 0..expected_outputs_len {
                        changed_mask[i] = true;
                    }
                }
                let base_chunk_i = state.chunk_i * n_comp_buf_frames;

                // Update data for all non-static frames (frames with differing pixels)
                for (comb_i, comb_outputs) in state.output_buf.iter().enumerate() {
                    let target_i = base_chunk_i + comb_i;
                    let target_comb = state.chunk_i * (n_comp_buf_frames + n_comp_frames_per_chunk)
                        + comb_i
                        + n_comp_frames_per_chunk;

                    // Only contains non-changed pixels (i.e., the ones we want to store)
                    let mut target_outputs = Vec::with_capacity(signals.len());

                    // Accumulate only the changed outputs
                    for (i, v) in comb_outputs.iter().enumerate() {
                        if changed_mask[i] && *v != 0 {
                            target_outputs
                                .push(CombinatorOutput::new(Arc::clone(&signals[i]), Some(*v)));
                        }
                    }

                    let ent_i = find_entity(&all_ents, all_data_combs[group_i][target_comb]);
                    all_ents[ent_i] = all_ents[ent_i].clone().with_control_behavior(mk_cb(
                        target_i as u32 * ticks_per_group,
                        (target_i as u32 + 1) * ticks_per_group,
                        target_outputs,
                    ));
                }

                // Make unchanged pixels data combinators
                if state.output_buf.len() > 1 {
                    let mut target_outputs = Vec::with_capacity(signals.len());

                    for i in 0..expected_outputs_len {
                        if !changed_mask[i] && outputs_0[i] != 0 {
                            target_outputs.push(CombinatorOutput::new(
                                Arc::clone(&signals[i]),
                                Some(outputs_0[i]),
                            ));
                        }
                    }

                    let ent_i = find_entity(
                        &all_ents,
                        all_data_combs[group_i]
                            [state.chunk_i * (n_comp_buf_frames + n_comp_frames_per_chunk)],
                    );
                    all_ents[ent_i] = all_ents[ent_i].clone().with_control_behavior(mk_cb(
                        base_chunk_i as u32 * ticks_per_group,
                        (base_chunk_i as u32 + state.output_buf.len() as u32) * ticks_per_group,
                        target_outputs,
                    ));
                }
                state.output_buf.clear();
            }
            state.chunk_i += 1;
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
            entities: all_ents,
            wires: all_wires,
            item: BLUEPRINT,
            label: args.name.clone(),
            version: BLUEPRINT_VERSION,
        },
    };

    Ok(blueprint)
}
fn invert_wires(ents: &mut Vec<Entity>, wires: &mut Vec<Wire>) {
    fn get_swap(x: u32) -> u32 {
        match x {
            1 => 2, // circuit_green / combinator_input_green -> circuit_red / combinator_input_red
            2 => 1, // circuit_red / combinator_input_red -> circuit_green / combinator_input_green
            3 => 4, // combinator_output_green -> combinator_output_red
            4 => 3, // combinator_output_red -> combinator_output_green
            _ => x, // usually just copper
        }
    }

    for e in ents.iter_mut() {
        if let Some(control_behavior) = &mut e.control_behavior {
            if let ControlBehavior::Decider {
                decider_conditions, ..
            } = control_behavior
            {
                decider_conditions.swap_networks();
            }
        }
    }
    for w in wires.iter_mut() {
        w[1] = get_swap(w[1]);
        w[3] = get_swap(w[3]);
    }
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
#[inline(always)]
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
/// * `prev_outputs` - Previous output values, for delta compression.
/// * `grayscale_bits` - Number of bits for grayscale conversion.
///
/// # Returns
///
/// A vector of JSON objects representing output filters.
#[inline(always)]
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
            packed_value |= match grayscale_bits {
                1 => (img.as_raw()[i] >= GRAYSCALE_THRESH) as u32,
                4 => (img.as_raw()[i] >> 4) as u32,
                8 => img.as_raw()[i] as u32,
                _ => {
                    return Err(JsValue::from_str("Unsupported grayscale bit depth"));
                }
            } << (grayscale_bits * j as u32);
        }
        outputs.push(packed_value as i32);
    }
    Ok(outputs)
}
