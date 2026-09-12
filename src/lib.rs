use crate::blueprint::BlueprintArgs;
use wasm_bindgen::prelude::*;
mod blueprint;
mod constants;
mod image_processing;
mod image_utils;
mod macros;
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
    options: JsValue,
    image_data: &[u8],
    on_group_ready: &js_sys::Function,
    send_chunk: &js_sys::Function,
) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let args: BlueprintArgs = serde_wasm_bindgen::from_value(options)?;

    // Process the image to extract frames and determine the effective FPS.
    let mut frame_data = image_processing::FrameData::new(image_data, &args)?;

    let blueprint = blueprint::generate_blueprint(&mut frame_data, &args)?;
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

    Reflect::set(
        &obj,
        &JsValue::from_str("label"),
        &JsValue::from_str(&args.name.to_string()),
    )?;

    on_group_ready.call1(&JsValue::NULL, &obj)?;

    Ok(())
}
