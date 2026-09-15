use crate::models::{Blueprint, BlueprintArgs, OutputFormat};
use crate::progress::{report_progress, set_progress};
use crate::streaming_writer::{ChunkQueue, StreamingWriter};
use crate::JsValue;
use base64::write::EncoderWriter;
use flate2::{write::ZlibEncoder, Compression};
use std::{
    collections::VecDeque,
    io::{self},
    sync::{Arc, Mutex},
};

const ENCODE_CHUNK_SIZE: usize = 100;

pub struct BlueprintJsonEncoder {
    blueprint: Blueprint,
    curr_i: usize,
    progress_format_str: &'static str,
    state: BlueprintEncoderState,
}
enum BlueprintEncoderState {
    Start,
    Entities,
    Wires,
    Finished,
}

impl BlueprintJsonEncoder {
    pub fn new(blueprint: Blueprint, progress_format_str: &'static str) -> Self {
        Self {
            blueprint: blueprint,
            curr_i: 0,
            progress_format_str,
            state: BlueprintEncoderState::Start,
        }
    }

    pub fn done(&self) -> bool {
        match self.state {
            BlueprintEncoderState::Finished => true,
            _ => false,
        }
    }

    pub fn next_chunk<W: io::Write>(&mut self, mut writer: &mut W) -> Result<(), JsValue> {
        let ents = &self.blueprint.blueprint.entities;
        let wires = &self.blueprint.blueprint.wires;
        // let n_ent_chunks = ents.len().div_ceil(ENCODE_CHUNK_SIZE);

        let w = |writer: &mut W, buf: &[u8]| -> Result<(), JsValue> {
            writer
                .write_all(buf)
                .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))
        };

        match self.state {
            BlueprintEncoderState::Start => {
                w(&mut writer, b"{\"blueprint\":")?;
                w(&mut writer, b"{\"entities\":[")?;
                self.state = BlueprintEncoderState::Entities;
            }
            BlueprintEncoderState::Entities => {
                let i = self.curr_i;
                let chunk = &ents[i..(i + ENCODE_CHUNK_SIZE).min(ents.len())];

                set_progress(
                    0.67,
                    0.98,
                    i as f64 / ents.len() as f64,
                    &format!("{} {}/{}...", self.progress_format_str, i, ents.len()),
                );
                if i > 0 {
                    w(&mut writer, b",")?; // Delimiter
                }
                let json_bytes = serde_json::to_vec(&chunk)
                    .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))?;

                // Extend from JSON bytes + remove surrounding brackets.
                w(&mut writer, &json_bytes[1..json_bytes.len() - 1])?;

                self.curr_i += chunk.len();

                if self.curr_i >= ents.len() {
                    w(&mut writer, b"]")?;
                    self.state = BlueprintEncoderState::Wires;
                }
            }
            BlueprintEncoderState::Wires => {
                w(&mut writer, b",\"wires\":")?;
                serde_json::to_writer(&mut *writer, &wires)
                    .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))?;
                w(&mut writer, b"}")?;

                report_progress(0.99, "Finishing...");
                w(&mut writer, b"}")?;
                self.state = BlueprintEncoderState::Finished;
            }
            BlueprintEncoderState::Finished => {
                return Err(JsValue::from_str(&format!("Already finished")))?
            }
        }

        Ok(())
    }
}
pub struct BlueprintEncoder {
    chunks: ChunkQueue,
    finished: bool,
    json_encoder: BlueprintJsonEncoder,
    zlib_encoder: Option<ZlibEncoder<EncoderWriter<StreamingWriter>>>,
}
impl BlueprintEncoder {
    pub fn new(blueprint: Blueprint, args: &BlueprintArgs) -> Self {
        let chunks = Arc::new(Mutex::new(VecDeque::new()));
        let writer = StreamingWriter::new(Arc::clone(&chunks));
        let mut zlib: Option<ZlibEncoder<EncoderWriter<StreamingWriter>>> = None;
        let format_str = if args.output_format == OutputFormat::Blueprint {
            "Converting to Blueprint"
        } else {
            "Converting to JSON"
        };

        if args.output_format == OutputFormat::Blueprint {
            let b64 = EncoderWriter::new(writer, base64::STANDARD);
            zlib = Some(ZlibEncoder::new(b64, Compression::best()))
        }

        Self {
            chunks,
            finished: false,
            json_encoder: BlueprintJsonEncoder::new(blueprint, format_str),
            zlib_encoder: zlib,
        }
    }

    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
        loop {
            // Return anything already produced by the previous call first
            if let Some(chunk) = self.chunks.lock().unwrap().pop_front() {
                return Ok(Some(chunk));
            } else if self.finished {
                return Ok(None); // If we're done, return None
            }

            if !self.json_encoder.done() {
                if self.zlib_encoder.is_some() {
                    self.json_encoder
                        .next_chunk(&mut self.zlib_encoder.as_mut().unwrap())?;
                } else {
                    let mut buf = Vec::new();
                    self.json_encoder.next_chunk(&mut buf)?;
                    self.chunks.lock().unwrap().push_back(buf);
                }
                continue;
            } else if self.zlib_encoder.is_none() {
                return Ok(None);
            }

            let zlib = self.zlib_encoder.take().unwrap();
            let mut b64 = zlib
                .finish()
                .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))?;
            let _writer = b64
                .finish()
                .map_err(|e| JsValue::from_str(&format!("JSON write error: {e}")))?;

            self.finished = true;
        }
    }
}
