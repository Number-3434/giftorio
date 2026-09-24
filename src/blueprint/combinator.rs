use crate::blueprint::{constants::*, macros::*, models::*, substation::*, util::*};
use crate::models::ImageRotation::*;
use glam::{dvec2, DVec2, IVec2};
use std::{collections::HashMap, io::Read, sync::Arc};
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
    occupied: &mut SubstationOccupied,
    base_ent_n: u32,
    start_pos: DVec2,
    max_cols_per_grp: u32,
    args: &BlueprintArgs,
) -> Result<(Vec<Entity>, Vec<Entity>, Vec<Wire>, (u32, u32, u32)), JsValue> {
    // Positions for combinators, for different widths.
    // TODO: Don't recalculate this for every group
    let comb_pos_data = load_comb_pos_json(include_str!("../data/combinator-positions.json"))?;
    let mut best_pos_data: Option<&CombinatorPositionData> = None;
    let mut pos_dict: HashMap<String, &Entity> = HashMap::new();
    let mut base_pos = start_pos;
    let use_compact_layout = max_cols_per_grp < 2;

    for d in &comb_pos_data {
        if d.dim.x <= max_cols_per_grp as f64
            && d.compression == args.signal_compression
            && d.grayscale_bits.contains(&args.grayscale_bits)
        {
            if d.dim.x > best_pos_data.map_or(0.0, |v| v.dim.x) {
                best_pos_data = Some(d);
            }
        }
    }

    if let Some(d) = best_pos_data {
        for en in d.blueprint.blueprint.entities.iter() {
            if let Some(tag) = en.player_description.as_ref() {
                pos_dict.insert(tag.clone(), en);
            }
        }
    }
    let mut first_connection_ent_n: Option<u32> = None;
    let mut curr_ent_n = base_ent_n;
    let mut other_ents = Vec::with_capacity(3); // shifters / etc
    let mut data_cbs = Vec::with_capacity(ticks_per_group as usize * 2); // data entities
    let mut wires = Vec::with_capacity(ticks_per_group as usize * 3 + 4);
    let use_delta_comp = args
        .signal_compression
        .as_ref()
        .is_some_and(|c| *c == SignalCompression::Delta);

    let gray_bits = args.grayscale_bits;
    let height = best_pos_data.map_or(0.0, |v| v.dim.y);
    let base_offset = base_pos; // Add a constant offset on dimensions

    base_pos.y -= height; // Set offset for combinators

    // Finds the position for the combinator with the given tag, based off the blueprint string
    // we decoded earlier
    let get_pos = |tag: &str| {
        let mut pos = pos_dict
            .get(tag)
            .expect(&format!("No '{tag}' entity found!"))
            .position
            + base_offset;
        pos.y -= height;
        return pos;
    };
    let get_dir = |tag: &str| pos_dict.get(tag).unwrap().direction.unwrap_or(0);

    if use_delta_comp {
        let mut en = Entity::new(curr_ent_n, Arc::clone(&DEC_CB), get_pos("delay"));
        let each = Signal::new_virtual(Arc::clone(&SIG_EACH));
        let dc = DeciderConditions {
            conditions: Some(vec![Condition::new(each.clone(), 0, COMP_NE)]),
            outputs: Some(vec![CombinatorOutput::new(Arc::from(each), None)]),
        };
        en = en.with_tag("delay comb").with_direction(get_dir("delay"));
        en = en.with_control_behavior(ControlBehavior::from_decider_conditions(dc));
        other_ents.push(en.with_description("Utility 1-tick delay combinator."));
        first_connection_ent_n = Some(curr_ent_n);
        curr_ent_n += 1;

        let desc = "Memory cell for delta compression. This holds all the signal values, then allows us to set any signal value by applying a delta. We can reach any number in one tick using overflow logic.";
        let mut en = Entity::new(curr_ent_n, Arc::clone(&DEC_CB), get_pos("memory"));
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
        en = en.with_direction(get_dir("memory"));
        en = en.with_control_behavior(ControlBehavior::from_decider_conditions(dc));
        other_ents.push(en.with_tag("memory comb").with_description(desc));

        // Self connection for memory
        wires.push([curr_ent_n, WIRE_G, curr_ent_n, WIRE_OUT_G]);
        wires.push(mkwires!(R other_ents; OUT "delay comb" => IN "memory comb"));
        curr_ent_n += 1;
    }

    if gray_bits > 0 {
        let mut en = Entity::new(curr_ent_n, Arc::clone(&ARI_CB), get_pos(">>"));
        let desc = "Shifts the input numbers until they are in the range of the current frame.";
        let ac = ArithmeticConditions {
            first_signal: Some(Signal::new_virtual(Arc::clone(&SIG_EACH))),
            second_signal: Some(Signal::new_virtual(Arc::clone(&SIG_F))),
            second_constant: None,
            operation: Some(OP_RSHIFT.to_owned()),
            output_signal: Some(Signal::new_virtual(Arc::clone(&SIG_EACH))),
        };
        en = en.with_tag(">> comb").with_direction(get_dir(">>"));
        en = en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(ac));
        other_ents.push(en.with_description(desc));

        if first_connection_ent_n.is_none() {
            first_connection_ent_n = Some(curr_ent_n);
        }
        curr_ent_n += 1;

        let mut en = Entity::new(curr_ent_n, Arc::clone(&ARI_CB), get_pos("AND"));
        let ac = arithmetic_virtual!(SIG_EACH AND match gray_bits { 1 => 1, 4 => 15, _ => 255 } => SIG_EACH);
        en = en.with_tag("AND comb").with_direction(get_dir("AND")).with_description("Filters out the bits that are not relevant for the current frame, after bit-shifting. (The value of each signal encodes multiple frames)");
        other_ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(ac)));
        wires.push(mkwires!(R other_ents; OUT ">> comb" => IN "AND comb"));

        if use_delta_comp {
            wires.push(mkwires!(G other_ents; OUT "memory comb" => IN ">> comb"));
            wires.push(mkwires!(R other_ents; OUT "delay comb" => IN ">> comb"));
        }
        curr_ent_n += 1;

        if gray_bits == 1 || gray_bits == 4 {
            let mut en = Entity::new(curr_ent_n, Arc::clone(&ARI_CB), get_pos("*"));
            let conds =
                arithmetic_virtual!(SIG_EACH * if gray_bits == 1 { 255 } else { 17 } => SIG_EACH);
            en = en.with_tag("* comb").with_direction(get_dir("*"));
            other_ents
                .push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));
            wires.push(mkwires!(R other_ents; OUT "AND comb" => IN "* comb"));
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
    let cb_midpoint = if use_compact_layout {
        dvec2(0.5, -1.0) // Base position for vertical layout
    } else {
        dvec2(1.0, -0.5) // Base position for horizontal layout
    };
    let mut offset: DVec2;
    let mut curr_pos: DVec2;
    let mut grid_pos = IVec2::ZERO; // Current grid position (not absolute, multiplied by combinator's bounding box)
    let mut prev_ents_n: Vec<Option<u32>> = vec![None; max_cols_per_grp as usize];

    // Generates combinators up to down, then left to right.
    for chunk_i in 0..ticks_per_group as usize {
        loop {
            offset = cb_midpoint * DVec2::from(1 + 2 * grid_pos); // Set target midpoint
            curr_pos = base_pos + offset;

            let mut req = |pos: DVec2| occupied.request(pos);

            if offset.x + cb_midpoint.x > max_cols_per_grp as f64 {
                grid_pos.x = 0;
                grid_pos.y += 1;
            } else if {
                use_compact_layout
                    && (!req(curr_pos - dvec2(0.0, 0.5)) || !req(curr_pos + dvec2(0.0, 0.5)))
                    || (!req(curr_pos - dvec2(0.5, 0.0)) || !req(curr_pos + dvec2(0.5, 0.0)))
            } {
                grid_pos.x += 1;
            } else {
                break;
            }
        }

        let mut en = Entity::new(curr_ent_n, Arc::clone(&DEC_CB), curr_pos);
        if chunk_i == 0 {
            en = en.with_tag("first data comb");
            if use_delta_comp {
                let ent_n = entity_idx_by_tag!(other_ents, "memory comb").expect("No memory comb!");
                wires.push([curr_ent_n, WIRE_OUT_G, ent_n, WIRE_G]);
            } else if gray_bits > 0 {
                let ent_n = entity_idx_by_tag!(other_ents, ">> comb").expect("No >> comb!");
                wires.push([curr_ent_n, WIRE_OUT_G, ent_n, WIRE_G]);
            }
        }
        data_cbs.push(en.with_direction(if use_compact_layout { DIR_U } else { DIR_R }));

        let prefer_horizontal = true ^ matches!(args.image_rotation, Deg0 | Deg180);
        let mut cands: Vec<usize> = Vec::new();

        if grid_pos.y > 0 && (!prefer_horizontal || grid_pos.x == 0) {
            cands.push(grid_pos.x as usize); // Vertical connection
        }
        if grid_pos.x > 0 && (prefer_horizontal || grid_pos.y == 0) {
            cands.push(grid_pos.x as usize - 1); // Horizontal connection
        }
        for i in cands {
            if let Some(prev) = prev_ents_n[i] {
                wires.push([prev, WIRE_R, curr_ent_n, WIRE_R]); // Timing signals wire
                wires.push([prev, WIRE_OUT_G, curr_ent_n, WIRE_OUT_G]); // Lamp data wire
            }
        }

        prev_ents_n[grid_pos.x as usize] = Some(curr_ent_n);
        curr_ent_n += 1;
        grid_pos.x += 1;
    }
    wires.push([
        first_connection_ent,
        WIRE_R,
        entity_idx_by_tag!(data_cbs, "first data comb").expect("No first data comb!"),
        WIRE_R,
    ]);

    return Ok((
        other_ents,
        data_cbs,
        wires,
        (first_connection_ent, comb_out_ent_n, curr_ent_n),
    ));
}

#[derive(serde::Serialize)]
struct CombinatorPositionData {
    compression: Option<SignalCompression>,
    dim: DVec2,
    grayscale_bits: Vec<u32>,
    blueprint: Blueprint,
}

fn load_comb_pos_json(json: &str) -> Result<Vec<CombinatorPositionData>, JsValue> {
    #[derive(serde::Deserialize)]
    struct CombinatorPositionJson {
        compression: Option<SignalCompression>,
        #[serde(rename = "grayscaleBits")]
        grayscale_bits: OneOrMany<u32>,
        blueprints: OneOrMany<String>,
    }

    fn get_bounding_box(en: &Entity) -> Result<DBounds2, JsValue> {
        let offset = DVec2::from(match en.name.as_ref() {
            name if name == &**ARI_CB || name == &**DEC_CB => match en.direction.unwrap_or(0) {
                DIR_U | DIR_D => (1.0, 2.0),
                DIR_L | DIR_R => (2.0, 1.0),
                _ => return Err(JsValue::from_str("Unsupported entity direction")),
            },
            name if name == &**CONSTANT_COMB => (1.0, 1.0),
            _ => return Err(JsValue::from_str("Unsupported entity type")),
        });
        Ok(DBounds2 {
            min: en.position - offset / 2.0,
            max: en.position + offset / 2.0,
        })
    }

    Ok(serde_json::from_str::<Vec<CombinatorPositionJson>>(&json)
        .map_js_err("JSON error")?
        .iter()
        .flat_map(|cb| {
            cb.blueprints.to_vec().into_iter().map(move |bp| {
                // Decode blueprint string (removing leading "0" prefix)
                let b64 = base64::decode(bp[1..].as_bytes()).map_js_err("base64 error")?;
                let mut zlib = flate2::read::ZlibDecoder::new(&b64[..]);
                let mut buf = String::new();

                zlib.read_to_string(&mut buf).map_js_err("base64 error")?;

                let mut blueprint = Blueprint::from_json(buf).map_js_err("JSON error")?;
                let mut bounds = DBounds2::default();

                for en in blueprint.blueprint.entities.iter() {
                    bounds.include(&get_bounding_box(en)?);
                }
                let offset = bounds.offset();

                for en in blueprint.blueprint.entities.iter_mut() {
                    en.position += offset;
                }

                Ok(CombinatorPositionData {
                    compression: cb.compression.clone(),
                    dim: bounds.dim(),
                    grayscale_bits: cb.grayscale_bits.to_vec(),
                    blueprint,
                })
            })
        })
        .collect::<Result<Vec<CombinatorPositionData>, JsValue>>()?)
}
