use wasm_bindgen::prelude::*;
mod blueprint;
mod blueprint_encoder;
mod constants;
mod image_processing;
mod image_utils;
mod macros;
mod models;
mod progress;
mod signals;
mod streaming_writer;

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
    send_chunk: &js_sys::Function,
) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let args: models::BlueprintArgs = serde_wasm_bindgen::from_value(options)?;

    // Process the image to extract frames and determine the effective FPS.
    let frame_data = image_processing::FrameData::new(image_data, args.clone())?;
    let mut encoder = blueprint_encoder::BlueprintEncoder::new_from_frame_data(frame_data, &args)?;

    while let Some(chunk) = encoder.next_chunk()? {
        let chunk = js_sys::Uint8Array::from(&chunk[..]);
        let promise = send_chunk.call1(&JsValue::NULL, &JsValue::from(chunk))?;
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;
    }

    use js_sys::{Object, Reflect};
    let obj = Object::new();

    Reflect::set(
        &obj,
        &JsValue::from_str("label"),
        &JsValue::from_str(&args.name.to_string()),
    )?;

    Ok(JsValue::from(obj))
}
