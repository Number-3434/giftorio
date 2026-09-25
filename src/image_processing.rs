use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_S};
use crate::image_utils::{animation_info, resize_dimensions, AnimationInfo};
use crate::models::{ImageRotation::*, *};
use crate::progress::set_progress;
use glam::{uvec2, DVec2, UVec2};
use image::{imageops, AnimationDecoder, ImageDecoder};
use std::{collections::VecDeque, time::Duration};
use wasm_bindgen::prelude::*;

#[inline(always)]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub struct FrameData<'a> {
    args: &'a BlueprintArgs,
    buf: Vec<(image::Frame, u32)>,
    curr_frame_idx: u32,
    curr_n_ms: u32,
    fps_timer: f64,
    frames: image::Frames<'a>,
    in_dim: UVec2,
    in_n_frames: u32,
    next_samp_idx: u32,
    out_dim_raw: DVec2,
    out_n_frames: u32,
    output_frames: VecDeque<image::Frame>,
    resize_filter: imageops::FilterType,
}

impl FrameData<'_> {
    /// Returns the dimensions of the output frames.
    ///
    /// Note: This is truncated from the raw dimensions instead of rounded.
    pub fn dimensions(&self) -> UVec2 {
        let (w, h) = resize_dimensions(
            self.in_dim.x,
            self.in_dim.y,
            self.out_dim_raw.x.round() as u32,
            self.out_dim_raw.y.round() as u32,
            false,
        );
        if matches!(self.args.image_rotation, Deg90 | Deg270) {
            uvec2(h, w)
        } else {
            uvec2(w, h)
        }
    }
    pub fn total_frames(&self) -> u32 {
        self.out_n_frames
    }
}
impl<'a> FrameData<'a> {
    pub fn new(image_data: &'a [u8], args: &'a BlueprintArgs) -> Result<Self, JsValue> {
        let frame_data = get_frames(image_data, &args)?;
        let in_dim = frame_data.dimensions();
        let n_frames = frame_data.n_frames();
        let scale_factor = (args.max_size as f64 / in_dim.x as f64)
            .min(args.max_size as f64 / in_dim.y as f64)
            .min(1.0);

        fn expected_output_frames(n_ms: u32, fps: u32, include_last_frame: bool) -> u32 {
            ((n_ms as u64 * fps as u64) / 1000) as u32 + include_last_frame as u32
        }

        let obj = Self {
            out_n_frames: if matches!(args.mode, Mode::Static { .. }) {
                1
            } else {
                expected_output_frames(
                    frame_data.total_duration_ms.as_millis() as u32,
                    args.target_fps.max(1),
                    args.last_frame,
                )
            },
            resize_filter: match args.sampling_filter {
                ResamplingFilter::Catrom => imageops::FilterType::CatmullRom,
                ResamplingFilter::Gaussian => imageops::FilterType::Gaussian,
                ResamplingFilter::Lanczos3 => imageops::FilterType::Lanczos3,
                ResamplingFilter::Nearest => imageops::FilterType::Nearest,
                ResamplingFilter::Triangle => imageops::FilterType::Triangle,
            },
            args,
            buf: Vec::new(),
            curr_frame_idx: 0,
            curr_n_ms: 0,
            fps_timer: now_ms(),
            frames: frame_data.frames,
            in_dim,
            in_n_frames: n_frames,
            next_samp_idx: 0,
            out_dim_raw: scale_factor * DVec2::from(in_dim),
            output_frames: VecDeque::new(),
        };

        Ok(obj)
    }
}
impl Iterator for FrameData<'_> {
    type Item = Result<image::DynamicImage, JsValue>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_frame_idx == 0 {
            self.fps_timer = now_ms();
        }
        loop {
            // Flush any remaining frames
            if let Some(frame) = self.output_frames.pop_front() {
                // Decode + resize
                let mut img = image::DynamicImage::ImageRgba8(frame.into_buffer());
                if self.args.grayscale_bits > 0 {
                    img = image::DynamicImage::ImageLuma8(img.to_luma8());
                }
                img = img.resize(
                    self.out_dim_raw.x.round() as u32,
                    self.out_dim_raw.y.round() as u32,
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

            // Note that this will auto-return None if self.frames.next() is None due to the ? operator
            let frame = match self.frames.next()? {
                Ok(frame) => frame,
                Err(e) => return Some(Err(JsValue::from_str(&format!("Decode error: {e}")))),
            };
            if matches!(self.args.mode, Mode::Static { .. }) {
                self.output_frames.push_back(frame);
                continue;
            }

            // % of prime number cuz i like seeing it go through every number :D
            if self.curr_frame_idx % 1 == 0 {
                set_progress(
                    0.00,
                    1.00,
                    self.curr_frame_idx as f64 / self.in_n_frames as f64,
                    &format!(
                        "Processing frame {} / {}  ({:.2} FPS)",
                        self.curr_frame_idx,
                        self.in_n_frames,
                        1000.0 * self.curr_frame_idx as f64 / (now_ms() - self.fps_timer)
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
            let mut samp_ms: u32;

            while {
                samp_ms = self.next_samp_idx * MS_PER_S as u32 / self.args.target_fps; // truncate
                samp_ms < self.curr_n_ms
                    && (self.args.last_frame || self.next_samp_idx < self.out_n_frames)
            } {
                let mut best_frame: Option<&image::Frame> = None;
                let mut best_delta = u32::MAX;

                // Find the closest frame to the current sample (forwards / backwards)
                for (img, t) in &self.buf {
                    let delta = samp_ms.abs_diff(*t);
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

pub struct ImageFrameData<'a> {
    pub frames: image::Frames<'a>,
    dimensions: UVec2,
    n_frames: u32,
    total_duration_ms: Duration,
}
impl ImageFrameData<'_> {
    pub fn dimensions(&self) -> UVec2 {
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
pub fn get_frames<'a>(data: &'a [u8], args: &BlueprintArgs) -> Result<ImageFrameData<'a>, JsValue> {
    let cursor = std::io::Cursor::new(data);
    let info: AnimationInfo;
    let dim: UVec2;
    let image_type = args.image_metadata.image_type.as_ref();

    Ok(ImageFrameData {
        frames: match image_type.expect("image type required").as_str() {
            "gif" => {
                let decoder = image::codecs::gif::GifDecoder::new(cursor).unwrap_throw();
                dim = UVec2::from(decoder.dimensions());
                info = animation_info(&data)?;
                decoder.into_frames()
            }
            "webp" => {
                let decoder = image::codecs::webp::WebPDecoder::new(cursor).unwrap_throw();
                dim = UVec2::from(decoder.dimensions());
                info = animation_info(&data)?;
                decoder.into_frames()
            }
            _ => {
                let frames = 1;
                let use_combs = matches!(args.mode, Mode::Static { combs: true, .. });
                let duration = Duration::from_millis(if use_combs { 1000 } else { 0 });
                dim = UVec2::from(args.image_metadata.image_size.expect("No image size"));
                info = AnimationInfo { frames, duration };
                image::Frames::new(Box::new(std::iter::once(Ok(image::Frame::new(
                    image::RgbaImage::from_raw(dim.x, dim.y, data.to_vec())
                        .expect("buffer length must be width * height * 4"),
                ))))) // bruh
            }
        },
        dimensions: dim,
        n_frames: info.frames,
        total_duration_ms: info.duration,
    })
}

/// Converts an RGB pixel to a single 24 bit integer (inside a u32, I know...).
///
/// Note: This is a fallback implementation for when the SIMD-accelerated version is not available.
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
