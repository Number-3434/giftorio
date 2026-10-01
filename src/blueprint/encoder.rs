use crate::blueprint::{blueprint::BlueprintGenerator, constants::*};
use crate::image_processing::FrameData;
use crate::models::{BlueprintArgs, OutputFormat};
use crate::streaming_writer::{ChunkQueue, StreamingWriter};
use crate::JsValue;
use base64::{
    engine::{general_purpose::STANDARD, GeneralPurpose},
    write::EncoderWriter,
    Engine,
};
use flate2::{write::ZlibEncoder, Compression};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use wasm_bindgen::UnwrapThrowExt;

pub struct BlueprintEncoder<'a> {
    blueprint_generator: BlueprintGenerator<'a>,
    chunks: ChunkQueue,
    finished: bool,
    started: bool,
    zlib_encoder: Option<ZlibEncoder<EncoderWriter<'a, GeneralPurpose, StreamingWriter>>>,
}
impl<'a> BlueprintEncoder<'a> {
    pub fn new(blueprint_generator: BlueprintGenerator<'a>, args: &BlueprintArgs) -> Self {
        let chunks = Arc::new(Mutex::new(VecDeque::new()));
        let writer = StreamingWriter::new(Arc::clone(&chunks));
        let mut zlib: Option<ZlibEncoder<_>> = None;

        if args.output_format == OutputFormat::Blueprint {
            let b64 = EncoderWriter::new(writer, &STANDARD);
            zlib = Some(ZlibEncoder::new(b64, Compression::best()))
        }

        Self {
            blueprint_generator,
            chunks,
            finished: false,
            started: false,
            zlib_encoder: zlib,
        }
    }

    pub fn new_from_frame_data(
        frame_data: Option<FrameData<'a>>,
        args: &'a BlueprintArgs,
    ) -> Result<Self, JsValue> {
        let gen = BlueprintGenerator::new(frame_data, args)?;
        Ok(Self::new(gen, args))
    }

    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
        // Handle Factorio version prefix
        if !self.started {
            self.started = true; // Set before we return to prevent infinite loop
            if self.zlib_encoder.is_some() {
                return Ok(Some(STANDARD.encode(FACTORIO_VERSION_PREFIX).into_bytes()));
            }
        }

        loop {
            // Return anything already produced by the previous call first
            if let Some(chunk) = self.chunks.lock().unwrap().pop_front() {
                return Ok(Some(chunk));
            } else if self.finished {
                return Ok(None); // If we're done, return None
            }

            if !self.blueprint_generator.done() {
                if self.zlib_encoder.is_some() {
                    self.blueprint_generator
                        .next_chunk(&mut self.zlib_encoder.as_mut().unwrap())?;
                } else {
                    let mut buf = Vec::new();
                    self.blueprint_generator.next_chunk(&mut buf)?;
                    self.chunks.lock().unwrap().push_back(buf);
                }
                continue;
            } else if self.zlib_encoder.is_none() {
                return Ok(None);
            }

            let zlib = self.zlib_encoder.take().unwrap();
            let mut b64 = zlib.finish().unwrap_throw();
            let _writer = b64.finish().unwrap_throw();

            self.finished = true;
        }
    }
}
