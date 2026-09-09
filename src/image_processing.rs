use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_SECOND};
use crate::progress::report_progress;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage};
use std::io::Cursor;
use wasm_bindgen::prelude::*;

fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let milliseconds = ms % 1000;

    return format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}");
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
) -> Result<image::Frames<'a>, JsValue> {
    let cursor = Cursor::new(image_data);

    match image_type {
        "gif" => {
            let decoder = image::codecs::gif::GifDecoder::new(cursor)
                .map_err(|e| JsValue::from_str(&format!("GIF decode error: {}", e)))?;

            Ok(decoder.into_frames())
        }

        "webp" => {
            let decoder = image::codecs::webp::WebPDecoder::new(cursor)
                .map_err(|e| JsValue::from_str(&format!("WebP decode error: {}", e)))?;

            Ok(decoder.into_frames())
        }

        _ => Err(JsValue::from_str(
            "Unsupported image type. Only 'gif' and 'webp' are allowed.",
        )),
    }
}

/// Processes the image by decoding frames, sampling, resizing, and optionally converting to grayscale.
///
/// # Arguments
///
/// * `image_data` - Raw image data.
/// * `image_type` - Image type (e.g., "gif" or "webp").
/// * `max_size` - Maximum width/height for downscaling.
/// * `target_fps` - Desired frames per second (limited by the original FPS).
/// * `grayscale_bits` - Number of bits for grayscale conversion (0 means full color).
///
/// # Returns
///
/// A tuple containing the processed frames (`DynamicImage`s) and the effective FPS.
pub fn process_image(
    image_data: &[u8],
    image_type: String,
    max_size: u32,
    target_fps: u32,
    grayscale_bits: u32,
    resampling_filter: String,
) -> Result<(Vec<DynamicImage>, u32), JsValue> {
    report_progress(0.0, "Starting single-pass decode...");

    // First frame determines dimensions
    let frame_0 = get_frames(image_data, &image_type)?
        .next()
        .ok_or_else(|| JsValue::from_str("No frames found"))?
        .map_err(|e| JsValue::from_str(&format!("Decode error: {e}")))?;

    let (width, height) = frame_0.buffer().dimensions();

    // Compute resize dimensions
    let scale_factor = (max_size as f64 / width as f64)
        .min(max_size as f64 / height as f64)
        .min(1.0);

    let new_width = (width as f64 * scale_factor).round() as u32;
    let new_height = (height as f64 * scale_factor).round() as u32;

    // Sanitize input FPS
    let effective_fps = target_fps.max(1);

    let mut processed = Vec::new(); // Output frames
    let mut rolling_buffer: Vec<(DynamicImage, u32)> = Vec::new(); // store (image, cumulative_time)
    let mut next_sample_frame_idx = 0; // Timestamp of when we should sample the next frame
    let mut total_ms = 0u32; // Total offset from start in ms

    let filter_type = match resampling_filter.as_str() {
        "catrom" => FilterType::CatmullRom,
        "gaussian" => FilterType::Gaussian,
        "lanczos3" => FilterType::Lanczos3,
        "nearest" => FilterType::Nearest,
        "triangle" => FilterType::Triangle,
        _ => return Err(JsValue::from_str("Invalid resampling filter type")),
    };

    // Streaming loop
    for (i, frame) in get_frames(image_data, &image_type)?.enumerate() {
        // % of prime number cuz i like seeing it go through every number :D
        report_progress(
            0.10, // We don't know the exact progress
            &format!(
                "Streaming frame {} ({})",
                i,
                format_duration(total_ms as u64)
            ),
        );

        let frame = frame.map_err(|e| JsValue::from_str(&format!("Decode error: {e}")))?;
        let (ms, _) = frame.delay().numer_denom_ms();
        let delay = if ms == 0 { DEFAULT_FRAME_DELAY_MS } else { ms };

        total_ms += delay;

        // Decode + resize
        let mut img = DynamicImage::ImageRgba8(frame.into_buffer());
        if grayscale_bits > 0 {
            img = DynamicImage::ImageLuma8(img.to_luma8());
        }
        let img = img.resize(new_width, new_height, filter_type);

        rolling_buffer.push((img, total_ms)); // Add to rolling buffer

        // Keep buffer bounded (e.g., last 0.5 seconds)
        while let Some((_, t)) = rolling_buffer.first() {
            if total_ms - *t > 500 {
                rolling_buffer.remove(0);
            } else {
                break;
            }
        }

        // Sample frames as long as we passed the next sample timestamp
        while ((next_sample_frame_idx as f64 * MS_PER_SECOND / effective_fps as f64) as u32)
            < total_ms
        {
            let mut best_img = None;
            let mut best_dt = u32::MAX;

            for (img, t) in &rolling_buffer {
                let dt = total_ms.abs_diff(*t);
                if dt < best_dt {
                    best_dt = dt;
                    best_img = Some(img.clone());
                }
            }

            if let Some(img) = best_img {
                processed.push(img);
            }
            next_sample_frame_idx += 1
        }
    }

    // Guarantee at least one frame
    if processed.is_empty() {
        if let Some((img, _)) = rolling_buffer.last() {
            processed.push(img.clone());
        }
    }

    Ok((processed, effective_fps))
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
pub fn rgb_to_int(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
