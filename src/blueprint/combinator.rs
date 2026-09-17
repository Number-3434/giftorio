use crate::blueprint::{macros::*, models::*};
use crate::constants::*;
use std::{collections::HashSet, sync::Arc};

/// Generates combinator entities and wiring for each frame group.
///
/// # Arguments
///
/// * `occupied_y` - Set of Y coordinates occupied by substations.
/// * `ticks_per_group` - Ticks per group.
/// * `base_ent_n` - Starting entity number.
/// * `base_dc_x` - Base X coordinate for decider combinators.
/// * `base_y` - Base Y coordinate for placement.
/// * `max_rows_per_group` - Maximum rows per group.
/// * `args` - Uses BlueprintArgs.grayscale_bits (affects extra combinators).
///
/// # Returns
///
/// A tuple containing combinator entities, their wires, and
/// (first connection entity, target output entity, next entity number).
///
/// Does not populate combinators with data.
pub fn generate_combinators(
    ticks_per_group: u64,
    occupied_y: &HashSet<i32>,
    base_ent_n: u32,
    base_dc_x: f64,
    base_y: f64,
    max_rows_per_group: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Entity>, Vec<Wire>, (u32, u32, u32)) {
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
    let shifter1_x = base_dc_x + (1 as f64) * 2.0;
    let cb2_x = base_dc_x + (2 as f64) * 2.0;

    if use_delta_comp {
        let mut en = Entity::new(curr_ent_n, DECIDER_COMB, (base_dc_x, base_y + 2.0));
        let dc = DeciderConditions {
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
        };
        en = en.with_tag("delay comb").with_direction(DIR_R);
        en = en.with_control_behavior(ControlBehavior::from_decider_conditions(dc));
        other_ents.push(en.with_description("Utility 1-tick delay combinator."));
        first_connection_ent_n = Some(curr_ent_n);
        curr_ent_n += 1;

        let desc = "Memory cell for delta compression. This holds all the signal values, then allows us to set any signal value by applying a delta. We can reach any number in one tick using overflow logic.";
        let mut en = Entity::new(curr_ent_n, DECIDER_COMB, (base_dc_x, base_y + 1.0));
        let dc = DeciderConditions {
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
            outputs: vec![
                CombinatorOutput::new(Arc::from(Signal::new_virtual(SIG_EACH)), None)
                    .with_networks(NetworkFilters::green()),
            ],
        };
        en = en.with_tag("memory comb").with_direction(DIR_L);
        en = en.with_control_behavior(ControlBehavior::from_decider_conditions(dc));
        other_ents.push(en.with_description(desc));

        // Self connection for memory
        wires.push([curr_ent_n, WIRE_G, curr_ent_n, WIRE_OUT_G]);
        wires.push(get_wires!(R other_ents; OUT "delay comb" => IN "memory comb"));
        curr_ent_n += 1;
    }

    if gray_bits > 0 {
        let mut en = Entity::new(curr_ent_n, ARITHMETIC_COMB, (shifter1_x, base_y + 1.0));
        let desc = "Shifts the input numbers until they are in the range of the current frame.";
        let ac = ArithmeticConditions {
            first_signal: Signal::new_virtual(SIG_EACH),
            second_signal: Some(Signal::new_virtual(SIG_F)),
            second_constant: None,
            operation: OP_RSHIFT,
            output_signal: Signal::new_virtual(SIG_EACH),
        };
        en = en.with_tag(">> comb").with_direction(DIR_R);
        en = en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(ac));
        other_ents.push(en.with_description(desc));

        if first_connection_ent_n.is_none() {
            first_connection_ent_n = Some(curr_ent_n);
        }
        curr_ent_n += 1;

        let mut en = Entity::new(curr_ent_n, ARITHMETIC_COMB, (cb2_x, base_y + 1.0));
        let ac = arithmetic_virtual!(SIG_EACH AND match gray_bits { 1 => 1, 4 => 15, _ => 255 } => SIG_EACH);
        en = en.with_tag("AND comb").with_direction(DIR_R).with_description("Filters out the bits that are not relevant for the current frame, after bit-shifting. (The value of each signal encodes multiple frames)");
        other_ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(ac)));
        wires.push(get_wires!(R other_ents; OUT ">> comb" => IN "AND comb"));

        if use_delta_comp {
            wires.push(get_wires!(G other_ents; OUT "memory comb" => IN ">> comb"));
            wires.push(get_wires!(R other_ents; OUT "delay comb" => IN ">> comb"));
        }

        // RSHIFT -> AND
        curr_ent_n += 1;

        if gray_bits == 1 || gray_bits == 4 {
            let mut en = Entity::new(curr_ent_n, ARITHMETIC_COMB, (cb2_x + 1.0, base_y + 2.0));
            let conds =
                arithmetic_virtual!(SIG_EACH * if gray_bits == 1 { 255 } else { 17 } => SIG_EACH);
            en = en.with_tag("* comb").with_direction(DIR_L);
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

        let en = Entity::new(curr_ent_n, DECIDER_COMB, (base_dc_x + x_offset, curr_y))
            .with_direction(DIR_R);

        if chunk_i == 0 {
            new_ent.push(en.with_tag("first data comb"));
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
        } else {
            new_ent.push(en);
        }

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

    return (
        other_ents,
        new_ent,
        wires,
        (first_connection_ent, comb_out_ent_n, curr_ent_n),
    );
}
