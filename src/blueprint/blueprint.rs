use crate::blueprint::{
    combinator::generate_combinators, constants::*, lamp::generate_lamps, models::*,
    signals::get_signals_with_quality, substation::*, timer::generate_timer, util::*,
};
use crate::image_processing::FrameData;
use crate::macros::log;
use glam::{dvec2, uvec2, UVec2};
use std::{io, sync::Arc};
use wasm_bindgen::JsValue;

const ENCODE_CHUNK_SIZE: usize = 100;

pub struct BlueprintGenerator<'a> {
    args: &'a BlueprintArgs,
    entity_data: EntityData,
    frame_info: Option<FrameInfo<'a>>,
    dim: UVec2,
    max_cols_per_grp: u32,
    n_groups: u32,
    should_generate_cbs: bool,
    signals: Vec<Arc<Signal>>,
    state: BlueprintState,
}
struct FrameInfo<'a> {
    curr_frame: image::DynamicImage,
    frame_data: FrameData<'a>,
    frames_per_cb: u32,
    group_data_combs: Vec<Vec<Entity>>,
    n_buf_frames: usize,
    n_frames_per_chunk: usize,
    n_scaled_frames: u32,
    next_frame: Option<image::DynamicImage>,
    patch_states: Vec<GroupPatchState>,
    ticks_per_frame: u32,
    use_delta_comp: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum BlueprintState {
    Start,
    Entities,
    Data,
    Wires,
    Finished,
}

struct EntityData {
    ents: Vec<Entity>,
    wires: Vec<Wire>,
    next_ent_n: u32,
}
struct GroupPatchState {
    chunk_i: usize, // which *chunk* we’re patching (not frame)
    frame_buf: Vec<image::DynamicImage>,
    sig_buf: Vec<Vec<i32>>,
}

impl<'a> BlueprintGenerator<'a> {
    pub fn new(
        frame_data: Option<FrameData<'a>>,
        args: &'a BlueprintArgs,
    ) -> Result<Self, JsValue> {
        let signals: Vec<Arc<Signal>> = get_signals_with_quality(&args);
        let gray_bits = args.grayscale_bits;
        let combinator_compression = args.signal_compression;
        let time_comp_win = combinator_compression.map_or(0, |c| match c {
            SignalCompression::Temporal { window } => window,
            _ => 0,
        });
        let mut frame_info: Option<FrameInfo> = None;
        let frame_dim: UVec2;

        if let Some(mut frame_data) = frame_data {
            let n_scaled_frames = frame_data.total_frames() * if time_comp_win > 0 { 2 } else { 1 };
            let frames_per_cb = if gray_bits > 0 { 32 / gray_bits } else { 1 };
            let n_buf_frames = (if time_comp_win > 0 {
                (time_comp_win * args.target_fps).div_ceil(1000 * frames_per_cb)
            } else {
                1
            }) as usize;
            frame_dim = frame_data.dimensions();
            frame_info = Some(FrameInfo {
                curr_frame: frame_data.next().transpose()?.unwrap(),
                frame_data,
                group_data_combs: Vec::new(),
                frames_per_cb,
                n_buf_frames,
                n_frames_per_chunk: if n_buf_frames > 1 { 1 } else { 0 },
                n_scaled_frames,
                next_frame: None,
                patch_states: Vec::new(),
                ticks_per_frame: (60.0 / args.target_fps as f64) as u32,
                use_delta_comp: combinator_compression
                    .is_some_and(|c| c == SignalCompression::Delta),
            })
        } else {
            frame_dim = uvec2(
                args.custom_width.expect("Must have width or frame data."),
                args.custom_height.expect("Must have height or frame data."),
            );
        }
        let max_lamp_cols_per_grp =
            (signals.len() as u32 / frame_dim.y).min(args.max_group_size.unwrap_or(frame_dim.x));
        if max_lamp_cols_per_grp < 1 {
            return Err(JsValue::from_str(
                "Not enough signals for even one column of lamps!",
            ));
        }
        let n_groups = frame_dim.x.div_ceil(max_lamp_cols_per_grp);

        log!("dim: {frame_dim}, n_groups: {n_groups}, max_lamp_cols_per_grp: {max_lamp_cols_per_grp}");

        if let Some(frame_info) = frame_info.as_mut() {
            frame_info.patch_states = (0..n_groups)
                .map(|_| GroupPatchState {
                    chunk_i: 0,
                    frame_buf: Vec::with_capacity(frame_info.frames_per_cb as usize),
                    sig_buf: Vec::with_capacity(frame_info.n_buf_frames),
                })
                .collect();
        }

        Ok(Self {
            args,
            frame_info,
            entity_data: EntityData {
                ents: Vec::new(),
                wires: Vec::new(),
                next_ent_n: 0u32,
            },
            dim: frame_dim,
            max_cols_per_grp: max_lamp_cols_per_grp,
            n_groups,
            should_generate_cbs: match args.mode {
                Mode::Lamps | Mode::LampGrid => false,
                _ => true,
            },
            signals,
            state: BlueprintState::Start,
        })
    }

    pub fn done(&mut self) -> bool {
        self.state == BlueprintState::Finished
    }

    pub fn next_chunk(&mut self, mut writer: &mut dyn io::Write) -> Result<bool, JsValue> {
        let old_state = self.state.clone();

        match old_state {
            BlueprintState::Start => {
                write_to(&mut writer, b"{\"blueprint\":{")?;
                let bp = BlueprintInner {
                    icons: Some(vec![Icon {
                        signal: Signal::new_virtual(Arc::clone(&DEC_CB)),
                        index: 1,
                    }]),
                    item: BLUEPRINT.to_string(),
                    label: Some(match self.args.mode {
                        Mode::Full => self.args.name.clone().unwrap_or("Blueprint".to_owned()),
                        Mode::Lamps => format!("{}x{} Lamps", self.dim.x, self.dim.y),
                        Mode::LampGrid => format!("{}x{} Lamp Grid", self.dim.x, self.dim.y),
                    }),
                    version: BLUEPRINT_VERSION,
                    entities: Vec::new(),    // don't serialize
                    wires: Some(Vec::new()), // don't serialize
                };
                write_json_trimmed_to(&mut writer, &bp)?;
                self.state = BlueprintState::Entities;
            }
            BlueprintState::Entities => self.place_entities(writer)?,
            BlueprintState::Data => {
                if self.should_generate_cbs {
                    self.populate_data(writer)?
                } else {
                    self.state = BlueprintState::Wires
                }
            }
            BlueprintState::Wires => self.make_wires(writer)?,
            BlueprintState::Finished => return Ok(false),
        }
        if old_state != self.state {
            match self.state {
                BlueprintState::Entities => write_to(&mut writer, b",\"entities\":[")?,
                BlueprintState::Wires => {
                    // Write remaining entities. Data combinators were NOT included in self.entity_data.ents
                    for ents in self.entity_data.ents.chunks(ENCODE_CHUNK_SIZE) {
                        write_to(&mut writer, b",")?;
                        write_json_trimmed_to(&mut writer, &ents)?; // Remove braces
                    }
                    write_to(&mut writer, b"],\"wires\":[")?;
                }
                BlueprintState::Finished => write_to(&mut writer, b"]}}")?,
                _ => {}
            }
        }
        Ok(true)
    }

    fn place_entities(&mut self, mut writer: &mut dyn io::Write) -> Result<(), JsValue> {
        let args = self.args;
        let info = &mut self.frame_info;
        let signals = &mut self.signals;
        let ent_data = &mut self.entity_data;

        if self.should_generate_cbs {
            let info = info.as_mut().unwrap();
            let stop = info.frame_data.total_frames() * info.ticks_per_frame;
            let (ents, wires) =
                generate_timer(stop, info.ticks_per_frame, info.frames_per_cb, &args);
            ent_data.next_ent_n = ents.iter().map(|e| e.entity_number).max().unwrap_or(0);
            ent_data.next_ent_n += 1;
            ent_data.ents.extend(ents);
            ent_data.wires.extend(wires);
        }
        let mut occupied =
            SubstationOccupied::new(dvec2(1.0, -1.0), args.substation_quality.clone());
        let mut prev_top_right_lamp_ent_n: Option<u32> = None;

        for group_i in 0..self.n_groups {
            let grp_left = group_i * self.max_cols_per_grp;
            let grp_width = (self.max_cols_per_grp).min(self.dim.x - grp_left);
            let (mut cb_in_ent_n, mut cb_out_ent_n) = (0, 0);

            #[allow(unused_variables)]
            let (grp_lamps, mut grp_lamp_wires, new_next_ent_n, top_right_lamp_ent_n) =
                generate_lamps(
                    &signals,
                    (grp_width, self.dim.y),
                    &mut occupied,
                    ent_data.next_ent_n,
                    dvec2(grp_left as f64, 0.0),
                    &args,
                );
            ent_data.next_ent_n = new_next_ent_n;
            let first_lamp = grp_lamps[0].entity_number;

            if self.should_generate_cbs {
                let info = info.as_mut().unwrap();
                let (other_ents, data_combs, wires, (in_ent_n, out_ent_n, new_base_ent_n)) =
                    generate_combinators(
                        info.n_scaled_frames.div_ceil(info.frames_per_cb),
                        &mut occupied,
                        ent_data.next_ent_n,
                        dvec2(grp_left as f64, -2.0),
                        self.max_cols_per_grp,
                        args,
                    )?;
                ent_data.next_ent_n = new_base_ent_n;
                if group_i == 0 {
                    ent_data.wires.push([3, WIRE_OUT_R, in_ent_n, WIRE_R]); // Connect timer
                }
                info.group_data_combs.push(data_combs);
                ent_data.ents.extend(other_ents);
                ent_data.wires.extend(wires);

                (cb_in_ent_n, cb_out_ent_n) = (in_ent_n, out_ent_n);
            }

            if self.should_generate_cbs {
                ent_data.wires.extend([
                    [first_lamp, WIRE_R, cb_in_ent_n, WIRE_R],
                    [first_lamp, WIRE_G, cb_out_ent_n, WIRE_OUT_G],
                ]);
            }

            // Connect previous lamps together
            if let Some(prev) = prev_top_right_lamp_ent_n {
                grp_lamp_wires.push([grp_lamps[0].entity_number, WIRE_R, prev, WIRE_R]);
            }
            prev_top_right_lamp_ent_n = Some(top_right_lamp_ent_n);

            if group_i > 0 {
                write_to(&mut writer, b",")?;
            }
            write_json_trimmed_to(&mut writer, &grp_lamps)?;
            ent_data.wires.extend(grp_lamp_wires);
        }
        let (ents, wires, _) = generate_substations(&mut occupied, ent_data.next_ent_n);
        ent_data.ents.extend(ents);
        ent_data.wires.extend(wires);
        ent_data.ents.sort_by_key(|e| e.entity_number);
        self.state = BlueprintState::Data;

        Ok(())
    }

    fn populate_data(&mut self, mut writer: &mut dyn io::Write) -> Result<(), JsValue> {
        let info = self.frame_info.as_mut().unwrap();

        let group_data_combs = &mut info.group_data_combs;
        let frame_data = &mut info.frame_data;
        let signals = &mut self.signals;

        let frames_per_cb = info.frames_per_cb;
        let gray_bits = self.args.grayscale_bits;
        let max_cols_per_grp = self.max_cols_per_grp;
        let (n_buf_frames, n_frames_per_chunk) = (info.n_buf_frames, info.n_frames_per_chunk);
        let n_groups = self.n_groups;
        let ticks_per_frame = info.ticks_per_frame;
        let use_delta_comp = info.use_delta_comp;
        let ticks_per_grp = ticks_per_frame * frames_per_cb;

        let mk_cb = |start: u32, end: u32, outputs: Vec<CombinatorOutput>| {
            ControlBehavior::from_decider_conditions(DeciderConditions {
                conditions: Some(vec![
                    Condition::new(
                        Signal::new_virtual(Arc::clone(&SIG_T)),
                        start as i32,
                        COMP_GE,
                    ),
                    Condition::new(Signal::new_virtual(Arc::clone(&SIG_T)), end as i32, COMP_LT)
                        .with_compare_type(COMP_AND),
                ]),
                outputs: Some(outputs),
            })
        };
        let mut output_ents: Vec<Entity> = Vec::with_capacity(ENCODE_CHUNK_SIZE);

        loop {
            info.next_frame = frame_data.next().transpose()?;
            let frame = &info.curr_frame;
            let is_last_frame = info.next_frame.is_none();

            for group_i in 0..n_groups as usize {
                let state = &mut info.patch_states[group_i]; // Individual state for each group of lamps
                let frame_sigs: Vec<i32>;
                let data_combs = &group_data_combs[group_i];

                let grp_left = group_i as u32 * max_cols_per_grp;
                let grp_right = ((group_i as u32 + 1) * max_cols_per_grp).min(self.dim.x);

                let img = frame.crop_imm(grp_left, 0, grp_right - grp_left, self.dim.y);
                let target_outputs_len = (img.width() * img.height()) as usize;

                if gray_bits > 0 {
                    state.frame_buf.push(img);
                    if state.frame_buf.len() < frames_per_cb as usize && !is_last_frame {
                        continue; // wait until we have a full chunk for this group
                    }
                    frame_sigs = grayscale_frames_to_outputs(&state.frame_buf, gray_bits)?;
                    state.frame_buf.clear();
                } else {
                    frame_sigs = color_frame_to_outputs(&img)?;
                }

                if frame_sigs.len() != target_outputs_len {
                    return Err(JsValue::from_str(&format!(
                        "Outputs length ({}) does not match frame size ({target_outputs_len}).",
                        frame_sigs.len(),
                    )));
                }

                if use_delta_comp && state.sig_buf.len() == 0 {
                    state.sig_buf.push(vec![0i32; target_outputs_len]); // Populate first frame with zeros
                } else if !use_delta_comp {
                    state.sig_buf.push(frame_sigs.clone()); // If we're using delta compression we only compare aginst the previous frame
                }

                if state.sig_buf.len() < n_buf_frames && !is_last_frame {
                    continue; // Accumulate until we have `compression_level` outputs,
                }

                if use_delta_comp {
                    // Delta compression branch
                    let prev_outputs = &state.sig_buf[0];
                    let mut target_outputs = Vec::with_capacity(target_outputs_len);

                    for i in 0..target_outputs_len {
                        let v = frame_sigs[i].wrapping_sub(prev_outputs[i]); // Uses overflow to reach any target value.
                        if v != 0 {
                            target_outputs
                                .push(CombinatorOutput::new(Arc::clone(&signals[i]), Some(v)));
                        }
                    }

                    // Update the data combinator
                    let en = &data_combs[state.chunk_i];
                    output_ents.push(en.clone().with_control_behavior(mk_cb(
                        1 + ((state.chunk_i as u32) * ticks_per_grp), // start frame
                        2 + ((state.chunk_i as u32) * ticks_per_grp), // end frame
                        target_outputs,
                    )));
                    state.sig_buf[0] = frame_sigs; // cache absolute output
                } else {
                    // Temporal compression branch
                    let mut changed_mask: Vec<bool> = vec![false; target_outputs_len]; // to find what changed
                    let outputs_0 = &state.sig_buf[0];
                    let base_chunk_i = state.chunk_i * n_buf_frames;

                    // If we cached more than 1 frame, we can apply temporal compression
                    if state.sig_buf.len() > 1 {
                        for comb_outputs in state.sig_buf.iter().skip(1) {
                            for i in 0..target_outputs_len {
                                if !changed_mask[i] && (&comb_outputs[i] != &outputs_0[i]) {
                                    changed_mask[i] = true;
                                }
                            }
                        }
                    } else {
                        for i in 0..target_outputs_len {
                            changed_mask[i] = true; // Handle only one frame, force everything changed
                        }
                    }

                    // Handle changed pixels
                    for (cb_i, cb_outputs) in state.sig_buf.iter().enumerate() {
                        let target_i = base_chunk_i + cb_i;
                        let target_comb = state.chunk_i * (n_buf_frames + n_frames_per_chunk)
                            + cb_i
                            + n_frames_per_chunk;

                        // Only contains non-changed pixels (i.e., the ones we want to store)
                        let mut target_outputs = Vec::with_capacity(signals.len());

                        // Accumulate only the changed outputs
                        for (i, v) in cb_outputs.iter().enumerate() {
                            if changed_mask[i] && *v != 0 {
                                target_outputs
                                    .push(CombinatorOutput::new(Arc::clone(&signals[i]), Some(*v)));
                            }
                        }
                        let en = &data_combs[target_comb];
                        output_ents.push(en.clone().with_control_behavior(mk_cb(
                            target_i as u32 * ticks_per_grp,
                            (target_i as u32 + 1) * ticks_per_grp,
                            target_outputs,
                        )));
                    }

                    // Handle unchanged pixels
                    if state.sig_buf.len() > 1 {
                        let mut target_outputs = Vec::with_capacity(signals.len());
                        for i in 0..target_outputs_len {
                            if !changed_mask[i] && outputs_0[i] != 0 {
                                target_outputs.push(CombinatorOutput::new(
                                    Arc::clone(&signals[i]),
                                    Some(outputs_0[i]),
                                ));
                            }
                        }
                        let en = &data_combs[state.chunk_i * (n_buf_frames + n_frames_per_chunk)];
                        output_ents.push(en.clone().with_control_behavior(mk_cb(
                            base_chunk_i as u32 * ticks_per_grp,
                            (base_chunk_i as u32 + state.sig_buf.len() as u32) * ticks_per_grp,
                            target_outputs,
                        )));
                    }
                    state.sig_buf.clear();
                }
                state.chunk_i += 1;
            }

            // Note: Always advance before checking if we have max entities
            if info.next_frame.is_some() {
                info.curr_frame = info.next_frame.take().unwrap();
            } else {
                self.state = BlueprintState::Wires;
                break;
            }
            if output_ents.len() >= ENCODE_CHUNK_SIZE {
                break;
            }
        }

        if output_ents.len() > 0 {
            write_to(&mut writer, b",")?;
            write_json_trimmed_to(&mut writer, &output_ents)?; // Remove braces
        }

        Ok(())
    }

    fn make_wires(&mut self, mut writer: &mut dyn io::Write) -> Result<(), JsValue> {
        let all_ents = &mut self.entity_data.ents;
        let all_wires = &mut self.entity_data.wires;

        // Swap all wires if requested (default uses red wires, so swap all for green).
        // Circuit network filters are already handled.
        if !self.args.prefer_green_wires {
            invert_wires(all_ents, all_wires);
        }
        for (i, wires) in all_wires.chunks(ENCODE_CHUNK_SIZE).enumerate() {
            if i > 0 {
                write_to(&mut writer, b",")?;
            }
            write_json_trimmed_to(&mut writer, wires)?;
        }
        self.state = BlueprintState::Finished;

        Ok(())
    }
}
