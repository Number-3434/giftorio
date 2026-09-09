use wasm_bindgen::prelude::*;

mod blueprint;
mod constants;
mod image_processing;
mod models;
mod progress;
mod signals;

#[derive(serde::Deserialize)]
pub struct BlueprintOptions {
    pub name: String,
    #[serde(rename = "imageType")]
    pub image_type: String,
    #[serde(rename = "useDLC")]
    pub use_dlc: bool,
    #[serde(rename = "targetFps")]
    pub target_fps: u32,
    #[serde(rename = "maxSize")]
    pub max_size: u32,
    #[serde(rename = "substationQuality")]
    pub substation_quality: String,
    #[serde(rename = "grayscaleBits")]
    pub grayscale_bits: u32,
    #[serde(rename = "resamplingFilter")]
    pub resampling_filter: String,
    #[serde(rename = "useGreenLampWires")]
    pub use_green_lamp_wires: bool,
    #[serde(rename = "useHorizontalLampWires")]
    pub use_horizontal_lamp_wires: bool,
}

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
    let options: BlueprintOptions = serde_wasm_bindgen::from_value(options)?;

    // Process the image to extract frames and determine the effective FPS.
    let (frames, fps) = image_processing::process_image(
        image_data,
        options.image_type,
        options.max_size,
        options.target_fps,
        options.grayscale_bits,
        options.resampling_filter,
    )?;
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames sampled!"));
    }

    let blueprint = blueprint::generate_blueprint(
        options.name.to_string(),
        fps,
        frames,
        options.use_dlc,
        options.grayscale_bits,
        options.substation_quality,
        options.use_green_lamp_wires,
        options.use_horizontal_lamp_wires,
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

    Reflect::set(
        &obj,
        &JsValue::from_str("label"),
        &JsValue::from_str(&options.name.to_string()),
    )?;

    on_group_ready.call1(&JsValue::NULL, &obj)?;

    Ok(())
}
