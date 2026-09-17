use crate::blueprint::{macros::*, models::*};
use crate::constants::*;
use std::sync::Arc;

/// Generates timer entities and wires for the blueprint's timing mechanism.
///
/// This is only run once for the entire blueprint.
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
    ticks_per_frame: u32,
    frames_per_comb: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>) {
    let gray_bits = args.grayscale_bits;
    let mut ents = Vec::new();
    let mut wires = Vec::new();

    let ent = Entity::new(1, CONSTANT_COMB, TIMER1_POS).with_direction(DIR_R);
    ents.push(ent.with_control_behavior(ControlBehavior::Constant {
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
    }));

    let mut ent = Entity::new(2, DECIDER_COMB, TIMER2_POS).with_direction(DIR_R);
    ent = ent.with_description("[virtual-signal=signal-T] increments 60 times per second up to the GIF's frame count, then resets to 0. It determines the current frame.");
    ents.push(ent.with_control_behavior(ControlBehavior::Decider {
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
    }));

    let ent = Entity::new(3, ARITHMETIC_COMB, TIMER3_POS).with_direction(DIR_R);
    let conds = arithmetic_virtual!(SIG_T - 1 => SIG_T);
    ents.push(ent.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

    wires.push([1, WIRE_R, 2, WIRE_R]);
    wires.push([2, WIRE_R, 2, WIRE_OUT_R]);
    wires.push([2, WIRE_R, 3, WIRE_R]);

    if gray_bits > 0 {
        let ent = Entity::new(4, ARITHMETIC_COMB, TIMER4_POS).with_direction(DIR_L);
        let conds = arithmetic_virtual!(SIG_T % (ticks_per_frame * frames_per_comb) => SIG_S);
        ents.push(ent.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

        let ent = Entity::new(5, ARITHMETIC_COMB, TIMER5_POS);
        let conds = arithmetic_virtual!(SIG_S / ticks_per_frame => SIG_F);
        ents.push(ent.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

        let mut ent = Entity::new(6, ARITHMETIC_COMB, TIMER6_POS).with_direction(DIR_R);
        let conds = arithmetic_virtual!(SIG_EACH * gray_bits => SIG_EACH);
        ent = ent.with_description("Calculates the bit shift for current frame.");
        ents.push(ent.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

        wires.push([3, WIRE_R, 4, WIRE_R]);
        wires.push([4, WIRE_OUT_R, 5, WIRE_R]);
        wires.push([5, WIRE_OUT_R, 6, WIRE_R]);
        wires.push([6, WIRE_OUT_R, 3, WIRE_OUT_R]);
    }

    return (ents, wires);
}
