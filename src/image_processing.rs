use crate::blueprint::BlueprintArgs;
use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_S};
use crate::image_utils::{animation_info, resize_dimensions, AnimationInfo};
use crate::progress::set_progress;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage, ImageDecoder};
use std::collections::VecDeque;
use std::io::Cursor;
use std::time::Duration;
use wasm_bindgen::prelude::*;

pub struct FrameData<'a> {
    buf: Vec<(DynamicImage, u32)>,
    curr_frame_idx: u32,
    curr_n_ms: u32,
    filter_type: FilterType,
    frames: image::Frames<'a>,
    grayscale_bits: u32,
    in_dim: (u32, u32),
    in_n_frames: u32,
    include_last_frame: bool,
    next_samp_idx: u32,
    out_dim_raw: (f64, f64),
    out_n_frames: u32,
    output_frames: VecDeque<DynamicImage>,
    target_fps: u32,
}
impl FrameData<'_> {
    /// Returns the dimensions of the output frames.
    ///
    /// Note: This is truncated from the raw dimensions instead of rounded.
    pub fn dimensions(&self) -> (u32, u32) {
        resize_dimensions(
            self.in_dim.0,
            self.in_dim.1,
            self.out_dim_raw.0.round() as u32,
            self.out_dim_raw.1.round() as u32,
            false,
        )
    }
    pub fn duration(&self) -> u32 {
        self.curr_n_ms
    }
    pub fn fps(&self) -> u32 {
        self.target_fps
    }
    pub fn grayscale_bits(&self) -> u32 {
        self.grayscale_bits
    }
    pub fn orig_dimensions(&self) -> (u32, u32) {
        self.in_dim
    }
    pub fn total_frames(&self) -> u32 {
        self.out_n_frames
    }
}
impl<'a> FrameData<'a> {
    pub fn new(image_data: &'a [u8], args: &BlueprintArgs) -> Result<Self, JsValue> {
        let frame_data = get_frames(&image_data, &args.image_type)?;

        let in_dim = frame_data.dimensions();
        let n_frames = frame_data.n_frames();

        let (w, h) = (in_dim.0 as f64, in_dim.1 as f64);
        let scale_factor = (args.max_size as f64 / w)
            .min(args.max_size as f64 / h)
            .min(1.0);

        fn expected_output_frames(
            total_duration_ms: u32,
            target_fps: u32,
            include_last_frame: bool,
        ) -> u32 {
            let step = 1000.0 / target_fps as f64;
            let mut expected = (total_duration_ms as f64 / step).floor() as u32;

            if include_last_frame {
                expected += 1; // 1 extra frame for the last frame
            }
            return expected;
        }

        let obj = Self {
            buf: Vec::new(),
            curr_frame_idx: 0,
            curr_n_ms: 0,
            filter_type: match args.resampling_filter.as_str() {
                "catrom" => FilterType::CatmullRom,
                "gaussian" => FilterType::Gaussian,
                "lanczos3" => FilterType::Lanczos3,
                "nearest" => FilterType::Nearest,
                "triangle" => FilterType::Triangle,
                _ => return Err(JsValue::from_str("Invalid resampling filter type")),
            },
            frames: frame_data.frames,
            grayscale_bits: args.grayscale_bits,
            in_dim,
            in_n_frames: n_frames,
            include_last_frame: args.include_last_frame,
            next_samp_idx: 0,
            out_dim_raw: ((w * scale_factor), (h * scale_factor)),
            out_n_frames: expected_output_frames(
                frame_data.total_duration_ms.as_millis() as u32,
                args.target_fps.max(1),
                args.include_last_frame,
            ),
            output_frames: VecDeque::new(),
            target_fps: args.target_fps.max(1),
        };

        Ok(obj)
    }
}
impl Iterator for FrameData<'_> {
    type Item = Result<DynamicImage, JsValue>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Flush any remaining frames
            if let Some(img) = self.output_frames.pop_front() {
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
            if self.curr_frame_idx % 3 == 0 {
                set_progress(
                    0.0,
                    0.50,
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

            // Decode + resize
            let mut img = DynamicImage::ImageRgba8(frame.into_buffer());
            if self.grayscale_bits > 0 {
                img = DynamicImage::ImageLuma8(img.to_luma8());
            }

            // Use raw dims for max precision
            let img = img.resize(
                (self.out_dim_raw.0).round() as u32,
                (self.out_dim_raw.1).round() as u32,
                self.filter_type,
            );

            self.buf.push((img, self.curr_n_ms)); // Add to rolling buffer
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
                sample_ms = (self.next_samp_idx as f64 * MS_PER_S / self.target_fps as f64) as u32;

                sample_ms < self.curr_n_ms
                    && (self.include_last_frame || self.next_samp_idx < self.out_n_frames)
            } {
                let mut best_img: Option<&DynamicImage> = None;
                let mut best_dt = u32::MAX;

                // Find the cloest frame to the current sample (forwards / backwards)
                for (img, t) in &self.buf {
                    let dt = sample_ms.abs_diff(*t);
                    if dt < best_dt {
                        best_dt = dt;
                        best_img = Some(img);
                    }
                }
                if let Some(img) = best_img {
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
    let frames = match image_type {
        "gif" => {
            let decoder = image::codecs::gif::GifDecoder::new(cursor)
                .map_err(|e| JsValue::from_str(&format!("GIF decode error: {}", e)))?;
            dimensions = decoder.dimensions();
            info = animation_info(&image_data)?;

            decoder.into_frames()
        }

        "webp" => {
            let decoder = image::codecs::webp::WebPDecoder::new(cursor)
                .map_err(|e| JsValue::from_str(&format!("WebP decode error: {}", e)))?;
            dimensions = decoder.dimensions();
            info = animation_info(&image_data)?;

            decoder.into_frames()
        }

        _ => {
            return Err(JsValue::from_str(
                "Unsupported image type. Only 'gif' and 'webp' are allowed.",
            ))
        }
    };

    Ok(ImageFrameData {
        frames,
        dimensions,
        n_frames: info.frames,
        total_duration_ms: info.duration,
    })
}

/// Converts an RGB pixel to a single 24 bit integer (inside a u32, I know...).
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
pub fn rgb_to_int(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
