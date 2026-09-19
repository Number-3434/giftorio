use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_S};
use crate::image_utils::{animation_info, resize_dimensions, AnimationInfo};
use crate::models::{BlueprintArgs, FlippedAxes, ImageRotation::*, ResamplingFilter};
use crate::progress::set_progress;
use image::{imageops, AnimationDecoder, ImageDecoder};
use std::{collections::VecDeque, io::Cursor, time::Duration};
use wasm_bindgen::prelude::*;

pub struct FrameData<'a> {
    args: BlueprintArgs,
    buf: Vec<(image::Frame, u32)>,
    curr_frame_idx: u32,
    curr_n_ms: u32,
    frames: image::Frames<'a>,
    in_dim: (u32, u32),
    in_n_frames: u32,
    next_samp_idx: u32,
    out_dim_raw: (f64, f64),
    out_n_frames: u32,
    output_frames: VecDeque<image::Frame>,
    resize_filter: imageops::FilterType,
}

impl FrameData<'_> {
    /// Returns the dimensions of the output frames.
    ///
    /// Note: This is truncated from the raw dimensions instead of rounded.
    pub fn dimensions(&mut self) -> (u32, u32) {
        let (w, h) = resize_dimensions(
            self.in_dim.0,
            self.in_dim.1,
            self.out_dim_raw.0.round() as u32,
            self.out_dim_raw.1.round() as u32,
            false,
        );
        if matches!(self.args.image_rotation, Deg90 | Deg270) {
            (h, w)
        } else {
            (w, h)
        }
    }
    pub fn total_frames(&self) -> u32 {
        self.out_n_frames
    }
}
impl<'a> FrameData<'a> {
    pub fn new(image_data: &'a [u8], args: BlueprintArgs) -> Result<Self, JsValue> {
        let frame_data = get_frames(&image_data, &args.image_type)?;
        let in_dim = frame_data.dimensions();
        let n_frames = frame_data.n_frames();

        let (w, h) = (in_dim.0 as f64, in_dim.1 as f64);
        let scale_factor = (args.max_size as f64 / w)
            .min(args.max_size as f64 / h)
            .min(1.0);

        fn expected_output_frames(n_ms: u32, fps: u32, include_last_frame: bool) -> u32 {
            ((n_ms as u64 * fps as u64) / 1000) as u32 + include_last_frame as u32
        }

        let obj = Self {
            out_n_frames: expected_output_frames(
                frame_data.total_duration_ms.as_millis() as u32,
                args.target_fps.max(1),
                args.last_frame,
            ),
            resize_filter: match args.sampling_filter {
                ResamplingFilter::Catrom => imageops::FilterType::CatmullRom,
                ResamplingFilter::Gaussian => imageops::FilterType::Gaussian,
                ResamplingFilter::Lanczos3 => imageops::FilterType::Lanczos3,
                ResamplingFilter::Nearest => imageops::FilterType::Nearest,
                ResamplingFilter::Triangle => imageops::FilterType::Triangle,
            },
            args: args,
            buf: Vec::new(),
            curr_frame_idx: 0,
            curr_n_ms: 0,
            frames: frame_data.frames,
            in_dim,
            in_n_frames: n_frames,
            next_samp_idx: 0,
            out_dim_raw: ((w * scale_factor), (h * scale_factor)),
            output_frames: VecDeque::new(),
        };

        Ok(obj)
    }
}
impl Iterator for FrameData<'_> {
    type Item = Result<image::DynamicImage, JsValue>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Flush any remaining frames
            if let Some(frame) = self.output_frames.pop_front() {
                // Decode + resize
                let mut img = image::DynamicImage::ImageRgba8(frame.into_buffer());
                if self.args.grayscale_bits > 0 {
                    img = image::DynamicImage::ImageLuma8(img.to_luma8());
                }
                img = img.resize(
                    (self.out_dim_raw.0).round() as u32,
                    (self.out_dim_raw.1).round() as u32,
                    self.resize_filter,
                ); // Use raw dims for max precision
                img = match self.args.flipped_axes {
                    FlippedAxes::X => img.fliph(),
                    FlippedAxes::Y => img.flipv(),
                    FlippedAxes::Both => img.flipv().fliph(),
                    _ => img,
                }; // Flip before rotate
                img = match self.args.image_rotation {
                    Deg90 => img.rotate90(),
                    Deg180 => img.rotate180(),
                    Deg270 => img.rotate270(),
                    _ => img,
                };
                return Some(Ok(img));
            }

            // Streaming loop
            let frame = match self.frames.next() {
                Some(frame) => match frame {
                    Ok(frame) => frame,
                    Err(e) => return Some(Err(JsValue::from_str(&format!("Decode error: {e}")))),
                },
                None => return None,
            };

            // % of prime number cuz i like seeing it go through every number :D
            if self.curr_frame_idx % 1 == 0 {
                set_progress(
                    0.00,
                    0.67,
                    self.curr_frame_idx as f64 / self.in_n_frames as f64,
                    &format!(
                        "Streaming frame {} /{} ({})",
                        self.curr_frame_idx,
                        self.in_n_frames,
                        format_duration(self.curr_n_ms as u64)
                    ),
                );
            }

            let (ms, _) = frame.delay().numer_denom_ms();
            let delay = if ms == 0 { DEFAULT_FRAME_DELAY_MS } else { ms };

            self.buf.push((frame, self.curr_n_ms)); // Add to rolling buffer
            self.curr_n_ms += delay; // Increment AFTER adding to buffer (we track start time, not end time)

            // Keep buffer bounded (e.g., last 0.5 seconds)
            while let Some((_, t)) = self.buf.first() {
                if self.curr_n_ms - *t > 500 {
                    self.buf.remove(0);
                } else {
                    break;
                }
            }

            // Sample frames as long as we passed the next sample timestamp
            // Note that we may return multiple frames in this loop,
            // so we accumulate them in the output_frames deque.
            let mut sample_ms: u32;

            while {
                sample_ms =
                    (self.next_samp_idx as f64 * MS_PER_S / self.args.target_fps as f64) as u32;
                sample_ms < self.curr_n_ms
                    && (self.args.last_frame || self.next_samp_idx < self.out_n_frames)
            } {
                let mut best_frame: Option<&image::Frame> = None;
                let mut best_delta = u32::MAX;

                // Find the closest frame to the current sample (forwards / backwards)
                for (img, t) in &self.buf {
                    let delta = sample_ms.abs_diff(*t);
                    if delta < best_delta {
                        best_delta = delta;
                        best_frame = Some(img);
                    }
                }
                if let Some(img) = best_frame {
                    self.output_frames.push_back(img.clone());
                }
                self.next_samp_idx += 1;
            }
            self.curr_frame_idx += 1;
        }
    }
}

fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let milliseconds = ms % 1000;

    return format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}");
}

pub struct ImageFrameData<'a> {
    pub frames: image::Frames<'a>,
    dimensions: (u32, u32),
    n_frames: u32,
    total_duration_ms: Duration,
}
impl ImageFrameData<'_> {
    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
    pub fn n_frames(&self) -> u32 {
        self.n_frames
    }
}

/// Decodes the provided image data into a vector of frames based on the image type.
///
/// Supported types: "gif" and "webp".
///
/// # Arguments
///
/// * `image_data` - A byte slice containing the image data.
/// * `image_type` - The type of the image ("gif" or "webp").
///
/// # Returns
///
/// An iterator of `Frame` objects or a JavaScript error.
pub fn get_frames<'a>(
    image_data: &'a [u8],
    image_type: &str,
) -> Result<ImageFrameData<'a>, JsValue> {
    let cursor = Cursor::new(image_data);
    let info: AnimationInfo;
    let dimensions: (u32, u32);

    Ok(ImageFrameData {
        frames: match image_type {
            "gif" => {
                let decoder = image::codecs::gif::GifDecoder::new(cursor).unwrap_throw();
                dimensions = decoder.dimensions();
                info = animation_info(&image_data)?;
                decoder.into_frames()
            }
            "webp" => {
                let decoder = image::codecs::webp::WebPDecoder::new(cursor).unwrap_throw();
                dimensions = decoder.dimensions();
                info = animation_info(&image_data)?;
                decoder.into_frames()
            }
            _ => {
                return Err(JsValue::from_str(
                    "Unsupported image type. Only 'gif' and 'webp' are allowed.",
                ))
            }
        },
        dimensions,
        n_frames: info.frames,
        total_duration_ms: info.duration,
    })
}

/// Converts an RGB pixel to a single 24 bit integer (inside a u32, I know...).
///
/// TODO: Test if this is actually needed; maybe we could use image.to_rgba8() and then
/// chunk into 4? WASM is always little-endian, so this would behave the same across
/// devices.
///
/// # Arguments
///
/// * `r` - Red channel (0–255).
/// * `g` - Green channel (0–255).
/// * `b` - Blue channel (0–255).
///
/// # Returns
///
/// An integer representing the RGB value.
#[inline(always)]
#[allow(dead_code)]
pub fn rgb_to_int(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
