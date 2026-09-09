use wasm_bindgen::prelude::*;

mod blueprint;
mod constants;
mod image_processing;
mod models;
mod progress;
mod signals;

/// Public entry point for WebAssembly.
///
/// # Parameters
///
/// - `image_data`: Byte array containing the GIF/WebP data.
/// - `image_type`: Type of the image ("gif" or "webp").
/// - `use_dlc`: Whether to use additional DLC signals.
/// - `target_fps`: Desired frames per second (won't exceed original FPS).
/// - `max_size`: Maximum dimension (width/height) for downscaling.
/// - `substation_quality`: Quality of substations to use.
/// - `grayscale_bits`: Number of bits for grayscale conversion (0 means full color).
///
/// # Returns
///
/// A Factorio blueprint string on success.
#[wasm_bindgen]
pub async fn run_blueprint(
    name: &str,
    image_data: &[u8],
    image_type: &str,
    use_dlc: bool,
    target_fps: u32,
    max_size: u32,
    substation_quality: String,
    grayscale_bits: u32,
    resampling_filter: String,
    on_group_ready: &js_sys::Function,
    send_chunk: &js_sys::Function,
) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    // Process the image to extract frames and determine the effective FPS.
    let (frames, fps) = image_processing::process_image(
        image_data,
        image_type,
        max_size,
        target_fps,
        grayscale_bits,
        resampling_filter,
    )?;
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames sampled!"));
    }

    let blueprint = blueprint::generate_blueprint(
        name.to_string(),
        fps,
        frames,
        use_dlc,
        grayscale_bits,
        substation_quality,
    )?;
    let mut encoder = blueprint::BlueprintEncoder::new(blueprint);
    let mut buf = Vec::new();

    while !encoder.done() {
        buf.clear();
        encoder.next_chunk(&mut buf)?;

        let chunk = js_sys::Uint8Array::from(&buf[..]);
        let promise = send_chunk.call1(&JsValue::NULL, &chunk)?;

        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;
    }

    use js_sys::{Object, Reflect};
    let obj = Object::new();

    Reflect::set(&obj, &JsValue::from_str("label"), &JsValue::from_str(name))?;

    on_group_ready.call1(&JsValue::NULL, &obj)?;

    Ok(())
}
