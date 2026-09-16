use crate::blueprint::BlueprintGenerator;
use crate::image_processing::FrameData;
use crate::models::{BlueprintArgs, OutputFormat};
use crate::streaming_writer::{ChunkQueue, StreamingWriter};
use crate::JsValue;
use base64::write::EncoderWriter;
use flate2::{write::ZlibEncoder, Compression};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub struct BlueprintEncoder<'a> {
    blueprint_generator: BlueprintGenerator<'a>,
    chunks: ChunkQueue,
    finished: bool,
    zlib_encoder: Option<ZlibEncoder<EncoderWriter<StreamingWriter>>>,
}
impl<'a> BlueprintEncoder<'a> {
    pub fn new(blueprint_generator: BlueprintGenerator<'a>, args: &BlueprintArgs) -> Self {
        let chunks = Arc::new(Mutex::new(VecDeque::new()));
        let writer = StreamingWriter::new(Arc::clone(&chunks));
        let mut zlib: Option<ZlibEncoder<EncoderWriter<StreamingWriter>>> = None;

        if args.output_format == OutputFormat::Blueprint {
            let b64 = EncoderWriter::new(writer, base64::STANDARD);
            zlib = Some(ZlibEncoder::new(b64, Compression::best()))
        }

        Self {
            blueprint_generator,
            chunks,
            finished: false,
            zlib_encoder: zlib,
        }
    }

    pub fn new_from_frame_data(
        frame_data: FrameData<'a>,
        args: &'a BlueprintArgs,
    ) -> Result<Self, JsValue> {
        let gen = BlueprintGenerator::new(frame_data, args)?;
        Ok(Self::new(gen, args))
    }

    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
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
