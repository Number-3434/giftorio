use crate::blueprint::{constants::*, macros::*, models::*, util::*};
use crate::macros::log;
use std::io::Read;
use std::{collections::HashSet, sync::Arc};
use wasm_bindgen::*;

/// Generates combinator entities and wiring for each frame group.
///
/// # Arguments
///
/// * `occupied_y` - Set of Y coordinates occupied by substations.
/// * `ticks_per_group` - Ticks per group.
/// * `base_ent_n` - Starting entity number.
/// * `start_pos` - Starting X and Y coordinates.
/// * `max_rows_per_group` - Maximum rows per group.
/// * `max_cols_per_grp` - Maximum columns per group. Used to detect if compact layout should be used.
/// * `args` - Uses BlueprintArgs.grayscale_bits (affects extra combinators).
///
/// # Returns
///
/// A tuple containing combinator entities, their wires, and
/// (first connection entity, target output entity, next entity number).
///
/// Does not populate combinators with data.
pub fn generate_combinators(
    ticks_per_group: u32,
    occupied_y: &HashSet<i32>,
    base_ent_n: u32,
    start_pos: (f64, f64),
    max_rows_per_group: u32,
    max_cols_per_grp: u32,
    args: &BlueprintArgs,
) -> Result<(Vec<Entity>, Vec<Entity>, Vec<Wire>, (u32, u32, u32)), JsValue> {
    let comb_pos_data =
        load_combinator_positions_json(include_str!("../data/combinator-positions.json"))?;
    let mut best_pos_data: Option<&CombinatorPositionData> = None;

    for d in comb_pos_data.iter() {
        log!(
            "Loaded blueprint: {}",
            serde_json::to_string_pretty(d).unwrap()
        );
        if Some(d.compression.clone()) == args.signal_compression
            && d.dim.1 <= max_cols_per_grp as f64
            && d.grayscale_bits.contains(&args.grayscale_bits)
        {
            best_pos_data = Some(d);
        }
    }
    log!(
        "best_pos_data: {}",
        serde_json::to_string_pretty(&best_pos_data).unwrap()
    );

    let mut first_connection_ent_n: Option<u32> = None;
    let mut curr_ent_n = base_ent_n;
    let mut other_ents = Vec::with_capacity(3); // shifters / etc
    let mut new_ent = Vec::with_capacity(ticks_per_group as usize * 2); // data entities
    let mut wires = Vec::with_capacity(ticks_per_group as usize * 3 + 4);
    let use_delta_comp = args
        .signal_compression
        .as_ref()
        .is_some_and(|c| *c == SignalCompression::Delta);

    let gray_bits = args.grayscale_bits;
    let mut base_y = start_pos.1;

    // base_y -= best_pos_data.unwrap_throw().dim.1 as f64;

    if use_delta_comp {
        let comp_base_y = base_y + 1.0 + if gray_bits > 0 { 0.0 } else { 0.0 };
        let mut en = Entity::new(
            curr_ent_n,
            Arc::clone(&DEC_CB),
            (start_pos.0, comp_base_y + 1.0),
        );
        let dc = DeciderConditions {
            conditions: Some(vec![Condition::new(
                Signal::new_virtual(Arc::clone(&SIG_EACH)),
                0,
                COMP_NE,
            )]),
            outputs: Some(vec![CombinatorOutput::new(
                Arc::from(Signal::new_virtual(Arc::clone(&SIG_EACH))),
                None,
            )]),
        };
        en = en.with_tag("delay comb").with_direction(DIR_R);
        en = en.with_control_behavior(ControlBehavior::from_decider_conditions(dc));
        other_ents.push(en.with_description("Utility 1-tick delay combinator."));
        first_connection_ent_n = Some(curr_ent_n);
        curr_ent_n += 1;

        let desc = "Memory cell for delta compression. This holds all the signal values, then allows us to set any signal value by applying a delta. We can reach any number in one tick using overflow logic.";
        let mut en = Entity::new(curr_ent_n, Arc::clone(&DEC_CB), (start_pos.0, comp_base_y));
        let dc = DeciderConditions {
            conditions: Some(vec![
                Condition::new(Signal::new_virtual(Arc::clone(&SIG_EACH)), 0, COMP_NE)
                    .with_first_signal_networks(NetworkFilters::green()),
                Condition::new(Signal::new_virtual(Arc::clone(&SIG_T)), 0, COMP_NE)
                    .with_compare_type(COMP_AND)
                    .with_first_signal_networks(NetworkFilters::red()),
            ]),
            outputs: Some(vec![CombinatorOutput::new(
                Arc::from(Signal::new_virtual(Arc::clone(&SIG_EACH))),
                None,
            )
            .with_networks(NetworkFilters::green())]),
        };
        en = en.with_direction(if gray_bits > 0 { DIR_R } else { DIR_L });
        en = en.with_control_behavior(ControlBehavior::from_decider_conditions(dc));
        other_ents.push(en.with_tag("memory comb").with_description(desc));

        // Self connection for memory
        wires.push([curr_ent_n, WIRE_G, curr_ent_n, WIRE_OUT_G]);
        wires.push(get_wires!(R other_ents; OUT "delay comb" => IN "memory comb"));
        curr_ent_n += 1;
    }

    if gray_bits > 0 {
        let mut en = Entity::new(
            curr_ent_n,
            Arc::clone(&ARI_CB),
            (start_pos.0 + 3.0, base_y + 1.0),
        );
        let desc = "Shifts the input numbers until they are in the range of the current frame.";
        let ac = ArithmeticConditions {
            first_signal: Some(Signal::new_virtual(Arc::clone(&SIG_EACH))),
            second_signal: Some(Signal::new_virtual(Arc::clone(&SIG_F))),
            second_constant: None,
            operation: Some(OP_RSHIFT.to_owned()),
            output_signal: Some(Signal::new_virtual(Arc::clone(&SIG_EACH))),
        };
        en = en.with_tag(">> comb").with_direction(DIR_R);
        en = en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(ac));
        other_ents.push(en.with_description(desc));

        if first_connection_ent_n.is_none() {
            first_connection_ent_n = Some(curr_ent_n);
        }
        curr_ent_n += 1;

        let mut en = Entity::new(
            curr_ent_n,
            Arc::clone(&ARI_CB),
            (start_pos.0 + 3.0, base_y + 2.0),
        );
        let ac = arithmetic_virtual!(SIG_EACH AND match gray_bits { 1 => 1, 4 => 15, _ => 255 } => SIG_EACH);
        en = en.with_tag("AND comb").with_direction(DIR_L).with_description("Filters out the bits that are not relevant for the current frame, after bit-shifting. (The value of each signal encodes multiple frames)");
        other_ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(ac)));
        wires.push(get_wires!(R other_ents; OUT ">> comb" => IN "AND comb"));

        if use_delta_comp {
            wires.push(get_wires!(G other_ents; OUT "memory comb" => IN ">> comb"));
            wires.push(get_wires!(R other_ents; OUT "delay comb" => IN ">> comb"));
        }
        curr_ent_n += 1;

        if gray_bits == 1 || gray_bits == 4 {
            let mut en = Entity::new(
                curr_ent_n,
                Arc::clone(&ARI_CB),
                (start_pos.0 + 1.5, base_y + 2.0),
            );
            let conds =
                arithmetic_virtual!(SIG_EACH * if gray_bits == 1 { 255 } else { 17 } => SIG_EACH);
            en = en.with_tag("* comb").with_direction(DIR_D);
            other_ents
                .push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));
            wires.push(get_wires!(R other_ents; OUT "AND comb" => IN "* comb"));
            curr_ent_n += 1;
        }
    } else if first_connection_ent_n.is_none() {
        first_connection_ent_n = Some(curr_ent_n);
    }

    let first_connection_ent = first_connection_ent_n.unwrap();
    let comb_out_ent_n = if gray_bits > 0 {
        (curr_ent_n - 1).max(base_ent_n)
    } else {
        entity_idx_by_tag!(other_ents, "memory comb").unwrap_or((curr_ent_n - 1).max(base_ent_n))
    };

    let mut is_first_dc = true;
    let (mut x_offset, mut y_offset) = (0.0, 0.0);
    let mut row_in_this_col = 0;
    let mut prev_first_dc_ent_n: Option<u32> = None;

    // Generates combinators up to down, then left ro right.
    for chunk_i in 0..ticks_per_group as usize {
        let mut curr_y = base_y - (row_in_this_col as f64) - y_offset;
        if occupied_y.contains(&(curr_y.floor() as i32)) {
            y_offset += 2.0;
            curr_y -= 2.0;
        }

        let mut en = Entity::new(
            curr_ent_n,
            Arc::clone(&DEC_CB),
            (start_pos.0 + x_offset, curr_y),
        )
        .with_direction(DIR_R);

        if chunk_i == 0 {
            en = en.with_tag("first data comb");
            if use_delta_comp {
                wires.push([
                    curr_ent_n,
                    WIRE_OUT_G,
                    entity_idx_by_tag!(other_ents, "memory comb").expect("No memory combinator!"),
                    WIRE_G,
                ]);
            } else if gray_bits > 0 {
                wires.push([
                    curr_ent_n,
                    WIRE_OUT_G,
                    entity_idx_by_tag!(other_ents, ">> comb").expect("No bitshift combinator!"),
                    WIRE_G,
                ]);
            }
        }
        new_ent.push(en);

        if !is_first_dc {
            // Wire to previous decider
            let prev_dc_ent_n = curr_ent_n - 1;
            wires.push([prev_dc_ent_n, WIRE_R, curr_ent_n, WIRE_R]);
            wires.push([prev_dc_ent_n, WIRE_OUT_G, curr_ent_n, WIRE_OUT_G]);
        } else {
            if let Some(prev) = prev_first_dc_ent_n {
                wires.push([prev, WIRE_R, curr_ent_n, WIRE_R]);
                wires.push([prev, WIRE_OUT_G, curr_ent_n, WIRE_OUT_G]);
            }
            prev_first_dc_ent_n = Some(curr_ent_n);
        }

        is_first_dc = false;
        curr_ent_n += 1;
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

    return Ok((
        other_ents,
        new_ent,
        wires,
        (first_connection_ent, comb_out_ent_n, curr_ent_n),
    ));
}

#[derive(serde::Serialize)]
struct CombinatorPositionData {
    compression: SignalCompression,
    dim: (f64, f64),
    grayscale_bits: Vec<u32>,
    blueprint: Blueprint,
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}
impl BoundingBox {
    pub fn dim(&self) -> (f64, f64) {
        (self.max_x - self.min_x, self.max_y - self.min_y)
    }

    /// Expands the bounding box to include the other bounding box.
    pub fn max(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    pub fn offset(&self) -> (f64, f64) {
        (-self.min_x, -self.min_y)
    }
}

fn load_combinator_positions_json(json: &str) -> Result<Vec<CombinatorPositionData>, JsValue> {
    #[derive(serde::Deserialize)]
    struct CombinatorPositionJson {
        compression: SignalCompression,
        #[serde(rename = "grayscaleBits")]
        grayscale_bits: OneOrMany<u32>,
        blueprint: String,
    }

    fn get_bounding_box(en: &Entity) -> Result<BoundingBox, JsValue> {
        let offset = match en.name.as_ref() {
            name if name == &**ARI_CB || name == &**DEC_CB => match en.direction.unwrap_or(0) {
                DIR_U | DIR_D => (1.0, 2.0),
                DIR_L | DIR_R => (2.0, 1.0),
                _ => return Err(JsValue::from_str("Unsupported entity direction")),
            },
            name if name == &**CONSTANT_COMB => (1.0, 1.0),
            _ => return Err(JsValue::from_str("Unsupported entity type")),
        };
        Ok(BoundingBox {
            min_x: en.position.x - offset.0 / 2.0,
            max_x: en.position.x + offset.0 / 2.0,
            min_y: en.position.y - offset.1 / 2.0,
            max_y: en.position.y + offset.1 / 2.0,
        })
    }

    Ok(serde_json::from_str::<Vec<CombinatorPositionJson>>(&json)
        .map_js_err("JSON error")?
        .iter()
        .map(|x| {
            // Decode blueprint string (removing leading "0" prefix)
            let b64 = base64::decode(x.blueprint[1..].as_bytes()).map_js_err("base64 error")?;
            let mut zlib = flate2::read::ZlibDecoder::new(&b64[..]);
            let mut buf = String::new();

            zlib.read_to_string(&mut buf).map_js_err("base64 error")?;

            let mut blueprint = Blueprint::from_json(buf).map_js_err("JSON error")?;
            let mut bounds = BoundingBox {
                min_x: f64::MAX,
                min_y: f64::MAX,
                max_x: f64::MIN,
                max_y: f64::MIN,
            };

            for en in blueprint.blueprint.entities.iter() {
                bounds = bounds.max(&get_bounding_box(en)?);
            }
            let offset = bounds.offset();

            for en in blueprint.blueprint.entities.iter_mut() {
                en.position.x += offset.0;
                en.position.y += offset.1;
            }

            Ok(CombinatorPositionData {
                compression: x.compression.clone(),
                dim: bounds.dim(),
                grayscale_bits: x.grayscale_bits.to_vec(),
                blueprint,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?)
}
