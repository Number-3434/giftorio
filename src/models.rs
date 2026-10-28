use glam::UVec2;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::sync::Arc;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Tsify, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_inline_default]
pub struct BlueprintArgs {
    #[serde(default)]
    #[tsify(type = "[number, number] | null | undefined")]
    pub custom_dimensions: Option<UVec2>,

    #[serde_inline_default(2)]
    pub display_margin_y: u32,

    #[serde(default)]
    pub dry_run: bool,

    #[serde_inline_default(FlippedAxes::None)]
    pub flipped_axes: FlippedAxes,

    #[serde(default)]
    pub grayscale_bits: u32,

    #[serde(default)]
    pub image_filters: Vec<ImageFilter>,
    pub image_args: BlueprintImageArgs,

    #[serde_inline_default(ImageRotation::Deg0)]
    pub image_rotation: ImageRotation,

    #[serde_inline_default(true)]
    pub include_power_poles: bool,

    #[serde(default)]
    pub max_group_size: Option<u32>,

    #[serde_inline_default(Mode::Other)]
    pub mode: Mode,

    #[serde(default)]
    pub name: Option<String>,

    #[serde_inline_default(OutputFormat::Unknown)]
    pub output_format: OutputFormat,

    #[serde(default)]
    pub power_pole_name: Option<Arc<str>>,

    #[serde(default)]
    pub power_pole_quality: Option<Arc<str>>,

    #[serde_inline_default(true)]
    pub prefer_green_wires: bool,

    #[serde_inline_default(true)]
    pub prefer_horizontal_wires: bool,

    #[serde(default)]
    pub resampling_filter: Option<ResamplingFilter>,

    #[serde(default)]
    pub signal_compression: Option<SignalCompression>,

    #[serde(default)]
    pub signal_sorting: Option<SignalSorting>,

    #[serde(default)]
    pub timing_signals: Option<TimingSignals>,
}

#[derive(Tsify, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintImageArgs {
    #[tsify(type = "[number, number] | null | undefined")]
    pub dimensions: Option<UVec2>,
    pub duration: Option<f64>,

    #[serde(default)]
    pub max_size: Option<u32>,
    pub mime_type: String,
    pub target_fps: u32,
}

#[derive(Tsify, Debug, Serialize, Deserialize)]
pub struct TimingSignals {
    pub f: Arc<crate::blueprint::models::SignalID>,
    pub p: Arc<crate::blueprint::models::SignalID>,
    pub s: Arc<crate::blueprint::models::SignalID>,
    pub t: Arc<crate::blueprint::models::SignalID>,
}

#[derive(Tsify, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageFilter {
    Blur(f32),
    Brightness(i32),
    Contrast(f32),
    HueRotate(i32),
    Unsharpen(f32, i32),
}

#[derive(Tsify, Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageRotation {
    #[default]
    #[serde(rename = "none")]
    Deg0 = 0,
    Deg90 = 90,
    Deg180 = 180,
    Deg270 = 270,
}

#[derive(Tsify, Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlippedAxes {
    #[default]
    None = 0,
    X = 1,
    Y = 2,
    Both = 3,
}

#[derive(Tsify, Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Other,
    Full,
    Lamps,
    LampGrid,

    #[serde(rename = "staticImage")]
    Static {
        #[serde(rename = "useConstantCombinators")]
        const_cb: bool,

        #[serde(rename = "useCombinators")]
        combs: bool,

        #[serde(rename = "useTimer")]
        timer: bool,
    },
}

#[derive(Tsify, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalSorting {
    None,
    Compression,
    Json,
}

#[derive(Tsify, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResamplingFilter {
    Catrom,
    Gaussian,
    Lanczos3,
    Nearest,
    Thumbnail,
    Triangle,
}

#[derive(Tsify, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalCompression {
    Delta,
    Temporal { window: u32 },
}

#[derive(Tsify, Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    #[default]
    Unknown,
    Blueprint,
    Json,
    RawCompressed,
}

#[derive(Tsify, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsVideoFrameData {
    #[serde(default)]
    pub is_finished: bool,

    #[serde(default)]
    pub timestamp: Option<f64>,

    #[serde(default)]
    pub duration: Option<f64>,
}

#[derive(Tsify, Deserialize)]
pub struct EncodeGifArgs {
    #[tsify(type = "[number, number]")]
    pub dim: UVec2,
    pub speed: u8,
}
